using Zax.Nes.Apu;
using Zax.Nes.Cartridges;
using Zax.Nes.Cpu;
using Zax.Nes.Input;
using Zax.Nes.Ppu;
using Zax.Nes.Rom;
using Zax.Nes.State;

namespace Zax.Nes;

/// <summary>
/// One NES machine, seen from the host (REQ-CORE-001, REQ-CORE-002).
/// </summary>
/// <remarks>
/// <para>
/// Everything an instance needs lives in the instance: no static mutable state anywhere in
/// the assembly, so two cores in one process are independent (REQ-ARCH-011, REQ-CORE-001).
/// Nothing advances unless the host calls <see cref="RunFrame(ushort)"/>: the core owns no
/// clock and no thread (REQ-ARCH-010).
/// </para>
/// <para>
/// <strong>Lot boundaries.</strong> L1a delivered the interface, ROM loading, the CPU, the
/// bus, the input ports, mapper 0 and savestates; L1b added PPU rendering, the tile emission
/// contract of SPEC-02 §3.5 and the Zapper; L1c the APU and the audio output path; L1d the
/// mappers of REQ-CORE-100, which completes the core.
/// </para>
/// </remarks>
public sealed class NesCore
{
    /// <summary>
    /// Name of the core, recorded at the head of savestate files (REQ-CORE-120). There is
    /// exactly one core.
    /// </summary>
    public const string CoreName = "Zaxnes";

    /// <summary>
    /// The name the core carried under the « Sen » working name. Still accepted when reading a
    /// state block, never written: AMD-029 §2 keeps states written before the rename loadable,
    /// and the file signature alone would not be enough since this name is checked too (Q-070).
    /// </summary>
    public const string LegacyCoreName = "Sen";

    /// <summary>Message a savestate written by a different core is refused with (REQ-CORE-120).</summary>
    public const string ForeignCoreMessage = "Save state used a different core.";

    /// <summary>
    /// Magic that opens a core savestate block. Kept as it is across the rename of AMD-029: it
    /// is a format datum, like the <c>_3dsen_</c> cover infix §2 keeps on purpose, and changing
    /// it would refuse every block written before (Q-070).
    /// </summary>
    internal const uint StateMagic = 0x304E4553; // "SEN0"

    /// <summary>
    /// Version of the savestate format. Bumped whenever the block layout changes; L1c added
    /// the APU channel, mixer and resampler state and L1d the mapper bank windows, latches and
    /// IRQ counters (REQ-CORE-090).
    /// </summary>
    internal const int StateVersion = 4;

    /// <summary>
    /// The savestate format version, as a host writes it in the metadata header of a savestate
    /// file (SPEC-09 REQ-PERS-056 point 2).
    /// </summary>
    public static int StateFormatVersion => StateVersion;

    private readonly Ppu2C02 _ppu = new();
    private readonly Apu2A03 _apu = new();
    private readonly InputPorts _input = new();
    private readonly NesBus _bus;
    private readonly Cpu6502 _cpu;

    private Cartridge? _cartridge;
    private RomImage? _rom;
    private NesRegion _region = NesRegion.Ntsc;
    private bool _batteryRamImported;

    /// <summary>Creates an initialised, stopped instance. No emulation runs before a ROM is loaded.</summary>
    public NesCore()
    {
        _bus = new NesBus(_ppu, _apu, _input);
        _cpu = new Cpu6502(_bus);
        _input.LightProbe = ProbeZapperLight;
        ApplyRegion();
    }

    /// <summary>
    /// Receives one frame batch per emulated frame (REQ-CORE-030). Setting it to
    /// <see langword="null"/> unregisters it; the emulation must behave identically either
    /// way (REQ-CORE-033).
    /// </summary>
    public VideoCallback? VideoCallback { get; set; }

    /// <summary>
    /// Supplies the Zapper's aimed position, queried once per frame for each Zapper port
    /// (REQ-CORE-060).
    /// </summary>
    public ZapperPositionSource? ZapperPositionSource { get; set; }

