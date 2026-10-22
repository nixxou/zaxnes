using Zax.Nes.Cartridges;
using Zax.Nes.State;

namespace Zax.Nes.Ppu;

/// <summary>
/// The 3dSen-specific tile emission contract (SPEC-02 §3.5) and the colour composition the
/// Zapper reads back from it (REQ-CORE-062).
/// </summary>
/// <remarks>
/// <para>
/// Every pattern byte needed here is read through the <em>inert</em> cartridge path
/// (<c>PpuPeek</c>), so capturing records cannot count an A12 rise, toggle an MMC2/MMC4
/// latch or disturb the $2007 read buffer: the emulation is identical whether or not the
/// host registered a video callback (REQ-CORE-033). Capture therefore always runs.
/// </para>
/// <para>
/// The records of a frame are cleared on the pre-render line, not when "run one frame"
/// starts: a frame runs from the end of one vblank to the start of the next, so the host
/// still receives the records of the frame that was just drawn during the vblank. The Zapper
/// no longer reads them there — AMD-021 R2-006 makes the vblank and the pre-render line see
/// the universal backdrop only.
/// </para>
/// </remarks>
public sealed partial class Ppu2C02
{
    /// <summary>
    /// Upper bound on the bytes the tile records of one frame occupy in a savestate, so
    /// that the host can announce a state size before serialising (REQ-ARCH-013).
    /// </summary>
    public const int MaxTileRecordStateSize = 12 + (TileBuffer.Capacity * 26);

    private readonly TileBuffer _tiles = new();

    /// <summary>Sprite capture order, by (x, OAM number), for mappers 9 and 10 (REQ-CORE-044).</summary>
    private readonly int[] _spriteOrder = new int[64];

    private int _maskX;
    private int _maskY;

    /// <summary>Tile records of the frame just drawn, in capture order (REQ-CORE-030).</summary>
    public ReadOnlySpan<TileRecord> TileRecords => _tiles.Records;

    /// <summary>Width of the unreliable left and right bands, in NES pixels (REQ-CORE-046).</summary>
    public int MaskX => _maskX;

    /// <summary>Width of the unreliable top and bottom bands, in NES pixels (REQ-CORE-046).</summary>
    public int MaskY => _maskY;

    /// <summary>Background records dropped by the capacity limit in this frame (REQ-CORE-032).</summary>
    public int DroppedBackgroundRecords => _tiles.DroppedBackground;

    /// <summary>Sprite records dropped by the capacity limit in this frame (REQ-CORE-032).</summary>
    public int DroppedSpriteRecords => _tiles.DroppedSprites;

    private void ResetTileState()
    {
        _tiles.Clear();
        _maskX = 0;
        _maskY = 0;
    }

    /// <summary>Starts a new rendered frame: the record list of the previous one is dropped.</summary>
    private void BeginRenderedFrame() => _tiles.Clear();

    /// <summary>
    /// Runs the captures owed by visible line <paramref name="line"/> at its first dot: the
    /// sprite band, and the masks on line 0.
    /// </summary>
    /// <remarks>
    /// Sprites still come before the background row of the same line (REQ-CORE-044): the row
    /// is now emitted at the <em>end</em> of that same line (R2-005), so nothing else can slip
    /// between the two.
    /// </remarks>
    private void BeginCaptureLine(int line)
    {
        if (line == 0)
        {
            // REQ-CORE-046 reads the masks from the state at the start of visible line 0;
            // R2-005 moved the background row, not this.
            ComputeMasks();
        }

        if (SpritesEnabled && (line & 7) == 7)
        {
            // REQ-CORE-039: on MMC2 and MMC4 the pattern a tile shows depends on the tiles
            // fetched before it on the line, through the $FD/$FE latches. A capture therefore
            // walks the line in display order and moves the latch forward as it goes — which
            // is why the sprites of one capture must be emitted by increasing x on those two
            // boards (REQ-CORE-044) — and puts the real latch back afterwards, so that the
            // emulation is unchanged whether or not records were built (REQ-CORE-033).
            int savedLatch = _cartridge?.ChrLatchState ?? 0;
            CaptureSprites(line);
            RestoreChrLatch(savedLatch);
        }
    }

    /// <summary>
    /// Emits the background row owed by visible line <paramref name="line"/>, at its last dot
    /// (R2-005).
    /// </summary>
    private void EndCaptureLine(int line)
    {
        if (BackgroundEnabled)
        {
            int savedLatch = _cartridge?.ChrLatchState ?? 0;
            CaptureBackgroundRow(line);
            RestoreChrLatch(savedLatch);
        }
    }

