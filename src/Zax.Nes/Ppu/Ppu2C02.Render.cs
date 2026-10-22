using Zax.Nes.Cartridges;
using Zax.Nes.State;

namespace Zax.Nes.Ppu;

/// <summary>
/// Rendering half of the PPU: scroll registers, background fetch pipeline, sprite
/// evaluation, sprite 0 hit, sprite overflow and the optional 256 x 240 indexed image
/// (REQ-CORE-048, REQ-CORE-112, REQ-CORE-116).
/// </summary>
/// <remarks>
/// The dot schedule is the one of the nesdev "PPU rendering" timing diagram: on the visible
/// lines and the pre-render line, four two-dot fetches per tile, the coarse X increment on
/// the eighth dot of each group, the Y increment at dot 256, the horizontal reload at
/// dot 257, the vertical reload at dots 280 to 304 of the pre-render line, the sprite
/// pattern fetches at dots 257 to 320 and two more tiles prefetched at dots 321 to 336.
/// </remarks>
public sealed partial class Ppu2C02
{
    /// <summary>Number of sprites the hardware can show on one line (nesdev).</summary>
    public const int SpritesPerLine = 8;

    private readonly byte[] _pixels = new byte[ScreenWidth * ScreenHeight];
    private readonly byte[] _secondaryOam = new byte[4 * SpritesPerLine];

    private readonly byte[] _spritePatternLow = new byte[SpritesPerLine];
    private readonly byte[] _spritePatternHigh = new byte[SpritesPerLine];
    private readonly byte[] _spriteAttribute = new byte[SpritesPerLine];
    private readonly byte[] _spriteX = new byte[SpritesPerLine];

    private byte _nametableLatch;
    private byte _attributeLatch;
    private byte _patternLowLatch;
    private byte _patternHighLatch;

    private ushort _patternShiftLow;
    private ushort _patternShiftHigh;
    private ushort _attributeShiftLow;
    private ushort _attributeShiftHigh;

    private byte _oamEvalLatch;
    private int _evalSprite;
    private int _evalByte;
    private int _evalFound;
    private int _overflowCopy;
    private bool _evalFinished;
    private bool _spriteZeroFound;

    private int _spritesLoaded;
    private bool _spriteZeroLoaded;

    /// <summary>
    /// The 256 x 240 indexed image of the frame just rendered: one NES colour index per
    /// pixel, row-major (REQ-CORE-048). Needed by the 2D Mode of SPEC-06.
    /// </summary>
    public ReadOnlySpan<byte> PixelFrame => _pixels;

    /// <summary>Number of sprites the evaluation of the previous line found, 0 to 8.</summary>
    public int SpritesOnLine => _spritesLoaded;

    private void ResetRenderState()
    {
        Array.Fill(_secondaryOam, (byte)0xFF);
        Array.Clear(_spritePatternLow);
        Array.Clear(_spritePatternHigh);
        Array.Clear(_spriteAttribute);
        Array.Clear(_spriteX);

        _nametableLatch = 0;
        _attributeLatch = 0;
        _patternLowLatch = 0;
        _patternHighLatch = 0;
        _patternShiftLow = 0;
        _patternShiftHigh = 0;
        _attributeShiftLow = 0;
        _attributeShiftHigh = 0;

        _oamEvalLatch = 0;
        _evalSprite = 0;
        _evalByte = 0;
        _evalFound = 0;
        _overflowCopy = 0;
        _evalFinished = false;
        _spriteZeroFound = false;
        _spritesLoaded = 0;
        _spriteZeroLoaded = false;
    }