    /// <summary>Whether a ROM is loaded and frames can run (REQ-CORE-005).</summary>
    public bool IsRunning => _cartridge is not null;

    /// <summary>Name of the loaded ROM, empty when none is loaded (REQ-CORE-014).</summary>
    public string RomName => _rom?.Name ?? string.Empty;

    /// <summary>Frame counter; +1 per emulated frame (REQ-CORE-006).</summary>
    public long FrameCount { get; set; }

    /// <summary>Master CPU cycle count since power-on.</summary>
    public long CycleCount => _bus.CycleCount;

    /// <summary>Tile grid dimensions: always (33, 31, 128) (REQ-CORE-031).</summary>
    public static VideoInfo VideoInfo => VideoInfo.Standard;

    /// <summary>The CPU, exposed for the trace comparison of REQ-CORE-110.</summary>
    public Cpu6502 Cpu => _cpu;

    /// <summary>The bus, exposed for tracing and for the tests.</summary>
    public NesBus Bus => _bus;

    /// <summary>The PPU: rendering, the tile contract and the Zapper probe.</summary>
    public Ppu2C02 Ppu => _ppu;

    /// <summary>The APU: the five channels, the frame counter and the audio output path.</summary>
    public Apu2A03 Apu => _apu;

    /// <summary>The controller ports.</summary>
    public InputPorts Input => _input;

    /// <summary>The cartridge, or <see langword="null"/> when no ROM is loaded.</summary>
    public Cartridge? Cartridge => _cartridge;

    /// <summary>Mapper numbers this build accepts (REQ-CORE-100, REQ-CORE-102).</summary>
    public static IReadOnlyList<int> SupportedMappers => MapperRegistry.SupportedMappers;

    // ---------------------------------------------------------------- lifetime

    /// <summary>
    /// Loads a ROM image (REQ-CORE-010, REQ-CORE-011).
    /// </summary>
    /// <param name="image">The bytes of a <c>.nes</c> file; archives are the host's problem.</param>
    /// <param name="name">ROM name to keep (REQ-CORE-014).</param>
    /// <returns>
    /// The detected region on success (REQ-CORE-013) — which the core does <em>not</em>
    /// apply by itself — or the cause of the failure.
    /// </returns>
    /// <remarks>
    /// On failure the instance is left stopped (REQ-CORE-011, Q-002). On success the
    /// machine is in its power-on state with the frame counter at 0 (REQ-CORE-003).
    /// </remarks>
    // Q-002: REQ-CORE-011 allows either "without changing the instance" or "leaving it
    // stopped"; the second branch is taken, because REQ-CORE-091 requires exactly that for
    // the other failure path and a half-loaded instance would contradict REQ-CORE-014.
    public RomLoadResult LoadRom(ReadOnlySpan<byte> image, string name)
    {
        ArgumentNullException.ThrowIfNull(name);

        // AMD-002 (answer to Q-001): the failure carries a typed cause and a technical
        // message; the text shown to the user is the host's business.
        if (!RomImage.TryLoad(image, name, out RomImage? rom, out RomLoadFailure failure, out string? error))
        {
            CloseRom();
            return RomLoadResult.Fail(failure, error!);
        }

        if (!MapperRegistry.TryCreate(
                rom!,
                out Cartridge? cartridge,
                out RomLoadFailure mapperFailure,
                out string? mapperError))
        {
            CloseRom();
            return RomLoadResult.Fail(mapperFailure, mapperError!);
        }

        _rom = rom;
        _cartridge = cartridge;
        _batteryRamImported = false;
        _bus.Attach(cartridge);

        PowerOn();
        return RomLoadResult.Ok(rom!.AnnouncedRegion);
    }

