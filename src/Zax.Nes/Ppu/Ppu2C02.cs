using Zax.Nes.Cartridges;
using Zax.Nes.State;

namespace Zax.Nes.Ppu;

/// <summary>
/// The PPU: frame timing, registers, VRAM, OAM, palette, rendering and the 3dSen tile
/// contract (REQ-CORE-015, REQ-CORE-020, SPEC-02 §3.5).
/// </summary>
/// <remarks>
/// <para>
/// Written from the public nesdev documentation ("PPU rendering", "PPU frame timing",
/// "PPU scrolling", "PPU sprite evaluation", "PPU registers", "PPU palettes"). The class is
/// split over three files: this one carries the registers, the PPU bus and the dot clock;
/// <c>Ppu2C02.Render.cs</c> carries the rendering pipeline and the 256 x 240 indexed frame;
/// <c>Ppu2C02.Tiles.cs</c> carries the tile-record emission of SPEC-02 §3.5 and the colour
/// composition the Zapper needs.
/// </para>
/// <para>
/// <strong>Open bus.</strong> The PPU has its own data-bus latch, separate from the CPU's
/// (nesdev, "PPU registers"; <c>ppu_open_bus</c>). Writing any register loads it; reads of
/// the write-only registers return it without refreshing it, and a bit that has not been
/// refreshed for about 600 ms decays to 0.
/// </para>
/// </remarks>
public sealed partial class Ppu2C02
{
    /// <summary>Size of nametable VRAM: 4 KiB, enough for four-screen mode (REQ-CORE-090).</summary>
    public const int VramSize = 4096;

    /// <summary>Size of OAM.</summary>
    public const int OamSize = 256;

    /// <summary>Size of palette RAM, in bytes (REQ-CORE-045).</summary>
    public const int PaletteSize = 32;

    /// <summary>Width of the optional 2D image, in pixels (REQ-CORE-048).</summary>
    public const int ScreenWidth = 256;

    /// <summary>Height of the optional 2D image, in pixels (REQ-CORE-048).</summary>
    public const int ScreenHeight = RegionTiming.VisibleScanlines;

    /// <summary>
    /// PPU dots after which an unrefreshed bit of the data-bus latch decays to 0. About
    /// 600 ms at the NTSC dot rate (nesdev; <c>ppu_open_bus</c> requires the decay to have
    /// happened within one second).
    /// </summary>
    private const long DecayDots = 3_200_000;

    private readonly byte[] _vram = new byte[VramSize];
    private readonly byte[] _oam = new byte[OamSize];
    private readonly byte[] _palette = new byte[PaletteSize];
    private readonly byte[] _paletteOut = new byte[PaletteSize];
    private readonly long[] _decayStamp = new long[8];

    private Cartridge? _cartridge;
    private RegionTiming _timing = RegionTiming.For(NesRegion.Ntsc);

    private int _dot;
    private int _scanline;
    private int _dotAccumulator;
    private bool _oddFrame;
    private long _ppuClock;

    private byte _control;
    private byte _mask;
    private bool _vblankFlag;
    private bool _spriteZeroHit;
    private bool _spriteOverflow;
    private bool _suppressVBlank;
    private byte _oamAddress;

    private ushort _v;
    private ushort _t;

    /// <summary>
    /// Shadow of <see cref="_v"/> that ignores the coarse-X increments of the tile fetch
    /// pipeline, so that it always holds the scroll of the <em>left edge</em> of the next
    /// line to be drawn. The tile capture of REQ-CORE-037 reads it; <see cref="_v"/> itself
    /// is already two tiles ahead by the time a visible line starts, because dots 321 to 336
    /// of the previous line prefetch its first two tiles.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Q-004: REQ-CORE-038 asks for "cx, cy, nametable" without saying at which instant to
    /// read them, and the raw register would place every background row 16 pixels to the
    /// left. It is refreshed wherever the hardware reloads the address — the horizontal
    /// reload at dot 257, the vertical reload at dots 280 to 304 of the pre-render line, the
    /// second $2006 write and the $2007 auto-increment — but never by a coarse-X increment.
    /// </para>
    /// <para>
    /// AMD-021 R2-005: the row of line L reflects the state <em>after</em> every CPU write
    /// made during line L, so the two reloads the hardware performs at the end of the line —
    /// the vertical increment at dot 256 and the horizontal reload at dot 257 — must not
    /// reach it. They land in <see cref="_vRowNext"/> instead, which becomes this value once
    /// the line is over.
    /// </para>
    /// </remarks>
    private ushort _vRowStart;

