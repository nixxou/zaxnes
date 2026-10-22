using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges.Mappers;

/// <summary>
/// iNES mapper 5, MMC5 (ExROM) (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// The most capable of the licensed boards (nesdev, "MMC5"). What REQ-CORE-100 requires at
/// P1 is implemented here: the four PRG modes ($5100, with $5113-$5117 choosing ROM or RAM
/// per window), the four CHR modes ($5101 with the two register sets $5120-$5127 and
/// $5128-$512B), up to 64 KiB of banked PRG-RAM with its two-step write protection
/// ($5102/$5103) and the scanline IRQ ($5203/$5204). The programmable nametable mapping
/// ($5105), the 1 KiB of ExRAM with its four modes ($5104, $5C00-$5FFF) and the fill-mode
/// nametable ($5106/$5107) come with it, as does the 8-by-8 multiplier ($5205/$5206).
/// </para>
/// <para>
/// Left out, as REQ-CORE-100 allows (P2 and P3): the extended attribute mode, which gives
/// every background tile its own pattern bank and palette; the vertical split screen
/// ($5200-$5202); and the expansion sound ($5000-$5015), which REQ-CORE-075 does not
/// require.
/// </para>
/// <para>
/// <strong>Two CHR register sets.</strong> With 8x16 sprites selected, background fetches
/// come from $5128-$512B and sprite fetches from $5120-$5127; with 8x8 sprites, whichever set
/// was written last serves everything. The tile emitter therefore says which kind of fetch it
/// is making (<see cref="ChrFetch"/>), so a background row and a sprite record of the same
/// frame can legitimately carry patterns from different banks — see Q-007.
/// </para>
/// </remarks>
internal sealed class Mmc5 : BankedCartridge
{
    /// <summary>Size of the board's own RAM, mapped at $5C00-$5FFF.</summary>
    private const int ExRamSize = 1024;

    /// <summary>PRG-RAM the board can address: eight banks of 8 KiB.</summary>
    internal const int MaxPrgRamSize = 64 * 1024;

    private readonly byte[] _exRam = new byte[ExRamSize];
    private readonly int[] _chrA = new int[8];
    private readonly int[] _chrB = new int[4];
    private readonly byte[] _prgBank = new byte[4];

    private int _prgMode;
    private int _chrMode;
    private int _chrUpper;
    private int _exRamMode;
    private int _nametableMapping;
    private byte _fillTile;
    private byte _fillAttribute;
    private byte _ramBank;
    private int _protect1;
    private int _protect2;
    private bool _lastSetWasB;
    private bool _spriteSize16;

    private int _irqTarget;
    private bool _irqEnabled;
    private bool _irqPending;
    private bool _inFrame;
    private int _scanlineCounter;

    private byte _multiplicand;
    private byte _multiplier;

    internal Mmc5(RomImage rom)
        : base(rom, MaxPrgRamSize)
    {
    }

    public override int MapperNumber => 5;

    public override bool IrqAsserted => _irqPending && _irqEnabled;

    /// <summary>Writing PRG-RAM takes the two magic values in $5102 and $5103 (nesdev).</summary>
    protected override bool PrgRamWritable => (_protect1 & 0x03) == 0x02 && (_protect2 & 0x03) == 0x01;

    public override void Reset()
    {
        base.Reset();

        Array.Clear(_exRam);
        Array.Clear(_chrA);
        Array.Clear(_chrB);

        // Power-on: PRG mode 3 with the last bank at $E000, which is the state the board is
        // documented to start in so that a reset vector can be fetched.
        _prgMode = 3;
        _prgBank[0] = 0;
        _prgBank[1] = 0x80;
        _prgBank[2] = 0x80;
        _prgBank[3] = 0xFF;
        _ramBank = 0;

        _chrMode = 0;
        _chrUpper = 0;
        _exRamMode = 0;
        _nametableMapping = 0;
        _fillTile = 0;
        _fillAttribute = 0;
        _protect1 = 0;
        _protect2 = 0;
        _lastSetWasB = false;
        _spriteSize16 = false;

        _irqTarget = 0;
        _irqEnabled = false;
        _irqPending = false;
        _inFrame = false;
        _scanlineCounter = 0;

        _multiplicand = 0;
        _multiplier = 0;

        UpdateBanks();
    }

