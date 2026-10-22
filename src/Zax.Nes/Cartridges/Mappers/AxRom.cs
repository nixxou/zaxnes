using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 7, AxROM (REQ-CORE-100).
/// </summary>
/// <remarks>
/// A write anywhere in $8000-$FFFF selects a 32 KiB PRG bank with bits 0 to 3 and, with
/// bit 4, which of the two console nametables the whole screen shows — the board has no
/// mirroring at all, only single-screen selection (nesdev, "AxROM"). CHR is 8 KiB of RAM.
/// </remarks>
internal sealed class AxRom : BankedCartridge
{
    private byte _register;

    internal AxRom(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 7;

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
        MapPrg32k(_register & 0x0F);
        MapChr8k(0);
        Mirroring = (_register & 0x10) != 0
            ? NametableMirroring.SingleScreenB
            : NametableMirroring.SingleScreenA;
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
