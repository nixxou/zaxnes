using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes.Cartridges;

/// <summary>
/// A cartridge: PRG and CHR banking, nametable layout, PRG-RAM and mapper IRQ
/// (REQ-CORE-100).
/// </summary>
/// <remarks>
/// <para>
/// This is the single extension point for the mappers of REQ-CORE-100. A mapper subclass
/// overrides the bus entry points, declares its nametable layout and, when it has one, its
/// IRQ line. The base class already provides the 8 KiB PRG-RAM at $6000-$7FFF
/// (REQ-CORE-012) and the CHR-RAM buffer, since nearly every board needs them;
/// <see cref="BankedCartridge"/> adds the bank window machinery the switching boards share.
/// </para>
/// <para>
/// <strong>Side-effect-free reads.</strong> The tile emitter reads pattern data
/// outside of real rendering and must not disturb the machine: no A12 IRQ counting
/// (MMC3), no latch toggling (MMC2/MMC4), no read-buffer change (REQ-CORE-033). Mappers
/// with such side effects therefore put them in <see cref="PpuRead(ushort, ChrFetch)"/> and
/// <see cref="OnPpuAddressBus"/>, never in <see cref="PpuPeek(ushort, ChrFetch)"/>, which is
/// what the tile emitter and the host memory accessors call.
/// </para>
/// </remarks>
public abstract class Cartridge
{
    /// <summary>
    /// Returned by <see cref="NametablePage"/> when the board leaves the layout to
    /// <see cref="Mirroring"/>, which is what every mapper but MMC5 does.
    /// </summary>
    public const int FollowMirroring = -1;

    /// <summary>
    /// Returned by <see cref="NametablePage"/> when the cartridge serves the nametable
    /// itself (MMC5 ExRAM and fill mode) through <see cref="ReadNametable"/>.
    /// </summary>
    public const int ServedByCartridge = -2;

    private readonly byte[] _prgRam;
    private readonly byte[] _chrRam;

    /// <summary>Initialises the shared cartridge state from <paramref name="rom"/>.</summary>
    /// <param name="rom">The ROM image.</param>
    /// <param name="minimumPrgRamSize">
    /// Minimum size of the PRG-RAM buffer, for boards that carry more than 8 KiB and bank it
    /// (MMC1 SOROM and SXROM, MMC5).
    /// </param>
    protected Cartridge(RomImage rom, int minimumPrgRamSize = 0)
    {
        ArgumentNullException.ThrowIfNull(rom);

        Rom = rom;
        _prgRam = new byte[Math.Max(
            Math.Max(rom.Header.PrgRamSize, InesHeader.DefaultPrgRamSize),
            minimumPrgRamSize)];
        _chrRam = new byte[rom.Header.ChrRamSize];
        Mirroring = rom.Header.Mirroring;
        LoadTrainer();
    }

    /// <summary>
    /// Copies the 512-byte trainer to $7000 inside the PRG-RAM (REQ-CORE-012).
    /// </summary>
    /// <remarks>
    /// The reference does not skip the trainer at all; handling it is an accepted
    /// divergence (REQ-CORE-012, notes). It is re-applied whenever PRG-RAM is cleared,
    /// because a power-on must leave the trainer in place.
    /// </remarks>
    private void LoadTrainer()
    {
        if (Rom.Header.HasTrainer
            && _prgRam.Length >= 0x1000 + InesHeader.TrainerSize)
        {
            Rom.Trainer.CopyTo(_prgRam.AsSpan(InesHeader.TrainerLoadAddress - 0x6000));
        }
    }

    /// <summary>The ROM image this cartridge was built from.</summary>
    public RomImage Rom { get; }

    /// <summary>iNES mapper number of this cartridge.</summary>
    public abstract int MapperNumber { get; }

    /// <summary>Current nametable layout (SPEC-02 §2, "current mirroring").</summary>
    public NametableMirroring Mirroring { get; protected set; }

    /// <summary>Whether the cartridge is asserting the CPU IRQ line.</summary>
    public virtual bool IrqAsserted => false;

    /// <summary>CPU cycles elapsed since power-on, as the cartridge sees them.</summary>
    /// <remarks>
    /// Boards that time anything in CPU cycles read it: the MMC1 shift register ignores a
    /// write made on the cycle right after another one, and the VRC4, FCG and SS88006 IRQ
    /// counters tick on it.
    /// </remarks>
    public long CpuCycle { get; private set; }

    /// <summary>PRG-RAM at $6000-$7FFF, 8 KiB unless the board or the header asks for more.</summary>
    protected Span<byte> PrgRam => _prgRam;

    /// <summary>CHR-RAM, empty when the cartridge has CHR-ROM.</summary>
    protected Span<byte> ChrRam => _chrRam;

    /// <summary>True when $6000-$7FFF is readable and writable. Mappers may disable it.</summary>
    protected virtual bool PrgRamEnabled => true;