    /// <summary>
    /// Row-start shadow the <em>next</em> line will use: where the end-of-line reloads of the
    /// hardware go (AMD-021 R2-005).
    /// </summary>
    private ushort _vRowNext;

    private byte _fineX;
    private bool _writeToggle;
    private byte _readBuffer;
    private byte _decay;
    private bool _skipLatch;

    /// <summary>Attaches <paramref name="cartridge"/> as the CHR source.</summary>
    public void Attach(Cartridge? cartridge) => _cartridge = cartridge;

    /// <summary>Timings in force. Changing region takes effect on the next frame (REQ-CORE-016).</summary>
    public RegionTiming Timing
    {
        get => _timing;
        set
        {
            ArgumentNullException.ThrowIfNull(value);
            _timing = value;
        }
    }

    /// <summary>Current dot inside the scanline, 0 to 340.</summary>
    public int Dot => _dot;

    /// <summary>Current scanline; 0 to 239 are visible, the last one is the pre-render line.</summary>
    public int Scanline => _scanline;

    /// <summary>Index of the pre-render line of the region in force.</summary>
    public int PreRenderScanline => _timing.ScanlinesPerFrame - 1;

    /// <summary>The VBlank flag, bit 7 of PPUSTATUS.</summary>
    public bool VBlankFlag => _vblankFlag;

    /// <summary>Sprite 0 hit, bit 6 of PPUSTATUS (REQ-CORE-112).</summary>
    public bool SpriteZeroHit => _spriteZeroHit;

    /// <summary>Sprite overflow, bit 5 of PPUSTATUS (REQ-CORE-116).</summary>
    public bool SpriteOverflow => _spriteOverflow;

    /// <summary>
    /// Set for one step when the frame boundary is crossed; "run one frame" stops on it
    /// (REQ-CORE-020).
    /// </summary>
    /// <remarks>
    /// It marks the start of vblank whether or not the VBlank <em>flag</em> was actually
    /// raised: a read of $2002 one dot earlier suppresses the flag (nesdev,
    /// <c>vbl_nmi_timing</c> test 2) but must not stop the host loop from seeing the frame
    /// boundary.
    /// </remarks>
    public bool VBlankJustStarted { get; private set; }

    /// <summary>Whether /NMI is being pulled low right now: VBlank flag set and NMI enabled.</summary>
    public bool NmiAsserted => _vblankFlag && (_control & 0x80) != 0;

    /// <summary>
    /// The /NMI line as the CPU sees it: sampled near the <em>start</em> of the CPU cycle,
    /// two PPU dots before the register access of that cycle takes effect.
    /// </summary>
    /// <remarks>
    /// The 6502 latches /NMI during the cycle, not at its end, so a write that disables NMI
    /// or a read of $2002 that clears the VBlank flag cannot undo an assertion that happened
    /// two dots or more earlier. That offset is what the one-PPU-clock resolution of
    /// <c>ppu_vbl_nmi</c> tests 5 to 8 and <c>vbl_nmi_timing</c> tests 5 to 7 measures
    /// (REQ-CORE-116).
    /// </remarks>
    public bool NmiLineSample { get; private set; }

    /// <summary>
    /// PPU dots between the /NMI sampling point and the end of the CPU cycle
    /// (see <see cref="NmiLineSample"/>).
    /// </summary>
    private const int NmiSampleLead = 2;

