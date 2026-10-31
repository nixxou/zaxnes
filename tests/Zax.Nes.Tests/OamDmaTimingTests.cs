namespace Zax.Nes.Tests;

/// <summary>
/// OAM DMA: the 513 or 514 cycles it steals from the CPU and the transfer itself
/// (nesdev "DMA"; REQ-CORE-110 cycle accounting, REQ-CORE-111 timing).
/// </summary>
public sealed class OamDmaTimingTests
{
    private static NesCore LoadedCore()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Dma.nes").Success);
        return core;
    }

    [Fact]
    public void REQ_CORE_111_oam_dma_steals_513_cycles_on_an_even_cycle_and_514_on_an_odd_one()
    {
        var stalls = new List<long>();

        foreach (int padding in new[] { 0, 1 })
        {
            NesCore core = LoadedCore();

            // The reset sequence left the clock at 7; one extra tick flips its parity.
            for (int i = 0; i < padding; i++)
            {
                core.Bus.Tick();
            }

            long before = core.Bus.CycleCount;
            core.Bus.Write(0x4014, 0x02);
            long elapsed = core.Bus.CycleCount - before;

            // The write cycle itself is not part of the stall.
            stalls.Add(elapsed - 1);
            Assert.Equal(1, core.Bus.OamDmaCount);
        }

        Assert.Equal<long[]>([513, 514], [.. stalls.Order()]);
        Assert.Contains(NesBus.OamDmaCyclesEven, stalls);
        Assert.Contains(NesBus.OamDmaCyclesOdd, stalls);
    }

    [Fact]
    public void REQ_CORE_111_oam_dma_copies_the_whole_page_into_oam()
    {
        NesCore core = LoadedCore();

        for (int i = 0; i < 256; i++)
        {
            core.WriteMemory((ushort)(0x0200 + i), (byte)(255 - i));
        }

        core.Bus.Write(0x4014, 0x02);

        for (int i = 0; i < 256; i++)
        {
            // Bits 2 to 4 of a sprite's byte 2 do not exist in the chip and read back as 0
            // (nesdev, "PPU OAM"; oam_read, ppu_open_bus test 10).
            byte expected = (i & 0x03) == 0x02
                ? (byte)((255 - i) & 0xE3)
                : (byte)(255 - i);

            Assert.Equal(expected, core.Ppu.Oam[i]);
        }
    }

    [Fact]
    public void REQ_CORE_111_oam_dma_starts_at_the_current_oam_address()
    {
        NesCore core = LoadedCore();

        for (int i = 0; i < 256; i++)
        {
            core.WriteMemory((ushort)(0x0300 + i), (byte)i);
        }

        core.WriteMemory(0x2003, 0x10); // OAMADDR = $10.
        core.Bus.Write(0x4014, 0x03);

        // The transfer wraps around inside OAM.
        Assert.Equal(0x00, core.Ppu.Oam[0x10]);
        Assert.Equal(0xEF, core.Ppu.Oam[0xFF]);
        Assert.Equal(0xF0, core.Ppu.Oam[0x00]);
        Assert.Equal(0xFF, core.Ppu.Oam[0x0F]);
    }

    /// <summary>
    /// REQ-CORE-111 — the 256 bytes are read <em>through the CPU bus</em>, so a page at $2000
    /// and above reads whatever is mapped there rather than work RAM. With page $20 the DMA
    /// performs 32 passes over the eight PPU registers: $2002 is read, which hands the VBlank
    /// flag over and clears it, exactly as a program reading $2002 by hand would (nesdev,
    /// "DMA", "PPU registers").
    /// </summary>
    [Fact]
    public void REQ_CORE_111_a_dma_from_the_ppu_register_page_reads_the_registers()
    {
        NesCore core = LoadedCore();

        // A pattern in work RAM at $0000-$00FF and its $2000 mirror distance apart: if the
        // DMA mistook the page for RAM, this is what OAM would hold.
        for (int i = 0; i < 256; i++)
        {
            core.WriteMemory((ushort)i, 0x5A);
        }

        // Stop at the start of a vblank, where the VBlank flag is up.
        core.RunFrame(0);
        Assert.True(core.Ppu.VBlankFlag);

        core.Bus.Write(0x4014, 0x20);

        // The very first read of the page is $2000, which the PPU does not drive, then
        // $2001, then $2002: the third byte carries the flag the DMA has just consumed.
        Assert.Equal(0x80, core.Ppu.Oam[0x02] & 0x80);
        Assert.False(core.Ppu.VBlankFlag);

        // The next pass over $2002, eight bytes later, no longer sees it.
        Assert.Equal(0x00, core.Ppu.Oam[0x0A] & 0x80);

        // Nothing in OAM came from work RAM.
        Assert.DoesNotContain((byte)0x5A, core.Ppu.Oam[..0x10].ToArray());
    }

    /// <summary>
    /// REQ-CORE-111 — a page in the APU and I/O range reads mostly <strong>open bus</strong>:
    /// $4000-$4013 and $4018-$401F drive nothing, so the read returns the last value that
    /// travelled on the bus, which is the page number the $4014 write itself put there.
    /// </summary>
    [Fact]
    public void REQ_CORE_111_a_dma_from_the_io_page_reads_the_open_bus()
    {
        NesCore core = LoadedCore();

        for (int i = 0; i < 256; i++)
        {
            core.WriteMemory((ushort)(0x0000 + i), 0x5A);
        }

        core.Bus.Write(0x4014, 0x40);

        // $4000-$4014 are write-only, so each read returns the last value that travelled on
        // the bus — the page number, which the $4014 write itself left there.
        for (int i = 0x00; i <= 0x14; i++)
        {
            Assert.Equal(0x40, core.Ppu.Oam[i]);
        }

        // $4015 is the one register of the page that does drive the bus. Nothing is playing,
        // so it reads 0 (bit 5 is the open bus, clear in $40) — and that 0 becomes the new
        // open bus, which every later read of the page, cartridge space at $4020-$40FF
        // included, then returns.
        Assert.Equal(0x00, core.Ppu.Oam[0x15]);
        Assert.Equal(0x00, core.Ppu.Oam[0x80]);
        Assert.Equal(0x00, core.Ppu.Oam[0xFF]);

        // Whatever the page held, none of it came from work RAM.
        Assert.DoesNotContain((byte)0x5A, core.Ppu.Oam.ToArray());
    }

    /// <summary>
    /// REQ-CORE-111 — a page in cartridge space reads the PRG-ROM the current banks show,
    /// again through the CPU bus (REQ-CORE-080).
    /// </summary>
    [Fact]
    public void REQ_CORE_111_a_dma_from_cartridge_space_reads_prg_rom()
    {
        // $8000 onwards: the endless loop, then a recognisable ramp.
        var program = new byte[256];
        program[0] = 0x4C;
        program[1] = 0x00;
        program[2] = 0x80;
        for (int i = 3; i < program.Length; i++)
        {
            program[i] = (byte)(0xC0 | (i & 0x0F));
        }

        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, program), "Dma.nes").Success);

        for (int i = 0; i < 256; i++)
        {
            core.WriteMemory((ushort)i, 0x5A);
        }

        core.Bus.Write(0x4014, 0x80);

        Assert.Equal(0x4C, core.Ppu.Oam[0x00]);
        Assert.Equal(0x00, core.Ppu.Oam[0x01]);
        Assert.Equal(0x80 & 0xE3, core.Ppu.Oam[0x02]);

        for (int i = 4; i < 256; i++)
        {
            byte expected = (byte)(0xC0 | (i & 0x0F));
            Assert.Equal((i & 0x03) == 0x02 ? (byte)(expected & 0xE3) : expected, core.Ppu.Oam[i]);
        }
    }

    [Fact]
    public void REQ_CORE_111_a_sta_4014_instruction_costs_its_own_cycles_plus_the_stall()
    {
        // LDA #$02 / STA $4014 / JMP $8000
        byte[] program = [0xA9, 0x02, 0x8D, 0x14, 0x40, 0x4C, 0x00, 0x80];

        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, program), "Dma.nes").Success);

        long cycles = 0;
        while (core.Bus.OamDmaCount == 0)
        {
            cycles = core.Cpu.Step();
        }

        // STA absolute is 4 cycles, plus a 513 or 514 cycle stall.
        Assert.Contains(cycles, new long[] { 517, 518 });
    }
}