    /// <summary>True when $6000-$7FFF accepts writes. Mappers may write-protect it.</summary>
    protected virtual bool PrgRamWritable => PrgRamEnabled;

    /// <summary>
    /// Battery-backed PRG-RAM, exposed so that a host can persist it (REQ-CORE-095,
    /// libretro "SAVE_RAM"). Empty when bit 1 of header byte 6 is clear.
    /// </summary>
    public Span<byte> BatteryRam => Rom.Header.HasBattery ? _prgRam : [];

    /// <summary>
    /// Reads a byte the cartridge drives on the CPU bus, in $4020-$FFFF.
    /// </summary>
    /// <returns>
    /// <see langword="false"/> when the cartridge does not drive the bus at this address,
    /// in which case the caller keeps the open-bus value (REQ-CORE-081).
    /// </returns>
    public virtual bool TryCpuRead(ushort address, out byte value)
    {
        if (address is >= 0x6000 and <= 0x7FFF && PrgRamEnabled && _prgRam.Length > 0)
        {
            value = _prgRam[PrgRamOffset(address)];
            return true;
        }

        return TryReadPrg(address, out value);
    }

    /// <summary>
    /// Side-effect-free counterpart of <see cref="TryCpuRead"/>, used by the host memory
    /// accessor (REQ-CORE-080, REQ-CORE-081).
    /// </summary>
    public virtual bool TryCpuPeek(ushort address, out byte value) => TryCpuRead(address, out value);

    /// <summary>Writes a byte from the CPU bus, in $4020-$FFFF.</summary>
    public virtual void CpuWrite(ushort address, byte value)
    {
        if (address is >= 0x6000 and <= 0x7FFF && _prgRam.Length > 0)
        {
            if (PrgRamWritable)
            {
                _prgRam[PrgRamOffset(address)] = value;
            }

            return;
        }

        WriteRegister(address, value);
    }

    /// <summary>
    /// Index in <see cref="PrgRam"/> of CPU address <paramref name="address"/> in
    /// $6000-$7FFF. Boards that bank their PRG-RAM override it (MMC1 SOROM, MMC5).
    /// </summary>
    protected virtual int PrgRamOffset(ushort address) => (address - 0x6000) % _prgRam.Length;

    /// <summary>
    /// Reads PRG-ROM, or a readable mapper register, at <paramref name="address"/>
    /// (typically $8000-$FFFF).
    /// </summary>
    protected abstract bool TryReadPrg(ushort address, out byte value);

    /// <summary>
    /// Handles a write to a mapper register. The base implementation ignores it, which is
    /// right for boards without registers such as NROM.
    /// </summary>
    protected virtual void WriteRegister(ushort address, byte value)
    {
    }

    /// <summary>
    /// Reads CHR at <paramref name="address"/> ($0000-$1FFF) as the PPU would, side
    /// effects included (latches, A12 counting).
    /// </summary>
    public abstract byte PpuRead(ushort address, ChrFetch fetch);

    /// <summary>Reads CHR as a background fetch would, side effects included.</summary>
    public byte PpuRead(ushort address) => PpuRead(address, ChrFetch.Background);

    /// <summary>
    /// Reads CHR without any side effect. The tile emitter and the host accessors
    /// use this one (REQ-CORE-033).
    /// </summary>
    public virtual byte PpuPeek(ushort address, ChrFetch fetch) => PpuRead(address, fetch);

    /// <summary>Reads CHR as a background fetch would, without any side effect.</summary>
    public byte PpuPeek(ushort address) => PpuPeek(address, ChrFetch.Background);

    /// <summary>Writes CHR-RAM at <paramref name="address"/> ($0000-$1FFF).</summary>
    public abstract void PpuWrite(ushort address, byte value);

    /// <summary>
    /// Opaque state of the CHR latches of a board whose banks depend on the patterns fetched
    /// before (MMC2 and MMC4, REQ-CORE-039). Always 0 on every other board.
    /// </summary>
    /// <remarks>
    /// The tile emitter reads it, walks the line forward with
    /// <see cref="AdvanceChrLatch"/> so that each record carries the pattern the hardware
    /// would show at that column, then writes the saved value back: the emulation is
    /// unchanged whether or not records were built (REQ-CORE-033).
    /// </remarks>
    public virtual int ChrLatchState { get; set; }

    /// <summary>
    /// Moves the CHR latches on as the fetch of pattern <paramref name="tile"/> from
    /// <paramref name="patternTable"/> ($0000 or $1000) would (REQ-CORE-039).
    /// </summary>
    public virtual void AdvanceChrLatch(int patternTable, int tile)
    {
    }

