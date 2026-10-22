using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 225, the 52-in-1 and 64-in-1 multicarts (REQ-CORE-100).
/// </summary>
/// <remarks>
/// Everything is carried by the <em>address</em> of a write to $8000-$FFFF (nesdev,
/// "INES Mapper 225"): bits 0 to 5 the 8 KiB CHR bank, bits 6 to 11 the 16 KiB PRG bank,
/// bit 12 the PRG size (0: 32 KiB, 1: 16 KiB mirrored), bit 13 the nametable layout and
/// bit 14 the 512 KiB block, which the larger carts use as the top bank bit. The four bytes
/// of scratch RAM some of these boards expose at $5800-$5FFF are not emulated: the menus of
/// the dumps in the required set do not use them.
/// </remarks>
internal sealed class Multicart225 : BankedCartridge
{
    private int _latch;

    internal Multicart225(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 225;

    protected override bool PrgRamEnabled => false;

    public override void Reset()
    {
        base.Reset();
        _latch = 0x8000;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _latch = address;
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        int block = (_latch >> 14) & 0x01;
        int chr = (_latch & 0x3F) | (block << 6);
        int prg = ((_latch >> 6) & 0x3F) | (block << 6);

        if ((_latch & 0x1000) != 0)
        {
            MapPrg16k(0, prg);
            MapPrg16k(2, prg);
        }
        else
        {
            MapPrg32k(prg >> 1);
        }

        MapChr8k(chr);
        Mirroring = (_latch & 0x2000) != 0
            ? NametableMirroring.Horizontal
            : NametableMirroring.Vertical;
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteInt32(_latch);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _latch = reader.ReadInt32();
    }
}