    // ---------------------------------------------------------------- CPU bus

    /// <inheritdoc />
    public override bool TryCpuRead(ushort address, out byte value)
    {
        switch (address)
        {
            case 0x5204:
            {
                value = (byte)((_irqPending ? 0x80 : 0x00) | (_inFrame ? 0x40 : 0x00));
                _irqPending = false;
                return true;
            }

            case 0x5205:
                value = (byte)(_multiplicand * _multiplier);
                return true;

            case 0x5206:
                value = (byte)((_multiplicand * _multiplier) >> 8);
                return true;

            case >= 0x5C00 and <= 0x5FFF:
                // ExRAM reads back only in the two RAM modes; the nametable modes leave the
                // CPU side open (nesdev).
                value = _exRamMode >= 2 ? _exRam[address - 0x5C00] : (byte)0;
                return true;

            default:
                return base.TryCpuRead(address, out value);
        }
    }

    /// <inheritdoc />
    /// <remarks>
    /// Reading $5204 acknowledges the IRQ, so the host accessor must not take that path
    /// (REQ-CORE-081).
    /// </remarks>
    public override bool TryCpuPeek(ushort address, out byte value)
    {
        if (address == 0x5204)
        {
            value = (byte)((_irqPending ? 0x80 : 0x00) | (_inFrame ? 0x40 : 0x00));
            return true;
        }

        return TryCpuRead(address, out value);
    }

    /// <inheritdoc />
    public override void CpuWrite(ushort address, byte value)
    {
        if (address is >= 0x5C00 and <= 0x5FFF)
        {
            if (_exRamMode != 3)
            {
                _exRam[address - 0x5C00] = value;
            }

            return;
        }

        if (address is >= 0x5100 and <= 0x5206)
        {
            WriteControl(address, value);
            return;
        }

        if (address >= 0x8000)
        {
            // A PRG window showing RAM takes writes; one showing ROM ignores them.
            TryWritePrgWindow(address, value);
            return;
        }

        base.CpuWrite(address, value);
    }

    private void WriteControl(ushort address, byte value)
    {
        switch (address)
        {
            case 0x5100:
                _prgMode = value & 0x03;
                UpdateBanks();
                break;

            case 0x5101:
                _chrMode = value & 0x03;
                break;

            case 0x5102:
                _protect1 = value;
                break;

            case 0x5103:
                _protect2 = value;
                break;

            case 0x5104:
                _exRamMode = value & 0x03;
                break;

            case 0x5105:
                _nametableMapping = value;
                break;

            case 0x5106:
                _fillTile = value;
                break;

            case 0x5107:
                _fillAttribute = (byte)(value & 0x03);
                break;

            case 0x5113:
                _ramBank = (byte)(value & 0x07);
                UpdateBanks();
                break;

            case >= 0x5114 and <= 0x5117:
                _prgBank[address - 0x5114] = value;
                UpdateBanks();
                break;

            case >= 0x5120 and <= 0x5127:
                _chrA[address - 0x5120] = value | (_chrUpper << 8);
                _lastSetWasB = false;
                break;

            case >= 0x5128 and <= 0x512B:
                _chrB[address - 0x5128] = value | (_chrUpper << 8);
                _lastSetWasB = true;
                break;

            case 0x5130:
                _chrUpper = value & 0x03;
                break;

            case 0x5203:
                _irqTarget = value;
                break;

            case 0x5204:
                _irqEnabled = (value & 0x80) != 0;
                break;

            case 0x5205:
                _multiplicand = value;
                break;

            case 0x5206:
                _multiplier = value;
                break;

            default:
                break;
        }
    }

    protected override int PrgRamOffset(ushort address)
        => ((_ramBank * PrgWindowSize) + (address - 0x6000)) % PrgRam.Length;