    /// <summary>
    /// Dot of the pre-render line at which PPUMASK decides whether the idle dot (0, 0) of
    /// the next frame is skipped (REQ-CORE-015).
    /// </summary>
    /// <remarks>
    /// nesdev describes the effect ("with rendering enabled, each odd PPU frame is one PPU
    /// clock shorter: the first idle tick of the first visible scanline is skipped") but not
    /// the dot at which the decision is latched. <c>ppu_vbl_nmi</c> test 10 measures it to
    /// one PPU clock by enabling and disabling background rendering around it, and it only
    /// passes with the mask read two dots before the end of the pre-render line.
    /// </remarks>
    private const int SkipDecisionDot = 338;

    /// <summary>PPUCTRL ($2000).</summary>
    public byte Control => _control;

    /// <summary>PPUMASK ($2001).</summary>
    public byte Mask => _mask;

    /// <summary>Whether background rendering is on (bit 3 of PPUMASK, REQ-CORE-037).</summary>
    public bool BackgroundEnabled => (_mask & 0x08) != 0;

    /// <summary>Whether sprite rendering is on (bit 4 of PPUMASK, REQ-CORE-041).</summary>
    public bool SpritesEnabled => (_mask & 0x10) != 0;

    /// <summary>Whether either layer is being rendered, which gates the odd-frame dot skip.</summary>
    public bool RenderingEnabled => (_mask & 0x18) != 0;

    /// <summary>
    /// Whether the leftmost column is masked for background or sprites (bits 1 and 2 of
    /// PPUMASK), which forces maskX to at least 8 (REQ-CORE-046).
    /// </summary>
    public bool LeftColumnMasked => (_mask & 0x02) == 0 || (_mask & 0x04) == 0;

    /// <summary>The nametable layout the cartridge currently selects.</summary>
    public NametableMirroring Mirroring => _cartridge?.Mirroring ?? NametableMirroring.Horizontal;

    /// <summary>Object attribute memory.</summary>
    public ReadOnlySpan<byte> Oam => _oam;

    /// <summary>Nametable VRAM, 4 KiB.</summary>
    public ReadOnlySpan<byte> Vram => _vram;

    /// <summary>OAM address register ($2003).</summary>
    public byte OamAddress => _oamAddress;

    /// <summary>Coarse X of the internal scroll register (SPEC-02 §2).</summary>
    public int CoarseX => _v & 0x1F;

    /// <summary>Coarse Y of the internal scroll register.</summary>
    public int CoarseY => (_v >> 5) & 0x1F;

    /// <summary>Nametable selection bits of the internal scroll register.</summary>
    public int NametableSelect => (_v >> 10) & 0x03;

    /// <summary>Fine Y scroll of the internal scroll register.</summary>
    public int FineY => (_v >> 12) & 0x07;

    /// <summary>Fine X scroll (the "x" register of nesdev).</summary>
    public int FineX => _fineX;

    /// <summary>
    /// Returns the 32 palette bytes of the batch, mirrors filled in (REQ-CORE-045).
    /// </summary>
    public ReadOnlySpan<byte> SnapshotPalette()
    {
        _palette.CopyTo(_paletteOut, 0);

        // $3F10, $3F14, $3F18 and $3F1C mirror $3F00, $3F04, $3F08 and $3F0C (nesdev).
        _paletteOut[0x10] = _palette[0x00];
        _paletteOut[0x14] = _palette[0x04];
        _paletteOut[0x18] = _palette[0x08];
        _paletteOut[0x1C] = _palette[0x0C];
        return _paletteOut;
    }

    /// <summary>
    /// Returns the machine to its power-on state: nametable RAM, OAM and palette RAM
    /// cleared (REQ-CORE-004). The palette starts at 0, which is why the public
    /// <c>power_up_palette</c> test is excluded (REQ-CORE-112).
    /// </summary>
    public void Reset()
    {
        Array.Clear(_vram);
        Array.Clear(_oam);
        Array.Clear(_palette);
        Array.Clear(_paletteOut);
        Array.Clear(_decayStamp);
        Array.Clear(_pixels);

        _dot = 0;
        _dotAccumulator = 0;
        _oddFrame = false;
        _ppuClock = 0;
        _control = 0;
        _mask = 0;
        _vblankFlag = false;
        _spriteZeroHit = false;
        _spriteOverflow = false;
        _suppressVBlank = false;
        _oamAddress = 0;
        _v = 0;
        _t = 0;
        _vRowStart = 0;
        _vRowNext = 0;
        _fineX = 0;
        _writeToggle = false;
        _readBuffer = 0;
        _decay = 0;
        _skipLatch = false;
        VBlankJustStarted = false;
        NmiLineSample = false;

        ResetRenderState();
        ResetTileState();

        // Just after a load or a reset the first frame starts at the beginning of vblank,
        // so the reset lands there (REQ-CORE-020, notes).
        _scanline = _timing.VBlankScanline;
        _dot = 1;
    }

