using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 3, CNROM (REQ-CORE-100).
/// </summary>
/// <remarks>
/// PRG is fixed; a write anywhere in $8000-$FFFF selects the 8 KiB CHR bank (nesdev,
/// "CNROM"). The original board only decodes two bits, but larger CNROM-like dumps use more,
/// so the whole byte is taken and wrapped to the CHR actually present. Bus conflicts, which
/// the board also has, are not emulated: every game of the family writes a value that matches
/// the byte underneath.
/// </remarks>
internal sealed class CnRom : BankedCartridge
{
    private byte _bank;

    internal CnRom(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 3;

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
        MapPrg32k(0);
        MapChr8k(_bank);
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