    private void UpdateBanks()
    {
        switch (_prgMode)
        {
            case 0:
                MapPrgWindow(0, _prgBank[3], 4);
                break;

            case 1:
                MapPrgWindow(0, _prgBank[1], 2);
                MapPrgWindow(2, _prgBank[3], 2);
                break;

            case 2:
                MapPrgWindow(0, _prgBank[1], 2);
                MapPrgWindow(2, _prgBank[2], 1);
                MapPrgWindow(3, _prgBank[3], 1);
                break;

            default:
                MapPrgWindow(0, _prgBank[0], 1);
                MapPrgWindow(1, _prgBank[1], 1);
                MapPrgWindow(2, _prgBank[2], 1);
                MapPrgWindow(3, _prgBank[3], 1);
                break;
        }
    }

    /// <summary>
    /// Maps <paramref name="windows"/> consecutive 8 KiB windows from <paramref name="slot"/>
    /// on, from the register value <paramref name="register"/>: bit 7 says ROM, and the bank
    /// number is in units of the window group's size. $5117 is always ROM.
    /// </summary>
    private void MapPrgWindow(int slot, byte register, int windows)
    {
        bool rom = (register & 0x80) != 0 || slot + windows == PrgWindows;
        int bank = register & 0x7F;

        // The bank number counts in 8 KiB units, so a 16 or 32 KiB window drops the bits the
        // larger granularity does not use.
        int first = bank & ~(windows - 1);

        for (int i = 0; i < windows; i++)
        {
            if (rom)
            {
                MapPrg8k(slot + i, first + i);
            }
            else
            {
                MapPrgRam8k(slot + i, (first + i) & 0x07);
            }
        }
    }

    // ---------------------------------------------------------------- CHR

    /// <inheritdoc />
    public override byte PpuRead(ushort address, ChrFetch fetch) => ReadChr(address, fetch);

    /// <inheritdoc />
    public override byte PpuPeek(ushort address, ChrFetch fetch) => ReadChr(address, fetch);

    /// <inheritdoc />
    public override void PpuWrite(ushort address, byte value)
    {
        if (!ChrIsRam)
        {
            return;
        }

        Span<byte> ram = ChrRam;
        if (ram.Length > 0)
        {
            ram[ChrAddress(address, ChrFetch.Background) % ram.Length] = value;
        }
    }

    private byte ReadChr(ushort address, ChrFetch fetch)
    {
        int offset = ChrAddress(address, fetch);

        if (ChrIsRam)
        {
            Span<byte> ram = ChrRam;
            return ram.Length > 0 ? ram[offset % ram.Length] : (byte)0;
        }

        ReadOnlySpan<byte> chr = Rom.ChrRom;
        return chr[offset % chr.Length];
    }

    /// <summary>
    /// Offset in CHR of PPU address <paramref name="address"/>, through the bank set the
    /// fetch kind and the sprite size select.
    /// </summary>
    private int ChrAddress(ushort address, ChrFetch fetch)
    {
        int index = address & 0x1FFF;
        bool useB = _spriteSize16 ? fetch == ChrFetch.Background : _lastSetWasB;

        switch (_chrMode)
        {
            case 0:
            {
                int bank = useB ? _chrB[3] : _chrA[7];
                return (bank * 0x2000) + index;
            }

            case 1:
            {
                int window = (index >> 12) & 0x01;
                int bank = useB ? _chrB[3] : _chrA[3 + (4 * window)];
                return (bank * 0x1000) + (index & 0x0FFF);
            }

            case 2:
            {
                int window = (index >> 11) & 0x03;
                int bank = useB ? _chrB[1 + (2 * (window & 1))] : _chrA[1 + (2 * window)];
                return (bank * 0x0800) + (index & 0x07FF);
            }

            default:
            {
                int window = (index >> 10) & 0x07;
                int bank = useB ? _chrB[window & 0x03] : _chrA[window];
                return (bank * 0x0400) + (index & 0x03FF);
            }
        }
    }

    // ---------------------------------------------------------------- nametables

    /// <inheritdoc />
    public override int NametablePage(int table) => ((_nametableMapping >> (table * 2)) & 0x03) switch
    {
        0 => 0,
        1 => 1,
        _ => ServedByCartridge,
    };