    /// <summary>
    /// Stops the emulation (REQ-CORE-005). "Run one frame" then does nothing until a ROM or
    /// a savestate is loaded again.
    /// </summary>
    public void CloseRom()
    {
        _cartridge = null;
        _rom = null;
        _batteryRamImported = false;
        _bus.Attach(null);
        _apu.DiscardSamples();
    }

    /// <summary>
    /// Resets the machine, which is equivalent to a power cycle (REQ-CORE-004,
    /// DEC-02-Q3): work RAM, PRG-RAM, nametable RAM, OAM and palette RAM cleared, mapper
    /// back to its initial state, APU silent, CPU restarted through the reset vector,
    /// frame counter at 0. The ROM and the port types are kept.
    /// </summary>
    /// <remarks>
    /// PRG-RAM is preserved when the host has imported battery RAM, so that a save the
    /// host persists is not lost (REQ-CORE-095).
    /// </remarks>
    public void Reset()
    {
        if (_cartridge is null)
        {
            return;
        }

        PowerOn();
    }

    private void PowerOn()
    {
        // Q-003: the region goes in first, so the PPU's power-on position is the start of
        // vblank of the region in force, whose scanline differs on DENDY (REQ-CORE-015,
        // REQ-CORE-020 notes). Without this a freshly loaded DENDY machine would owe a
        // transition frame.
        ApplyRegion();

        _bus.Reset();
        _ppu.Reset();
        _apu.Reset();
        _input.Reset();
        _cartridge!.Reset();
        _cartridge.ClearVolatileRam(clearPrgRam: !_batteryRamImported);
        _cpu.Reset();
        FrameCount = 0;
    }

    // ---------------------------------------------------------------- region

    /// <summary>
    /// TV region in force. Changing it between two frames takes effect on the next frame
    /// and does not reset the machine (REQ-CORE-016).
    /// </summary>
    /// <remarks>
    /// <para>
    /// The core never applies the region detected at load time by itself: the host decides,
    /// according to its "automatic region" setting (REQ-CORE-013).
    /// </para>
    /// <para>
    /// Q-003: the change does not move the scan, so switching to or from DENDY — whose
    /// vblank is on line 291 instead of 241 — costs one transition frame of a different
    /// length. The nominal durations of REQ-CORE-015 describe the steady state.
    /// </para>
    /// </remarks>
    public NesRegion Region
    {
        get => _region;
        set
        {
            _ = RegionTiming.For(value);
            _region = value;
            ApplyRegion();
        }
    }

    /// <summary>Timings of the region in force (REQ-CORE-015).</summary>
    public RegionTiming Timing => RegionTiming.For(_region);

    private void ApplyRegion()
    {
        RegionTiming timing = RegionTiming.For(_region);
        _ppu.Timing = timing;
        _apu.Timing = timing;
    }

    // ---------------------------------------------------------------- ports

    /// <summary>Returns the device type of <paramref name="port"/> (REQ-CORE-024).</summary>
    public ControllerPortType GetPortType(int port) => _input.GetPortType(port);

    /// <summary>
    /// Sets the device type of <paramref name="port"/>; allowed at any time between two
    /// frames (REQ-CORE-024).
    /// </summary>
    public void SetPortType(int port, ControllerPortType type) => _input.SetPortType(port, type);

    // ---------------------------------------------------------------- audio

    /// <summary>Output sample rate in Hz, 22 050 to 96 000 (REQ-CORE-070).</summary>
    public int SampleRate
    {
        get => _apu.SampleRate;
        set => _apu.SampleRate = value;
    }

    /// <summary>
    /// Speed factor: it only retimes the audio, never the emulation (REQ-CORE-072). Fast
    /// forward uses 1.5 (DEC-07-Q2).
    /// </summary>
    public double Speed
    {
        get => _apu.Speed;
        set => _apu.Speed = value;
    }

    /// <summary>Fine adjustment of the resampling ratio, ±0.5 % at most (REQ-CORE-070).</summary>
    public double AudioRatioAdjustment
    {
        get => _apu.RatioAdjustment;
        set => _apu.RatioAdjustment = value;
    }

