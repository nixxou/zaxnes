using Zax.Nes.Cartridges;

namespace Zax.Nes.Tests;

/// <summary>
/// The mapper IRQ counters of REQ-CORE-100 and REQ-CORE-114: what clocks them, when they
/// fire and how they are acknowledged.
/// </summary>
/// <remarks>
/// The counters are driven straight from the bus clock, with no CPU program involved: the
/// test sets the registers through the host memory write of REQ-CORE-082 and then advances
/// the machine one cycle at a time, which makes the measured delays exact. The MMC3 counter
/// needs rendering to be on, since what it counts is the PPU address bus.
/// </remarks>
public sealed class MapperIrqTests
{
    /// <summary>Advances the bus until the cartridge asserts its IRQ, or the budget runs out.</summary>
    /// <returns>Cycles consumed, or -1 when the IRQ never came.</returns>
    private static long TickUntilIrq(NesCore core, long budget)
    {
        Cartridge cartridge = core.Cartridge!;
        for (long cycle = 1; cycle <= budget; cycle++)
        {
            core.Bus.Tick();
            if (cartridge.IrqAsserted)
            {
                return cycle;
            }
        }

        return -1;
    }

    // ---------------------------------------------------------------- MMC3 and RAMBO-1

    /// <summary>
    /// Switches rendering on with the background patterns at $0000 and the sprite patterns at
    /// $1000, which is the arrangement that makes A12 rise once per line (nesdev, "MMC3").
    /// </summary>
    private static void EnableScanlineCounting(NesCore core)
    {
        core.WriteMemory(0x2000, 0x08);
        core.WriteMemory(0x2001, 0x18);
    }

    [Fact]
    public void REQ_CORE_114_the_mmc3_counter_fires_once_every_latch_plus_one_lines()
    {
        NesCore core = MapperFixture.Load(4, chrBanks1k: 32);
        EnableScanlineCounting(core);

        core.MapperWrite(0xC000, 20); // Latch.
        core.MapperWrite(0xC001, 0x00); // Reload on the next clock.
        core.MapperWrite(0xE001, 0x00); // Enable.

        Assert.True(TickUntilIrq(core, 200_000) > 0);

        core.MapperWrite(0xE000, 0x00); // Acknowledge.
        core.MapperWrite(0xE001, 0x00);
        Assert.False(core.Cartridge!.IrqAsserted);

        long period = TickUntilIrq(core, 200_000);

        // The counter reloads with 20 and then counts down to 0, so 21 clocks — one per
        // rendered line, the pre-render line included. A line is 341/3 CPU cycles.
        Assert.InRange(period, 20 * 341 / 3, 22 * 341 / 3);
    }

    [Fact]
    public void REQ_CORE_114_the_mmc3_counter_does_not_run_when_both_tables_share_a_half_of_chr()
    {
        // With the background and the sprites both fetching from $0xxx, A12 never rises
        // during rendering and the counter stands still (nesdev; mmc3_test "A12 clocking").
        NesCore core = MapperFixture.Load(4, chrBanks1k: 32);
        core.WriteMemory(0x2000, 0x00);
        core.WriteMemory(0x2001, 0x18);

        core.MapperWrite(0xC000, 1);
        core.MapperWrite(0xC001, 0x00);
        core.MapperWrite(0xE001, 0x00);

        Assert.Equal(-1, TickUntilIrq(core, 120_000));
    }