    private void RestoreChrLatch(int savedLatch)
    {
        if (_cartridge is not null)
        {
            _cartridge.ChrLatchState = savedLatch;
        }
    }

    /// <summary>
    /// maskX and maskY, decided from the state at the start of visible line 0
    /// (REQ-CORE-046).
    /// </summary>
    private void ComputeMasks()
    {
        (_maskX, _maskY) = Mirroring switch
        {
            NametableMirroring.Horizontal => (8, 0),
            NametableMirroring.Vertical => (0, 8),
            _ => (0, 0),
        };

        if (LeftColumnMasked)
        {
            _maskX = Math.Max(_maskX, 8);
        }
    }

    // ---------------------------------------------------------------- background rows

    /// <summary>
    /// Emits the background row of line <paramref name="line"/> when the fine Y scroll used
    /// to draw it is 1 (REQ-CORE-037, REQ-CORE-038).
    /// </summary>
    private void CaptureBackgroundRow(int line)
    {
        // AMD-021 R2-005 (supersedes point 3 of the answer to Q-004 and DEC-02-Q4 on this
        // point): the scroll is the left-edge shadow of the internal address read at the END
        // of the line, so a $2006 write made during line L reaches the row of line L itself,
        // while the vertical increment and the horizontal reload of the end of the line only
        // reach line L + 1. The one-line tolerance of REQ-CORE-037 no longer applies to the
        // instant of capture.
        int fineY = (_vRowStart >> 12) & 0x07;
        if (fineY != 1)
        {
            return;
        }

        // Exception of the original: no row is captured on line 0 under vertical mirroring
        // (REQ-CORE-037, justification).
        if (line == 0 && Mirroring == NametableMirroring.Vertical)
        {
            return;
        }

        int coarseX = _vRowStart & 0x1F;
        int coarseY = (_vRowStart >> 5) & 0x1F;
        int nametable = (_vRowStart >> 10) & 0x03;
        int fineXScroll = _fineX;
        int patternTable = (_control & 0x10) != 0 ? 0x1000 : 0x0000;
        int rowIndex = line / 8;
        var y = (short)(line - 1);

        for (int k = 0; k <= 32; k++)
        {
            int x = (8 * k) - fineXScroll;
            if (x >= ScreenWidth)
            {
                // k = 32 with fine X at 0: a row has 32 records, otherwise 33
                // (REQ-CORE-038).
                continue;
            }

            int column = coarseX + k;
            int table = nametable;
            if (column > 31)
            {
                // The horizontally neighbouring nametable, mirroring then applied.
                table ^= 1;
                column -= 32;
            }

            var nametableAddress = (ushort)(0x2000 | (table << 10) | (coarseY << 5) | column);
            byte tile = ReadNametableByte(nametableAddress);

            var attributeAddress = (ushort)(0x23C0 | (table << 10) | ((coarseY & 0x1C) << 1) | (column >> 2));
            byte attribute = ReadNametableByte(attributeAddress);
            int shift = ((coarseY & 0x02) << 1) | (column & 0x02);
            var palette = (byte)((attribute >> shift) & 0x03);

            (ulong plane0, ulong plane1) = ReadPatternPlanes(
                patternTable + (tile * 16),
                ChrFetch.Background);
            _cartridge?.AdvanceChrLatch(patternTable, tile);

            var record = new TileRecord(
                plane0,
                plane1,
                (short)x,
                y,
                (rowIndex * 33) + k,
                mirrorH: false,
                mirrorV: false,
                hidden: false,
                palette,
                background: true);

            _tiles.Add(in record);
        }
    }

    // ---------------------------------------------------------------- sprites

