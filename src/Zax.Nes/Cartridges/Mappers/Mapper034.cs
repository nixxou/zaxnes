using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 34, BNROM and NINA-001 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// Two unrelated boards share the number (nesdev, "INES Mapper 034"). BNROM switches 32 KiB
/// of PRG from a write anywhere in $8000-$FFFF and has no CHR banking; NINA-001 has three
/// registers at the very top of the PRG-RAM window — $7FFD for the 32 KiB PRG bank, $7FFE and
/// $7FFF for the two 4 KiB CHR banks. The ranges do not overlap, so both are decoded: a
/// BNROM dump has no CHR-ROM and never writes $7FFD-$7FFF, and a NINA-001 dump never writes
/// $8000-$FFFF. The register writes also land in PRG-RAM, as they do on the board.
/// </remarks>
internal sealed class Mapper034 : BankedCartridge
{
    private byte _prgBank;
    private byte _chrLow;
    private byte _chrHigh;

    internal Mapper034(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 34;

    public override void Reset()
    {
        base.Reset();
        _prgBank = 0;
        _chrLow = 0;
        _chrHigh = 1;
        UpdateBanks();
    }

    public override void CpuWrite(ushort address, byte value)
    {
        switch (address)
        {
            case 0x7FFD:
                _prgBank = value;
                UpdateBanks();
                break;

            case 0x7FFE:
                _chrLow = value;
                UpdateBanks();
                break;

            case 0x7FFF:
                _chrHigh = value;
                UpdateBanks();
                break;

            default:
                break;
        }

        base.CpuWrite(address, value);
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _prgBank = value;
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        MapPrg32k(_prgBank);

        if (Rom.ChrRom.Length > 0)
        {
            MapChr4k(0, _chrLow);
            MapChr4k(4, _chrHigh);
        }
        else
        {
            MapChr8k(0);
        }
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteByte(_prgBank);
        writer.WriteByte(_chrLow);
        writer.WriteByte(_chrHigh);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _prgBank = reader.ReadByte();
        _chrLow = reader.ReadByte();
        _chrHigh = reader.ReadByte();
    }
}
