namespace Zax.Nes.Apu;

/// <summary>
/// A source of cartridge expansion audio, whose output the APU adds to its own
/// (REQ-CORE-073, REQ-CORE-075).
/// </summary>
/// <remarks>
/// Only the Famicom's cartridge connector carries an audio input pin, so no NES cartridge can
/// make such a channel sound; the boards that use it are Famicom boards. REQ-CORE-075 requires
/// the MMC5's own sound (<see cref="Cartridges.Mappers.Mmc5"/>) and leaves the others — VRC6,
/// VRC7, Namco 163, Sunsoft 5B, FDS — optional.
/// </remarks>
public interface IExpansionAudio
{
    /// <summary>
    /// <c>M</c> of REQ-CORE-073: the expansion audio's output in the mixer's integer units,
    /// 0 for a board without expansion audio.
    /// </summary>
    int ExpansionAudioOutput { get; }
}
