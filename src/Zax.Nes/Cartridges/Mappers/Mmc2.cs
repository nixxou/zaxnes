using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mappers 9 and 10, MMC2 (PxROM) and MMC4 (FxROM) (REQ-CORE-100, REQ-CORE-039).
/// </summary>
/// <remarks>
/// <para>
/// The two boards of the $FD/$FE latches. Each 4 KiB CHR window has <em>two</em> bank
/// registers and a latch that says which one is in force; the latch flips when the PPU reads
/// the pattern of tile $FD or $FE from that window, so what a tile shows depends on the tiles
/// fetched before it (nesdev, "MMC2" and "MMC4"). MMC2 only reacts to the single addresses
/// $0FD8 and $0FE8 for the low window, MMC4 to the whole eight-byte rows $0FD8-$0FDF and
/// $0FE8-$0FEF; both react to $1FD8-$1FDF and $1FE8-$1FEF for the high window.
/// </para>
/// <para>
/// PRG differs: MMC2 switches 8 KiB at $8000 and fixes the last three banks above it, MMC4
/// switches 16 KiB at $8000 and fixes the last 16 KiB at $C000. MMC4 boards also carry
/// battery-backed PRG-RAM.
/// </para>
/// <para>
/// <strong>REQ-CORE-033 and REQ-CORE-039.</strong> Only a real fetch moves a latch:
/// <see cref="PpuRead(ushort, ChrFetch)"/> serves the byte and then flips, while
/// <see cref="PpuPeek(ushort, ChrFetch)"/> — the path the tile emitter uses — never does. The
/// emitter instead walks a line in display order and calls
/// <see cref="AdvanceChrLatch"/> itself, then restores the latch it found, so each record
/// carries the pattern the hardware would show at that column without the capture being
/// observable.
/// </para>
/// </remarks>
internal sealed class Mmc2 : BankedCartridge
{
    private readonly bool _isMmc4;
    private readonly byte[] _chr = new byte[4];
    private byte _prg;
    private int _latchLow;
    private int _latchHigh;

    internal Mmc2(RomImage rom, int mapperNumber)
        : base(rom)
    {
        MapperNumber = mapperNumber;
        _isMmc4 = mapperNumber == 10;
    }

    public override int MapperNumber { get; }

    /// <inheritdoc />
    public override int ChrLatchState
    {
        get => _latchLow | (_latchHigh << 1);
        set
        {
            _latchLow = value & 0x01;
            _latchHigh = (value >> 1) & 0x01;
            UpdateBanks();
        }
    }

    public override void Reset()
    {
        base.Reset();

        Array.Clear(_chr);
        _prg = 0;

        // Power-on: both latches select the $FE register (nesdev).
        _latchLow = 1;
        _latchHigh = 1;

        UpdateBanks();
    }

    protected override void WriteRegister(ushort address, byte value)
    {
        switch (address & 0xF000)
        {
            case 0xA000:
                _prg = (byte)(value & 0x0F);
                break;

            case 0xB000:
                _chr[0] = (byte)(value & 0x1F);
                break;

            case 0xC000:
                _chr[1] = (byte)(value & 0x1F);
                break;

            case 0xD000:
                _chr[2] = (byte)(value & 0x1F);
                break;

            case 0xE000:
                _chr[3] = (byte)(value & 0x1F);
                break;

            case 0xF000:
                Mirroring = (value & 0x01) != 0
                    ? NametableMirroring.Horizontal
                    : NametableMirroring.Vertical;
                return;

            default:
                return;
        }

        UpdateBanks();
    }

    private void UpdateBanks()
    {
        if (_isMmc4)
        {
            MapPrg16k(0, _prg);
            MapPrg16k(2, -1);
        }
        else
        {
            MapPrg8k(0, _prg);
            MapPrg8k(1, -3);
            MapPrg8k(2, -2);
            MapPrg8k(3, -1);
        }

        MapChr4k(0, _chr[_latchLow]);
        MapChr4k(4, _chr[2 + _latchHigh]);
    }

    // ---------------------------------------------------------------- latches

    /// <inheritdoc />
    public override byte PpuRead(ushort address, ChrFetch fetch)
    {
        byte value = ReadChrWindow(address);
        MoveLatches((ushort)(address & 0x1FFF));
        return value;
    }

    /// <inheritdoc />
    public override byte PpuPeek(ushort address, ChrFetch fetch) => ReadChrWindow(address);

    /// <inheritdoc />
    public override void AdvanceChrLatch(int patternTable, int tile)
    {
        int which = (patternTable & 0x1000) != 0 ? 1 : 0;
        if (tile == 0xFD)
        {
            SetLatch(which, 0);
        }
        else if (tile == 0xFE)
        {
            SetLatch(which, 1);
        }
    }

    private void MoveLatches(ushort address)
    {
        if (address < 0x1000)
        {
            if (_isMmc4 ? address is >= 0x0FD8 and <= 0x0FDF : address == 0x0FD8)
            {
                SetLatch(0, 0);
            }
            else if (_isMmc4 ? address is >= 0x0FE8 and <= 0x0FEF : address == 0x0FE8)
            {
                SetLatch(0, 1);
            }

            return;
        }

        if (address is >= 0x1FD8 and <= 0x1FDF)
        {
            SetLatch(1, 0);
        }
        else if (address is >= 0x1FE8 and <= 0x1FEF)
        {
            SetLatch(1, 1);
        }
    }

    private void SetLatch(int which, int value)
    {
        if (which == 0)
        {
            if (_latchLow == value)
            {
                return;
            }

            _latchLow = value;
        }
        else
        {
            if (_latchHigh == value)
            {
                return;
            }

            _latchHigh = value;
        }

        UpdateBanks();
    }

    // ---------------------------------------------------------------- savestate

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);
        writer.WriteRaw(_chr);
        writer.WriteByte(_prg);
        writer.WriteInt32(_latchLow);
        writer.WriteInt32(_latchHigh);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);
        reader.ReadInto(_chr);
        _prg = reader.ReadByte();
        _latchLow = reader.ReadInt32();
        _latchHigh = reader.ReadInt32();
    }
}
