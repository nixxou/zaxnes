using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges;

/// <summary>
/// Base class of every switching board: four 8 KiB PRG windows at $8000-$FFFF and eight
/// 1 KiB CHR windows at $0000-$1FFF (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// All the boards of REQ-CORE-100 switch PRG in multiples of 8 KiB and CHR in multiples of
/// 1 KiB, so one pair of window tables expresses every layout: a mapper only recomputes the
/// windows when one of its registers changes, and the bus path stays a table lookup. Bank
/// numbers are wrapped modulo the number of banks the ROM actually has, which is what lets a
/// board select "the last bank" with a negative index and keeps a short dump from reading out
/// of bounds.
/// </para>
/// <para>
/// A PRG window may also show PRG-RAM, which MMC5 needs ($5113-$5117 pick ROM or RAM per
/// window); CHR windows address CHR-ROM when the cartridge has any and CHR-RAM otherwise.
/// </para>
/// </remarks>
public abstract class BankedCartridge : Cartridge
{
    /// <summary>Size of a PRG window, in bytes.</summary>
    public const int PrgWindowSize = 0x2000;

    /// <summary>Size of a CHR window, in bytes.</summary>
    public const int ChrWindowSize = 0x0400;

    /// <summary>Number of PRG windows between $8000 and $FFFF.</summary>
    public const int PrgWindows = 4;

    /// <summary>Number of CHR windows between $0000 and $1FFF.</summary>
    public const int ChrWindows = 8;

    private readonly int[] _prgOffset = new int[PrgWindows];
    private readonly bool[] _prgIsRam = new bool[PrgWindows];
    private readonly int[] _chrOffset = new int[ChrWindows];

