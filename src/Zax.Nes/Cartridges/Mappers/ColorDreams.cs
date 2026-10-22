using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 11, Color Dreams (REQ-CORE-100).
/// </summary>
/// <remarks>
/// One register at $8000-$FFFF: bits 0 to 1 select the 32 KiB PRG bank, bits 4 to 7 the
/// 8 KiB CHR bank (nesdev, "Color Dreams"). Mirroring is fixed by the header.
/// </remarks>
internal sealed class ColorDreams : BankedCartridge
{
    private byte _register;

    internal ColorDreams(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 11;

    public override void Reset()
    {
        base.Reset();
        _register = 0;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _register = value;
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        MapPrg32k(_register & 0x03);
        MapChr8k(_register >> 4);
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteByte(_register);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _register = reader.ReadByte();
    }
}
