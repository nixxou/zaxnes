namespace Zax.Nes.State;

/// <summary>
/// The in-memory state of a core (AMD-041 REQ-CORE-121): the buffer
/// <see cref="NesCore.CaptureState"/> fills and <see cref="NesCore.RestoreState"/> reads back.
/// </summary>
/// <remarks>
/// <para>
/// This is <b>not</b> the portable savestate of REQ-CORE-090: it carries no magic number, no
/// format version, no core name and no ROM image, it is never written to a file and it is only
/// ever read back by the very instance that wrote it, in the same session. That is what the notes
/// of REQ-LOOP-065 allow, and it is what makes a capture and a restore at every real frame
/// affordable: the same object is reused, so steady-state run-ahead allocates nothing.
/// </para>
/// <para>
/// The size is bounded by <see cref="NesCore.MaxCoreStateSize"/>, as REQ-CORE-121 requires.
/// </para>
/// </remarks>
public sealed class CoreState
{
    private readonly StateWriter _writer;

    /// <summary>Creates an empty state.</summary>
    /// <param name="capacity">
    /// Initial capacity in bytes; the buffer grows by itself if it is too small.
    /// <see cref="NesCore.MaxCoreStateSize"/> is the value that makes it grow exactly never.
    /// </param>
    public CoreState(int capacity = 128 * 1024)
        => _writer = new StateWriter(capacity);

    /// <summary>Bytes the captured state occupies, 0 when nothing was captured yet.</summary>
    public int Length => _writer.Length;

    /// <summary>Whether no state has been captured into this buffer.</summary>
    public bool IsEmpty => _writer.Length == 0;

    /// <summary>The captured bytes, for a test that wants to compare two captures.</summary>
    public ReadOnlySpan<byte> AsSpan() => _writer.AsSpan();

    /// <summary>Empties the buffer and hands the writer to <see cref="NesCore.CaptureState"/>.</summary>
    internal StateWriter Begin()
    {
        _writer.Reset();
        return _writer;
    }

    /// <summary>A reader over the captured bytes, for <see cref="NesCore.RestoreState"/>.</summary>
    internal StateReader Read() => new(_writer.AsMemory(), 0);
}
