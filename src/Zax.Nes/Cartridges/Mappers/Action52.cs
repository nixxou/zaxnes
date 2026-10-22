using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 228, Action 52 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// The board spreads its PRG over several chips and splits the CHR bank between the address
/// and the data of the write (nesdev, "INES Mapper 228"). From the address of a write to
/// $8000-$FFFF: bits 0 to 3 are the upper four bits of the 8 KiB CHR bank, bit 5 the PRG size
/// (0: 32 KiB, 1: 16 KiB mirrored), bits 6 to 10 the PRG bank, bits 11 to 12 the chip select
/// and bit 13 the nametable layout; bits 0 to 1 of the value complete the CHR bank. Chip
/// select 3 repeats chip 2, as on the board.
/// </remarks>
internal sealed class Action52 : BankedCartridge
{
    private int _latch;
    private byte _value;

    internal Action52(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 228;

    protected override bool PrgRamEnabled => false;

    public override void Reset()
    {
        base.Reset();
        _latch = 0x8000;
        _value = 0;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _latch = address;
            _value = value;
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        int chip = (_latch >> 11) & 0x03;
        if (chip == 3)
        {
            chip = 2;
        }

        int prg = ((_latch >> 6) & 0x1F) | (chip << 5);

        if ((_latch & 0x20) != 0)
        {
            MapPrg16k(0, prg);
            MapPrg16k(2, prg);
        }
        else
        {
            MapPrg32k(prg >> 1);
        }

        MapChr8k(((_latch & 0x0F) << 2) | (_value & 0x03));
        Mirroring = (_latch & 0x2000) != 0
            ? NametableMirroring.Horizontal
            : NametableMirroring.Vertical;
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteInt32(_latch);
        writer.WriteByte(_value);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _latch = reader.ReadInt32();
        _value = reader.ReadByte();
    }
}