    /// <summary>Advances the PPU by the dots of one CPU cycle of the current region.</summary>
    public void StepCpuCycle()
    {
        int accumulator = _dotAccumulator + _timing.DotsPerCycleNumerator;
        int dots = accumulator / _timing.DotsPerCycleDenominator;
        _dotAccumulator = accumulator - (dots * _timing.DotsPerCycleDenominator);

        int sampleAfter = Math.Max(dots - NmiSampleLead, 0);
        for (int dot = 0; dot < dots; dot++)
        {
            if (dot == sampleAfter)
            {
                NmiLineSample = NmiAsserted;
            }

            StepDot();
        }
    }

    /// <summary>Clears the one-step frame boundary flag; the core calls this each frame.</summary>
    public void ClearFrameBoundary() => VBlankJustStarted = false;

    private void StepDot()
    {
        AdvanceCounters();
        ProcessDot();
    }

    private void AdvanceCounters()
    {
        _ppuClock++;
        _dot++;

        if (_dot <= RegionTiming.DotsPerScanline - 1)
        {
            return;
        }

        _dot = 0;
        _scanline++;

        if (_scanline < _timing.ScanlinesPerFrame)
        {
            return;
        }

        _scanline = 0;

        if (_oddFrame && _timing.SkipsDotOnOddFrames && _skipLatch)
        {
            // With rendering on, the idle dot (0, 0) is skipped on odd frames, which makes
            // the frame one dot shorter (NTSC only, REQ-CORE-015). The mask is read here,
            // at the dot that is dropped, and not one dot earlier: that single dot is what
            // ppu_vbl_nmi test 10 measures.
            _dot = 1;
        }

        _oddFrame = !_oddFrame;
    }

    private void ProcessDot()
    {
        if (_scanline == PreRenderScanline && _dot == SkipDecisionDot)
        {
            _skipLatch = RenderingEnabled;
        }

        if (_dot == 1)
        {
            if (_scanline == _timing.VBlankScanline)
            {
                // The frame boundary always fires; the flag itself can be suppressed by a
                // read of $2002 one dot earlier (nesdev, "PPU frame timing").
                VBlankJustStarted = true;
                if (!_suppressVBlank)
                {
                    _vblankFlag = true;
                }

                _suppressVBlank = false;
            }
            else if (_scanline == PreRenderScanline)
            {
                _vblankFlag = false;
                _spriteZeroHit = false;
                _spriteOverflow = false;
                _suppressVBlank = false;
                BeginRenderedFrame();
            }

            // Boards that count rendered lines rather than A12 rises (MMC5) are told where
            // the scan is.
            _cartridge?.OnPpuScanline(_scanline, RenderingEnabled);
        }

        // The sprite captures of SPEC-02 §3.5 run on the first dot of the visible line that
        // is always reached: the idle dot 0 of line 0 is dropped on a skipped odd frame.
        if (_scanline < RegionTiming.VisibleScanlines && _dot == 1)
        {
            BeginCaptureLine(_scanline);
        }

        RunRenderDot();

        // The background row of line L is captured once the line is over, so that it shows
        // the state left by every CPU write made during the line (R2-005). The two
        // end-of-line reloads of the hardware are kept out of it by _vRowNext, which only
        // becomes the row start of the line that follows.
        if (_dot == RegionTiming.DotsPerScanline - 1)
        {
            if (_scanline < RegionTiming.VisibleScanlines)
            {
                EndCaptureLine(_scanline);
            }

            _vRowStart = _vRowNext;
        }
    }

