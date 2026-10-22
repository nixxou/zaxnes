using Zax.App.Audio;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Platform;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for the normative items of <c>AMD-021</c> §1 and §4.2, as corrected
/// by <c>AMD-022</c>: the three mappers (R2-001, R2-002, R2-003), the instant of capture of a
/// background row (R2-005, criterion 1), the Zapper light during and outside the visible scan
/// (R2-006), the candidate order after a removal (R2-008) and the sounds of the options window
/// (R2-013).
/// </summary>
/// <remarks>
/// The items that need a GL context live in <see cref="Amd021SceneTests"/> (R2-009, R2-010,
/// R2-014); R2-011 is judged in <see cref="InputCoreTests"/>, where the superseded assertions
/// of <c>Q-058</c> were. Every assertion here comes from the text of the amendment, not from
/// the implementers' transcription of it.
/// </remarks>
public sealed class Amd021Tests
{
    // ================================================================ mappers

    /// <summary>
    /// A PRG image in which every 8 KiB bank carries its own number at offset 0 and the number
    /// of the 16 KiB bank it belongs to at offset 1 — the "ROM synthétique … dont chaque banque
    /// de 16 Kio porte son numéro" of the criteria of R2-001 and R2-002, read at both
    /// granularities because R2-002 mode 2 switches 8 KiB windows.
    /// </summary>
    private static byte[] NumberedRom(int mapper, int prgBanks16k, int chrBanks8k = 0, byte flags6 = 0)
    {
        int prg = prgBanks16k * 16 * 1024;
        int chr = chrBanks8k * 8 * 1024;
        byte[] image = new byte[16 + prg + chr];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = (byte)prgBanks16k;
        image[5] = (byte)chrBanks8k;
        image[6] = (byte)(((mapper & 0x0F) << 4) | flags6);
        image[7] = (byte)(mapper & 0xF0);

        for (int bank8 = 0; bank8 < prgBanks16k * 2; bank8++)
        {
            int at = 16 + (bank8 * 8 * 1024);
            image[at] = (byte)bank8;
            image[at + 1] = (byte)(bank8 / 2);
        }

        for (int bank = 0; bank < chrBanks8k; bank++)
        {
            image[16 + prg + (bank * 8 * 1024)] = (byte)bank;
        }

        return image;
    }

    private static NesCore Load(int mapper, int prgBanks16k, int chrBanks8k = 0, byte flags6 = 0)
    {
        NesCore core = new();
        Zax.Nes.Rom.RomLoadResult load =
            core.LoadRom(NumberedRom(mapper, prgBanks16k, chrBanks8k, flags6), $"Mapper{mapper}.nes");
        Assert.True(load.Success, load.Error);
        return core;
    }

    /// <summary>The 16 KiB bank number the window at <paramref name="address"/> shows.</summary>
    private static int Bank16(NesCore core, ushort address) => core.ReadMemory((ushort)(address + 1));

    /// <summary>The 8 KiB bank number the window at <paramref name="address"/> shows.</summary>
    private static int Bank8(NesCore core, ushort address) => core.ReadMemory(address);

    /// <summary>
    /// Loads one of the four Action 53 registers: a write to $5000-$5FFF picks it by bits 7
    /// and 0 of the <em>value</em>, a write to $8000-$FFFF loads it (R2-001).
    /// </summary>
    private static void Action53(NesCore core, byte select, byte value)
    {
        core.WriteMemory(0x5000, select);
        core.WriteMemory(0x8000, value);
    }

