using Zax.Nes.State;

namespace Zax.Nes.Cartridges;

/// <summary>
/// Detects the rises of PPU address line A12 that clock an MMC3-style scanline counter
/// (REQ-CORE-114).
/// </summary>
/// <remarks>
/// <para>
/// The MMC3 does not see scanlines: it watches A12 through a low-pass filter and counts the
/// rising edges (nesdev, "MMC3"). With the background patterns at $0000 and the sprite
/// patterns at $1000 — or the other way round — A12 rises exactly once per rendered line,
/// which is what gives the board its scanline counter; a game that puts both layers in the
/// same half of CHR gets no IRQ at all, and one that writes $2006 by hand can clock the
/// counter outside rendering, which is how <c>mmc3_test</c> drives it.
/// </para>
/// <para>
/// The filter is what makes the difference: A12 also dips for the two-dot nametable and
/// attribute fetches of every tile, and those dips must <em>not</em> count. The board needs
/// A12 to stay low for roughly three CPU cycles before it will accept a rise, so the window
/// below is expressed in PPU dots and rejects anything shorter.
/// </para>
/// </remarks>
internal sealed class A12Watcher
{
    /// <summary>
    /// Dots A12 must have stayed low for a rise to count. Three CPU cycles are about nine
    /// dots; ten rejects the four-dot dips of the fetch pipeline and accepts the long lows of
    /// the sprite fetch phase and of the $2006 writes <c>mmc3_test</c> uses.
    /// </summary>
    private const long FilterDots = 10;

    private bool _high;
    private long _lowSince;

    /// <summary>
    /// Feeds the address the PPU put on its bus and reports whether it counts as a rise.
    /// </summary>
    public bool Rose(ushort address, long ppuClock)
    {
        bool high = (address & 0x1000) != 0;
        bool rose = high && !_high && ppuClock - _lowSince >= FilterDots;

        if (!high && _high)
        {
            _lowSince = ppuClock;
        }

        _high = high;
        return rose;
    }

    /// <summary>Returns the watcher to its power-on state.</summary>
    public void Reset()
    {
        _high = false;
        _lowSince = 0;
    }

    /// <summary>Appends the watcher state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        writer.WriteBool(_high);
        writer.WriteInt64(_lowSince);
    }

    /// <summary>Restores the watcher state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        _high = reader.ReadBool();
        _lowSince = reader.ReadInt64();
    }
}