    /// <summary>
    /// Emits one record per OAM sprite whose 8-pixel band starts in the band of line
    /// <paramref name="line"/> (REQ-CORE-041, REQ-CORE-042).
    /// </summary>
    /// <remarks>
    /// The hardware limit of eight sprites per line does <em>not</em> apply: all 64 sprites
    /// are emitted, which is what keeps 3D sprites from flickering (REQ-CORE-041, notes).
    /// Left-column masking removes no record either; it is only reported through maskX.
    /// </remarks>
    private void CaptureSprites(int line)
    {
        int band = line / 8;
        int count = 0;

        for (int n = 0; n < 64; n++)
        {
            int top = _oam[n * 4] + 1;
            if (top >= RegionTiming.VisibleScanlines || top / 8 != band)
            {
                continue;
            }

            _spriteOrder[count++] = n;
        }

        if (count == 0)
        {
            return;
        }

        // Mappers 9 and 10 emit the sprites of one capture by increasing x, OAM number
        // breaking ties (REQ-CORE-044). The reference uses an unstable sort there; the
        // deterministic tie-break is required by REQ-ARCH-012.
        if (_cartridge?.MapperNumber is 9 or 10)
        {
            SortSpritesByX(count);
        }

        bool tall = (_control & 0x20) != 0;
        int spriteTable = (_control & 0x08) != 0 ? 0x1000 : 0x0000;

        for (int i = 0; i < count; i++)
        {
            int n = _spriteOrder[i];
            byte oamY = _oam[n * 4];
            byte tile = _oam[(n * 4) + 1];
            byte attribute = _oam[(n * 4) + 2];
            byte oamX = _oam[(n * 4) + 3];

            bool mirrorH = (attribute & 0x40) != 0;
            bool mirrorV = (attribute & 0x80) != 0;
            bool hidden = (attribute & 0x20) != 0;
            var palette = (byte)(attribute & 0x03);
            var top = (short)(oamY + 1);

            if (!tall)
            {
                EmitSprite(n, oamX, top, spriteTable + (tile * 16), mirrorH, mirrorV, hidden, palette);
                continue;
            }

            // 8x16: the table comes from bit 0 of the tile number and the pair is
            // (T & $FE, T | 1); vertical flip swaps the halves as well as flipping each.
            int tallTable = (tile & 1) != 0 ? 0x1000 : 0x0000;
            int upper = mirrorV ? (tile | 1) : (tile & 0xFE);
            int lower = mirrorV ? (tile & 0xFE) : (tile | 1);

            EmitSprite(n, oamX, top, tallTable + (upper * 16), mirrorH, mirrorV, hidden, palette);

            if (top + 8 < RegionTiming.VisibleScanlines)
            {
                EmitSprite(
                    64 + n,
                    oamX,
                    (short)(top + 8),
                    tallTable + (lower * 16),
                    mirrorH,
                    mirrorV,
                    hidden,
                    palette);
            }
        }
    }

    private void SortSpritesByX(int count)
    {
        // Insertion sort: the lists are at most 64 long and the order must be stable so
        // that equal x values stay in OAM order (REQ-CORE-044, REQ-ARCH-012).
        for (int i = 1; i < count; i++)
        {
            int current = _spriteOrder[i];
            byte currentX = _oam[(current * 4) + 3];
            int j = i - 1;

            while (j >= 0 && _oam[(_spriteOrder[j] * 4) + 3] > currentX)
            {
                _spriteOrder[j + 1] = _spriteOrder[j];
                j--;
            }

            _spriteOrder[j + 1] = current;
        }
    }

    private void EmitSprite(
        int index,
        byte x,
        short y,
        int patternAddress,
        bool mirrorH,
        bool mirrorV,
        bool hidden,
        byte palette)
    {
        (ulong plane0, ulong plane1) = ReadPatternPlanes(patternAddress, ChrFetch.Sprite);
        _cartridge?.AdvanceChrLatch(patternAddress & 0x1000, (patternAddress >> 4) & 0xFF);

        var record = new TileRecord(
            plane0,
            plane1,
            x,
            y,
            index,
            mirrorH,
            mirrorV,
            hidden,
            palette,
            background: false);

        _tiles.Add(in record);
    }

    /// <summary>
    /// Reads the 16 CHR bytes of the pattern at <paramref name="address"/> through the
    /// side-effect-free path and packs them as REQ-CORE-034 requires.
    /// </summary>
    private (ulong Plane0, ulong Plane1) ReadPatternPlanes(int address, ChrFetch fetch)
    {
        ulong plane0 = 0;
        ulong plane1 = 0;

        for (int row = 0; row < 8; row++)
        {
            plane0 |= (ulong)ReadVramThroughBus((ushort)(address + row), PpuBusAccess.Inert, fetch)
                << (8 * row);
            plane1 |= (ulong)ReadVramThroughBus((ushort)(address + 8 + row), PpuBusAccess.Inert, fetch)
                << (8 * row);
        }

        return (plane0, plane1);
    }

    // ---------------------------------------------------------------- Zapper

    /// <summary>
    /// First column of the right-hand band the Zapper always sees as the universal backdrop
    /// (AMD-021 R2-006).
    /// </summary>
    public const int ZapperBackdropColumn = 239;