    /// <summary>Initialises the window tables to a plain linear layout.</summary>
    protected BankedCartridge(RomImage rom, int minimumPrgRamSize = 0)
        : base(rom, minimumPrgRamSize)
    {
        for (int slot = 0; slot < PrgWindows; slot++)
        {
            MapPrg8k(slot, slot);
        }

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            MapChr1k(slot, slot);
        }
    }

    /// <summary>Number of 8 KiB PRG banks the ROM holds, at least one.</summary>
    protected int PrgBankCount8k => Math.Max(Rom.PrgRom.Length / PrgWindowSize, 1);

    /// <summary>Number of 1 KiB CHR banks the cartridge holds, at least one.</summary>
    protected int ChrBankCount1k => Math.Max(ChrSize / ChrWindowSize, 1);

    /// <summary>Number of 8 KiB PRG-RAM banks the cartridge holds, at least one.</summary>
    protected int PrgRamBankCount8k => Math.Max(PrgRam.Length / PrgWindowSize, 1);

    /// <summary>Whether the CHR windows address CHR-RAM rather than CHR-ROM.</summary>
    protected bool ChrIsRam => Rom.ChrRom.Length == 0;

    private int ChrSize => ChrIsRam ? ChrRam.Length : Rom.ChrRom.Length;

    // ---------------------------------------------------------------- window setters

    /// <summary>Maps 8 KiB PRG bank <paramref name="bank"/> at window <paramref name="slot"/>.</summary>
    /// <param name="slot">Window number, 0 for $8000 to 3 for $E000.</param>
    /// <param name="bank">Bank number; negative counts back from the last bank.</param>
    protected void MapPrg8k(int slot, int bank)
    {
        _prgOffset[slot] = Wrap(bank, PrgBankCount8k) * PrgWindowSize;
        _prgIsRam[slot] = false;
    }

    /// <summary>Maps a 16 KiB PRG bank over two windows starting at <paramref name="slot"/>.</summary>
    protected void MapPrg16k(int slot, int bank)
    {
        MapPrg8k(slot, (bank * 2) + 0);
        MapPrg8k(slot + 1, (bank * 2) + 1);
    }

    /// <summary>Maps a 32 KiB PRG bank over the whole $8000-$FFFF range.</summary>
    protected void MapPrg32k(int bank)
    {
        for (int slot = 0; slot < PrgWindows; slot++)
        {
            MapPrg8k(slot, (bank * 4) + slot);
        }
    }

    /// <summary>Shows 8 KiB of PRG-RAM at window <paramref name="slot"/> (MMC5).</summary>
    protected void MapPrgRam8k(int slot, int bank)
    {
        _prgOffset[slot] = Wrap(bank, PrgRamBankCount8k) * PrgWindowSize;
        _prgIsRam[slot] = true;
    }

    /// <summary>Maps 1 KiB CHR bank <paramref name="bank"/> at window <paramref name="slot"/>.</summary>
    protected void MapChr1k(int slot, int bank)
        => _chrOffset[slot] = Wrap(bank, ChrBankCount1k) * ChrWindowSize;

    /// <summary>Maps a 2 KiB CHR bank over two windows starting at <paramref name="slot"/>.</summary>
    protected void MapChr2k(int slot, int bank)
    {
        MapChr1k(slot, (bank * 2) + 0);
        MapChr1k(slot + 1, (bank * 2) + 1);
    }

    /// <summary>Maps a 4 KiB CHR bank over four windows starting at <paramref name="slot"/>.</summary>
    protected void MapChr4k(int slot, int bank)
    {
        for (int i = 0; i < 4; i++)
        {
            MapChr1k(slot + i, (bank * 4) + i);
        }
    }

    /// <summary>Maps an 8 KiB CHR bank over the whole $0000-$1FFF range.</summary>
    protected void MapChr8k(int bank)
    {
        for (int slot = 0; slot < ChrWindows; slot++)
        {
            MapChr1k(slot, (bank * 8) + slot);
        }
    }

    /// <summary>
    /// Wraps <paramref name="bank"/> into [0, <paramref name="count"/>[; a negative bank
    /// counts back from the end, so -1 is the last bank.
    /// </summary>
    private static int Wrap(int bank, int count)
    {
        int wrapped = bank % count;
        return wrapped < 0 ? wrapped + count : wrapped;
    }

    // ---------------------------------------------------------------- bus

    /// <inheritdoc />
    protected override bool TryReadPrg(ushort address, out byte value)
    {
        if (address < 0x8000)
        {
            value = 0;
            return false;
        }

        int slot = (address - 0x8000) / PrgWindowSize;
        int offset = _prgOffset[slot] + (address & (PrgWindowSize - 1));

        if (_prgIsRam[slot])
        {
            Span<byte> ram = PrgRam;
            value = ram.Length > 0 ? ram[offset % ram.Length] : (byte)0;
            return true;
        }

        ReadOnlySpan<byte> prg = Rom.PrgRom;
        value = prg[offset % prg.Length];
        return true;
    }

    /// <summary>Writes a PRG window that currently shows PRG-RAM (MMC5).</summary>
    /// <returns><see langword="false"/> when the window shows ROM, which ignores writes.</returns>
    protected bool TryWritePrgWindow(ushort address, byte value)
    {
        if (address < 0x8000)
        {
            return false;
        }

        int slot = (address - 0x8000) / PrgWindowSize;
        if (!_prgIsRam[slot])
        {
            return false;
        }

        Span<byte> ram = PrgRam;
        if (ram.Length > 0 && PrgRamWritable)
        {
            ram[(_prgOffset[slot] + (address & (PrgWindowSize - 1))) % ram.Length] = value;
        }

        return true;
    }

    /// <inheritdoc />
    public override byte PpuRead(ushort address, ChrFetch fetch) => ReadChrWindow(address);

    /// <inheritdoc />
    public override void PpuWrite(ushort address, byte value) => WriteChrWindow(address, value);

    /// <summary>Reads CHR through the window table, without any side effect.</summary>
    protected byte ReadChrWindow(ushort address)
    {
        int offset = ChrOffset(address);
        if (ChrIsRam)
        {
            Span<byte> ram = ChrRam;
            return ram.Length > 0 ? ram[offset % ram.Length] : (byte)0;
        }

        ReadOnlySpan<byte> chr = Rom.ChrRom;
        return chr[offset % chr.Length];
    }

    /// <summary>Writes CHR-RAM through the window table; ignored on a CHR-ROM board.</summary>
    protected void WriteChrWindow(ushort address, byte value)
    {
        if (!ChrIsRam)
        {
            return;
        }

        Span<byte> ram = ChrRam;
        if (ram.Length > 0)
        {
            ram[ChrOffset(address) % ram.Length] = value;
        }
    }

    private int ChrOffset(ushort address)
    {
        int index = address & 0x1FFF;
        return _chrOffset[index / ChrWindowSize] + (index & (ChrWindowSize - 1));
    }

    // ---------------------------------------------------------------- savestate

    /// <inheritdoc />
    public override void SaveState(StateWriter writer)
    {
        base.SaveState(writer);

        for (int slot = 0; slot < PrgWindows; slot++)
        {
            writer.WriteInt32(_prgOffset[slot]);
            writer.WriteBool(_prgIsRam[slot]);
        }

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            writer.WriteInt32(_chrOffset[slot]);
        }
    }

    /// <inheritdoc />
    public override void LoadState(StateReader reader)
    {
        base.LoadState(reader);

        for (int slot = 0; slot < PrgWindows; slot++)
        {
            _prgOffset[slot] = reader.ReadInt32();
            _prgIsRam[slot] = reader.ReadBool();
        }

        for (int slot = 0; slot < ChrWindows; slot++)
        {
            _chrOffset[slot] = reader.ReadInt32();
        }
    }
}
