using System.Diagnostics;
using Silk.NET.OpenGL;

namespace Zax.App;

/// <summary>
/// The bound AMD-052 REQ-LOOP-077 puts on the rendered frames the GPU may keep waiting for
/// display: at <c>video.max_queued_frames</c> = 1 the program waits, before it starts drawing a
/// frame, for the previous one to be finished by the GPU (lowest latency); at 2 it may run one
/// frame ahead (smoother).
/// </summary>
/// <remarks>
/// <para>
/// The instrument is the synchronisation object of GL 3.2 core, which REQ-SCN-056 allows without
/// any extension: a fence placed after the last command of frame <i>n</i>
/// (<c>glFenceSync</c>) and queried with <c>glClientWaitSync</c>. The queue keeps the fences of
/// the frames the GPU has not confirmed yet, and <see cref="BeginFrame"/> blocks until no more
/// than <c>Depth − 1</c> of them are left — so at depth 1 the fence of frame <i>n</i> is
/// signalled before the first command of frame <i>n + 1</i>, which is the acceptance criterion
/// word for word, and at depth 2 nothing is required of it.
/// </para>
/// <para>
/// This is presentation, hence the desktop frontend: the setting is <b>moot in libretro</b>,
/// where the swap chain and the presentation belong to the host (REQ-PERS-086) and the engine
/// draws into a target the host supplies (REQ-SCN-057).
/// </para>
/// <para>
/// The wait is deliberately placed <b>after</b> the emulated frames of the tick have run and
/// before the first drawing command, so that it cannot shift the cadence of the emulated frames
/// (REQ-LOOP-010): the frame clock is absolute — frame <i>n</i> is due at <c>t0 + n / f</c> —
/// so a wait inside a rendered frame is not a delay in the sense of REQ-LOOP-074 either, it
/// only consumes some of the slack before the next frame is due.
/// </para>
/// </remarks>
public sealed class GpuFrameQueue : IDisposable
{
    /// <summary>The smallest depth the setting offers, and the lowest latency.</summary>
    public const int MinDepth = 1;

    /// <summary>The largest depth the setting offers, which is the default.</summary>
    public const int MaxDepth = 2;

    /// <summary>
    /// How long a single <c>glClientWaitSync</c> call waits, in nanoseconds. A frame that takes
    /// longer than a tenth of a second is waited for again; the bound only exists so that a
    /// driver that loses a fence cannot hang the program for ever.
    /// </summary>
    private const ulong WaitSliceNanoseconds = 100UL * 1000UL * 1000UL;

    /// <summary>How many times a lost fence is waited for before it is given up on.</summary>
    private const int WaitSlices = 20;

    /// <summary><c>GL_SYNC_FLUSH_COMMANDS_BIT</c>, so a fence cannot wait on an unflushed queue.</summary>
    private const uint FlushCommandsBit = 0x0000_0001u;

    private readonly GL _gl;
    private readonly Func<double> _clock;

    // At most MaxDepth fences are ever outstanding, oldest first: the frame the GPU is drawing
    // and, at depth 2, the one it has already been handed.
    private readonly Queue<(nint Sync, double Started)> _fences = new();

    private double _frameStarted;
    private bool _disposed;

    /// <summary>Creates a queue on the context of <paramref name="gl"/>.</summary>
    /// <param name="gl">The GL API of the current context.</param>
    /// <param name="clock">
    /// Monotonic clock in seconds, for <see cref="LastLatencySeconds"/>; the default is a
    /// <see cref="Stopwatch"/>.
    /// </param>
    public GpuFrameQueue(GL gl, Func<double>? clock = null)
    {
        ArgumentNullException.ThrowIfNull(gl);

        if (clock is null)
        {
            Stopwatch watch = Stopwatch.StartNew();
            clock = () => watch.Elapsed.TotalSeconds;
        }

        _gl = gl;
        _clock = clock;
    }

    /// <summary>
    /// The depth in force, that is <c>video.max_queued_frames</c> clamped to the domain of
    /// REQ-LOOP-077.
    /// </summary>
    public int Depth { get; set; } = MaxDepth;

    /// <summary>How many frames the GPU has not confirmed finished yet.</summary>
    public int Outstanding => _fences.Count;

    /// <summary>How many times <see cref="BeginFrame"/> has blocked on a fence.</summary>
    public long Waits { get; private set; }

    /// <summary>
    /// The latency of the last frame the GPU confirmed: the real time between the moment the
    /// emulation of that frame was over — the start of its rendering — and the moment the
    /// program knew the frame was drawn and could be presented. This is the quantity the third
    /// acceptance criterion of REQ-LOOP-077 compares between the two depths; it is 0 until a
    /// frame has been confirmed.
    /// </summary>
    public double LastLatencySeconds { get; private set; }

    /// <summary>
    /// Called once per rendered frame, after the emulated frames of the tick and before the
    /// first drawing command. Blocks until at most <c>Depth − 1</c> frames are still in the
    /// GPU's hands.
    /// </summary>
    /// <param name="depth">The value of <c>video.max_queued_frames</c>.</param>
    public void BeginFrame(int depth)
    {
        Depth = Math.Clamp(depth, MinDepth, MaxDepth);
        _frameStarted = _clock();

        while (_fences.Count >= Depth)
        {
            RetireOldest();
        }
    }

    /// <summary>
    /// Called once per rendered frame, after its last drawing command. Places the fence the next
    /// <see cref="BeginFrame"/> waits on.
    /// </summary>
    public unsafe void EndFrame()
    {
        nint sync = (nint)(void*)_gl.FenceSync(SyncCondition.SyncGpuCommandsComplete, 0u);
        if (sync == 0)
        {
            // A driver that refuses a fence leaves the queue unbounded, which is the behaviour
            // of depth 2 on every frame: nothing else degrades.
            return;
        }

        _fences.Enqueue((sync, _frameStarted));
    }

    /// <summary>
    /// Waits for every frame still in flight and drops their fences — what closing the window
    /// does, and what a test does before it reads a measurement back.
    /// </summary>
    public void Flush()
    {
        while (_fences.Count > 0)
        {
            RetireOldest();
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Flush();
    }

    private void RetireOldest()
    {
        (nint sync, double started) = _fences.Dequeue();
        Waits++;

        for (int slice = 0; slice < WaitSlices; slice++)
        {
            GLEnum result = _gl.ClientWaitSync(
                sync,
                FlushCommandsBit,
                WaitSliceNanoseconds);

            if (result is GLEnum.AlreadySignaled or GLEnum.ConditionSatisfied or GLEnum.WaitFailed)
            {
                break;
            }
        }

        LastLatencySeconds = _clock() - started;
        _gl.DeleteSync(sync);
    }
}
