namespace Zax.Nes.Tests;

/// <summary>
/// Frame boundaries, region timings and the video callback (REQ-CORE-015, REQ-CORE-016,
/// REQ-CORE-020, REQ-CORE-021).
/// </summary>
public sealed class FrameTimingTests
{
    [Theory]
    [InlineData(NesRegion.Ntsc, 29780.5, 0.5)]
    [InlineData(NesRegion.Pal, 33247.5, 0.5)]
    [InlineData(NesRegion.Dendy, 35464.0, 1.0)]
    [InlineData(NesRegion.Multi, 29780.5, 0.5)]
    public void REQ_CORE_015_average_cpu_cycles_per_frame(NesRegion region, double expected, double tolerance)
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);
        core.Region = region;

        // A region change takes effect on the next frame and does not move the PPU, so the
        // first frame after it can be short: it runs from wherever the scan was to the
        // vblank line of the new region (REQ-CORE-016). The steady-state length is what
        // REQ-CORE-015 is about.
        core.RunFrame(0);

        const int frames = 1000;
        long total = 0;
        for (int frame = 0; frame < frames; frame++)
        {
            total += core.RunFrame(0);
        }

        double average = (double)total / frames;
        Assert.InRange(average, expected - tolerance, expected + tolerance);
    }

    [Theory]
    [InlineData(NesRegion.Ntsc, 1_789_773, 262, 241, 60, true)]
    [InlineData(NesRegion.Pal, 1_662_607, 312, 241, 50, false)]
    [InlineData(NesRegion.Dendy, 1_773_448, 312, 291, 50, false)]
    [InlineData(NesRegion.Multi, 1_789_773, 262, 241, 60, true)]
    public void REQ_CORE_015_timing_table(
        NesRegion region, int clock, int scanlines, int vblankLine, int fps, bool skipsDot)
    {
        RegionTiming timing = RegionTiming.For(region);

        Assert.Equal(clock, timing.CpuClockHz);
        Assert.Equal(scanlines, timing.ScanlinesPerFrame);
        Assert.Equal(vblankLine, timing.VBlankScanline);
        Assert.Equal(fps, timing.NominalFramesPerSecond);
        Assert.Equal(skipsDot, timing.SkipsDotOnOddFrames);
        Assert.Equal(341, RegionTiming.DotsPerScanline);
        Assert.Equal(240, RegionTiming.VisibleScanlines);
    }

    [Fact]
    public void REQ_CORE_015_noise_and_dmc_period_tables_are_pal_only_for_pal()
    {
        Assert.True(RegionTiming.For(NesRegion.Pal).UsesPalNoiseAndDmcPeriods);
        Assert.False(RegionTiming.For(NesRegion.Ntsc).UsesPalNoiseAndDmcPeriods);
        Assert.False(RegionTiming.For(NesRegion.Multi).UsesPalNoiseAndDmcPeriods);
        Assert.False(RegionTiming.For(NesRegion.Dendy).UsesPalNoiseAndDmcPeriods);
    }

    [Fact]
    public void REQ_CORE_016_region_can_change_between_frames_without_resetting()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.WriteMemory(0x0200, 0x5A);
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        core.Region = NesRegion.Pal;

        long total = 0;
        const int frames = 200;
        for (int frame = 0; frame < frames; frame++)
        {
            total += core.RunFrame(0);
        }

        Assert.InRange((double)total / frames, 33247.0, 33248.0);

        // RAM is untouched and the frame counter kept counting.
        Assert.Equal(0x5A, core.ReadMemory(0x0200));
        Assert.Equal(210, core.FrameCount);
    }

    [Fact]
    public void REQ_CORE_020_the_video_callback_runs_exactly_once_per_frame()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        int callbacks = 0;
        core.VideoCallback = (in FrameBatch _) => callbacks++;

        for (int frame = 0; frame < 60; frame++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(60, callbacks);
    }

    [Fact]
    public void REQ_CORE_021_the_callback_runs_even_when_the_batch_is_empty()
    {
        // With PPUMASK = $00 nothing is rendered, so the batch holds no record, and the
        // callback must still be invoked once per frame.
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        int callbacks = 0;
        int records = -1;
        int paletteLength = -1;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            callbacks++;
            records = batch.Records.Length;
            paletteLength = batch.Palette.Length;
        };

        core.WriteMemory(0x2001, 0x00);
        core.RunFrame(0);

        Assert.Equal(1, callbacks);
        Assert.Equal(0, records);
        Assert.Equal(32, paletteLength); // REQ-CORE-030: always 32 palette bytes.
    }

    [Fact]
    public void REQ_CORE_020_a_frame_ends_at_the_start_of_vblank()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        for (int frame = 0; frame < 10; frame++)
        {
            core.RunFrame(0);

            // The frame stops on the dot the VBlank flag rises, so the PPU is parked at
            // the very beginning of vblank.
            Assert.Equal(core.Timing.VBlankScanline, core.Ppu.Scanline);
            Assert.True(core.Ppu.VBlankFlag);
            Assert.InRange(core.Ppu.Dot, 1, 16);
        }
    }

    [Fact]
    public void REQ_CORE_020_the_nmi_handler_has_barely_started_when_a_frame_returns()
    {
        // Main loop keeps NMI enabled; the handler at $8010 increments $0010.
        var program = new byte[0x13];

        // $8000: LDA #$80 / STA $2000 / JMP $8000
        byte[] main = [0xA9, 0x80, 0x8D, 0x00, 0x20, 0x4C, 0x00, 0x80];
        main.CopyTo(program, 0);

        // $8010: INC $0010 / RTI
        byte[] handler = [0xE6, 0x10, 0x40];
        handler.CopyTo(program, 0x10);

        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, program, nmiVector: 0x8010), "Nmi.nes").Success);

        const int frames = 50;
        for (int frame = 1; frame <= frames; frame++)
        {
            core.RunFrame(0);

            byte counter = core.ReadMemory(0x0010);
            Assert.InRange(counter, frame - 1, frame);
        }
    }

    [Fact]
    public void REQ_CORE_010_region_timing_rejects_an_unknown_region()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => RegionTiming.For((NesRegion)42));
    }
}
