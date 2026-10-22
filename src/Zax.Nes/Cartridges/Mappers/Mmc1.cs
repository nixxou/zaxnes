using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 1, MMC1 (SxROM), SUROM and SOROM included (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// The four registers are not written directly: each write to $8000-$FFFF shifts one bit
/// into a five-bit serial register, least significant bit first, and the fifth write commits
/// the value to the register the address selects — $8000 control, $A000 CHR bank 0, $C000
/// CHR bank 1, $E000 PRG bank. A write whose bit 7 is set clears the shift register and
/// forces the PRG mode to "last bank fixed at $C000" (nesdev, "MMC1").
/// </para>
/// <para>
/// Two writes on consecutive CPU cycles — what a read-modify-write instruction produces —
/// only count once: the chip holds its write-enable low for a cycle afterwards, so the
/// second write is dropped. Several games depend on it.
/// </para>
/// <para>
/// <strong>512 KiB boards.</strong> On SUROM and SXROM the PRG bank register only reaches
/// 256 KiB, and bit 4 of the CHR bank register supplies the top address line; the fixed
/// windows then sit at the ends of the selected 256 KiB block, not of the image. Those boards
/// carry CHR-RAM, so CHR bank 0 is the only register that matters and it is the one read here.
/// Bits 2 and 3 of the same register bank the PRG-RAM of SOROM and SXROM, which is only done
/// when the header actually announces more than 8 KiB: an iNES 1.0 header cannot say, and a
/// game of the plain SNROM family writes those two bits without meaning anything by them.
/// </para>
/// </remarks>
internal sealed class Mmc1 : BankedCartridge
{
    private int _shift;
    private int _shiftCount;
    private byte _control;
    private byte _chr0;
    private byte _chr1;
    private byte _prg;
    private long _lastWriteCycle;

    internal Mmc1(RomImage rom)
        : base(rom)
    {
    }

    public override int MapperNumber => 1;

    /// <summary>Bit 4 of the PRG register disables the PRG-RAM chip (nesdev, "MMC1").</summary>
    protected override bool PrgRamEnabled => (_prg & 0x10) == 0;

    public override void Reset()
    {
        base.Reset();

        _shift = 0;
        _shiftCount = 0;

        // Power-on: the last bank is fixed at $C000, which is the only state the chip is
        // known to come up in (nesdev); everything else starts at 0.
        _control = 0x0C;
        _chr0 = 0;
        _chr1 = 0;
        _prg = 0;

        // Far enough back that the very first write is never taken for the second half of a
        // read-modify-write pair, and without the risk of an overflow in the comparison.
        _lastWriteCycle = -4;

        UpdateBanks();
    }

    protected override int PrgRamOffset(ushort address)
    {
        Span<byte> ram = PrgRam;
        int bank = ram.Length > PrgWindowSize ? (_chr0 >> 2) & 0x03 : 0;
        return ((bank * PrgWindowSize) + (address - 0x6000)) % ram.Length;
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        if (address < 0x8000)
        {
            return;
        }

        bool ignored = CpuCycle - _lastWriteCycle < 2;
        _lastWriteCycle = CpuCycle;
        if (ignored)
        {
            return;
        }

        if ((value & 0x80) != 0)
        {
            _shift = 0;
            _shiftCount = 0;
            _control |= 0x0C;
            UpdateBanks();
            return;
        }

        _shift |= (value & 0x01) << _shiftCount;
        if (++_shiftCount < 5)
        {
            return;
        }

        var data = (byte)(_shift & 0x1F);
        _shift = 0;
        _shiftCount = 0;

        switch (address & 0x6000)
        {
            case 0x0000:
                _control = data;
                break;

            case 0x2000:
                _chr0 = data;
                break;

            case 0x4000:
                _chr1 = data;
                break;

            default:
                _prg = data;
                break;
        }

        UpdateBanks();
    }

    private void UpdateBanks()
    {
        // A18 of a 512 KiB board, in 16 KiB units.
        int outer = Rom.PrgRom.Length > 256 * 1024 ? ((_chr0 >> 4) & 0x01) << 4 : 0;
        int bank = (_prg & 0x0F) | outer;

        switch ((_control >> 2) & 0x03)
        {
            case 0:
            case 1:
                // 32 KiB at $8000, the low bit of the bank number ignored.
                MapPrg32k(bank >> 1);
                break;

            case 2:
                // First bank of the block fixed at $8000, switchable at $C000.
                MapPrg16k(0, outer);
                MapPrg16k(2, bank);
                break;

            default:
                // Switchable at $8000, last bank of the block fixed at $C000.
                MapPrg16k(0, bank);
                MapPrg16k(2, outer | 0x0F);
                break;
        }

        if ((_control & 0x10) != 0)
        {
            MapChr4k(0, _chr0);
            MapChr4k(4, _chr1);
        }
        else
        {
            MapChr8k(_chr0 >> 1);
        }

        Mirroring = (_control & 0x03) switch
        {
            0 => NametableMirroring.SingleScreenA,
            1 => NametableMirroring.SingleScreenB,
            2 => NametableMirroring.Vertical,
            _ => NametableMirroring.Horizontal,
        };
    }

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteInt32(_shift);
        writer.WriteInt32(_shiftCount);
        writer.WriteByte(_control);
        writer.WriteByte(_chr0);
        writer.WriteByte(_chr1);
        writer.WriteByte(_prg);
        writer.WriteInt64(_lastWriteCycle);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        _shift = reader.ReadInt32();
        _shiftCount = reader.ReadInt32();
        _control = reader.ReadByte();
        _chr0 = reader.ReadByte();
        _chr1 = reader.ReadByte();
        _prg = reader.ReadByte();
        _lastWriteCycle = reader.ReadInt64();
    }
}
