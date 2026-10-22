using Silk.NET.OpenAL;

namespace Zax.Platform;

/// <summary>
/// The audio device of SPEC-11 §3.1, through OpenAL.
/// </summary>
/// <remarks>
/// <para>
/// 48 000 Hz if the device accepts it and blocks of 512 frames (REQ-AUD-001, REQ-AUD-003). The
/// core produces its samples directly at the output rate, so no resampling happens here
/// (SPEC-02 REQ-CORE-070).
/// </para>
/// <para>
/// The device is deliberately dumb: it asks a filler — the mixer of SPEC-11, which owns the
/// gains, the fades, the menu music, the interface sounds and the all-or-nothing rule of
/// REQ-AUD-003 — for interleaved stereo blocks and queues them. The emulation is never clocked
/// by the audio (REQ-AUD-004).
/// </para>
/// </remarks>
public sealed unsafe class AudioOutput : IDisposable
{
    /// <summary>Preferred output rate, 48 000 Hz (REQ-AUD-001).</summary>
    public const int PreferredSampleRate = 48_000;

    /// <summary>Frames per output block, about 10.7 ms at 48 kHz (REQ-AUD-003).</summary>
    public const int BlockFrames = 512;

    /// <summary>Channels of the output: stereo, the console signal duplicated (REQ-AUD-002).</summary>
    public const int Channels = 2;

    /// <summary>Blocks kept queued; 4 × 512 frames is about 43 ms of latency.</summary>
    public const int BlockCount = 4;

    private readonly short[] _block = new short[BlockFrames * Channels];
    private readonly float[] _samples = new float[BlockFrames * Channels];
    private readonly Queue<uint> _free = [];

    private ALContext? _alc;
    private AL? _al;
    private Device* _device;
    private Context* _context;
    private uint _source;
    private uint[] _buffers = [];
    private bool _disposed;

    /// <summary>Whether a device and a context are currently open.</summary>
    public bool IsOpen => _al is not null;

    /// <summary>The rate the device was opened at, or 0 when it is closed.</summary>
    public int SampleRate { get; private set; }

    /// <summary>Number of blocks handed to the device since it was opened.</summary>
    public long Blocks { get; private set; }

    /// <summary>Why the last <see cref="TryOpen"/> failed, or null.</summary>
    public string? LastError { get; private set; }

    /// <summary>
    /// Opens the default device. Returns false and records <see cref="LastError"/> rather than
    /// throwing, so that a machine without a sound device can still run the renderer.
    /// </summary>
    public bool TryOpen()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        if (IsOpen)
        {
            return true;
        }

        try
        {
            _alc = ALContext.GetApi();
            _al = AL.GetApi();
            _device = _alc.OpenDevice(string.Empty);
            if (_device is null)
            {
                throw new InvalidOperationException("OpenAL could not open the default device.");
            }

            _context = _alc.CreateContext(_device, null);
            if (_context is null)
            {
                throw new InvalidOperationException("OpenAL could not create a context.");
            }

            _alc.MakeContextCurrent(_context);

            _source = _al.GenSource();
            _buffers = new uint[BlockCount];
            for (int i = 0; i < BlockCount; i++)
            {
                _buffers[i] = _al.GenBuffer();
                _free.Enqueue(_buffers[i]);
            }

            _al.SetSourceProperty(_source, SourceFloat.Gain, 1f);
            SampleRate = PreferredSampleRate;
            LastError = null;
            return true;
        }
        catch (Exception ex)
        {
            LastError = ex.Message;
            Close();
            return false;
        }
    }

    /// <summary>
    /// Keeps the device fed: as long as a buffer is free, asks <paramref name="fill"/> for one
    /// interleaved stereo block and queues it. Called once per rendered frame; nothing happens
    /// when the output is closed.
    /// </summary>
    /// <param name="fill">
    /// Fills the whole buffer — <c>512 × 2</c> values — with interleaved stereo audio. The
    /// mixer always produces a complete block, using silence where the core has nothing
    /// (REQ-AUD-003).
    /// </param>
    /// <returns>How many blocks were queued.</returns>
    public int Pump(Action<float[]> fill)
    {
        ArgumentNullException.ThrowIfNull(fill);
        if (_al is null)
        {
            return 0;
        }

        Recycle();

        int queued = 0;
        while (_free.Count > 0)
        {
            fill(_samples);

            for (int i = 0; i < _samples.Length; i++)
            {
                float value = Math.Clamp(_samples[i], -1f, 1f);
                _block[i] = (short)MathF.Round(value * 32767f);
            }

            uint buffer = _free.Dequeue();
            fixed (short* data = _block)
            {
                _al.BufferData(
                    buffer,
                    BufferFormat.Stereo16,
                    data,
                    _block.Length * sizeof(short),
                    SampleRate);
            }

            _al.SourceQueueBuffers(_source, 1, &buffer);
            Blocks++;
            queued++;
        }

        _al.GetSourceProperty(_source, GetSourceInteger.SourceState, out int state);
        if ((SourceState)state != SourceState.Playing)
        {
            _al.GetSourceProperty(_source, GetSourceInteger.BuffersQueued, out int pending);
            if (pending > 0)
            {
                _al.SourcePlay(_source);
            }
        }

        return queued;
    }

    /// <summary>
    /// Stops the source and drops everything queued: what a game load, a state load or a reset
    /// does (REQ-AUD-010).
    /// </summary>
    public void Flush()
    {
        if (_al is null)
        {
            return;
        }

        _al.SourceStop(_source);
        Recycle();
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Close();
    }

    private void Recycle()
    {
        if (_al is null)
        {
            return;
        }

        _al.GetSourceProperty(_source, GetSourceInteger.BuffersProcessed, out int processed);
        for (int i = 0; i < processed; i++)
        {
            uint buffer = 0;
            _al.SourceUnqueueBuffers(_source, 1, &buffer);
            if (buffer != 0)
            {
                _free.Enqueue(buffer);
            }
        }
    }

    private void Close()
    {
        if (_al is not null)
        {
            if (_source != 0)
            {
                _al.SourceStop(_source);
                _al.DeleteSource(_source);
                _source = 0;
            }

            foreach (uint buffer in _buffers)
            {
                if (buffer != 0)
                {
                    _al.DeleteBuffer(buffer);
                }
            }

            _buffers = [];
            _free.Clear();
        }

        if (_alc is not null)
        {
            if (_context is not null)
            {
                _alc.MakeContextCurrent(null);
                _alc.DestroyContext(_context);
                _context = null;
            }

            if (_device is not null)
            {
                _alc.CloseDevice(_device);
                _device = null;
            }
        }

        _al?.Dispose();
        _al = null;
        _alc?.Dispose();
        _alc = null;
        SampleRate = 0;
    }
}