    // ---------------------------------------------------------------- registers

    /// <summary>Reads one of the eight registers, side effects included.</summary>
    /// <param name="register">Register number, 0 to 7 ($2000 + register).</param>
    public byte Read(int register)
    {
        ApplyDecay();

        switch (register & 0x07)
        {
            case 2:
            {
                // Bits 0 to 4 of PPUSTATUS come from the PPU's own bus latch and are not
                // refreshed by the read; bits 5 to 7 are the flags and do refresh it
                // (ppu_open_bus tests 6 and 7).
                var status = (byte)(_decay & 0x1F);
                if (_vblankFlag)
                {
                    status |= 0x80;
                }

                if (_spriteZeroHit)
                {
                    status |= 0x40;
                }

                if (_spriteOverflow)
                {
                    status |= 0x20;
                }

                RefreshDecay(status, 0xE0);

                // Reading the flag one dot before it would be raised suppresses it for the
                // whole frame (nesdev; vbl_nmi_timing test 2, case 8).
                if (_scanline == _timing.VBlankScanline && _dot == 0)
                {
                    _suppressVBlank = true;
                }

                _vblankFlag = false;
                _writeToggle = false;
                return status;
            }

            case 4:
            {
                byte value = ReadOam();
                RefreshDecay(value, 0xFF);
                return value;
            }

            case 7:
            {
                var address = (ushort)(_v & 0x3FFF);
                byte value;
                if (IsPaletteAddress(address))
                {
                    // Palette reads are immediate and only drive six bits; the buffer still
                    // takes the mirrored nametable byte "underneath" the palette (nesdev).
                    value = (byte)((_palette[PaletteIndex(address)] & 0x3F) | (_decay & 0xC0));
                    RefreshDecay(value, 0x3F);
                    _readBuffer = ReadVramThroughBus((ushort)(address & 0x2FFF), PpuBusAccess.Cpu);
                }
                else
                {
                    value = _readBuffer;
                    RefreshDecay(value, 0xFF);
                    _readBuffer = ReadVramThroughBus(address, PpuBusAccess.Cpu);
                }

                IncrementVramAddress();
                return value;
            }

            default:
                // $2000, $2001, $2003, $2005 and $2006 are write-only: the read returns the
                // PPU's bus latch and does not refresh it (ppu_open_bus tests 2 and 5).
                return _decay;
        }
    }

    /// <summary>
    /// Reads a register without any side effect, for the host memory accessor
    /// (REQ-CORE-081).
    /// </summary>
    public byte Peek(int register) => (register & 0x07) switch
    {
        2 => (byte)((_decay & 0x1F)
            | (_vblankFlag ? 0x80 : 0x00)
            | (_spriteZeroHit ? 0x40 : 0x00)
            | (_spriteOverflow ? 0x20 : 0x00)),
        4 => _oam[_oamAddress],
        7 => IsPaletteAddress((ushort)(_v & 0x3FFF))
            ? _palette[PaletteIndex((ushort)(_v & 0x3FFF))]
            : _readBuffer,
        _ => _decay,
    };

