namespace Zax.Nes.Tests;

/// <summary>
/// The soft reset of AMD-044 REQ-CORE-122: the RESET button of the console, as opposed to the
/// power cycle REQ-CORE-004 keeps calling a reset (DEC-02-Q3, unchanged).
/// </summary>
public sealed class SoftResetTests
{
    /// <summary>
    /// REQ-CORE-122: the internal RAM, the save RAM and the nametable RAM survive a soft reset,
    /// where the hard one clears all three (REQ-CORE-004, DEC-02-Q3).
    /// </summary>
    [Fact]
    public void REQ_CORE_122_a_soft_reset_keeps_the_ram_and_the_save_ram()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.WriteMemory(0x0300, 0x55);
        core.WriteMemory(0x6000, 0x66);
        WriteVram(core, 0x2010, 0x77);

        core.RunFrame(0);

        core.SoftReset();

        Assert.Equal(0x55, core.ReadMemory(0x0300));
        Assert.Equal(0x66, core.ReadMemory(0x6000));
        Assert.Equal(0x77, core.Ppu.Vram[core.Ppu.MapNametable(0x2010)]);
        Assert.Equal("Minimal.nes", core.RomName);
        Assert.True(core.IsRunning);

        // The hard reset still clears all three: DEC-02-Q3 is untouched.
        core.Reset();
        Assert.Equal(0x00, core.ReadMemory(0x0300));
        Assert.Equal(0x00, core.ReadMemory(0x6000));
        Assert.Equal(0x00, core.Ppu.Vram[core.Ppu.MapNametable(0x2010)]);
    }

    /// <summary>
    /// REQ-CORE-122: the CHR RAM of a cartridge that has one survives a soft reset. The ROM
    /// declares no CHR bank, so the mapper gives the PPU 8 KiB of writable pattern memory.
    /// </summary>
    [Fact]
    public void REQ_CORE_122_a_soft_reset_keeps_the_chr_ram()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(ChrRamImage(), "ChrRam.nes").Success);

        WriteVram(core, 0x0000, 0x5A);
        Assert.Equal(0x5A, ReadVram(core, 0x0000));

        core.SoftReset();

        Assert.Equal(0x5A, ReadVram(core, 0x0000));

        core.Reset();
        Assert.Equal(0x00, ReadVram(core, 0x0000));
    }

    /// <summary>
    /// REQ-CORE-122: A, X, Y and every flag but <c>I</c> are kept, S drops by 3 without anything
    /// being written to the stack, and PC comes from the reset vector (nesdev, "CPU power up
    /// state", <i>after reset</i>).
    /// </summary>
    [Fact]
    public void REQ_CORE_122_the_cpu_keeps_its_registers_and_sets_i()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.Cpu.A = 0x12;
        core.Cpu.X = 0x34;
        core.Cpu.Y = 0x56;
        core.Cpu.S = 0x80;
        core.Cpu.P = Zax.Nes.Cpu.Cpu6502.FlagC | Zax.Nes.Cpu.Cpu6502.FlagN;

        // Three bytes of the stack page, to show that the three decrements write nothing.
        core.WriteMemory(0x0180, 0xAA);
        core.WriteMemory(0x017F, 0xBB);
        core.WriteMemory(0x017E, 0xCC);

        core.SoftReset();

        Assert.Equal(0x12, core.Cpu.A);
        Assert.Equal(0x34, core.Cpu.X);
        Assert.Equal(0x56, core.Cpu.Y);
        Assert.Equal(0x80 - 3, core.Cpu.S);
        Assert.Equal(0x8000, core.Cpu.PC);

        Assert.True(core.Cpu.GetFlag(Zax.Nes.Cpu.Cpu6502.FlagI));
        Assert.True(core.Cpu.GetFlag(Zax.Nes.Cpu.Cpu6502.FlagC));
        Assert.True(core.Cpu.GetFlag(Zax.Nes.Cpu.Cpu6502.FlagN));

        Assert.Equal(0xAA, core.ReadMemory(0x0180));
        Assert.Equal(0xBB, core.ReadMemory(0x017F));
        Assert.Equal(0xCC, core.ReadMemory(0x017E));

        // No interrupt is left waiting.
        Assert.False(core.Cpu.NmiPending);
    }

    /// <summary>
    /// REQ-CORE-122: the PPU follows its documented <i>after reset</i> state — PPUCTRL and
    /// PPUMASK at 0, the write latch cleared, the <c>$2007</c> read buffer at 0 — while
    /// OAMADDR, the nametable RAM and the OAM stay as they were.
    /// </summary>
    [Fact]
    public void REQ_CORE_122_the_ppu_follows_its_after_reset_state()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        WriteVram(core, 0x2010, 0x77);

        core.Ppu.Write(0, 0x9E);
        core.Ppu.Write(1, 0x1E);
        core.Ppu.Write(3, 0x40);

        // One half of a $2006 pair leaves the write latch set.
        core.Ppu.Write(6, 0x21);

        core.SoftReset();

        Assert.Equal(0, core.Ppu.Control);
        Assert.Equal(0, core.Ppu.Mask);
        Assert.Equal(0x40, core.Ppu.OamAddress);

        // The latch is cleared, so the next two writes form a complete address again and the
        // read lands on $2010 rather than on $2110.
        Assert.Equal(0x77, ReadVram(core, 0x2010));
    }

    /// <summary>
    /// REQ-CORE-122: the APU goes silent as a <c>$4015 = 0</c> write would, and the frame IRQ
    /// flag goes away. The <c>$4017</c> mode is unchanged.
    /// </summary>
    [Fact]
    public void REQ_CORE_122_the_apu_is_silenced_and_4015_reads_zero()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        // Arm the two pulse channels and let their length counters load.
        core.WriteMemory(0x4015, 0x0F);
        core.WriteMemory(0x4000, 0x30);
        core.WriteMemory(0x4003, 0x08);
        core.WriteMemory(0x4004, 0x30);
        core.WriteMemory(0x4007, 0x08);
        core.RunFrame(0);
        Assert.NotEqual(0, core.Apu.PeekStatus() & 0x03);

        core.SoftReset();

        Assert.Equal(0, core.Apu.PeekStatus());
        Assert.False(core.Apu.IrqAsserted);
    }

    /// <summary>
    /// REQ-CORE-006 as amended by AMD-044: the core frame counter restarts from 0 on a soft
    /// reset, as it does when a ROM is loaded and on a hard reset.
    /// </summary>
    [Fact]
    public void REQ_CORE_006_the_frame_counter_restarts_from_zero()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        for (int frame = 0; frame < 5; frame++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(5, core.FrameCount);

        core.SoftReset();

        Assert.Equal(0, core.FrameCount);

        core.RunFrame(0);
        Assert.Equal(1, core.FrameCount);
    }

    /// <summary>
    /// REQ-CORE-122: the latch and the shift registers of the controller ports go to zero. The
    /// buttons the player is holding are read again on the next frame, so a held A still
    /// reaches the game.
    /// </summary>
    [Fact]
    public void REQ_CORE_122_the_port_latch_and_shift_registers_are_cleared()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.SetPortType(1, ControllerPortType.Zapper);

        // The player holds A: the strobe latches it and the first read hands it over.
        core.RunFrame(0x0001);
        core.Input.Write(0x01);
        core.Input.Write(0x00);
        Assert.Equal(1, core.Input.Read(0, 0) & 0x01);

        core.SoftReset();

        // The latch is down and the shift registers are empty, so a read with no strobe
        // returns 0 even though the button is still held.
        Assert.Equal(0, core.Input.Read(0, 0) & 0x01);

        // The port types survive, exactly as they do across a hard reset (REQ-CORE-004).
        Assert.Equal(ControllerPortType.Zapper, core.GetPortType(1));

        // The next frame reads the held buttons again (AMD-044 §2).
        core.RunFrame(0x0001);
        core.Input.Write(0x01);
        core.Input.Write(0x00);
        Assert.Equal(1, core.Input.Read(0, 0) & 0x01);
    }

    /// <summary>
    /// REQ-CORE-122 complement: a home-made ROM that counts its resets in RAM shows 3 after
    /// three soft resets. The program increments <c>$0010</c> once and then loops, so the count
    /// is the number of times the reset vector has been taken.
    /// </summary>
    [Fact]
    public void REQ_CORE_122_a_rom_that_counts_its_resets_in_ram_shows_three()
    {
        // INC $10 at $8000, then JMP $8002 — the jump to itself: exactly one increment per
        // trip through the reset vector.
        NesCore core = new();
        Assert.True(core.LoadRom(
            TestRoms.Minimal(program: [0xE6, 0x10, 0x4C, 0x02, 0x80]),
            "Counter.nes").Success);

        core.RunFrame(0);
        Assert.Equal(1, core.ReadMemory(0x0010));

        for (int reset = 0; reset < 2; reset++)
        {
            core.SoftReset();
            core.RunFrame(0);
        }

        Assert.Equal(3, core.ReadMemory(0x0010));

        // The hard reset, being a power cycle, loses the count (REQ-CORE-004, DEC-02-Q3).
        core.Reset();
        core.RunFrame(0);
        Assert.Equal(1, core.ReadMemory(0x0010));
    }

    /// <summary>A soft reset with no ROM loaded does nothing at all (REQ-CORE-005).</summary>
    [Fact]
    public void REQ_CORE_122_a_soft_reset_without_a_rom_does_nothing()
    {
        NesCore core = new();
        core.SoftReset();

        Assert.False(core.IsRunning);
        Assert.Equal(0, core.FrameCount);
    }

    /// <summary>Writes one PPU-bus byte through $2006 / $2007, as a game would.</summary>
    private static void WriteVram(NesCore core, ushort address, byte value)
    {
        core.Ppu.Write(6, (byte)(address >> 8));
        core.Ppu.Write(6, (byte)address);
        core.Ppu.Write(7, value);
    }

    /// <summary>
    /// Reads one PPU-bus byte through $2006 / $2007. The first $2007 read returns the buffer,
    /// so two are needed.
    /// </summary>
    private static byte ReadVram(NesCore core, ushort address)
    {
        core.Ppu.Write(6, (byte)(address >> 8));
        core.Ppu.Write(6, (byte)address);

        // The first read returns the stale buffer and loads the byte at the address; the
        // second one hands that byte over.
        core.Ppu.Read(7);
        return core.Ppu.Read(7);
    }

    /// <summary>
    /// An NROM image declaring <b>no</b> CHR bank, which gives the cartridge CHR RAM
    /// (<c>InesHeader.DefaultChrRamSize</c>). The three vectors point at an endless loop.
    /// </summary>
    private static byte[] ChrRamImage()
    {
        const int prgSize = 2 * 16 * 1024;
        byte[] image = new byte[16 + prgSize];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x02;
        image[5] = 0x00;
        image[6] = 0x01;

        image[16] = 0x4C;
        image[17] = 0x00;
        image[18] = 0x80;

        int vectors = 16 + prgSize - 6;
        for (int i = 0; i < 3; i++)
        {
            image[vectors + (2 * i)] = 0x00;
            image[vectors + (2 * i) + 1] = 0x80;
        }

        return image;
    }
}