    [Fact] // AMD-021 R2-001 (REQ-CORE-100) — the four vectors of the criterion
    public void R2_001_the_action_53_vectors_of_the_criterion()
    {
        // 1 MiB: 64 banks of 16 KiB, each carrying its number.
        // (ext = 5, mode = 0) -> 10 / 11.
        NesCore core = Load(28, 64);
        Action53(core, 0x81, 5);
        Assert.Equal(10, Bank16(core, 0x8000));
        Assert.Equal(11, Bank16(core, 0xC000));

        // (ext = 2, mode = $18, int = 3) -> 4 / 7.
        core = Load(28, 64);
        Action53(core, 0x80, 0x18);
        Action53(core, 0x81, 2);
        Action53(core, 0x01, 3);
        Assert.Equal(4, Bank16(core, 0x8000));
        Assert.Equal(7, Bank16(core, 0xC000));

        // (ext = 2, mode = $1C, int = 3) -> 7 / 5.
        core = Load(28, 64);
        Action53(core, 0x80, 0x1C);
        Action53(core, 0x81, 2);
        Action53(core, 0x01, 3);
        Assert.Equal(7, Bank16(core, 0x8000));
        Assert.Equal(5, Bank16(core, 0xC000));

        // A CHR write of $10 while the mode is 0 -> screen B.
        core = Load(28, 64);
        Assert.Equal(NametableMirroring.SingleScreenA, core.Cartridge!.Mirroring);
        Action53(core, 0x00, 0x10);
        Assert.Equal(NametableMirroring.SingleScreenB, core.Cartridge!.Mirroring);
    }

    [Fact] // AMD-021 R2-001 — power-on (ext = 63, int = 15, mode = 0)
    public void R2_001_action_53_powers_on_with_the_last_bank_at_c000()
    {
        NesCore core = Load(28, 64);

        // E = 126, m = 1, PRG mode 0: bank 126 at $8000 and 127 at $C000, both past the end of
        // a 64-bank image and brought back by the rule of AMD-022 (Q-059.3).
        Assert.Equal(62, Bank16(core, 0x8000));
        Assert.Equal(63, Bank16(core, 0xC000));
    }

    [Fact] // AMD-022 Q-059.3 — a bank number past the end is taken modulo the bank count
    public void R2_001_a_bank_number_past_the_end_is_reduced_modulo_the_bank_count()
    {
        // 48 banks of 16 KiB (768 KiB) is not a power of two, so a modulo and a mask of the
        // next power of two disagree: 126 mod 48 = 30, while 126 & 63 = 62 is out of the image.
        NesCore core = Load(28, 48);
        Assert.Equal(30, Bank16(core, 0x8000));
        Assert.Equal(31, Bank16(core, 0xC000));

        // Same rule on a bank number the game itself asks for: ext = 40 means E = 80.
        Action53(core, 0x81, 40);
        Assert.Equal(80 % 48, Bank16(core, 0x8000));
        Assert.Equal(81 % 48, Bank16(core, 0xC000));
    }

    [Fact] // AMD-021 R2-001 — the mirroring bits of the mode register
    public void R2_001_the_mode_register_carries_the_four_nametable_layouts()
    {
        NesCore core = Load(28, 64);
        foreach ((byte mode, NametableMirroring expected) in new (byte, NametableMirroring)[]
        {
            (0x00, NametableMirroring.SingleScreenA),
            (0x01, NametableMirroring.SingleScreenB),
            (0x02, NametableMirroring.Vertical),
            (0x03, NametableMirroring.Horizontal),
        })
        {
            Action53(core, 0x80, mode);
            Assert.Equal(expected, core.Cartridge!.Mirroring);
        }

        // Once bit 1 of the mode is set, a CHR or inner-PRG write no longer touches bit 0.
        Action53(core, 0x80, 0x02);
        Action53(core, 0x00, 0x10);
        Assert.Equal(NametableMirroring.Vertical, core.Cartridge!.Mirroring);
        Action53(core, 0x01, 0x10);
        Assert.Equal(NametableMirroring.Vertical, core.Cartridge!.Mirroring);
    }

