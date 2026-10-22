using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 2, UxROM (REQ-CORE-100).
/// </summary>
/// <remarks>
/// A write anywhere in $8000-$FFFF selects the 16 KiB bank at $8000; the last bank is fixed
/// at $C000 and CHR is not switched at all (nesdev, "UxROM"). The board has no PRG-RAM, but
/// the 8 KiB the header always grants are left in place: no game of the family reads
/// $6000-$7FFF, and REQ-CORE-012 asks for the window to exist unless the mapper disables it.
/// </remarks>
internal sealed class UxRom : BankedCartridge
{
    private byte _bank;

    internal UxRom(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 2;

    public override void Reset()
    {
        base.Reset();
        _bank = 0;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _bank = value;
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        MapPrg16k(0, _bank);
        MapPrg16k(2, -1);
        MapChr8k(0);
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteByte(_bank);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _bank = reader.ReadByte();
    }
}
