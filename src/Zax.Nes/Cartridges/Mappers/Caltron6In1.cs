using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 41, Caltron 6-in-1 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// The outer register is selected by the <em>address</em> written in $6000-$67FF, not by the
/// data: its bits 0 to 1 pick the 32 KiB PRG bank, bit 2 the nametable layout and bits 3 to 4
/// the upper half of the CHR bank. The lower two bits of the CHR bank then come from the
/// <em>data</em> of a write to $8000-$FFFF (nesdev, "INES Mapper 041").
/// </remarks>
internal sealed class Caltron6In1 : BankedCartridge
{
    private byte _outer;
    private byte _chrLow;

    internal Caltron6In1(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 41;

    protected override bool PrgRamEnabled => false;

    public override void Reset()
    {
        base.Reset();
        _outer = 0;
        _chrLow = 0;
        UpdateBanks();
    }

    public override void CpuWrite(ushort address, byte value)
    {
        if (address is >= 0x6000 and <= 0x67FF)
        {
            _outer = (byte)(address & 0x1F);
            UpdateBanks();
            return;
        }

        base.CpuWrite(address, value);
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _chrLow = (byte)(value & 0x03);
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        MapPrg32k(_outer & 0x03);
        MapChr8k((((_outer >> 3) & 0x03) << 2) | _chrLow);
        Mirroring = (_outer & 0x04) != 0
            ? NametableMirroring.Horizontal
            : NametableMirroring.Vertical;
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteByte(_outer);
        writer.WriteByte(_chrLow);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _outer = reader.ReadByte();
        _chrLow = reader.ReadByte();
    }
}
