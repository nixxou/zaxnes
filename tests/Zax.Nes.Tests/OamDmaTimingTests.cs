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