    /// <inheritdoc />
    public override byte ReadNametable(ushort address)
    {
        int table = ((address - 0x2000) >> 10) & 0x03;
        int mode = (_nametableMapping >> (table * 2)) & 0x03;

        if (mode == 2)
        {
            // ExRAM as a nametable: only the two nametable modes show it to the PPU.
            return _exRamMode < 2 ? _exRam[address & 0x03FF] : (byte)0;
        }

        // Fill mode: one tile everywhere, one attribute everywhere.
        return (address & 0x03FF) >= 0x03C0
            ? (byte)(_fillAttribute * 0x55)
            : _fillTile;
    }

    /// <inheritdoc />
    public override void WriteNametable(ushort address, byte value)
    {
        int table = ((address - 0x2000) >> 10) & 0x03;
        if (((_nametableMapping >> (table * 2)) & 0x03) == 2 && _exRamMode < 2)
        {
            _exRam[address & 0x03FF] = value;
        }
    }

    // ---------------------------------------------------------------- scanline IRQ

    /// <inheritdoc />
    /// <remarks>
    /// The board counts the lines it sees being rendered and raises its IRQ on the one
    /// $5203 names; outside rendering the in-frame flag drops and the count restarts, which is
    /// what lets a game tell a vblank from a split (nesdev, "MMC5").
    /// </remarks>
    public override void OnPpuScanline(int scanline, bool renderingEnabled)
    {
        if (!renderingEnabled || scanline >= RegionTiming.VisibleScanlines)
        {
            _inFrame = false;
            _scanlineCounter = 0;
            return;
        }

        _inFrame = true;
        _scanlineCounter = scanline + 1;

        if (_irqTarget != 0 && _scanlineCounter == _irqTarget)
        {
            _irqPending = true;
        }
    }

    /// <inheritdoc />
    /// <remarks>
    /// The sprite size decides whether the two CHR register sets are used separately, so the
    /// board has to follow PPUCTRL.
    /// </remarks>
    public override void OnPpuControl(byte control) => _spriteSize16 = (control & 0x20) != 0;

    // ---------------------------------------------------------------- savestate

    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);

        writer.WriteBlob(_exRam);
        for (int i = 0; i < _chrA.Length; i++)
        {
            writer.WriteInt32(_chrA[i]);
        }

        for (int i = 0; i < _chrB.Length; i++)
        {
            writer.WriteInt32(_chrB[i]);
        }

        writer.WriteRaw(_prgBank);
        writer.WriteInt32(_prgMode);
        writer.WriteInt32(_chrMode);
        writer.WriteInt32(_chrUpper);
        writer.WriteInt32(_exRamMode);
        writer.WriteInt32(_nametableMapping);
        writer.WriteByte(_fillTile);
        writer.WriteByte(_fillAttribute);
        writer.WriteByte(_ramBank);
        writer.WriteInt32(_protect1);
        writer.WriteInt32(_protect2);
        writer.WriteBool(_lastSetWasB);
        writer.WriteBool(_spriteSize16);
        writer.WriteInt32(_irqTarget);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_irqPending);
        writer.WriteBool(_inFrame);
        writer.WriteInt32(_scanlineCounter);
        writer.WriteByte(_multiplicand);
        writer.WriteByte(_multiplier);
    }

    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);

        reader.ReadBlobInto(_exRam);
        for (int i = 0; i < _chrA.Length; i++)
        {
            _chrA[i] = reader.ReadInt32();
        }

        for (int i = 0; i < _chrB.Length; i++)
        {
            _chrB[i] = reader.ReadInt32();
        }

        reader.ReadInto(_prgBank);
        _prgMode = reader.ReadInt32();
        _chrMode = reader.ReadInt32();
        _chrUpper = reader.ReadInt32();
        _exRamMode = reader.ReadInt32();
        _nametableMapping = reader.ReadInt32();
        _fillTile = reader.ReadByte();
        _fillAttribute = reader.ReadByte();
        _ramBank = reader.ReadByte();
        _protect1 = reader.ReadInt32();
        _protect2 = reader.ReadInt32();
        _lastSetWasB = reader.ReadBool();
        _spriteSize16 = reader.ReadBool();
        _irqTarget = reader.ReadInt32();
        _irqEnabled = reader.ReadBool();
        _irqPending = reader.ReadBool();
        _inFrame = reader.ReadBool();
        _scanlineCounter = reader.ReadInt32();
        _multiplicand = reader.ReadByte();
        _multiplier = reader.ReadByte();
    }
}
