using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 16, Bandai FCG and LZ93D50 (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// Sixteen registers, selected by the low four bits of the address: eight 1 KiB CHR banks,
/// the 16 KiB PRG bank at $8000, the nametable layout, and a 16-bit IRQ counter clocked by
/// the CPU clock (nesdev, "Bandai FCG board"). The last PRG bank is fixed at $C000. The
/// earlier FCG-1 and FCG-2 boards decode the registers at $6000-$7FFF and the LZ93D50 at
/// $8000-$FFFF; a plain iNES header does not say which, so both ranges are decoded, and the
/// $6000 window therefore carries no PRG-RAM — these boards have a serial EEPROM instead,
/// which REQ-CORE-100 leaves at P3 and which is not emulated.
/// </para>
/// <para>
/// The IRQ counter follows the LZ93D50: $x00B and $x00C load a latch, a write to $x00A
/// copies the latch into the counter and acknowledges the pending IRQ, and the IRQ fires when
/// the counter underflows past zero.
/// </para>
/// </remarks>
internal sealed class BandaiFcg : BankedCartridge
{
    private readonly byte[] _chr = new byte[8];
    private byte _prg;
    private int _irqLatch;
    private int _irqCounter;
    private bool _irqEnabled;
    private bool _irqPending;

    internal BandaiFcg(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 16;

    public override bool IrqAsserted => _irqPending;

    protected override bool PrgRamEnabled => false;

    public override void Reset()
    {
        base.Reset();

        Array.Clear(_chr);
        _prg = 0;
        _irqLatch = 0;
        _irqCounter = 0;
        _irqEnabled = false;
        _irqPending = false;

        UpdateBanks();
    }

    public override void CpuWrite(ushort address, byte value)
    {
        if (address >= 0x6000)
        {
            WriteRegister(address, value);
            return;
        }

        base.CpuWrite(address, value);
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address < 0x6000)
        {
            return;
        }

        switch (address & 0x000F)
        {
            case >= 0x0 and <= 0x7:
                _chr[address & 0x07] = value;
                UpdateBanks();
                break;

            case 0x8:
                _prg = value;
                UpdateBanks();
                break;

            case 0x9:
                Mirroring = (value & 0x03) switch
                {
                    0 => NametableMirroring.Vertical,
                    1 => NametableMirroring.Horizontal,
                    2 => NametableMirroring.SingleScreenA,
                    _ => NametableMirroring.SingleScreenB,
                };
                break;

            case 0xA:
                _irqPending = false;
                _irqEnabled = (value & 0x01) != 0;
                _irqCounter = _irqLatch;
                break;

            case 0xB:
                _irqLatch = (_irqLatch & 0xFF00) | value;
                break;

            case 0xC:
                _irqLatch = (_irqLatch & 0x00FF) | (value << 8);
                break;

            default:
                // $x00D is the EEPROM port (REQ-CORE-100: P3).
                break;
        }
    }

    private void UpdateBanks()
    {
        MapPrg16k(0, _prg);
        MapPrg16k(2, -1);

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

        _irqCounter--;
        if (_irqCounter < 0)
        {
            _irqCounter = 0xFFFF;
            _irqPending = true;
        }
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteRaw(_chr);
        writer.WriteByte(_prg);
        writer.WriteInt32(_irqLatch);
        writer.WriteInt32(_irqCounter);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_irqPending);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        reader.ReadInto(_chr);
        _prg = reader.ReadByte();
        _irqLatch = reader.ReadInt32();
        _irqCounter = reader.ReadInt32();
        _irqEnabled = reader.ReadBool();
        _irqPending = reader.ReadBool();
    }
}