    /// <summary>
    /// Signals that the PPU put <paramref name="address"/> on its address bus during real
    /// rendering or a $2006/$2007 access. Boards that count A12 rises (MMC3 and compatibles)
    /// hook their scanline counter here. Never called for a peek (REQ-CORE-033).
    /// </summary>
    /// <param name="address">PPU address, 14 bits.</param>
    /// <param name="ppuClock">
    /// PPU dot count since power-on, which the A12 filter of MMC3-style boards needs to tell
    /// a real rise from the short dips of the fetch pipeline.
    /// </param>
    public virtual void OnPpuAddressBus(ushort address, long ppuClock)
    {
    }

    /// <summary>
    /// Signals a write to PPUCTRL ($2000). MMC5 needs it: the sprite size decides whether its
    /// two CHR register sets are used separately (nesdev, "MMC5").
    /// </summary>
    public virtual void OnPpuControl(byte control)
    {
    }

    /// <summary>
    /// Signals the start of a scanline to boards that count them outside the PPU address bus
    /// (MMC5, whose IRQ follows the rendered line, nesdev "MMC5").
    /// </summary>
    /// <param name="scanline">Line about to be rendered, 0 to 239 for a visible line.</param>
    /// <param name="renderingEnabled">Whether either layer is being rendered.</param>
    public virtual void OnPpuScanline(int scanline, bool renderingEnabled)
    {
    }

    /// <summary>
    /// Which physical 1 KiB page of console VRAM the logical nametable
    /// <paramref name="table"/> (0 to 3) shows.
    /// </summary>
    /// <returns>
    /// A page number 0 to 3, <see cref="FollowMirroring"/> when the layout is the one
    /// <see cref="Mirroring"/> describes, or <see cref="ServedByCartridge"/> when the
    /// cartridge supplies the bytes itself.
    /// </returns>
    public virtual int NametablePage(int table) => FollowMirroring;

    /// <summary>Reads a nametable byte the cartridge serves itself (MMC5 ExRAM, fill mode).</summary>
    public virtual byte ReadNametable(ushort address) => 0;

    /// <summary>Writes a nametable byte the cartridge serves itself (MMC5 ExRAM).</summary>
    public virtual void WriteNametable(ushort address, byte value)
    {
    }

    /// <summary>Advances the cartridge by one CPU cycle.</summary>
    public void StepCpuCycle()
    {
        CpuCycle++;
        OnCpuCycle();
    }

    /// <summary>
    /// Advances cycle-counted mapper logic by one CPU cycle (VRC, FCG and SS88006 IRQ
    /// timers).
    /// </summary>
    protected virtual void OnCpuCycle()
    {
    }

    /// <summary>
    /// Returns the cartridge to its power-on state (REQ-CORE-004). PRG-RAM is cleared by
    /// the core, not here, so that a host which reloaded battery RAM can keep it
    /// (REQ-CORE-095).
    /// </summary>
    /// <remarks>
    /// Every mapper overrides it to put its registers back to their power-on values and
    /// recompute its bank windows; the core calls it right after the cartridge is built and
    /// on every reset, so a cartridge is never used before it has been powered on.
    /// </remarks>
    public virtual void Reset()
    {
        Mirroring = Rom.Header.Mirroring;
        CpuCycle = 0;
    }

    /// <summary>Clears PRG-RAM and CHR-RAM (part of power-on, REQ-CORE-004).</summary>
    public void ClearVolatileRam(bool clearPrgRam)
    {
        if (clearPrgRam)
        {
            Array.Clear(_prgRam);
            LoadTrainer();
        }

        Array.Clear(_chrRam);
    }

    /// <summary>Appends the mapper state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public virtual void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);
        writer.WriteBlob(_prgRam);
        writer.WriteBlob(_chrRam);
        writer.WriteByte((byte)Mirroring);
        writer.WriteInt64(CpuCycle);
    }

    /// <summary>Restores the mapper state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public virtual void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);
        reader.ReadBlobInto(_prgRam);
        reader.ReadBlobInto(_chrRam);
        Mirroring = (NametableMirroring)reader.ReadByte();
        CpuCycle = reader.ReadInt64();
    }

    /// <summary>
    /// Reads CHR-ROM if present, CHR-RAM otherwise, with the address wrapped to the
    /// available size. Helper for the simple boards.
    /// </summary>
    protected byte ReadChrDirect(int offset)
    {
        ReadOnlySpan<byte> chrRom = Rom.ChrRom;
        if (chrRom.Length > 0)
        {
            return chrRom[(int)((uint)offset % (uint)chrRom.Length)];
        }

        return _chrRam.Length > 0 ? _chrRam[(int)((uint)offset % (uint)_chrRam.Length)] : (byte)0;
    }

    /// <summary>Writes CHR-RAM, ignoring writes when the cartridge has CHR-ROM.</summary>
    protected void WriteChrDirect(int offset, byte value)
    {
        if (Rom.ChrRom.Length == 0 && _chrRam.Length > 0)
        {
            _chrRam[(int)((uint)offset % (uint)_chrRam.Length)] = value;
        }
    }
}
