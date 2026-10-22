using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 32, Irem G-101 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// Two switchable 8 KiB PRG banks and eight 1 KiB CHR banks (nesdev, "Irem G-101"):
/// $8000-$8FFF selects the first PRG bank, $9000-$9FFF carries the nametable layout in bit 0
/// and the PRG mode in bit 1, $A000-$AFFF selects the second PRG bank and $B000-$B007 the
/// eight CHR banks. In mode 0 the switchable bank is at $8000 and $C000 holds the
/// second-to-last bank; in mode 1 they trade places. $E000 always holds the last bank.
/// </remarks>
internal sealed class IremG101 : BankedCartridge
{
    private readonly byte[] _chr = new byte[8];
    private byte _prg0;
    private byte _prg1;
    private bool _swapPrg;

    internal IremG101(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 32;

    public override void Reset()
    {
        base.Reset();
        Array.Clear(_chr);
        _prg0 = 0;
        _prg1 = 1;
        _swapPrg = false;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        switch (address & 0xF000)
        {
            case 0x8000:
                _prg0 = value;
                break;

            case 0x9000:
                Mirroring = (value & 0x01) != 0
                    ? NametableMirroring.Horizontal
                    : NametableMirroring.Vertical;
                _swapPrg = (value & 0x02) != 0;
                break;

            case 0xA000:
                _prg1 = value;
                break;

            case 0xB000:
                _chr[address & 0x07] = value;
                break;

            default:
                return;
        }

        UpdateBanks();
    }

    private void UpdateBanks()
    {
        if (_swapPrg)
        {
            MapPrg8k(0, -2);
            MapPrg8k(2, _prg0);
        }
        else
        {
            MapPrg8k(0, _prg0);
            MapPrg8k(2, -2);
        }

        MapPrg8k(1, _prg1);
        MapPrg8k(3, -1);

        for (int slot = 0; slot < 8; slot++)
        {
            MapChr1k(slot, _chr[slot]);
        }
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteRaw(_chr);
        writer.WriteByte(_prg0);
        writer.WriteByte(_prg1);
        writer.WriteBool(_swapPrg);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        reader.ReadInto(_chr);
        _prg0 = reader.ReadByte();
        _prg1 = reader.ReadByte();
        _swapPrg = reader.ReadBool();
    }
}