    /// <summary>Writes one of the eight registers.</summary>
    /// <param name="register">Register number, 0 to 7 ($2000 + register).</param>
    /// <param name="value">Byte written.</param>
    public void Write(int register, byte value)
    {
        ApplyDecay();

        // A write to any PPU register, $2002 included, loads the whole bus latch
        // (ppu_open_bus test 2).
        RefreshDecay(value, 0xFF);

        switch (register & 0x07)
        {
            case 0:
                _control = value;
                _t = (ushort)((_t & 0xF3FF) | ((value & 0x03) << 10));
                _cartridge?.OnPpuControl(value);
                break;

            case 1:
                _mask = value;
                break;

            case 3:
                _oamAddress = value;
                break;

            case 4:
                WriteOam(value);
                break;

            case 5:
                if (!_writeToggle)
                {
                    _t = (ushort)((_t & 0xFFE0) | (value >> 3));
                    _fineX = (byte)(value & 0x07);
                }
                else
                {
                    _t = (ushort)((_t & 0x8FFF) | ((value & 0x07) << 12));
                    _t = (ushort)((_t & 0xFC1F) | ((value & 0xF8) << 2));
                }

                _writeToggle = !_writeToggle;
                break;

            case 6:
                if (!_writeToggle)
                {
                    _t = (ushort)((_t & 0x00FF) | ((value & 0x3F) << 8));
                }
                else
                {
                    _t = (ushort)((_t & 0xFF00) | value);
                    _v = _t;

                    // A CPU write reaches the row of the line it is made in (R2-005).
                    _vRowStart = _v;
                    _vRowNext = _v;

                    // The new address reaches the cartridge: this is how a game clocks an
                    // MMC3 counter by hand, and what mmc3_test measures (REQ-CORE-114).
                    NotifyAddressBus((ushort)(_v & 0x3FFF));
                }

                _writeToggle = !_writeToggle;
                break;

            case 7:
                WriteVramThroughBus((ushort)(_v & 0x3FFF), value, PpuBusAccess.Cpu);
                IncrementVramAddress();
                break;

            default:
                // $2002 is read-only; the write only loaded the bus latch.
                break;
        }
    }

    /// <summary>Writes one OAM byte, used by OAM DMA ($4014).</summary>
    public void WriteOamDma(byte value) => WriteOam(value);

    /// <summary>
    /// Reads OAM through $2004.
    /// </summary>
    /// <remarks>
    /// Bits 2 to 4 of a sprite's attribute byte do not exist in the chip, so the masked
    /// value is what was stored (see <see cref="WriteOam"/>). During the 64 dots in which
    /// the secondary OAM is cleared, reads return $FF (nesdev, "PPU sprite evaluation").
    /// </remarks>
    private byte ReadOam()
    {
        if (RenderingEnabled
            && _scanline < RegionTiming.VisibleScanlines
            && _dot is >= 1 and <= 64)
        {
            return 0xFF;
        }

        return _oam[_oamAddress];
    }

    /// <summary>
    /// Writes OAM at the current address and post-increments it.
    /// </summary>
    /// <remarks>
    /// The three unimplemented bits of a sprite's byte 2 are dropped, so they read back as
    /// 0 (nesdev, "PPU OAM"; <c>oam_read</c> and <c>ppu_open_bus</c> test 10).
    /// </remarks>
    private void WriteOam(byte value)
    {
        _oam[_oamAddress] = (_oamAddress & 0x03) == 0x02 ? (byte)(value & 0xE3) : value;
        _oamAddress++;
    }

    private void IncrementVramAddress()
    {
        _v = (ushort)((_v + ((_control & 0x04) != 0 ? 32 : 1)) & 0x7FFF);

        // A CPU write reaches the row of the line it is made in (R2-005).
        _vRowStart = _v;
        _vRowNext = _v;

        // The incremented address goes back on the PPU bus, so a $2007 access that steps
        // across $1000 clocks an MMC3 counter — which is what mmc3_irq_tests "A12 clocking"
        // measures (tests 5 and 6).
        NotifyAddressBus((ushort)(_v & 0x3FFF));
    }

    // ---------------------------------------------------------------- bus latch decay

    private void ApplyDecay()
    {
        for (int bit = 0; bit < 8; bit++)
        {
            if (_ppuClock - _decayStamp[bit] > DecayDots)
            {
                _decay &= (byte)~(1 << bit);
            }
        }
    }

    private void RefreshDecay(byte value, byte mask)
    {
        _decay = (byte)((_decay & ~mask) | (value & mask));

        for (int bit = 0; bit < 8; bit++)
        {
            if ((mask & (1 << bit)) != 0)
            {
                _decayStamp[bit] = _ppuClock;
            }
        }
    }

    // ---------------------------------------------------------------- PPU memory