    /// <summary>
    /// Fills <paramref name="destination"/> with audio, all or nothing (REQ-CORE-071). May
    /// be called from another thread than the one running frames.
    /// </summary>
    public int ReadAudio(Span<float> destination, bool stereo = false)
        => _apu.ReadSamples(destination, stereo);

    // ---------------------------------------------------------------- frames

    /// <summary>
    /// Runs one frame with <paramref name="inputWord"/> (REQ-CORE-020, REQ-CORE-022).
    /// </summary>
    /// <param name="inputWord">
    /// Port 1 in bits 0 to 7, port 2 in bits 8 to 15; for a controller the order is A, B,
    /// Select, Start, Up, Down, Left, Right (SPEC-02 §4.4).
    /// </param>
    /// <returns>
    /// The number of CPU cycles the frame took, or 0 when no ROM is loaded
    /// (REQ-CORE-005).
    /// </returns>
    /// <remarks>
    /// The frame runs from where the previous one stopped to the start of the next vblank.
    /// The video callback is then invoked once, synchronously (REQ-CORE-020,
    /// REQ-CORE-021).
    /// </remarks>
    public long RunFrame(ushort inputWord)
    {
        if (_cartridge is null)
        {
            return 0;
        }

        _input.BeginFrame(inputWord);

        // The aimed position is sampled once, at the start of the frame, and stays fixed
        // for its whole duration (REQ-CORE-060).
        if (ZapperPositionSource is { } source)
        {
            for (int port = 0; port < InputPorts.PortCount; port++)
            {
                if (_input.GetPortType(port) == ControllerPortType.Zapper)
                {
                    _input.SetZapperPosition(port, source(port));
                }
            }
        }

        long startCycle = _bus.CycleCount;

        _ppu.ClearFrameBoundary();
        do
        {
            _cpu.Step();
        }
        while (!_ppu.VBlankJustStarted);

        FrameCount++;

        // The callback is invoked once, synchronously, even when the batch is empty
        // (REQ-CORE-020, REQ-CORE-021).
        if (VideoCallback is { } callback)
        {
            var batch = new FrameBatch(
                _ppu.TileRecords,
                _ppu.SnapshotPalette(),
                _ppu.MaskX,
                _ppu.MaskY);
            callback(in batch);
        }

        return _bus.CycleCount - startCycle;
    }

    /// <summary>
    /// The optional 256 x 240 indexed image of the frame just run, one NES colour index per
    /// pixel (REQ-CORE-048). The "2D Mode" of SPEC-06 draws from it.
    /// </summary>
    public ReadOnlySpan<byte> PixelFrame => _ppu.PixelFrame;

    /// <summary>
    /// The tile records of the frame just run, same span the video callback received
    /// (REQ-CORE-030). Valid until the next frame runs.
    /// </summary>
    public ReadOnlySpan<TileRecord> TileRecords => _ppu.TileRecords;

    /// <summary>Unreliable left and right border width of the last frame (REQ-CORE-046).</summary>
    public int MaskX => _ppu.MaskX;

    /// <summary>Unreliable top and bottom border width of the last frame (REQ-CORE-046).</summary>
    public int MaskY => _ppu.MaskY;

    /// <summary>
    /// The Zapper's light detection (REQ-CORE-062), delegated to the PPU, which owns both
    /// the scan position and the tile records the displayed colour is composed from.
    /// </summary>
    private bool ProbeZapperLight(ZapperPosition position)
        => _ppu.SeesLightAt(position.X, position.Y);

    // ---------------------------------------------------------------- memory

    /// <summary>
    /// Reads a byte at a 16-bit CPU address, with no side effect (REQ-CORE-080). Callable
    /// between frames and from inside the video callback; this is Lua's <c>ReadMem</c>
    /// (SPEC-05).
    /// </summary>
    public byte ReadMemory(ushort address) => _bus.Peek(address);