    /// <summary>
    /// Runs everything the current dot owes: the fetch pipeline, sprite evaluation and the
    /// pixel of the visible lines.
    /// </summary>
    private void RunRenderDot()
    {
        bool visible = _scanline < RegionTiming.VisibleScanlines;
        bool renderLine = visible || _scanline == PreRenderScanline;

        if (renderLine && RenderingEnabled)
        {
            if (_dot == 1)
            {
                // Dots 1 to 64 clear the secondary OAM to $FF; the unused slots then make
                // the hardware fetch pattern $FF, which boards watching A12 can see.
                Array.Fill(_secondaryOam, (byte)0xFF);
            }

            RunBackgroundPipeline();

            if (visible)
            {
                RunSpriteEvaluation();
            }
            else if (_dot == 1)
            {
                // Sprite evaluation does not run on the pre-render line, so no sprite is
                // shown on line 0 (nesdev, "PPU sprite evaluation").
                _evalFound = 0;
                _spriteZeroFound = false;
            }

            RunSpriteFetches();

            if (_scanline == PreRenderScanline && _dot is >= 280 and <= 304)
            {
                TransferAddressY();
            }
        }

        if (visible && _dot is >= 1 and <= ScreenWidth)
        {
            RenderPixel(_dot - 1);
        }
    }

    // ---------------------------------------------------------------- background

    private void RunBackgroundPipeline()
    {
        if ((_dot is >= 2 and <= 257) || (_dot is >= 321 and <= 337))
        {
            ShiftBackground();

            switch ((_dot - 1) % 8)
            {
                case 0:
                    LoadBackgroundShifters();
                    _nametableLatch = ReadVramThroughBus((ushort)(0x2000 | (_v & 0x0FFF)), PpuBusAccess.Render);
                    break;

                case 2:
                    _attributeLatch = FetchAttribute();
                    break;

                case 3:
                    NotifyPatternAddress(0);
                    break;

                case 4:
                    _patternLowLatch = FetchPattern(0);
                    break;

                case 5:
                    NotifyPatternAddress(8);
                    break;

                case 6:
                    _patternHighLatch = FetchPattern(8);
                    break;

                case 7:
                    IncrementCoarseX();
                    break;

                default:
                    break;
            }
        }

        if (_dot == 256)
        {
            IncrementY();
        }
        else if (_dot == 257)
        {
            TransferAddressX();
        }
        else if (_dot is 338 or 340)
        {
            // Two more nametable fetches the hardware performs and discards; they matter
            // only for boards that watch the PPU address bus.
            _nametableLatch = ReadVramThroughBus((ushort)(0x2000 | (_v & 0x0FFF)), PpuBusAccess.Render);
        }
    }

    private byte FetchAttribute()
    {
        var address = (ushort)(0x23C0
            | (_v & 0x0C00)
            | ((_v >> 4) & 0x38)
            | ((_v >> 2) & 0x07));

        byte attribute = ReadVramThroughBus(address, PpuBusAccess.Render);
        int shift = ((_v >> 4) & 0x04) | (_v & 0x02);
        return (byte)((attribute >> shift) & 0x03);
    }

    private byte FetchPattern(int plane) => ReadVramThroughBus(PatternAddress(plane), PpuBusAccess.Render);

    private ushort PatternAddress(int plane)
    {
        int table = (_control & 0x10) != 0 ? 0x1000 : 0x0000;
        return (ushort)(table + (_nametableLatch * 16) + ((_v >> 12) & 0x07) + plane);
    }

    /// <summary>
    /// Puts the address of the coming pattern fetch on the cartridge's bus, one dot before
    /// the byte is latched.
    /// </summary>
    /// <remarks>
    /// Each fetch takes two dots and the address is driven for both of them, so a board
    /// watching A12 sees it one dot before the data arrives. That dot is measurable: it is
    /// what <c>mmc3_test_2</c> test 4 pins down, to PPU-clock accuracy, for both
    /// $2000 = $08 and $2000 = $10 (REQ-CORE-114).
    /// </remarks>
    private void NotifyPatternAddress(int plane) => NotifyAddressBus(PatternAddress(plane));

    private void ShiftBackground()
    {
        _patternShiftLow <<= 1;
        _patternShiftHigh <<= 1;
        _attributeShiftLow <<= 1;
        _attributeShiftHigh <<= 1;
    }

