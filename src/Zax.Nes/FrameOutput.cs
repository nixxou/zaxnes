namespace Zax.Nes;

/// <summary>
/// What one emulated frame hands to the host (AMD-041 REQ-LOOP-065, REQ-LOOP-068).
/// </summary>
/// <remarks>
/// <para>
/// Run-ahead emulates the same frame more than once: the <b>real</b> frame, whose state persists,
/// then the frames in advance, of which only the last one — the <b>displayed</b> frame — is seen
/// and heard. The frames that are neither displayed nor heard still have to be emulated
/// <em>exactly</em>: what these flags switch off is the output path alone, never anything the
/// emulation reads back. Timing, the sprite 0 flag, the IRQs, the tile records and the Zapper's
/// light detection are unchanged whatever the value.
/// </para>
/// <para>
/// A frame with every flag set is an ordinary frame: that is what <see cref="NesCore.RunFrame(ushort)"/>
/// runs and what a host with no run-ahead ever asks for.
/// </para>
/// </remarks>
[Flags]
public enum FrameOutput
{
    /// <summary>
    /// Nothing leaves the core: no video callback, no audio sample, no pixel image. A hidden
    /// run-ahead frame on a machine with no Zapper (REQ-LOOP-068).
    /// </summary>
    None = 0,

    /// <summary>
    /// The video callback of REQ-CORE-020 is invoked with the frame batch, which is what drives
    /// the 2D → 3D conversion and the Lua frame hooks (REQ-LOOP-067).
    /// </summary>
    Video = 1,

    /// <summary>
    /// The samples the APU produced during the frame reach the output queue (REQ-CORE-071). With
    /// the flag clear the resampler still runs — its state is part of the machine — but its
    /// output is dropped instead of being queued, so the frame is silent.
    /// </summary>
    Audio = 2,

    /// <summary>
    /// The PPU composes its 256 × 240 indexed image (REQ-CORE-048). Required of every emulated
    /// frame when a port carries a Zapper (REQ-LOOP-068); optional otherwise, the sprite 0 flag
    /// and the tile records being produced either way.
    /// </summary>
    Pixels = 4,

    /// <summary>Everything: an ordinary frame.</summary>
    All = Video | Audio | Pixels,
}
