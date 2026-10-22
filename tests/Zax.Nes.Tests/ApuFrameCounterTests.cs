namespace Zax.Nes.Tests;

/// <summary>
/// The APU parts the CPU can observe: the frame counter IRQ, the length counters and the
/// $4015 status byte (REQ-CORE-004, REQ-CORE-081, REQ-CORE-111).
/// </summary>
/// <remarks>
/// Only what the CPU can see is covered here, because <c>cpu_interrupts_v2</c>
/// (REQ-CORE-111) depends on it. The channels themselves are in
/// <see cref="ApuChannelTests"/> and the output path in <see cref="ApuOutputTests"/>.
/// </remarks>
public sealed class ApuFrameCounterTests
{
    /// <summary>Period of the four-step frame sequence in NTSC CPU cycles (nesdev).</summary>
    private const int NtscFourStepPeriod = 29830;

    private static NesCore LoadedCore()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Apu.nes").Success);
        return core;
    }

    [Fact]
    public void REQ_CORE_111_the_four_step_sequence_raises_the_frame_irq()
    {
        NesCore core = LoadedCore();

        // Four-step mode, IRQ enabled.
        core.WriteMemory(0x4017, 0x00);
        Assert.False(core.Apu.IrqAsserted);

        for (int i = 0; i < NtscFourStepPeriod + 8; i++)
        {
            core.Bus.Tick();
        }

        Assert.True(core.Apu.IrqAsserted);

        // Reading $4015 clears the flag.
        Assert.Equal(0x40, core.Apu.ReadStatus() & 0x40);
        Assert.False(core.Apu.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_111_the_inhibit_bit_suppresses_the_frame_irq()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x4017, 0x40); // Four-step mode, IRQ inhibited.

        for (int i = 0; i < 2 * NtscFourStepPeriod; i++)
        {
            core.Bus.Tick();
        }

        Assert.False(core.Apu.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_111_the_five_step_sequence_never_raises_the_frame_irq()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x4017, 0x80); // Five-step mode.

        for (int i = 0; i < 2 * 37282; i++)
        {
            core.Bus.Tick();
        }

        Assert.False(core.Apu.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_111_setting_the_inhibit_bit_clears_a_pending_frame_irq()
    {
        NesCore core = LoadedCore();
        core.WriteMemory(0x4017, 0x00);

        for (int i = 0; i < NtscFourStepPeriod + 8; i++)
        {
            core.Bus.Tick();
        }

        Assert.True(core.Apu.IrqAsserted);

        core.WriteMemory(0x4017, 0x40);
        Assert.False(core.Apu.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_081_peeking_4015_does_not_clear_the_frame_irq()
    {
        NesCore core = LoadedCore();
        core.WriteMemory(0x4017, 0x00);

        for (int i = 0; i < NtscFourStepPeriod + 8; i++)
        {
            core.Bus.Tick();
        }

        Assert.Equal(0x40, core.Apu.PeekStatus() & 0x40);
        Assert.True(core.Apu.IrqAsserted);

        // Only a real CPU read clears it.
        core.Apu.ReadStatus();
        Assert.False(core.Apu.IrqAsserted);
    }

    [Fact]
    public void REQ_CORE_081_4015_reports_the_channels_whose_length_counter_is_running()
    {
        NesCore core = LoadedCore();

        // Enable pulse 1 and the triangle, then load their length counters.
        core.WriteMemory(0x4015, 0x05);
        core.WriteMemory(0x4003, 0xF8); // Pulse 1 length index 31 → 30.
        core.WriteMemory(0x400B, 0xF8); // Triangle.

        Assert.Equal(0x05, core.Apu.PeekStatus() & 0x0F);

        // A disabled channel's counter is cleared at once.
        core.WriteMemory(0x4015, 0x04);
        Assert.Equal(0x04, core.Apu.PeekStatus() & 0x0F);
    }

    [Fact]
    public void REQ_CORE_081_a_length_counter_is_not_loaded_while_its_channel_is_disabled()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x4015, 0x00);
        core.WriteMemory(0x4003, 0xF8);

        Assert.Equal(0x00, core.Apu.PeekStatus() & 0x0F);
    }

    [Fact]
    public void REQ_CORE_111_half_frames_count_the_length_counters_down()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x4015, 0x01); // Pulse 1 on.
        core.WriteMemory(0x4000, 0x00); // Length counter not halted.
        core.WriteMemory(0x4003, 0x08); // Length index 1 → 254.
        core.WriteMemory(0x4017, 0x00); // Four-step mode, sequencer restarted.

        Assert.Equal(0x01, core.Apu.PeekStatus() & 0x01);

        // 254 half frames, two per sequence period, plus a margin.
        for (int i = 0; i < 128 * NtscFourStepPeriod; i++)
        {
            core.Bus.Tick();
        }

        Assert.Equal(0x00, core.Apu.PeekStatus() & 0x01);
    }

    [Fact]
    public void REQ_CORE_111_a_halted_length_counter_never_reaches_zero()
    {
        NesCore core = LoadedCore();

        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, 0x20); // Halt flag set.
        core.WriteMemory(0x4003, 0x08);
        core.WriteMemory(0x4017, 0x00);

        for (int i = 0; i < 300 * NtscFourStepPeriod; i++)
        {
            core.Bus.Tick();
        }

        Assert.Equal(0x01, core.Apu.PeekStatus() & 0x01);
    }

    [Fact]
    public void REQ_CORE_004_reset_silences_the_apu_and_clears_its_irq()
    {
        NesCore core = LoadedCore();
        core.WriteMemory(0x4015, 0x0F);
        core.WriteMemory(0x4003, 0xF8);
        core.WriteMemory(0x4017, 0x00);

        for (int i = 0; i < NtscFourStepPeriod + 8; i++)
        {
            core.Bus.Tick();
        }

        Assert.True(core.Apu.IrqAsserted);

        core.Reset();

        Assert.False(core.Apu.IrqAsserted);
        Assert.Equal(0x00, core.Apu.PeekStatus());
        Assert.Equal(0, core.Apu.AvailableSamples);
    }

    [Fact]
    public void REQ_CORE_070_one_frame_produces_the_documented_number_of_samples()
    {
        // Cross-check of the per-frame figure the resampler is calibrated on.
        Assert.Equal(800.0, RegionTiming.For(NesRegion.Ntsc).SamplesPerFrame(48_000));
        Assert.Equal(960.0, RegionTiming.For(NesRegion.Pal).SamplesPerFrame(48_000));
        Assert.Equal(960.0, RegionTiming.For(NesRegion.Dendy).SamplesPerFrame(48_000));
        Assert.Equal(800.0, RegionTiming.For(NesRegion.Multi).SamplesPerFrame(48_000));
    }
}