    private void LoadBackgroundShifters()
    {
        _patternShiftLow = (ushort)((_patternShiftLow & 0xFF00) | _patternLowLatch);
        _patternShiftHigh = (ushort)((_patternShiftHigh & 0xFF00) | _patternHighLatch);
        _attributeShiftLow = (ushort)((_attributeShiftLow & 0xFF00) | ((_attributeLatch & 1) != 0 ? 0xFF : 0x00));
        _attributeShiftHigh = (ushort)((_attributeShiftHigh & 0xFF00) | ((_attributeLatch & 2) != 0 ? 0xFF : 0x00));
    }

    private void IncrementCoarseX()
    {
        if ((_v & 0x001F) == 0x001F)
        {
            _v = (ushort)((_v & ~0x001F) ^ 0x0400);
        }
        else
        {
            _v++;
        }
    }

    private void IncrementY()
    {
        if ((_v & 0x7000) != 0x7000)
        {
            _v += 0x1000;
            return;
        }

        _v = (ushort)(_v & ~0x7000);
        int coarseY = (_v >> 5) & 0x1F;

        if (coarseY == 29)
        {
            coarseY = 0;
            _v ^= 0x0800;
        }
        else if (coarseY == 31)
        {
            // Coarse Y 30 and 31 address the attribute tables; the hardware wraps without
            // switching nametable (REQ-CORE-038, notes).
            coarseY = 0;
        }
        else
        {
            coarseY++;
        }

        _v = (ushort)((_v & ~0x03E0) | (coarseY << 5));
    }

    private void TransferAddressX()
    {
        _v = (ushort)((_v & ~0x041F) | (_t & 0x041F));

        // R2-005: the horizontal reload of dot 257 — and the vertical increment of dot 256
        // that precedes it — belong to the next line, not to the row captured for this one.
        _vRowNext = _v;
    }

    private void TransferAddressY()
    {
        _v = (ushort)((_v & ~0x7BE0) | (_t & 0x7BE0));

        // The vertical reload only happens on the pre-render line, which captures nothing, so
        // it may as well take effect at once.
        _vRowStart = _v;
        _vRowNext = _v;
    }

    // ---------------------------------------------------------------- sprites

    /// <summary>
    /// Sprite evaluation for the <em>next</em> line: dots 1 to 64 clear the secondary OAM,
    /// dots 65 to 256 scan the 64 sprites (nesdev, "PPU sprite evaluation"). The overflow
    /// flag follows the hardware, byte-increment bug included.
    /// </summary>
    private void RunSpriteEvaluation()
    {
        if (_dot == 65)
        {
            _evalSprite = _oamAddress >> 2;
            _evalByte = _oamAddress & 0x03;
            _evalFound = 0;
            _overflowCopy = 0;
            _evalFinished = false;
            _spriteZeroFound = false;
        }

        if (_dot is < 65 or > 256)
        {
            return;
        }

        if ((_dot & 1) == 1)
        {
            // Odd dots read primary OAM.
            _oamEvalLatch = _oam[((_evalSprite * 4) + _evalByte) & 0xFF];
            return;
        }

        if (_evalFinished)
        {
            return;
        }

        if (_evalFound < SpritesPerLine)
        {
            _secondaryOam[(_evalFound * 4) + _evalByte] = _oamEvalLatch;

            if (_evalByte == 0)
            {
                if (InSpriteRange(_oamEvalLatch))
                {
                    if (_evalSprite == 0)
                    {
                        _spriteZeroFound = true;
                    }

                    _evalByte = 1;
                }
                else
                {
                    NextSprite();
                }
            }
            else
            {
                _evalByte++;
                if (_evalByte == 4)
                {
                    _evalByte = 0;
                    _evalFound++;
                    NextSprite();
                }
            }

            return;
        }

        // Eight sprites found: the ninth in-range sprite raises the overflow flag, and the
        // hardware then increments both the sprite and the byte index, which is the famous
        // false-positive bug.
        if (_overflowCopy > 0)
        {
            _overflowCopy--;
            _evalByte++;
            if (_evalByte == 4)
            {
                _evalByte = 0;
                NextSprite();
            }
        }
        else if (InSpriteRange(_oamEvalLatch))
        {
            _spriteOverflow = true;
            _overflowCopy = 3;
            _evalByte = 1;
        }
        else
        {
            _evalByte = (_evalByte + 1) & 0x03;
            NextSprite();
        }
    }