    /// <summary>
    /// Reads the PPU bus.
    /// </summary>
    /// <param name="address">PPU address, masked to 14 bits.</param>
    /// <param name="access">
    /// Why the access happens. <see cref="PpuBusAccess.Render"/> and
    /// <see cref="PpuBusAccess.Cpu"/> put the address on the cartridge's bus and may have
    /// mapper side effects; the tile emitter passes <see cref="PpuBusAccess.Inert"/>
    /// (REQ-CORE-033).
    /// </param>
    /// <param name="fetch">Which pattern fetch the access belongs to (MMC5 bank sets).</param>
    public byte ReadVramThroughBus(
        ushort address,
        PpuBusAccess access,
        ChrFetch fetch = ChrFetch.Background)
    {
        address &= 0x3FFF;

        if (access != PpuBusAccess.Inert)
        {
            NotifyAddressBus(address);
        }

        if (address < 0x2000)
        {
            if (_cartridge is null)
            {
                return 0;
            }

            return access == PpuBusAccess.Inert
                ? _cartridge.PpuPeek(address, fetch)
                : _cartridge.PpuRead(address, fetch);
        }

        if (!IsPaletteAddress(address))
        {
            return ReadNametableByte(address);
        }

        return _palette[PaletteIndex(address)];
    }

    /// <summary>Writes the PPU bus at <paramref name="address"/>.</summary>
    /// <param name="address">PPU address, masked to 14 bits.</param>
    /// <param name="value">Byte to write.</param>
    /// <param name="access">
    /// Why the access happens; a <see cref="PpuBusAccess.Cpu"/> write also puts the address
    /// on the cartridge's bus.
    /// </param>
    public void WriteVramThroughBus(ushort address, byte value, PpuBusAccess access = PpuBusAccess.Cpu)
    {
        address &= 0x3FFF;

        if (access != PpuBusAccess.Inert)
        {
            NotifyAddressBus(address);
        }

        if (address < 0x2000)
        {
            _cartridge?.PpuWrite(address, value);
            return;
        }

        if (!IsPaletteAddress(address))
        {
            WriteNametableByte(address, value);
            return;
        }

        // Palette entries are 6 bits wide.
        _palette[PaletteIndex(address)] = (byte)(value & 0x3F);
    }

    /// <summary>
    /// Tells the cartridge which address the PPU is driving, so that an MMC3-style board can
    /// count its A12 rises (REQ-CORE-114).
    /// </summary>
    private void NotifyAddressBus(ushort address) => _cartridge?.OnPpuAddressBus(address, _ppuClock);

    /// <summary>
    /// Reads a nametable byte, from console VRAM or from the cartridge when the board
    /// supplies its own nametable (MMC5 ExRAM and fill mode).
    /// </summary>
    /// <remarks>
    /// <para>
    /// There is no side effect to worry about here, so the tile capture of REQ-CORE-037 goes
    /// through the same path as rendering.
    /// </para>
    /// <para>
    /// Q-062: only a board that is present can answer <see cref="Cartridge.ServedByCartridge"/>,
    /// so the null-forgiving dereference this used to carry was unreachable. It is written as a
    /// test anyway — the dot loop must hold with no cartridge attached, whatever the page value,
    /// and nothing on this path may dereference without checking.
    /// </para>
    /// </remarks>
    public byte ReadNametableByte(ushort address)
    {
        int page = CartridgeNametablePage(address);
        if (page == Cartridge.ServedByCartridge && _cartridge is { } cartridge)
        {
            return cartridge.ReadNametable(address);
        }

        return page >= 0
            ? _vram[(page << 10) | (address & 0x03FF)]
            : _vram[MapNametable(address)];
    }

    private void WriteNametableByte(ushort address, byte value)
    {
        int page = CartridgeNametablePage(address);
        if (page == Cartridge.ServedByCartridge && _cartridge is { } cartridge)
        {
            cartridge.WriteNametable(address, value);
            return;
        }

        if (page >= 0)
        {
            _vram[(page << 10) | (address & 0x03FF)] = value;
            return;
        }

        _vram[MapNametable(address)] = value;
    }

    private int CartridgeNametablePage(ushort address)
        => _cartridge?.NametablePage(((address - 0x2000) >> 10) & 0x03) ?? Cartridge.FollowMirroring;

    /// <summary>Whether <paramref name="address"/> falls in palette RAM, $3F00-$3FFF.</summary>
    private static bool IsPaletteAddress(ushort address) => (address & 0x3F00) == 0x3F00;

