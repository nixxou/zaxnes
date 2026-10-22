using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 13, CPROM (REQ-CORE-100).
/// </summary>
/// <remarks>
/// 32 KiB of fixed PRG and 16 KiB of CHR-RAM: the first 4 KiB bank is fixed at $0000 and
/// bits 0 to 1 of the register at $8000-$FFFF pick the 4 KiB bank shown at $1000 (nesdev,
/// "CPROM"). The header of a CPROM dump announces no CHR-ROM, so the 32 KiB of CHR-RAM
/// REQ-CORE-012 grants are larger than the board's 16 KiB; only the first 16 KiB are
/// addressable, which is what the two-bit register can reach.
/// </remarks>
internal sealed class CpRom : BankedCartridge
{
    private byte _bank;

    internal CpRom(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 13;

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
            _bank = (byte)(value & 0x03);
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        MapPrg32k(0);
        MapChr4k(0, 0);
        MapChr4k(4, _bank);
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