    [Fact]
    public void REQ_CORE_114_writing_2006_clocks_the_mmc3_counter_by_hand()
    {
        // Rendering off: the counter can still be clocked by moving the VRAM address across
        // $1000, which is how mmc3_test drives it.
        NesCore core = MapperFixture.Load(4, chrBanks1k: 32);
        core.WriteMemory(0x2001, 0x00);

        core.MapperWrite(0xC000, 1);
        core.MapperWrite(0xC001, 0x00);
        core.MapperWrite(0xE001, 0x00);

        for (int clock = 0; clock < 2; clock++)
        {
            Assert.False(core.Cartridge!.IrqAsserted);

            core.WriteMemory(0x2006, 0x00);
            core.WriteMemory(0x2006, 0x00);
            for (int cycle = 0; cycle < 8; cycle++)
            {
                core.Bus.Tick();
            }

            core.WriteMemory(0x2006, 0x10);
            core.WriteMemory(0x2006, 0x00);
            for (int cycle = 0; cycle < 8; cycle++)
            {
                core.Bus.Tick();
            }
        }

        Assert.True(core.Cartridge!.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_114_clearing_the_mmc3_counter_through_c001_does_not_fire()
    {
        // "The IRQ flag is not set when the counter is cleared by writing to $C001"
        // (mmc3_test readme); the reload happens on the next clock instead.
        NesCore core = MapperFixture.Load(4, chrBanks1k: 32);
        core.WriteMemory(0x2001, 0x00);

        core.MapperWrite(0xC000, 5);
        core.MapperWrite(0xE001, 0x00);
        core.MapperWrite(0xC001, 0x00);

        Assert.False(core.Cartridge!.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_100_the_mmc3_counter_is_silent_while_irqs_are_disabled()
    {
        NesCore core = MapperFixture.Load(4, chrBanks1k: 32);
        EnableScanlineCounting(core);

        core.MapperWrite(0xC000, 2);
        core.MapperWrite(0xC001, 0x00);
        core.MapperWrite(0xE000, 0x00); // Disabled.

        Assert.Equal(-1, TickUntilIrq(core, 60_000));
    }

    [Fact]
    public void REQ_CORE_100_the_rambo1_counter_can_run_on_the_cpu_clock()
    {
        NesCore core = MapperFixture.Load(64, chrBanks1k: 32);
        core.WriteMemory(0x2001, 0x00); // No rendering: only the CPU clock is left.

        core.MapperWrite(0xC000, 10);
        core.MapperWrite(0xC001, 0x01); // Cycle mode, reload pending.
        core.MapperWrite(0xE001, 0x00);

        long cycles = TickUntilIrq(core, 1000);

        // One clock every four CPU cycles and 11 clocks to go from the reload to zero, less
        // the few cycles the register writes themselves consumed.
        Assert.InRange(cycles, 4 * 10, (4 * 11) + 8);
    }

    // ---------------------------------------------------------------- VRC4

    [Theory]
    [InlineData(21)]
    [InlineData(23)]
    [InlineData(25)]
    [InlineData(27)]
    public void REQ_CORE_100_the_vrc4_counter_counts_up_on_the_cpu_clock(int mapper)
    {
        NesCore core = MapperFixture.Load(mapper, chrBanks1k: 32);

        WriteVrc(core, mapper, 0xF000, 0, 0x00); // Latch low nibble.
        WriteVrc(core, mapper, 0xF000, 1, 0x0F); // Latch high nibble: $F0.
        WriteVrc(core, mapper, 0xF000, 2, 0x06); // Enable, cycle mode.

        long cycles = TickUntilIrq(core, 1000);

        // $F0 counts up to $FF and fires on the sixteenth clock.
        Assert.Equal(16, cycles);

        // Acknowledging clears the line and reloads from the latch.
        WriteVrc(core, mapper, 0xF000, 3, 0x00);
        Assert.False(core.Cartridge!.IrqAsserted);
        Assert.Equal(-1, TickUntilIrq(core, 8));
    }

    [Fact]
    public void REQ_CORE_100_the_vrc4_counter_can_run_on_a_scanline_prescaler()
    {
        NesCore core = MapperFixture.Load(23, chrBanks1k: 32);

        core.WriteMemory(0xF000, 0x0F);
        core.WriteMemory(0xF001, 0x0F); // Latch $FF: one clock is enough.
        core.WriteMemory(0xF002, 0x02); // Enable, scanline mode.

        long cycles = TickUntilIrq(core, 1000);

        // The prescaler divides the CPU clock by 341/3, which is one scanline.
        Assert.InRange(cycles, 113, 115);
    }

    [Fact]
    public void REQ_CORE_100_vrc2_has_no_irq_counter()
    {
        NesCore core = MapperFixture.Load(22, chrBanks1k: 32);

        core.WriteMemory(0xF000, 0x0F);
        core.WriteMemory(0xF001, 0x0F);
        core.WriteMemory(0xF002, 0x06);

        Assert.Equal(-1, TickUntilIrq(core, 2000));
    }

    // ---------------------------------------------------------------- FCG and SS88006

    [Fact]
    public void REQ_CORE_100_the_bandai_fcg_counter_counts_down_on_the_cpu_clock()
    {
        NesCore core = MapperFixture.Load(16, chrBanks1k: 32);

        core.WriteMemory(0x800B, 0x64); // Latch low byte: 100.
        core.WriteMemory(0x800C, 0x00);
        core.WriteMemory(0x800A, 0x01); // Enable, counter loaded from the latch.

        long cycles = TickUntilIrq(core, 1000);

        // 100 down to 0 is 100 cycles, and the underflow on the next one fires.
        Assert.Equal(101, cycles);

        core.WriteMemory(0x800A, 0x00); // Acknowledge and disable.
        Assert.False(core.Cartridge!.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_100_the_ss88006_counter_can_be_narrowed_to_four_bits()
    {
        NesCore core = MapperFixture.Load(18, chrBanks1k: 32);

        for (int nibble = 0; nibble < 4; nibble++)
        {
            core.WriteMemory((ushort)(0xE000 + nibble), nibble == 0 ? (byte)0x05 : (byte)0x00);
        }

        core.WriteMemory(0xF000, 0x00); // Copy the latch into the counter.
        core.WriteMemory(0xF001, 0x09); // Enable, four-bit counter.

        Assert.Equal(6, TickUntilIrq(core, 200));
    }

    // ---------------------------------------------------------------- MMC5

    [Fact]
    public void REQ_CORE_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names()
    {
        NesCore core = MapperFixture.Load(5, chrBanks1k: 32);

        core.WriteMemory(0x2001, 0x18); // Rendering on: the board can see the lines.
        core.WriteMemory(0x5203, 60);
        core.WriteMemory(0x5204, 0x80); // Enable.

        Assert.True(TickUntilIrq(core, 200_000) > 0);
        Assert.Equal(59, core.Ppu.Scanline);

        // Bit 7 of $5204 reports the pending IRQ and reading it acknowledges; bit 6 says the
        // scan is inside a frame.
        Assert.Equal(0xC0, core.ReadMemory(0x5204) & 0xC0); // A peek leaves it pending.
        Assert.True(core.Cartridge!.IrqAsserted);

        Assert.True(core.Cartridge.TryCpuRead(0x5204, out byte status));
        Assert.Equal(0xC0, status & 0xC0);
        Assert.False(core.Cartridge.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_100_the_mmc5_scanline_irq_stays_quiet_without_rendering()
    {
        NesCore core = MapperFixture.Load(5, chrBanks1k: 32);

        core.WriteMemory(0x2001, 0x00);
        core.WriteMemory(0x5203, 60);
        core.WriteMemory(0x5204, 0x80);

        Assert.Equal(-1, TickUntilIrq(core, 120_000));
    }

    // ---------------------------------------------------------------- helpers

    /// <summary>
    /// Writes VRC register <paramref name="index"/> of the $x000 group at
    /// <paramref name="group"/>, through the two address lines <paramref name="mapper"/>
    /// wires to the chip.
    /// </summary>
    private static void WriteVrc(NesCore core, int mapper, int group, int index, byte value)
    {
        int offset = mapper switch
        {
            21 => ((index & 1) << 1) | ((index & 2) << 1),
            22 or 25 => ((index & 1) << 1) | ((index & 2) >> 1),
            _ => index,
        };

        core.WriteMemory((ushort)(group | offset), value);
    }
}