    /// <summary>
    /// Universal backdrop colour, entry 0 of the palette ($3F00), as the Zapper sees it
    /// (REQ-CORE-062, AMD-021 R2-006).
    /// </summary>
    public byte UniversalBackdropColor => _palette[PaletteIndex(0x3F00)];

    /// <summary>
    /// Whether the Zapper sees light at (<paramref name="x"/>, <paramref name="y"/>)
    /// (REQ-CORE-062, AMD-021 R2-006).
    /// </summary>
    /// <remarks>
    /// <para>
    /// The three conditions of REQ-CORE-062: the aim is inside the image, the scan has
    /// already passed that pixel in the frame being drawn, and the colour displayed there is
    /// light.
    /// </para>
    /// <para>
    /// R2-006 replaces what "the colour displayed there" means in two cases, and in doing so
    /// supersedes points 1 and 3 of the answer to Q-005 and the notes of REQ-CORE-062. Outside
    /// the visible scan — the vblank and the pre-render line — the records of the frame just
    /// drawn are no longer consulted at all: the colour considered is the universal backdrop,
    /// so a game that polls during the vblank sees light exactly when entry 0 of the palette
    /// is light. And from column 239 on, the colour considered is the universal backdrop as
    /// well, whatever the records say.
    /// </para>
    /// </remarks>
    public bool SeesLightAt(int x, int y)
    {
        if (x is < 0 or >= ScreenWidth || y is < 0 or >= RegionTiming.VisibleScanlines)
        {
            return false;
        }

        // R2-006: every line past the last visible one — the post-render line, the vblank and
        // the pre-render line — reads the backdrop and nothing else. Q-059: the post-render
        // line is neither of the two R2-006 names, and is read with them.
        if (_scanline >= RegionTiming.VisibleScanlines)
        {
            return NesColors.IsLight(UniversalBackdropColor);
        }

        if (_scanline <= y)
        {
            return false;
        }

        // R2-006: the right-hand band is always seen with the universal backdrop colour —
        // light when that colour is light, which the previous reading of Q-005 point 3 never
        // allowed.
        return NesColors.IsLight(x >= ZapperBackdropColumn
            ? UniversalBackdropColor
            : ComposeRecordedColor(x, y));
    }

    /// <summary>
    /// Colour displayed at (<paramref name="x"/>, <paramref name="y"/>), composed from the
    /// records already emitted in the current frame (REQ-CORE-062).
    /// </summary>
    /// <remarks>
    /// NES composition order: a non-transparent pixel of the first sprite (lowest index)
    /// that is not "hidden"; otherwise a non-transparent background pixel; otherwise a
    /// non-transparent pixel of a hidden sprite; otherwise the universal backdrop.
    /// </remarks>
    public byte ComposeRecordedColor(int x, int y)
    {
        int frontIndex = int.MaxValue;
        int frontEntry = -1;
        int behindIndex = int.MaxValue;
        int behindEntry = -1;
        int backgroundEntry = -1;

        foreach (TileRecord record in _tiles.Records)
        {
            int column = x - record.X;
            int row = y - record.Y;
            if (column is < 0 or > 7 || row is < 0 or > 7)
            {
                continue;
            }

            int pixel = record.DisplayedPixel(column, row);
            if (pixel == 0)
            {
                continue;
            }

            if (record.Background)
            {
                // Q-005: two background rows can cover the same pixel after a mid-frame
                // scroll change; the one captured later wins, which is the order the scan
                // drew them in.
                backgroundEntry = (record.Palette * 4) + pixel;
                continue;
            }

            int entry = 0x10 + (record.Palette * 4) + pixel;
            if (record.Hidden)
            {
                if (record.Index < behindIndex)
                {
                    behindIndex = record.Index;
                    behindEntry = entry;
                }
            }
            else if (record.Index < frontIndex)
            {
                frontIndex = record.Index;
                frontEntry = entry;
            }
        }

        int chosen = frontEntry >= 0 ? frontEntry
            : backgroundEntry >= 0 ? backgroundEntry
            : behindEntry >= 0 ? behindEntry
            : 0;

        return _palette[PaletteIndex((ushort)(0x3F00 | chosen))];
    }

    // ---------------------------------------------------------------- savestate

    private void SaveTileState(StateWriter writer)
    {
        writer.WriteInt32(_maskX);
        writer.WriteInt32(_maskY);
        _tiles.SaveState(writer);
    }

    private void LoadTileState(StateReader reader)
    {
        _maskX = reader.ReadInt32();
        _maskY = reader.ReadInt32();
        _tiles.LoadState(reader);
    }
}