    private void NextSprite()
    {
        _evalSprite++;
        if (_evalSprite >= 64)
        {
            _evalSprite = 0;
            _evalFinished = true;
        }
    }

    private bool InSpriteRange(byte spriteY)
    {
        int height = (_control & 0x20) != 0 ? 16 : 8;
        return _scanline >= spriteY && _scanline < spriteY + height;
    }

    /// <summary>
    /// Dots 257 to 320 fetch the patterns of the eight sprites of the next line and hold
    /// OAMADDR at 0 (nesdev).
    /// </summary>
    private void RunSpriteFetches()
    {
        if (_dot is < 257 or > 320)
        {
            return;
        }

        _oamAddress = 0;

        if (_dot == 257)
        {
            _spritesLoaded = _evalFound;
            _spriteZeroLoaded = _spriteZeroFound;
        }

        int slot = (_dot - 257) >> 3;
        int step = (_dot - 257) & 0x07;

        if (step is 3 or 5)
        {
            // The address of the coming fetch is already on the bus (see
            // NotifyPatternAddress).
            NotifyAddressBus((ushort)(SpritePatternAddress(slot) + (step == 3 ? 0 : 8)));
            return;
        }

        if (step is not (4 or 6))
        {
            return;
        }

        ushort address = SpritePatternAddress(slot);
        byte data = ReadVramThroughBus(
            (ushort)(address + (step == 4 ? 0 : 8)),
            PpuBusAccess.Render,
            ChrFetch.Sprite);

        if (step == 4)
        {
            _spritePatternLow[slot] = data;
            _spriteAttribute[slot] = _secondaryOam[(slot * 4) + 2];
            _spriteX[slot] = _secondaryOam[(slot * 4) + 3];
        }
        else
        {
            _spritePatternHigh[slot] = data;
        }
    }

    private ushort SpritePatternAddress(int slot)
    {
        byte spriteY = _secondaryOam[slot * 4];
        byte tile = _secondaryOam[(slot * 4) + 1];
        byte attribute = _secondaryOam[(slot * 4) + 2];
        bool tall = (_control & 0x20) != 0;
        bool flipV = (attribute & 0x80) != 0;

        int row = _scanline - spriteY;
        if (slot >= _spritesLoaded)
        {
            // The unused slots still fetch, so that boards watching A12 see the same bus
            // activity as on hardware.
            row = 0;
        }

        if (!tall)
        {
            int table = (_control & 0x08) != 0 ? 0x1000 : 0x0000;
            int line = flipV ? 7 - (row & 7) : row & 7;
            return (ushort)(table + (tile * 16) + line);
        }

        int tallTable = (tile & 1) != 0 ? 0x1000 : 0x0000;
        int index = tile & 0xFE;
        int tallRow = flipV ? 15 - (row & 15) : row & 15;
        if (tallRow >= 8)
        {
            index |= 1;
            tallRow -= 8;
        }

        return (ushort)(tallTable + (index * 16) + tallRow);
    }

    // ---------------------------------------------------------------- composition