    [Fact] // AMD-021 R2-002 (REQ-CORE-100) — the three vectors of the criterion
    public void R2_002_the_mapper_15_vectors_of_the_criterion()
    {
        // $8001 <- $05: mode 1, B = 5 at $8000 and the last 16 KiB bank (63) at $C000.
        NesCore core = Load(15, 64);
        core.WriteMemory(0x8001, 0x05);
        Assert.Equal(5, Bank16(core, 0x8000));
        Assert.Equal(63, Bank16(core, 0xC000));

        // $8002 <- $85: mode 2, B = 5, h = 1 -> the 8 KiB bank 11 in the four windows.
        core = Load(15, 64);
        core.WriteMemory(0x8002, 0x85);
        Assert.Equal(11, Bank8(core, 0x8000));
        Assert.Equal(11, Bank8(core, 0xA000));
        Assert.Equal(11, Bank8(core, 0xC000));
        Assert.Equal(11, Bank8(core, 0xE000));

        // $8000 <- $40: bit 6 set -> horizontal mirroring.
        core = Load(15, 64);
        Assert.Equal(NametableMirroring.Vertical, core.Cartridge!.Mirroring);
        core.WriteMemory(0x8000, 0x40);
        Assert.Equal(NametableMirroring.Horizontal, core.Cartridge!.Mirroring);
    }

    [Fact] // AMD-021 R2-002 — power-on and the two remaining layouts
    public void R2_002_mapper_15_shows_the_first_thirty_two_kib_at_power_on()
    {
        NesCore core = Load(15, 64);
        Assert.Equal(0, Bank16(core, 0x8000));
        Assert.Equal(1, Bank16(core, 0xC000));

        // Mode 0: B and B + 1.
        core.WriteMemory(0x8000, 0x07);
        Assert.Equal(7, Bank16(core, 0x8000));
        Assert.Equal(8, Bank16(core, 0xC000));

        // Mode 3: B in both windows.
        core.WriteMemory(0x8003, 0x09);
        Assert.Equal(9, Bank16(core, 0x8000));
        Assert.Equal(9, Bank16(core, 0xC000));
    }

    [Fact] // AMD-021 R2-003 and §4.2 (REQ-CORE-100) — mapper 101
    public void R2_003_the_mapper_101_vectors_of_the_criterion()
    {
        // A synthetic mapper 101 ROM whose every 8 KiB CHR bank carries its number.
        NesCore core = Load(101, 2, chrBanks8k: 4);
        Assert.Equal(0, core.Cartridge!.PpuPeek(0x0000));

        // $6000 <- $06: bits 13 and 14 of the address are set -> CHR bank 6 & 3 = 2.
        core.WriteMemory(0x6000, 0x06);
        Assert.Equal(2, core.Cartridge!.PpuPeek(0x0000));

        // $E000 <- $03: bits 13 and 14 set as well -> bank 3.
        core.WriteMemory(0xE000, 0x03);
        Assert.Equal(3, core.Cartridge!.PpuPeek(0x0000));

        // $8000 <- $01: bit 13 is clear -> no change.
        core.WriteMemory(0x8000, 0x01);
        Assert.Equal(3, core.Cartridge!.PpuPeek(0x0000));

        // $A000 and $C000 are the two other addresses that do not carry both bits.
        core.WriteMemory(0xA000, 0x01);
        core.WriteMemory(0xC000, 0x01);
        Assert.Equal(3, core.Cartridge!.PpuPeek(0x0000));
    }

    // =============================================== R2-005, instant of capture

    /// <summary>Advances the PPU, three dots at a time, to <paramref name="scanline"/>.</summary>
    private static void StepToLine(NesCore core, int scanline)
    {
        for (int guard = 0; guard < 400_000 && core.Ppu.Scanline != scanline; guard++)
        {
            core.Bus.Tick();
        }

        Assert.Equal(scanline, core.Ppu.Scanline);
    }

    /// <summary>Advances inside the current scanline to at least <paramref name="dot"/>.</summary>
    private static void StepToDot(NesCore core, int dot)
    {
        int line = core.Ppu.Scanline;
        for (int guard = 0; guard < 200 && core.Ppu.Dot < dot; guard++)
        {
            core.Bus.Tick();
        }

        Assert.Equal(line, core.Ppu.Scanline);
        Assert.True(core.Ppu.Dot >= dot);
    }

