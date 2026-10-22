using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 4, MMC3 (TxROM) and MMC6 (REQ-CORE-100, REQ-CORE-114).
/// </summary>
/// <remarks>
/// <para>
/// Eight bank registers written through the $8000/$8001 pair, a mirroring latch at $A000, a
/// PRG-RAM protect register at $A001 and the four IRQ registers at $C000-$E001 (nesdev,
/// "MMC3"). Bit 6 of the bank select swaps the switchable PRG window between $8000 and
/// $C000; bit 7 swaps the 2 KiB and 1 KiB halves of CHR.
/// </para>
/// <para>
/// <strong>The scanline counter.</strong> The chip counts rises of PPU address line A12,
/// filtered so that the short dips of the fetch pipeline do not count
/// (<see cref="A12Watcher"/>); with the two pattern tables in different halves of CHR that is
/// exactly once per rendered line, which is what makes the status bars of REQ-CORE-114 steady.
/// The counting rule is the "revision B" one REQ-CORE-100 asks for, as measured by blargg's
/// <c>mmc3_test</c>: on a clock the counter reloads when it is zero or when $C001 asked for a
/// reload, otherwise it decrements; the IRQ flag is then set if the counter is zero and IRQs
/// are enabled — so a latch of 0 fires on every clock, and clearing the counter through
/// $C001 does not fire on its own.
/// </para>
/// <para>
/// <strong>MMC6</strong> (NES 2.0 submapper 1) replaces the 8 KiB PRG-RAM window with 1 KiB
/// at $7000, enabled by bit 5 of the bank select and with separate read and write enables per
/// half in $A001. A plain iNES header cannot tell the two chips apart, so a header without a
/// submapper is taken as an MMC3 — which is why the MMC6 test ROM of <c>mmc3_test</c> is
/// reported rather than required (REQ-CORE-114 only demands tests 1 to 5).
/// </para>
/// </remarks>
internal sealed class Mmc3 : BankedCartridge
{
    private readonly A12Watcher _a12 = new();
    private readonly byte[] _bank = new byte[8];
    private readonly bool _isMmc6;
    private readonly bool _fourScreen;

    private byte _select;
    private byte _protect;
    private byte _irqLatch;
    private int _irqCounter;
    private bool _irqReload;
    private bool _irqEnabled;
    private bool _irqPending;

    internal Mmc3(RomImage rom)
        : base(rom)
    {
        _isMmc6 = rom.Header.IsNes20 && rom.Header.Submapper == 1;
        _fourScreen = rom.Header.Mirroring == NametableMirroring.FourScreen;
    }

    public override int MapperNumber => 4;

    public override bool IrqAsserted => _irqPending;

    protected override bool PrgRamEnabled => _isMmc6
        ? (_select & 0x20) != 0
        : (_protect & 0x80) != 0;

    protected override bool PrgRamWritable => PrgRamEnabled && (_protect & 0x40) == 0;

    public override void Reset()
    {
        base.Reset();

        Array.Clear(_bank);
        _bank[6] = 0;
        _bank[7] = 1;
        _select = 0;

        // The power-on value of $A001 is not documented; PRG-RAM enabled and writable is the
        // only choice that lets a game — or blargg's own MMC3 test ROMs, which report through
        // $6000 without ever writing $A001 — use the RAM out of a cold start.
        _protect = 0x80;
        _irqLatch = 0;
        _irqCounter = 0;
        _irqReload = false;
        _irqEnabled = false;
        _irqPending = false;
        _a12.Reset();

        UpdateBanks();
    }

    // ---------------------------------------------------------------- CPU bus

    /// <inheritdoc />
    /// <remarks>
    /// MMC6 only decodes $7000-$7FFF, where its 1 KiB of RAM is mirrored, and gates each
    /// 512-byte half separately: bits 4 and 6 of $A001 enable reading, bits 5 and 7 writing.
    /// A disabled half reads back as 0.
    /// </remarks>
    public override bool TryCpuRead(ushort address, out byte value)
    {
        if (_isMmc6 && address is >= 0x6000 and <= 0x7FFF)
        {
            value = 0;
            if (address < 0x7000 || !PrgRamEnabled)
            {
                return address >= 0x7000;
            }

            if (Mmc6HalfReadable(address))
            {
                value = PrgRam[Mmc6Offset(address)];
            }

            return true;
        }

        return base.TryCpuRead(address, out value);
    }

    /// <inheritdoc />
    public override void CpuWrite(ushort address, byte value)
    {
        if (_isMmc6 && address is >= 0x6000 and <= 0x7FFF)
        {
            if (address >= 0x7000 && PrgRamEnabled && Mmc6HalfWritable(address))
            {
                PrgRam[Mmc6Offset(address)] = value;
            }

            return;
        }

        base.CpuWrite(address, value);
    }

    private int Mmc6Offset(ushort address) => (address & 0x03FF) % PrgRam.Length;

    private bool Mmc6HalfReadable(ushort address)
        => (_protect & ((address & 0x0200) != 0 ? 0x40 : 0x10)) != 0;

    private bool Mmc6HalfWritable(ushort address)
        => (_protect & ((address & 0x0200) != 0 ? 0x80 : 0x20)) != 0;

    /// <inheritdoc />
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
                _bank[_select & 0x07] = value;
                UpdateBanks();
                break;

            case 0xA000:
                if (!_fourScreen)
                {
                    Mirroring = (value & 0x01) != 0
                        ? NametableMirroring.Horizontal
                        : NametableMirroring.Vertical;
                }

                break;

            case 0xA001:
                _protect = value;
                break;

            case 0xC000:
                _irqLatch = value;
                break;

            case 0xC001:
                _irqCounter = 0;
                _irqReload = true;
                break;

            case 0xE000:
                _irqEnabled = false;
                _irqPending = false;
                break;

            default:
                _irqEnabled = true;
                break;
        }
    }

    private void UpdateBanks()
    {
        if ((_select & 0x40) != 0)
        {
            MapPrg8k(0, -2);
            MapPrg8k(2, _bank[6]);
        }
        else
        {
            MapPrg8k(0, _bank[6]);
            MapPrg8k(2, -2);
        }

        MapPrg8k(1, _bank[7]);
        MapPrg8k(3, -1);

        int high = (_select & 0x80) != 0 ? 4 : 0;
        int low = 4 - high;

        MapChr2k(high + 0, _bank[0] >> 1);
        MapChr2k(high + 2, _bank[1] >> 1);
        MapChr1k(low + 0, _bank[2]);
        MapChr1k(low + 1, _bank[3]);
        MapChr1k(low + 2, _bank[4]);
        MapChr1k(low + 3, _bank[5]);
    }

    // ---------------------------------------------------------------- IRQ

    /// <inheritdoc />
    public override void OnPpuAddressBus(ushort address, long ppuClock)
    {
        if (_a12.Rose(address, ppuClock))
        {
            ClockIrqCounter();
        }
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

    // ---------------------------------------------------------------- savestate

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);

        writer.WriteRaw(_bank);
        writer.WriteByte(_select);
        writer.WriteByte(_protect);
        writer.WriteByte(_irqLatch);
        writer.WriteInt32(_irqCounter);
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
        _protect = reader.ReadByte();
        _irqLatch = reader.ReadByte();
        _irqCounter = reader.ReadInt32();
        _irqReload = reader.ReadBool();
        _irqEnabled = reader.ReadBool();
        _irqPending = reader.ReadBool();
        _a12.LoadState(reader);
    }
}
