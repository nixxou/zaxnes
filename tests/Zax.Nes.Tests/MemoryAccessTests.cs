namespace Zax.Nes.Tests;

/// <summary>
/// Host memory access and open bus (REQ-CORE-080 to REQ-CORE-082, REQ-CORE-095).
/// </summary>
public sealed class MemoryAccessTests
{
    private static NesCore LoadedCore(byte headerByte6 = 0x01, byte[]? program = null)
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(headerByte6, program), "Minimal.nes").Success);
        return core;
    }

    [Fact]
    public void REQ_CORE_080_work_ram_is_mirrored_every_2_kib()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x0012, 0x42);

        Assert.Equal(0x42, core.ReadMemory(0x0012));
        Assert.Equal(0x42, core.ReadMemory(0x0812));
        Assert.Equal(0x42, core.ReadMemory(0x1012));
        Assert.Equal(0x42, core.ReadMemory(0x1812));
    }

    [Fact]
    public void REQ_CORE_080_reading_fffc_returns_the_reset_vector_of_the_current_bank()
    {
        NesCore core = LoadedCore();

        // The minimal ROM points every vector at $8000.
        Assert.Equal(0x00, core.ReadMemory(0xFFFC));
        Assert.Equal(0x80, core.ReadMemory(0xFFFD));
    }

    [Fact]
    public void REQ_CORE_080_prg_ram_is_readable_and_writable()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x6000, 0x11);
        core.WriteMemory(0x7FFF, 0x22);

        Assert.Equal(0x11, core.ReadMemory(0x6000));
        Assert.Equal(0x22, core.ReadMemory(0x7FFF));
    }

    [Fact]
    public void REQ_CORE_081_peeking_2002_does_not_clear_the_vblank_flag()
    {
        NesCore core = LoadedCore();

        // A frame ends at the start of vblank, so the flag is set here.
        core.RunFrame(0);
        Assert.True(core.Ppu.VBlankFlag);

        // The host accessor returns 0 for $2000-$401F and leaves the PPU alone.
        Assert.Equal(0x00, core.ReadMemory(0x2002));
        Assert.True(core.Ppu.VBlankFlag);

        // The PPU's own non-destructive view agrees and is equally inert.
        Assert.Equal(0x80, core.Ppu.Peek(2) & 0x80);
        Assert.True(core.Ppu.VBlankFlag);

        // A real CPU read, on the other hand, still clears it.
        Assert.Equal(0x80, core.Ppu.Read(2) & 0x80);
        Assert.False(core.Ppu.VBlankFlag);
        Assert.Equal(0x00, core.Ppu.Peek(2) & 0x80);
    }

    [Fact]
    public void REQ_CORE_081_registers_and_the_expansion_area_peek_as_zero()
    {
        NesCore core = LoadedCore();

        for (ushort address = 0x2000; address <= 0x401F; address++)
        {
            Assert.Equal(0, core.ReadMemory(address));
        }

        Assert.Equal(0, core.ReadMemory(0x4020));
        Assert.Equal(0, core.ReadMemory(0x5FFF));
    }

    [Fact]
    public void REQ_CORE_082_writing_the_palette_through_2006_and_2007_shows_up_in_the_batch()
    {
        NesCore core = LoadedCore();

        // Three frames first, as SPEC-02 §5.1 asks, then program the palette.
        for (int i = 0; i < 3; i++)
        {
            core.RunFrame(0);
        }

        core.WriteMemory(0x2006, 0x3F);
        core.WriteMemory(0x2006, 0x00);
        core.WriteMemory(0x2007, 0x30);

        byte entry0 = 0xFF;
        core.VideoCallback = (in FrameBatch batch) => entry0 = batch.Palette[0];
        core.RunFrame(0);

        Assert.Equal(0x30, entry0);
    }

    [Fact]
    public void REQ_CORE_045_palette_vector_v7()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 3; i++)
        {
            core.RunFrame(0);
        }

        WritePalette(core, 0x3F00, 0x0F);
        WritePalette(core, 0x3F01, 0x30);
        WritePalette(core, 0x3F11, 0x16);

        byte[] palette = CapturePalette(core);
        Assert.Equal(0x0F, palette[0]);
        Assert.Equal(0x30, palette[1]);
        Assert.Equal(0x16, palette[17]);

        // Writing $3F10 lands on entry 0, and the mirror at 16 follows (REQ-CORE-045).
        WritePalette(core, 0x3F10, 0x21);
        palette = CapturePalette(core);
        Assert.Equal(0x21, palette[0]);
        Assert.Equal(0x21, palette[16]);
    }

    [Fact]
    public void REQ_CORE_045_palette_entries_are_six_bits_wide()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 3; i++)
        {
            core.RunFrame(0);
        }

        WritePalette(core, 0x3F03, 0xFF);

        Assert.Equal(0x3F, CapturePalette(core)[3]);
    }

    [Fact]
    public void REQ_CORE_082_writing_work_ram_has_the_same_effect_as_a_cpu_write()
    {
        // The program copies $0200 to $0201 forever, so a host write at $0200 must show up
        // at $0201 on the next frame.
        byte[] program = [0xAD, 0x00, 0x02, 0x8D, 0x01, 0x02, 0x4C, 0x00, 0x80];
        NesCore core = LoadedCore(0x01, program);

        core.WriteMemory(0x0200, 0x5E);
        core.RunFrame(0);

        Assert.Equal(0x5E, core.ReadMemory(0x0201));
    }

    [Fact]
    public void REQ_CORE_095_battery_ram_is_exported_and_imported_only_with_a_battery()
    {
        NesCore withoutBattery = LoadedCore();
        Assert.True(withoutBattery.ExportBatteryRam().IsEmpty);
        Assert.False(withoutBattery.ImportBatteryRam(new byte[8192]));

        byte[] image = TestRoms.Minimal();
        image[6] |= 0x02; // Battery-backed PRG-RAM.

        var core = new NesCore();
        Assert.True(core.LoadRom(image, "Battery.nes").Success);

        core.WriteMemory(0x6000, 0xAB);
        core.WriteMemory(0x7FFF, 0xCD);

        byte[] exported = core.ExportBatteryRam().ToArray();
        Assert.Equal(8192, exported.Length);
        Assert.Equal(0xAB, exported[0]);
        Assert.Equal(0xCD, exported[^1]);

        // Reload the ROM, import the save: the contents come back.
        Assert.True(core.LoadRom(image, "Battery.nes").Success);
        Assert.Equal(0x00, core.ReadMemory(0x6000));
        Assert.True(core.ImportBatteryRam(exported));
        Assert.Equal(0xAB, core.ReadMemory(0x6000));
        Assert.Equal(0xCD, core.ReadMemory(0x7FFF));

        // And a reset no longer wipes it (REQ-CORE-095).
        core.Reset();
        Assert.Equal(0xAB, core.ReadMemory(0x6000));
    }

    [Fact]
    public void REQ_CORE_081_unmapped_reads_return_the_open_bus()
    {
        // $4018-$401F is disabled, so a CPU read there sees the last value on the bus.
        NesCore core = LoadedCore();
        core.Bus.Poke(0x0000, 0x00);

        byte first = core.Bus.Read(0x0200); // Work RAM: 0.
        Assert.Equal(0x00, first);

        core.Bus.Poke(0x0201, 0x5A);
        Assert.Equal(0x5A, core.Bus.Read(0x0201));
        Assert.Equal(0x5A, core.Bus.Read(0x4018));
    }

    private static void WritePalette(NesCore core, ushort address, byte value)
    {
        core.WriteMemory(0x2006, (byte)(address >> 8));
        core.WriteMemory(0x2006, (byte)address);
        core.WriteMemory(0x2007, value);
    }

    private static byte[] CapturePalette(NesCore core)
    {
        byte[] captured = new byte[32];
        core.VideoCallback = (in FrameBatch batch) => batch.Palette.CopyTo(captured);
        core.RunFrame(0);
        core.VideoCallback = null;
        return captured;
    }
}
