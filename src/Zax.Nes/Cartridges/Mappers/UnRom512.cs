using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 30, UNROM 512 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// A homebrew board: one register at $8000-$FFFF holding the 16 KiB PRG bank in bits 0 to 4,
/// the 8 KiB CHR-RAM bank in bits 5 to 6 and, when the header announces four-screen, a
/// one-screen nametable select in bit 7; the last PRG bank stays at $C000 (nesdev,
/// "UNROM 512"). The self-flashing feature of the board is not emulated: it writes the
/// cartridge's own flash chip, which has no meaning for a ROM image.
/// </remarks>
internal sealed class UnRom512 : BankedCartridge
{
    private readonly bool _mapperControlledMirroring;
    private byte _register;

    internal UnRom512(RomImage rom)
        : base(rom)
    {
        _mapperControlledMirroring = rom.Header.Mirroring == NametableMirroring.FourScreen;
    }

    public override int MapperNumber => 30;

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
        MapPrg16k(0, _register & 0x1F);
        MapPrg16k(2, -1);
        MapChr8k((_register >> 5) & 0x03);

        if (_mapperControlledMirroring)
        {
            Mirroring = (_register & 0x80) != 0
                ? NametableMirroring.SingleScreenB
                : NametableMirroring.SingleScreenA;
        }
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
