using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 18, Jaleco SS88006 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// Every bank register is written one nibble at a time, the low nibble at the even address
/// and the high nibble at the odd one (nesdev, "Jaleco SS88006"): $8000-$8003 and
/// $9000-$9001 carry the three switchable 8 KiB PRG banks, $A000-$D003 the eight 1 KiB CHR
/// banks, $E000-$E003 the four nibbles of the 16-bit IRQ latch, $F000 reloads the counter and
/// acknowledges, $F001 enables the IRQ and selects how many bits of the counter are compared,
/// and $F002 holds the nametable layout. The last 8 KiB bank is fixed at $E000.
/// </para>
/// <para>
/// The counter is clocked by the CPU clock and fires when the part of it the width field
/// selects runs out, which is how the board can produce both long and very short delays.
/// </para>
/// </remarks>
internal sealed class JalecoSs88006 : BankedCartridge
{
    private readonly byte[] _prg = new byte[3];
    private readonly byte[] _chr = new byte[8];

    private int _irqLatch;
    private int _irqCounter;
    private int _irqWidth;
    private bool _irqEnabled;
    private bool _irqPending;

    internal JalecoSs88006(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 18;

    public override bool IrqAsserted => _irqPending;

    public override void Reset()
    {
        base.Reset();

        _prg[0] = 0;
        _prg[1] = 1;
        _prg[2] = 2;
        Array.Clear(_chr);

        _irqLatch = 0;
        _irqCounter = 0;
        _irqWidth = 16;
        _irqEnabled = false;
        _irqPending = false;

        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address < 0x8000)
        {
            return;
        }

        int group = (address >> 12) & 0x07;
        int index = address & 0x03;
        var nibble = (byte)(value & 0x0F);

        switch (group)
        {
            // $8000-$9003: the three PRG banks.
            case 0:
                SetNibble(ref _prg[index >> 1], index & 1, nibble);
                UpdateBanks();
                break;

            case 1:
                if (index < 2)
                {
                    SetNibble(ref _prg[2], index, nibble);
                    UpdateBanks();
                }

                break;

            // $A000-$D003: the eight CHR banks.
            case 2:
            case 3:
            case 4:
            case 5:
                SetNibble(ref _chr[((group - 2) * 2) + (index >> 1)], index & 1, nibble);
                UpdateBanks();
                break;

            // $E000-$E003: the four nibbles of the IRQ latch.
            case 6:
                _irqLatch = (_irqLatch & ~(0x0F << (index * 4))) | (nibble << (index * 4));
                break;

            default:
                WriteIrqControl(index, value);
                break;
        }
    }

    private void WriteIrqControl(int index, byte value)
    {
        switch (index)
        {
            case 0:
                _irqCounter = _irqLatch;
                _irqPending = false;
                break;

            case 1:
                _irqEnabled = (value & 0x01) != 0;
                _irqPending = false;
                _irqWidth = (value & 0x08) != 0 ? 4
                    : (value & 0x04) != 0 ? 8
                    : (value & 0x02) != 0 ? 12
                    : 16;
                break;

            case 2:
                Mirroring = (value & 0x03) switch
                {
                    0 => NametableMirroring.Horizontal,
                    1 => NametableMirroring.Vertical,
                    2 => NametableMirroring.SingleScreenA,
                    _ => NametableMirroring.SingleScreenB,
                };
                break;

            default:
                break;
        }
    }

    private static void SetNibble(ref byte register, int half, byte nibble)
        => register = half == 0
            ? (byte)((register & 0xF0) | nibble)
            : (byte)((register & 0x0F) | (nibble << 4));

    private void UpdateBanks()
    {
        MapPrg8k(0, _prg[0]);
        MapPrg8k(1, _prg[1]);
        MapPrg8k(2, _prg[2]);
        MapPrg8k(3, -1);

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            MapChr1k(slot, _chr[slot]);
        }
    }

    /// <inheritdoc />
    protected override void OnCpuCycle()
    {
        if (!_irqEnabled)
        {
            return;
        }

        int mask = _irqWidth == 16 ? 0xFFFF : (1 << _irqWidth) - 1;
        int counted = _irqCounter & mask;

        if (counted == 0)
        {
            _irqPending = true;
            _irqCounter = (_irqCounter & ~mask) | mask;
            return;
        }

        _irqCounter = (_irqCounter & ~mask) | (counted - 1);
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteRaw(_prg);
        writer.WriteRaw(_chr);
        writer.WriteInt32(_irqLatch);
        writer.WriteInt32(_irqCounter);
        writer.WriteInt32(_irqWidth);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_irqPending);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        reader.ReadInto(_prg);
        reader.ReadInto(_chr);
        _irqLatch = reader.ReadInt32();
        _irqCounter = reader.ReadInt32();
        _irqWidth = reader.ReadInt32();
        _irqEnabled = reader.ReadBool();
        _irqPending = reader.ReadBool();
    }
}