    private void RenderPixel(int x)
    {
        int backgroundPixel = 0;
        int backgroundPalette = 0;

        if (RenderingEnabled && BackgroundEnabled && (x >= 8 || (_mask & 0x02) != 0))
        {
            int mux = 0x8000 >> _fineX;
            backgroundPixel = ((_patternShiftLow & mux) != 0 ? 1 : 0)
                | ((_patternShiftHigh & mux) != 0 ? 2 : 0);
            backgroundPalette = ((_attributeShiftLow & mux) != 0 ? 1 : 0)
                | ((_attributeShiftHigh & mux) != 0 ? 2 : 0);
        }

        int spritePixel = 0;
        int spritePalette = 0;
        bool spriteBehind = false;
        bool spriteIsZero = false;

        if (RenderingEnabled && SpritesEnabled && (x >= 8 || (_mask & 0x04) != 0))
        {
            for (int slot = 0; slot < _spritesLoaded; slot++)
            {
                int column = x - _spriteX[slot];
                if (column is < 0 or > 7)
                {
                    continue;
                }

                byte attribute = _spriteAttribute[slot];
                int bit = (attribute & 0x40) != 0 ? column : 7 - column;
                int pixel = ((_spritePatternLow[slot] >> bit) & 1)
                    | (((_spritePatternHigh[slot] >> bit) & 1) << 1);

                if (pixel == 0)
                {
                    continue;
                }

                spritePixel = pixel;
                spritePalette = attribute & 0x03;
                spriteBehind = (attribute & 0x20) != 0;
                spriteIsZero = slot == 0 && _spriteZeroLoaded;
                break;
            }
        }

        // Sprite 0 hit needs both layers on, a non-transparent pixel in each, and never
        // happens at x = 255 (nesdev, "PPU sprite 0 hits").
        if (spriteIsZero && spritePixel != 0 && backgroundPixel != 0 && x != ScreenWidth - 1)
        {
            _spriteZeroHit = true;
        }

        int entry;
        if (spritePixel != 0 && (backgroundPixel == 0 || !spriteBehind))
        {
            entry = 0x10 + (spritePalette * 4) + spritePixel;
        }
        else if (backgroundPixel != 0)
        {
            entry = (backgroundPalette * 4) + backgroundPixel;
        }
        else
        {
            entry = 0;
        }

        byte color = _palette[PaletteIndex((ushort)(0x3F00 | entry))];

        // The greyscale bit of PPUMASK is a video effect: it changes the image but neither
        // the tile records nor the palette handed to the engine (REQ-CORE-047).
        if ((_mask & 0x01) != 0)
        {
            color &= 0x30;
        }

        _pixels[(_scanline * ScreenWidth) + x] = color;
    }

    // ---------------------------------------------------------------- savestate

    private void SaveRenderState(StateWriter writer)
    {
        writer.WriteRaw(_secondaryOam);
        writer.WriteRaw(_spritePatternLow);
        writer.WriteRaw(_spritePatternHigh);
        writer.WriteRaw(_spriteAttribute);
        writer.WriteRaw(_spriteX);

        writer.WriteByte(_nametableLatch);
        writer.WriteByte(_attributeLatch);
        writer.WriteByte(_patternLowLatch);
        writer.WriteByte(_patternHighLatch);
        writer.WriteUInt16(_patternShiftLow);
        writer.WriteUInt16(_patternShiftHigh);
        writer.WriteUInt16(_attributeShiftLow);
        writer.WriteUInt16(_attributeShiftHigh);

        writer.WriteByte(_oamEvalLatch);
        writer.WriteInt32(_evalSprite);
        writer.WriteInt32(_evalByte);
        writer.WriteInt32(_evalFound);
        writer.WriteInt32(_overflowCopy);
        writer.WriteBool(_evalFinished);
        writer.WriteBool(_spriteZeroFound);
        writer.WriteInt32(_spritesLoaded);
        writer.WriteBool(_spriteZeroLoaded);
    }

    private void LoadRenderState(StateReader reader)
    {
        reader.ReadInto(_secondaryOam);
        reader.ReadInto(_spritePatternLow);
        reader.ReadInto(_spritePatternHigh);
        reader.ReadInto(_spriteAttribute);
        reader.ReadInto(_spriteX);

        _nametableLatch = reader.ReadByte();
        _attributeLatch = reader.ReadByte();
        _patternLowLatch = reader.ReadByte();
        _patternHighLatch = reader.ReadByte();
        _patternShiftLow = reader.ReadUInt16();
        _patternShiftHigh = reader.ReadUInt16();
        _attributeShiftLow = reader.ReadUInt16();
        _attributeShiftHigh = reader.ReadUInt16();

        _oamEvalLatch = reader.ReadByte();
        _evalSprite = reader.ReadInt32();
        _evalByte = reader.ReadInt32();
        _evalFound = reader.ReadInt32();
        _overflowCopy = reader.ReadInt32();
        _evalFinished = reader.ReadBool();
        _spriteZeroFound = reader.ReadBool();
        _spritesLoaded = reader.ReadInt32();
        _spriteZeroLoaded = reader.ReadBool();
    }
}
