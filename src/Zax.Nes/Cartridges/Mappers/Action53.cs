using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 28, Action 53 (REQ-CORE-100, AMD-021 R2-001).
/// </summary>
/// <remarks>
/// <para>
/// The homebrew multicart board of the Action 53 collections. A write to $5000-$5FFF selects
/// one of four registers with bits 7 and 0 of the <em>value</em> — which is why the test ROM
/// of the board calls them $00, $01, $80 and $81 — and a write to $8000-$FFFF loads the
/// selected one:
/// </para>
/// <list type="bullet">
/// <item>$00: 8 KiB CHR-RAM bank, 2 bits;</item>
/// <item>$01: inner PRG bank, 4 bits, in 16 KiB units;</item>
/// <item>$80: mode, 6 bits — bits 0 to 1 the nametable layout (0 screen A, 1 screen B,
/// 2 vertical, 3 horizontal), bits 2 to 3 the PRG layout, bits 4 to 5 the size of the game
/// the inner bank switches inside;</item>
/// <item>$81: outer PRG bank, 6 bits, in <strong>32 KiB</strong> units.</item>
/// </list>
/// <para>
/// The outer bank selects a game inside the collection and the inner bank switches within it,
/// so the two are combined through the size mask <c>m = 2^(t+1) − 1</c> (in 16 KiB banks,
/// <c>t</c> = bits 4-5 of the mode): the outer bank <c>E = 2 × outer</c> is aligned down to
/// the game's size and the inner bank <c>I</c> fills the low bits. PRG layout 0 and 1 show a
/// 32 KiB game, layout 2 is UNROM (the fixed window at $8000) and layout 3 is the inverted
/// form.
/// </para>
/// <para>
/// Two details of R2-001 that the plain register description hides: while bit 1 of the mode
/// is clear, a write to the CHR or inner-PRG register also replaces bit 0 of the mode with
/// bit 4 of the value written — that is how a single-screen game flips between the two pages
/// without touching the mode register — and power-on leaves outer = 63, inner = 15, mode = 0,
/// which puts the last 16 KiB of the image at $C000.
/// </para>
/// </remarks>
internal sealed class Action53 : BankedCartridge
{
    private int _select;
    private byte _chrBank;
    private byte _innerPrg;
    private byte _mode;
    private byte _outerPrg;

    internal Action53(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 28;

    protected override bool PrgRamEnabled => false;

    public override void Reset()
    {
        base.Reset();
        _select = 0;
        _chrBank = 0;
        _innerPrg = 15;
        _mode = 0;
        _outerPrg = 63;
        UpdateBanks();
    }

    public override void CpuWrite(ushort address, byte value)
    {
        if (address is >= 0x5000 and <= 0x5FFF)
        {
            _select = ((value & 0x80) >> 6) | (value & 0x01);
            return;
        }

        base.CpuWrite(address, value);
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address < 0x8000)
        {
            return;
        }

        switch (_select)
        {
            case 0:
                _chrBank = (byte)(value & 0x03);
                TakeMirroringBit(value);
                break;

            case 1:
                _innerPrg = (byte)(value & 0x0F);
                TakeMirroringBit(value);
                break;

            case 2:
                _mode = (byte)(value & 0x3F);
                break;

            default:
                _outerPrg = (byte)(value & 0x3F);
                break;
        }

        UpdateBanks();
    }

    /// <summary>
    /// While bit 1 of the mode is clear — the two single-screen layouts — a CHR or inner-PRG
    /// write also sets bit 0 of the mode from bit 4 of the value (R2-001).
    /// </summary>
    private void TakeMirroringBit(byte value)
    {
        if ((_mode & 0x02) == 0)
        {
            _mode = (byte)((_mode & ~0x01) | ((value >> 4) & 0x01));
        }
    }

    private void UpdateBanks()
    {
        MapChr8k(_chrBank);

        Mirroring = (_mode & 0x03) switch
        {
            0 => NametableMirroring.SingleScreenA,
            1 => NametableMirroring.SingleScreenB,
            2 => NametableMirroring.Vertical,
            _ => NametableMirroring.Horizontal,
        };

        // Bits 4 to 5 say how large the selected game is: the mask is the number of 16 KiB
        // banks inside it minus one. Q-059: R2-001 does not say how a bank number past the end
        // of the image is resolved — power-on asks for bank 126 — and BankedCartridge wraps it
        // modulo the number of banks the image holds, as every other board here does.
        int m = (2 << ((_mode >> 4) & 0x03)) - 1;
        int outer = _outerPrg * 2;
        int inner = _innerPrg;

        switch ((_mode >> 2) & 0x03)
        {
            case 0:
            case 1:
                int bank32 = (outer & ~m) | ((inner * 2) & m);
                MapPrg16k(0, bank32);
                MapPrg16k(2, bank32 + 1);
                break;

            case 2:
                MapPrg16k(0, outer);
                MapPrg16k(2, (outer & ~m) | (inner & m));
                break;

            default:
                MapPrg16k(0, (outer & ~m) | (inner & m));
                MapPrg16k(2, outer + 1);
                break;
        }
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteInt32(_select);
        writer.WriteByte(_chrBank);
        writer.WriteByte(_innerPrg);
        writer.WriteByte(_mode);
        writer.WriteByte(_outerPrg);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _select = reader.ReadInt32();
        _chrBank = reader.ReadByte();
        _innerPrg = reader.ReadByte();
        _mode = reader.ReadByte();
        _outerPrg = reader.ReadByte();
    }
}