    /// <summary>
    /// Maps a nametable address to an offset in the 4 KiB VRAM, using the layout the
    /// cartridge currently selects (SPEC-02 §2).
    /// </summary>
    public int MapNametable(ushort address)
    {
        int index = (address - 0x2000) & 0x0FFF;
        int table = index >> 10;
        int offset = index & 0x03FF;

        int physical = Mirroring switch
        {
            NametableMirroring.Horizontal => table >> 1,
            NametableMirroring.Vertical => table & 1,
            NametableMirroring.SingleScreenA => 0,
            NametableMirroring.SingleScreenB => 1,
            _ => table,
        };

        return (physical << 10) | offset;
    }

    /// <summary>
    /// Index in palette RAM of <paramref name="address"/>: $3F10, $3F14, $3F18 and $3F1C
    /// are redirected to $3F00, $3F04, $3F08 and $3F0C (REQ-CORE-045).
    /// </summary>
    private static int PaletteIndex(ushort address)
    {
        int index = address & 0x1F;
        if ((index & 0x13) == 0x10)
        {
            index &= 0x0F;
        }

        return index;
    }

    // ---------------------------------------------------------------- savestate

    /// <summary>Appends the PPU state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);

        writer.WriteBlob(_vram);
        writer.WriteBlob(_oam);
        writer.WriteBlob(_palette);
        writer.WriteInt32(_dot);
        writer.WriteInt32(_scanline);
        writer.WriteInt32(_dotAccumulator);
        writer.WriteBool(_oddFrame);
        writer.WriteInt64(_ppuClock);
        writer.WriteByte(_control);
        writer.WriteByte(_mask);
        writer.WriteBool(_vblankFlag);
        writer.WriteBool(_spriteZeroHit);
        writer.WriteBool(_spriteOverflow);
        writer.WriteBool(_suppressVBlank);
        writer.WriteByte(_oamAddress);
        writer.WriteUInt16(_v);
        writer.WriteUInt16(_t);
        writer.WriteUInt16(_vRowStart);
        writer.WriteUInt16(_vRowNext);
        writer.WriteByte(_fineX);
        writer.WriteBool(_writeToggle);
        writer.WriteByte(_readBuffer);
        writer.WriteByte(_decay);
        writer.WriteBool(_skipLatch);
        for (int bit = 0; bit < _decayStamp.Length; bit++)
        {
            writer.WriteInt64(_decayStamp[bit]);
        }

        writer.WriteBool(VBlankJustStarted);
        writer.WriteBool(NmiLineSample);

        SaveRenderState(writer);
        SaveTileState(writer);
    }

    /// <summary>Restores the PPU state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);

        reader.ReadBlobInto(_vram);
        reader.ReadBlobInto(_oam);
        reader.ReadBlobInto(_palette);
        _dot = reader.ReadInt32();
        _scanline = reader.ReadInt32();
        _dotAccumulator = reader.ReadInt32();
        _oddFrame = reader.ReadBool();
        _ppuClock = reader.ReadInt64();
        _control = reader.ReadByte();
        _mask = reader.ReadByte();
        _vblankFlag = reader.ReadBool();
        _spriteZeroHit = reader.ReadBool();
        _spriteOverflow = reader.ReadBool();
        _suppressVBlank = reader.ReadBool();
        _oamAddress = reader.ReadByte();
        _v = reader.ReadUInt16();
        _t = reader.ReadUInt16();
        _vRowStart = reader.ReadUInt16();
        _vRowNext = reader.ReadUInt16();
        _fineX = reader.ReadByte();
        _writeToggle = reader.ReadBool();
        _readBuffer = reader.ReadByte();
        _decay = reader.ReadByte();
        _skipLatch = reader.ReadBool();
        for (int bit = 0; bit < _decayStamp.Length; bit++)
        {
            _decayStamp[bit] = reader.ReadInt64();
        }

        VBlankJustStarted = reader.ReadBool();
        NmiLineSample = reader.ReadBool();

        LoadRenderState(reader);
        LoadTileState(reader);
    }
}