    /// <summary>
    /// A core showing a 32 × 30 nametable in which the cells of nametable row <c>r</c> all hold
    /// the pattern number <c>r + 1</c>, background rendering on and no scroll, with the write
    /// latch of <c>$2006</c> left clear.
    /// </summary>
    private static NesCore RowProbe()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Horizontal);
        SpecRom.FillOam(core, 0xFF);
        for (int row = 0; row < 30; row++)
        {
            byte[] cells = new byte[32];
            Array.Fill(cells, (byte)(row + 1));
            SpecRom.Vram(core, 0x2000 + (32 * row), cells);
        }

        core.WriteMemory(0x2005, 0);
        core.WriteMemory(0x2005, 0);
        core.WriteMemory(0x2000, 0x00);
        core.WriteMemory(0x2001, 0x0A);
        core.RunFrame(0);
        return core;
    }

    /// <summary>
    /// Runs one rendered frame by hand and, at <paramref name="dot"/> of line
    /// <paramref name="line"/>, writes <paramref name="address"/> to <c>$2006</c>. Returns the
    /// screen lines the background rows of that frame were captured for, which REQ-CORE-038
    /// reports as <c>y = L − 1</c>.
    /// </summary>
    private static (int[] Ys, TileRecord[] Records) FrameWithMidLineWrite(
        NesCore core, int line, int dot, int? address)
    {
        StepToLine(core, 0);
        if (address is { } target)
        {
            StepToLine(core, line);
            StepToDot(core, dot);
            core.WriteMemory(0x2006, (byte)(target >> 8));
            core.WriteMemory(0x2006, (byte)(target & 0xFF));
        }

        StepToLine(core, 241);
        TileRecord[] records = [.. core.Ppu.TileRecords.ToArray().Where(r => r.Background)];
        return ([.. records.Select(r => (int)r.Y).Distinct().Order()], records);
    }

    [Fact] // AMD-021 R2-005, criterion 1 (REQ-CORE-037)
    public void R2_005_a_mid_line_write_of_fine_y_one_emits_the_row_of_that_line()
    {
        // Without a write the rows sit on lines 1, 9 … 233, that is y = 0, 8 … 232.
        NesCore core = RowProbe();
        (int[] plain, _) = FrameWithMidLineWrite(core, 0, 0, address: null);
        Assert.Equal([.. Enumerable.Range(0, 30).Select(j => 8 * j)], plain);

        // $2006 written during line 100, with coarse Y 12 and fine Y 1. The capture reflects
        // the state left by the write, so line 100 does emit a row, and its y is 99.
        (int[] ys, TileRecord[] records) = FrameWithMidLineWrite(core, 100, 100, address: 0x1180);
        Assert.Contains(99, ys);

        // The nametables are read in that same state: the row carries nametable row 12.
        Assert.All(
            records.Where(r => r.Y == 99),
            record => Assert.Equal(SpecRom.Plane0(0, 13), record.Plane0));

        // Line 97 is a fine-Y-1 line of its own and was captured before the write; the rows
        // after the write follow the new fine Y, so the line-105 row of the plain frame is
        // gone and line 108 carries one instead.
        Assert.Contains(96, ys);
        Assert.DoesNotContain(104, ys);
        Assert.Contains(107, ys);
    }

    [Fact] // AMD-021 R2-005, criterion 1 — the write made in the horizontal blanking
    public void R2_005_a_write_in_the_horizontal_blanking_still_reaches_the_row_of_its_line()
    {
        // The split of a real game writes $2006 in the horizontal blanking, after the vertical
        // increment of dot 256. The row of line 100 must still show it: that is the whole
        // point of R2-005. The increment of the line is then simply gone — a $2006 write
        // replaces the internal address outright — so line 101 keeps fine Y 1 and carries a
        // row of its own, which the hardware does too.
        NesCore core = RowProbe();
        (int[] ys, TileRecord[] records) = FrameWithMidLineWrite(core, 100, 280, address: 0x1180);

        Assert.Contains(99, ys);
        Assert.Contains(100, ys);
        Assert.DoesNotContain(104, ys);
        Assert.All(
            records.Where(r => r.Y == 99),
            record => Assert.Equal(SpecRom.Plane0(0, 13), record.Plane0));
    }

    [Fact] // AMD-021 R2-005, criterion 1 (second half)
    public void R2_005_a_mid_line_write_of_fine_y_two_suppresses_the_row_of_that_line()
    {
        NesCore core = RowProbe();

        // Line 97 would carry a row (y = 96). Writing an address of fine Y 2 during that line
        // makes the end-of-line state fine Y 2, and no row is emitted for the line.
        (int[] ys, _) = FrameWithMidLineWrite(core, 97, 280, address: 0x2180);
        Assert.DoesNotContain(96, ys);

        // The earlier rows are untouched.
        Assert.Equal([.. Enumerable.Range(0, 12).Select(j => 8 * j)], [.. ys.Where(y => y < 96)]);
    }

    // ====================================================== R2-006, Zapper light

    [Fact] // AMD-021 R2-006 §4.2 (a) (REQ-CORE-062)
    public void R2_006_a_read_during_vblank_sees_only_the_universal_backdrop()
    {
        // (100, 100) sits on a solid white sprite. During vblank the colour considered is the
        // universal backdrop, so a dark backdrop means no light even though a white sprite was
        // drawn there.
        NesCore dark = WhiteSpriteAt(0x0F);
        StepToLine(dark, 245);
        Assert.False(dark.Ppu.SeesLightAt(100, 100));

        // With a light backdrop ($30) the same read does see light.
        NesCore light = WhiteSpriteAt(0x30);
        StepToLine(light, 245);
        Assert.True(light.Ppu.SeesLightAt(100, 100));

        // Mid-scan, past the pixel, the records are what count again: the white sprite lights
        // up over the dark backdrop. This is what makes the vblank rule observable.
        NesCore scan = WhiteSpriteAt(0x0F);
        StepToLine(scan, 150);
        Assert.True(scan.Ppu.SeesLightAt(103, 103));
    }

    [Fact] // AMD-021 R2-006 §1 and AMD-022 Q-059.5 — post-render and pre-render lines
    public void R2_006_every_line_outside_the_visible_scan_sees_the_backdrop()
    {
        foreach (int line in new[] { 240, 245, 261 })
        {
            NesCore dark = WhiteSpriteAt(0x0F);
            StepToLine(dark, line);
            Assert.False(dark.Ppu.SeesLightAt(100, 100), $"line {line}, dark backdrop");

            NesCore light = WhiteSpriteAt(0x30);
            StepToLine(light, line);
            Assert.True(light.Ppu.SeesLightAt(100, 100), $"line {line}, light backdrop");
        }
    }

    [Fact] // AMD-021 R2-006 §4.2 (b) (REQ-CORE-062)
    public void R2_006_a_pixel_at_or_past_column_239_always_sees_the_backdrop()
    {
        Assert.Equal(239, Zax.Nes.Ppu.Ppu2C02.ZapperBackdropColumn);

        // Two 8 × 8 sprites on line 100: one over the columns 231 to 238, one over 240 to 247.
        // With a dark backdrop and white sprites, column 238 lights up and column 240 does
        // not, although both show the same white sprite colour.
        NesCore dark = SpriteAt(100, backdrop: 0x0F, spriteColor: 0x30, 231, 240);
        StepToLine(dark, 150);
        Assert.True(dark.Ppu.SeesLightAt(238, 100));
        Assert.False(dark.Ppu.SeesLightAt(240, 100));

        // With a light backdrop and black sprites it is the other way round: column 240 lights
        // up because only the backdrop is looked at there, and column 238 does not.
        NesCore light = SpriteAt(100, backdrop: 0x30, spriteColor: 0x0F, 231, 240);
        StepToLine(light, 150);
        Assert.False(light.Ppu.SeesLightAt(238, 100));
        Assert.True(light.Ppu.SeesLightAt(240, 100));

        // The rule holds for every column from 239 on, whatever is drawn there.
        for (int x = 239; x < 248; x++)
        {
            Assert.False(dark.Ppu.SeesLightAt(x, 100), $"dark backdrop, x = {x}");
            Assert.True(light.Ppu.SeesLightAt(x, 100), $"light backdrop, x = {x}");
        }
    }

    /// <summary>A rendered frame with a solid white sprite at (100, 100) over <paramref name="backdrop"/>.</summary>
    private static NesCore WhiteSpriteAt(byte backdrop) => SpriteAt(100, backdrop, 0x30, 100);

    /// <summary>
    /// A core that has rendered one frame showing 8 × 8 sprites of colour
    /// <paramref name="spriteColor"/> on the eight lines starting at <paramref name="y"/>, at
    /// the columns <paramref name="xs"/>, over the universal backdrop
    /// <paramref name="backdrop"/>, and is stopped at the start of the next frame.
    /// </summary>
    private static NesCore SpriteAt(int y, byte backdrop, byte spriteColor, params int[] xs)
    {
        NesCore core = SpecRom.Warm();
        SpecRom.Vram(core, 0x3F00, backdrop);

        // Pattern $80 of table 0 draws colour 1 in its leftmost column and colour 2 in the
        // other seven, so both entries of the sprite sub-palette carry the wanted colour.
        SpecRom.Vram(core, 0x3F11, spriteColor);
        SpecRom.Vram(core, 0x3F12, spriteColor);
        SpecRom.FillOam(core, 0xFF);

        // OAM y is the line above the first line of the sprite.
        for (int n = 0; n < xs.Length; n++)
        {
            SpecRom.Oam(core, n, (byte)(y - 1), tile: 0x80, attribute: 0x00, x: (byte)xs[n]);
        }

        core.WriteMemory(0x2000, 0x00);
        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0);
        core.RunFrame(0);
        return core;
    }

    // ================================================ R2-008, candidate order

    private static TileImageKey Image(int id) => new(ulong.MaxValue, 1UL << id, false, false);

    private static EnginePattern Pattern(ImageRegistry registry, params int[] ids)
    {
        TileImage?[] cells = [.. ids.Select(id => (TileImage?)registry.Intern(Image(id)))];
        EnginePattern pattern = new(ids.Length, 1, cells, permanent: true);
        pattern.SetSelfConnected(true);
        pattern.AddInstance(new PatternInstance());
        return pattern;
    }

    [Fact] // AMD-021 R2-008 §4.2 (REQ-E3D-091)
    public void R2_008_a_newcomer_takes_the_place_of_the_last_pattern_removed()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage x = registry.Intern(Image(0));

        // P1 then P2, both holding the image X.
        EnginePattern p1 = Pattern(registry, 0, 1);
        EnginePattern p2 = Pattern(registry, 0, 2);
        patterns.Register(p1);
        patterns.Register(p2);
        Assert.Equal<EnginePattern[]>([p1, p2], [.. x.PermanentPatterns]);

        // P1 removed, then P3 registered: P3 takes the place P1 held, so the candidates of X
        // are P3 then P2 — not P2 then P3 as a pure order of registration would give.
        patterns.Remove(p1);
        EnginePattern p3 = Pattern(registry, 0, 3);
        patterns.Register(p3);
        Assert.Equal<EnginePattern[]>([p3, p2], [.. x.PermanentPatterns]);
    }

    [Fact] // AMD-021 R2-008 §4.2, second half
    public void R2_008_without_a_removal_a_newcomer_goes_last()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage x = registry.Intern(Image(0));

        EnginePattern p1 = Pattern(registry, 0, 1);
        EnginePattern p2 = Pattern(registry, 0, 2);
        EnginePattern p3 = Pattern(registry, 0, 3);
        patterns.Register(p1);
        patterns.Register(p2);
        patterns.Register(p3);

        Assert.Equal<EnginePattern[]>([p1, p2, p3], [.. x.PermanentPatterns]);
    }

    [Fact] // AMD-022 Q-059.4 — the freed place is consumed by the first pattern that takes it
    public void R2_008_a_freed_place_serves_one_registration_only()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage x = registry.Intern(Image(0));

        EnginePattern p1 = Pattern(registry, 0, 1);
        EnginePattern p2 = Pattern(registry, 0, 2);
        patterns.Register(p1);
        patterns.Register(p2);
        patterns.Remove(p1);

        EnginePattern p3 = Pattern(registry, 0, 3);
        EnginePattern p4 = Pattern(registry, 0, 4);
        patterns.Register(p3);
        patterns.Register(p4);

        // P3 took the place; P4 finds none free and goes last.
        Assert.Equal<EnginePattern[]>([p3, p2, p4], [.. x.PermanentPatterns]);
    }

    [Fact] // AMD-021 R2-008 — "la place du dernier pattern retiré", per image
    public void R2_008_the_place_is_the_one_the_last_removal_freed_in_that_image()
    {
        ImageRegistry registry = new();
        PatternRegistry patterns = new();
        TileImage x = registry.Intern(Image(0));

        EnginePattern p1 = Pattern(registry, 0, 1);
        EnginePattern p2 = Pattern(registry, 0, 2);
        EnginePattern p3 = Pattern(registry, 0, 3);
        patterns.Register(p1);
        patterns.Register(p2);
        patterns.Register(p3);

        // Two removals: the place the newcomer takes is the one the *last* removal freed.
        patterns.Remove(p1);
        patterns.Remove(p3);
        Assert.Equal<EnginePattern[]>([p2], [.. x.PermanentPatterns]);

        EnginePattern p4 = Pattern(registry, 0, 4);
        patterns.Register(p4);
        Assert.Equal<EnginePattern[]>([p2, p4], [.. x.PermanentPatterns]);

        // And the place is kept per image: a pattern that sits at index 0 of one image and at
        // index 1 of another frees both.
        ImageRegistry other = new();
        PatternRegistry others = new();
        TileImage a = other.Intern(Image(0));
        TileImage b = other.Intern(Image(1));
        EnginePattern onlyB = Pattern(other, 1);
        EnginePattern both = Pattern(other, 0, 1);
        EnginePattern onlyA = Pattern(other, 0);
        others.Register(onlyB);
        others.Register(both);
        others.Register(onlyA);
        Assert.Equal<EnginePattern[]>([both, onlyA], [.. a.PermanentPatterns]);
        Assert.Equal<EnginePattern[]>([onlyB, both], [.. b.PermanentPatterns]);

        others.Remove(both);
        EnginePattern newcomer = Pattern(other, 0, 1);
        others.Register(newcomer);
        Assert.Equal<EnginePattern[]>([newcomer, onlyA], [.. a.PermanentPatterns]);
        Assert.Equal<EnginePattern[]>([onlyB, newcomer], [.. b.PermanentPatterns]);
    }

    // ============================================ R2-010, no shadow on a transparent

    /// <summary>
    /// R2-010 turns the "DEVRAIT" of REQ-SCN-032 into a "NE DOIT PAS" for every transparent
    /// tile and for the HUD, whatever the flags of its motif and of its layer say. The rule
    /// lives in the renderer, not in the engine's draw list — the engine still reports the
    /// flags of the motif and the layer — so what is checked here is that every tile the rule
    /// covers is classified in a material the renderer excludes, and the rendered result
    /// itself is measured by
    /// <see cref="Amd021SceneTests.R2_010_a_transparent_receiver_is_the_same_with_and_without_shadows"/>.
    /// </summary>
    [Fact] // AMD-021 R2-010 (REQ-SCN-032)
    public void R2_010_every_tile_the_rule_covers_is_transparent_or_hud()
    {
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 8, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 8, ProfileLayer.L1, alpha: 0.5f)
            .Pattern(SceneFixture.Solid(1, 2), depth: 8, ProfileLayer.L2, alpha: 0.5f)
            .Pattern(SceneFixture.Solid(1, 3), depth: 8, ProfileLayer.Ui)
            .Pattern(SceneFixture.Solid(1, 4), depth: 8, ProfileLayer.Ui, alpha: 0.5f)
            .Tile(SceneFixture.Solid(1, 0), 0, 96)
            .Tile(SceneFixture.Solid(1, 1), 16, 96)
            .Tile(SceneFixture.Solid(1, 2), 32, 96)
            .Tile(SceneFixture.Solid(1, 3), 48, 96)
            .Tile(SceneFixture.Solid(1, 4), 64, 96);

        SceneState scene = fixture.Build();
        Dictionary<int, DrawTile> byColumn = scene.DrawList!.Tiles
            .ToDictionary(static tile => tile.Tile.Position.X);

        // The opaque L1 tile is the control: opaque, and it does receive.
        Assert.Equal(MaterialClass.Opaque, At(0).Material);
        Assert.True(At(0).ReceiveShadow, "an opaque L1 tile receives");

        // Every tile R2-010 covers is either transparent or HUD, the two classes the renderer
        // takes the shadow away from.
        Assert.Equal(MaterialClass.Transparent, At(16).Material);
        Assert.Equal(MaterialClass.Transparent, At(32).Material);
        Assert.Equal(SceneContainer.Interface, At(48).Container);
        Assert.Equal(SceneContainer.Interface, At(64).Container);

        // None of them casts either (REQ-SCN-032 unchanged on that point).
        foreach (int column in new[] { 16, 32, 48, 64 })
        {
            Assert.False(At(column).CastShadow, $"column {column} must not cast");
        }

        DrawTile At(int pixelX)
        {
            int cell = (pixelX / 8) + 1;
            Assert.True(byColumn.ContainsKey(cell), $"no tile in cell column {cell}.");
            return byColumn[cell];
        }
    }

    // ================================================ R2-013, sounds of the options

    [Fact] // AMD-021 R2-013 and §4.2 (REQ-AUD-041)
    public void R2_013_speed_and_fov_are_the_only_silent_settings_of_the_options_window()
    {
        // The two settings the amendment names, and only those two.
        Assert.Equal<string[]>(
            ["camera.fov", "emu.game_speed"],
            [.. UiDriver.SilentOptionKeys().Order(StringComparer.Ordinal)]);

        Assert.False(UiDriver.PlaysValidationSound("emu.game_speed"));
        Assert.False(UiDriver.PlaysValidationSound("camera.fov"));

        // Every other row of every tab of the window plays the validation sound exactly once
        // per change of value.
        var rows = UiDriver.OptionRows();
        Assert.NotEmpty(rows);
        foreach ((string tab, string key, _, _, _) in rows)
        {
            bool silent = key is "emu.game_speed" or "camera.fov";
            Assert.Equal(!silent, UiDriver.PlaysValidationSound(key));
            Assert.NotEmpty(tab);
        }

        // The two silent ones are rows of the window, not names absent from it.
        Assert.Contains(rows, row => row.Key == "emu.game_speed");
        Assert.Contains(rows, row => row.Key == "camera.fov");
    }

    [Fact] // AMD-021 R2-013 — one sound per change, counted on the mixer
    public void R2_013_a_change_of_value_plays_the_validation_sound_exactly_once()
    {
        // REQ-AUD-040 gives the interface a single voice, so two sounds cannot be told apart by
        // "a sound is playing": the mixer's counter is the observable.
        AudioMixer mixer = new(48_000);
        WaveAudio validation = new(new float[480], 1, 48_000);

        long Change(string key)
        {
            long before = mixer.SoundsPlayed;
            if (UiDriver.PlaysValidationSound(key))
            {
                mixer.PlayClip(validation);
            }

            return mixer.SoundsPlayed - before;
        }

        Assert.Equal(1, Change("render.shadow"));
        Assert.Equal(1, Change("render.shadow"));
        Assert.Equal(1, Change("audio.volume"));
        Assert.Equal(0, Change("emu.game_speed"));
        Assert.Equal(0, Change("camera.fov"));
    }
}