    /// <summary>
    /// Writes a byte with the same effect a CPU write would have, without advancing
    /// emulated time (REQ-CORE-082). This is Lua's <c>WriteMem</c> (SPEC-05).
    /// </summary>
    public void WriteMemory(ushort address, byte value) => _bus.Poke(address, value);

    /// <summary>
    /// Battery-backed PRG-RAM, for a host that wants to persist it (REQ-CORE-095). Empty
    /// when the ROM has no battery.
    /// </summary>
    public ReadOnlySpan<byte> ExportBatteryRam()
        => _cartridge is null ? [] : _cartridge.BatteryRam;

    /// <summary>
    /// Loads battery-backed PRG-RAM (REQ-CORE-095). Once imported, a reset no longer
    /// clears it.
    /// </summary>
    /// <returns><see langword="false"/> when the ROM has no battery-backed PRG-RAM.</returns>
    public bool ImportBatteryRam(ReadOnlySpan<byte> data)
    {
        if (_cartridge is null)
        {
            return false;
        }

        Span<byte> target = _cartridge.BatteryRam;
        if (target.IsEmpty)
        {
            return false;
        }

        data[..Math.Min(data.Length, target.Length)].CopyTo(target);
        _batteryRamImported = true;
        return true;
    }

    // ---------------------------------------------------------------- savestates

    /// <summary>
    /// Returns a self-contained state block, ROM image and ROM name included
    /// (REQ-CORE-090).
    /// </summary>
    /// <param name="includeRomImage">
    /// Pass <see langword="false"/> for the in-memory rollback history of SPEC-07, which
    /// trades self-containment for size; such a block must never be written to disk
    /// (REQ-CORE-093).
    /// </param>
    /// <exception cref="InvalidOperationException">No ROM is loaded.</exception>
    public byte[] SaveState(bool includeRomImage = true)
    {
        if (_cartridge is null || _rom is null)
        {
            throw new InvalidOperationException("No ROM is loaded: there is no state to save.");
        }

        var writer = new StateWriter(_rom.Raw.Length + 32 * 1024);

        writer.WriteInt32(unchecked((int)StateMagic));
        writer.WriteInt32(StateVersion);
        writer.WriteString(CoreName);
        writer.WriteBool(includeRomImage);
        writer.WriteString(_rom.Name);
        writer.WriteBlob(includeRomImage ? _rom.Raw : []);
        writer.WriteByte((byte)_region);
        writer.WriteInt64(FrameCount);

        _bus.SaveState(writer);
        _cpu.SaveState(writer);
        _ppu.SaveState(writer);
        _apu.SaveState(writer);
        _input.SaveState(writer);
        _cartridge.SaveState(writer);

        return writer.ToArray();
    }

    /// <summary>
    /// Upper bound on the size of a state block for the ROM currently loaded, announced
    /// before serialisation and valid for the whole session (REQ-ARCH-013).
    /// </summary>
    public int MaxStateSize()
    {
        int romBytes = _rom?.Raw.Length ?? 0;
        int fixedBytes =
            NesBus.RamSize
            + Ppu2C02.VramSize + Ppu2C02.OamSize + Ppu2C02.PaletteSize
            + (64 * 1024) // PRG-RAM: 8 KiB usually, up to 64 KiB on MMC5.
            + InesHeader.DefaultChrRamSize
            + 1024 // MMC5 ExRAM.
            + Ppu2C02.MaxTileRecordStateSize
            + Apu2A03.MaxStateSize
            + 4096; // headers, registers, counters and the length prefixes

        return romBytes + fixedBytes + 256 + (_rom?.Name.Length * 4 ?? 0);
    }

