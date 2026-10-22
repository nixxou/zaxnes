using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mappers 21, 22, 23, 25 and 27: Konami VRC2 and VRC4, all wirings (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// The chips are the same; what the iNES number really says is which two address lines the
/// board wired to the chip's two register-select pins (nesdev, "VRC2 and VRC4"):
/// </para>
/// <list type="table">
/// <item><term>21</term><description>VRC4a (A1, A2) and VRC4c (A6, A7)</description></item>
/// <item><term>22</term><description>VRC2a (A1, A0 — swapped), whose CHR banks are in
/// 2 KiB units, so the register value is shifted right by one</description></item>
/// <item><term>23</term><description>VRC4f (A0, A1), VRC4e (A2, A3) and VRC2b</description></item>
/// <item><term>25</term><description>VRC4b (A1, A0) and VRC4d (A3, A2)</description></item>
/// <item><term>27</term><description>a pirate VRC4 with eight-bit CHR registers, wired like
/// mapper 23</description></item>
/// </list>
/// <para>
/// A game only ever uses one wiring, so the candidate lines of a number are ORed together,
/// which is what lets a single class serve every variant. The registers are then: $8000 and
/// $A000 the two switchable 8 KiB PRG banks, $9000 the nametable layout, $9002 the PRG swap
/// mode, $B000-$E003 the low and high halves of the eight 1 KiB CHR banks and $F000-$F003 the
/// IRQ. VRC2 has no IRQ and only one mirroring bit; writes to the registers it lacks are
/// harmless.
/// </para>
/// <para>
/// <strong>The VRC4 IRQ.</strong> An eight-bit counter that counts <em>up</em> and fires when
/// it passes $FF, reloading from the latch. In cycle mode it is clocked by the CPU clock; in
/// scanline mode a prescaler divides the CPU clock by 341/3, which gives one clock per
/// scanline without the board having to see the PPU at all.
/// </para>
/// </remarks>
internal sealed class Vrc2Vrc4 : BankedCartridge
{
    private readonly int[] _chr = new int[8];
    private readonly bool _hasIrq;
    private readonly bool _chrShift;
    private readonly int _chrHighMask;

    private byte _prg0;
    private byte _prg1;
    private bool _swapPrg;

    private byte _irqLatch;
    private int _irqCounter;
    private int _prescaler;
    private bool _irqEnabled;
    private bool _irqEnableAfterAck;
    private bool _irqCycleMode;
    private bool _irqPending;

    internal Vrc2Vrc4(RomImage rom, int mapperNumber)
        : base(rom)
    {
        MapperNumber = mapperNumber;

        // VRC2a (mapper 22) has neither the IRQ nor the 1 KiB CHR granularity.
        _hasIrq = mapperNumber != 22;
        _chrShift = mapperNumber == 22;

        // The pirate board of mapper 27 widens the high half of each CHR register from five
        // bits to eight, which is how it reaches more than 512 KiB of CHR.
        _chrHighMask = mapperNumber == 27 ? 0xFF : 0x1F;
    }

    public override int MapperNumber { get; }

    public override bool IrqAsserted => _irqPending;

    public override void Reset()
    {
        base.Reset();

        Array.Clear(_chr);
        _prg0 = 0;
        _prg1 = 1;
        _swapPrg = false;

        _irqLatch = 0;
        _irqCounter = 0;
        _prescaler = 341;
        _irqEnabled = false;
        _irqEnableAfterAck = false;
        _irqCycleMode = false;
        _irqPending = false;

        UpdateBanks();
    }

    /// <summary>
    /// Index of the register a write to <paramref name="address"/> selects, 0 to 3, from the
    /// two address lines this board's number wires to the chip.
    /// </summary>
    private int RegisterIndex(ushort address)
    {
        int a = address;
        return MapperNumber switch
        {
            21 => (((a >> 1) | (a >> 6)) & 1) | ((((a >> 2) | (a >> 7)) & 1) << 1),
            22 or 25 => (((a >> 1) | (a >> 3)) & 1) | (((a | (a >> 2)) & 1) << 1),
            _ => ((a | (a >> 2)) & 1) | ((((a >> 1) | (a >> 3)) & 1) << 1),
        };
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address < 0x8000)
        {
            return;
        }

        int index = RegisterIndex(address);

        switch (address & 0xF000)
        {
            case 0x8000:
                _prg0 = (byte)(value & 0x1F);
                UpdateBanks();
                break;

            case 0x9000:
                if (index < 2)
                {
                    Mirroring = _hasIrq
                        ? (value & 0x03) switch
                        {
                            0 => NametableMirroring.Vertical,
                            1 => NametableMirroring.Horizontal,
                            2 => NametableMirroring.SingleScreenA,
                            _ => NametableMirroring.SingleScreenB,
                        }
                        : (value & 0x01) != 0
                            ? NametableMirroring.Horizontal
                            : NametableMirroring.Vertical;
                }
                else
                {
                    _swapPrg = (value & 0x02) != 0;
                    UpdateBanks();
                }

                break;

            case 0xA000:
                _prg1 = (byte)(value & 0x1F);
                UpdateBanks();
                break;

            case 0xB000:
            case 0xC000:
            case 0xD000:
            case 0xE000:
            {
                int group = ((address & 0xF000) - 0xB000) >> 12;
                int slot = (group * 2) + (index >> 1);
                if ((index & 1) == 0)
                {
                    _chr[slot] = (_chr[slot] & ~0x0F) | (value & 0x0F);
                }
                else
                {
                    _chr[slot] = (_chr[slot] & 0x0F) | ((value & _chrHighMask) << 4);
                }

                UpdateBanks();
                break;
            }

            default:
                if (_hasIrq)
                {
                    WriteIrqRegister(index, value);
                }

                break;
        }
    }

    private void WriteIrqRegister(int index, byte value)
    {
        switch (index)
        {
            case 0:
                _irqLatch = (byte)((_irqLatch & 0xF0) | (value & 0x0F));
                break;

            case 1:
                _irqLatch = (byte)((_irqLatch & 0x0F) | ((value & 0x0F) << 4));
                break;

            case 2:
                _irqEnableAfterAck = (value & 0x01) != 0;
                _irqEnabled = (value & 0x02) != 0;
                _irqCycleMode = (value & 0x04) != 0;
                if (_irqEnabled)
                {
                    _irqCounter = _irqLatch;
                    _prescaler = 341;
                }

                _irqPending = false;
                break;

            default:
                _irqPending = false;
                _irqEnabled = _irqEnableAfterAck;
                break;
        }
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

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            MapChr1k(slot, _chrShift ? _chr[slot] >> 1 : _chr[slot]);
        }
    }

    /// <inheritdoc />
    protected override void OnCpuCycle()
    {
        if (!_hasIrq || !_irqEnabled)
        {
            return;
        }

        if (_irqCycleMode)
        {
            ClockIrqCounter();
            return;
        }

        _prescaler -= 3;
        if (_prescaler > 0)
        {
            return;
        }

        _prescaler += 341;
        ClockIrqCounter();
    }

    private void ClockIrqCounter()
    {
        if (_irqCounter == 0xFF)
        {
            _irqCounter = _irqLatch;
            _irqPending = true;
            return;
        }

        _irqCounter++;
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            writer.WriteInt32(_chr[slot]);
        }

        writer.WriteByte(_prg0);
        writer.WriteByte(_prg1);
        writer.WriteBool(_swapPrg);
        writer.WriteByte(_irqLatch);
        writer.WriteInt32(_irqCounter);
        writer.WriteInt32(_prescaler);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_irqEnableAfterAck);
        writer.WriteBool(_irqCycleMode);
        writer.WriteBool(_irqPending);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            _chr[slot] = reader.ReadInt32();
        }

        _prg0 = reader.ReadByte();
        _prg1 = reader.ReadByte();
        _swapPrg = reader.ReadBool();
        _irqLatch = reader.ReadByte();
        _irqCounter = reader.ReadInt32();
        _prescaler = reader.ReadInt32();
        _irqEnabled = reader.ReadBool();
        _irqEnableAfterAck = reader.ReadBool();
        _irqCycleMode = reader.ReadBool();
        _irqPending = reader.ReadBool();
    }
}
