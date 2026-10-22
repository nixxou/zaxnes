using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 15, "100-in-1 Contra Function 16" (REQ-CORE-100, AMD-021 R2-002).
/// </summary>
/// <remarks>
/// <para>
/// A multicart with no CHR banking. Every write to $8000-$FFFF loads the same register: bits
/// 0 to 5 of the value are the 16 KiB bank <c>B</c>, bit 6 the nametable layout (0 vertical,
/// 1 horizontal) and bit 7 the half-bank bit <c>h</c>; the two low bits of the address choose
/// the PRG layout:
/// </para>
/// <list type="bullet">
/// <item>$8000: 32 KiB — bank B at $8000 and B + 1 at $C000;</item>
/// <item>$8001: bank B at $8000 and the <strong>last</strong> 16 KiB bank of the image at
/// $C000;</item>
/// <item>$8002: the 8 KiB bank <c>2B + h</c> in all four windows;</item>
/// <item>$8003: bank B at $8000 and at $C000.</item>
/// </list>
/// <para>
/// Power-on is layout 0 with B = 0, so the first 32 KiB of the image are visible.
/// </para>
/// </remarks>
internal sealed class Mapper015 : BankedCartridge
{
    private int _mode;
    private byte _value;

    internal Mapper015(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 15;

    public override void Reset()
    {
        base.Reset();
        _mode = 0;
        _value = 0;
        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address >= 0x8000)
        {
            _mode = address & 0x03;
            _value = value;
            UpdateBanks();
        }
    }

    private void UpdateBanks()
    {
        int bank = _value & 0x3F;
        int half = (_value >> 7) & 0x01;

        switch (_mode)
        {
            case 0:
                MapPrg16k(0, bank);
                MapPrg16k(2, bank + 1);
                break;

            case 1:
                MapPrg16k(0, bank);

                // -1 counts back from the end: the last 16 KiB bank of the image.
                MapPrg16k(2, -1);
                break;

            case 2:
                for (int slot = 0; slot < PrgWindows; slot++)
                {
                    MapPrg8k(slot, (bank * 2) + half);
                }

                break;

            default:
                MapPrg16k(0, bank);
                MapPrg16k(2, bank);
                break;
        }

        MapChr8k(0);
        Mirroring = (_value & 0x40) != 0
            ? NametableMirroring.Horizontal
            : NametableMirroring.Vertical;
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteInt32(_mode);
        writer.WriteByte(_value);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _mode = reader.ReadInt32();
        _value = reader.ReadByte();
    }
}