    /// <summary>
    /// Restores a state block (REQ-CORE-091).
    /// </summary>
    /// <param name="data">Array holding the block, possibly with other data around it.</param>
    /// <param name="offset">Index the block starts at.</param>
    /// <param name="consumed">Bytes the block occupied, so the host can read on.</param>
    /// <param name="romName">The stored ROM name, empty on failure.</param>
    /// <param name="error">Cause of the failure, or <see langword="null"/> on success.</param>
    /// <returns>
    /// <see langword="false"/> on an invalid or truncated block, in which case the core is
    /// left stopped (REQ-CORE-005) and the name returned is empty.
    /// </returns>
    public bool TryLoadState(
        ReadOnlyMemory<byte> data,
        int offset,
        out int consumed,
        out string romName,
        out string? error)
        => TryLoadState(data, offset, out consumed, out romName, out _, out error);

    /// <summary>
    /// Restores a state block, reporting the typed cause of a failure as well as its
    /// message (REQ-CORE-091; AMD-002, answer to Q-001).
    /// </summary>
    public bool TryLoadState(
        ReadOnlyMemory<byte> data,
        int offset,
        out int consumed,
        out string romName,
        out StateLoadFailure failure,
        out string? error)
    {
        consumed = 0;
        romName = string.Empty;
        failure = StateLoadFailure.None;

        try
        {
            var reader = new StateReader(data, offset);

            if (unchecked((uint)reader.ReadInt32()) != StateMagic)
            {
                throw new InvalidStateException("Corrupt savestate: wrong magic number.");
            }

            int version = reader.ReadInt32();
            if (version != StateVersion)
            {
                throw new InvalidStateException(
                    $"Corrupt savestate: format version {version} is not supported (expected {StateVersion}).");
            }

            string coreName = reader.ReadString();
            if (!string.Equals(coreName, CoreName, StringComparison.Ordinal)
                && !string.Equals(coreName, LegacyCoreName, StringComparison.Ordinal))
            {
                // REQ-CORE-120 fixes this message to the character; it is one of the few
                // literal strings of the spec (AMD-002, answer to Q-001).
                failure = StateLoadFailure.ForeignCore;
                error = ForeignCoreMessage;
                CloseRom();
                return false;
            }

            bool hasRomImage = reader.ReadBool();
            string storedName = reader.ReadString();
            byte[] romBytes = reader.ReadBlob();
            var region = (NesRegion)reader.ReadByte();
            long frameCount = reader.ReadInt64();

            RomImage? rom = _rom;
            if (hasRomImage)
            {
                // The block is self-contained: it restores even with no ROM loaded
                // (REQ-CORE-090).
                if (!RomImage.TryLoad(romBytes, storedName, out rom, out string? romError))
                {
                    throw new InvalidStateException($"Corrupt savestate: {romError}");
                }
            }
            else if (rom is null)
            {
                throw new InvalidStateException(
                    "Corrupt savestate: the block carries no ROM image and none is loaded.");
            }

            if (!MapperRegistry.TryCreate(rom!, out Cartridge? cartridge, out string? mapperError))
            {
                throw new InvalidStateException($"Corrupt savestate: {mapperError}");
            }

            // Everything is read into the live objects only once the header checks out, so
            // that a truncation leaves the core stopped rather than half-restored.
            _bus.LoadState(reader);
            _cpu.LoadState(reader);
            _ppu.LoadState(reader);
            _apu.LoadState(reader);
            _input.LoadState(reader);
            cartridge!.LoadState(reader);

            _rom = rom;
            _cartridge = cartridge;
            _batteryRamImported = false;
            _bus.Attach(cartridge);
            _region = region;
            ApplyRegion();
            FrameCount = frameCount;
            _apu.DropPendingSamples(); // REQ-CORE-074, Q-006.

            consumed = reader.Consumed;
            romName = storedName;
            error = null;
            return true;
        }
        catch (InvalidStateException exception)
        {
            CloseRom();
            consumed = 0;
            romName = string.Empty;
            failure = exception.Failure;
            error = exception.Message;
            return false;
        }
    }
}
