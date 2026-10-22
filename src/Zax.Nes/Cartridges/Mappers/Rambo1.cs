using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 64, Tengen RAMBO-1 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// An MMC3-like board with three switchable PRG windows and a second CHR mode (nesdev,
/// "RAMBO-1"). The bank select at $8000 holds the register number in bits 0 to 3, the CHR
/// 1 KiB mode in bit 5, the PRG mode in bit 6 and the CHR inversion in bit 7; $8001 carries
/// the data. Registers 6, 7 and 15 are the PRG banks — $8000/$A000/$C000 in PRG mode 0 and
/// shifted round in mode 1, with the last bank always at $E000. In the 1 KiB CHR mode,
/// registers 8 and 9 supply the second kilobyte of the first two windows.
/// </para>
/// <para>
/// The IRQ counter can be clocked either by A12 rises, like an MMC3, or by the CPU clock
/// divided by four; bit 0 of the value written to $C001 chooses. The counting rule itself is
/// the MMC3 "revision B" one, which no public test ROM pins down for this board.
/// </para>
/// </remarks>
internal sealed class Rambo1 : BankedCartridge
{
    private readonly A12Watcher _a12 = new();
    private readonly byte[] _bank = new byte[16];

    private byte _select;
    private byte _irqLatch;
    private int _irqCounter;
    private int _cycleDivider;
    private bool _cycleMode;
    private bool _irqReload;
    private bool _irqEnabled;
    private bool _irqPending;

    internal Rambo1(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 64;

    public override bool IrqAsserted => _irqPending;

    public override void Reset()
    {
        base.Reset();

        Array.Clear(_bank);
        _select = 0;
        _irqLatch = 0;
        _irqCounter = 0;
        _cycleDivider = 0;
        _cycleMode = false;
        _irqReload = false;
        _irqEnabled = false;
        _irqPending = false;
        _a12.Reset();

        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address < 0x8000)
        {
            return;
        }

        switch (address & 0xE001)
        {
            case 0x8000:
                _select = value;
                UpdateBanks();
                break;

            case 0x8001:
                _bank[_select & 0x0F] = value;
                UpdateBanks();
                break;

            case 0xA000:
                Mirroring = (value & 0x01) != 0
                    ? NametableMirroring.Horizontal
                    : NametableMirroring.Vertical;
                break;

            case 0xC000:
                _irqLatch = value;
                break;

            case 0xC001:
                _cycleMode = (value & 0x01) != 0;
                _cycleDivider = 0;
                _irqReload = true;
                break;

            case 0xE000:
                _irqEnabled = false;
                _irqPending = false;
                break;

            case 0xE001:
                _irqEnabled = true;
                break;

            default:
                break;
        }
    }

    private void UpdateBanks()
    {
        if ((_select & 0x40) != 0)
        {
            MapPrg8k(0, _bank[15]);
            MapPrg8k(1, _bank[6]);
            MapPrg8k(2, _bank[7]);
        }
        else
        {
            MapPrg8k(0, _bank[6]);
            MapPrg8k(1, _bank[7]);
            MapPrg8k(2, _bank[15]);
        }

        MapPrg8k(3, -1);

        int high = (_select & 0x80) != 0 ? 4 : 0;
        int low = 4 - high;

        if ((_select & 0x20) != 0)
        {
            // 1 KiB mode: registers 8 and 9 fill the odd kilobytes of the first two windows.
            MapChr1k(high + 0, _bank[0]);
            MapChr1k(high + 1, _bank[8]);
            MapChr1k(high + 2, _bank[1]);
            MapChr1k(high + 3, _bank[9]);
        }
        else
        {
            MapChr2k(high + 0, _bank[0] >> 1);
            MapChr2k(high + 2, _bank[1] >> 1);
        }

        MapChr1k(low + 0, _bank[2]);
        MapChr1k(low + 1, _bank[3]);
        MapChr1k(low + 2, _bank[4]);
        MapChr1k(low + 3, _bank[5]);
    }

    /// <inheritdoc />
    public override void OnPpuAddressBus(ushort address, long ppuClock)
    {
        if (!_cycleMode && _a12.Rose(address, ppuClock))
        {
            ClockIrqCounter();
        }
    }

    /// <inheritdoc />
    protected override void OnCpuCycle()
    {
        if (!_cycleMode)
        {
            return;
        }

        if (++_cycleDivider < 4)
        {
            return;
        }

        _cycleDivider = 0;
        ClockIrqCounter();
    }

    private void ClockIrqCounter()
    {
        if (_irqCounter == 0 || _irqReload)
        {
            _irqCounter = _irqLatch;
            _irqReload = false;
        }
        else
        {
            _irqCounter--;
        }

        if (_irqCounter == 0 && _irqEnabled)
        {
            _irqPending = true;
        }
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteRaw(_bank);
        writer.WriteByte(_select);
        writer.WriteByte(_irqLatch);
        writer.WriteInt32(_irqCounter);
        writer.WriteInt32(_cycleDivider);
        writer.WriteBool(_cycleMode);
        writer.WriteBool(_irqReload);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_irqPending);
        _a12.SaveState(writer);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        reader.ReadInto(_bank);
        _select = reader.ReadByte();
        _irqLatch = reader.ReadByte();
        _irqCounter = reader.ReadInt32();
        _cycleDivider = reader.ReadInt32();
        _cycleMode = reader.ReadBool();
        _irqReload = reader.ReadBool();
        _irqEnabled = reader.ReadBool();
        _irqPending = reader.ReadBool();
        _a12.LoadState(reader);
    }
}
