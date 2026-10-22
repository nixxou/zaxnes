namespace Zax.Nes.Tests;

/// <summary>
/// Life cycle and the operation set of REQ-CORE-002 (REQ-CORE-001 to REQ-CORE-006,
/// REQ-CORE-031).
/// </summary>
public sealed class CoreLifecycleTests
{
    [Fact]
    public void REQ_CORE_031_video_info_is_33_by_31_with_128_sprites()
    {
        VideoInfo info = NesCore.VideoInfo;

        Assert.Equal(33, info.Width);
        Assert.Equal(31, info.Height);
        Assert.Equal(128, info.SpriteCount);
        Assert.Equal(1023, VideoInfo.MaxBackgroundRecords);
        Assert.Equal(128, VideoInfo.MaxSpriteRecords);
        Assert.Equal(1151, VideoInfo.MaxRecords);
    }

    [Fact]
    public void REQ_CORE_002_core_name_is_a_stable_non_empty_string()
    {
        Assert.False(string.IsNullOrWhiteSpace(NesCore.CoreName));
        Assert.Equal("Zaxnes", NesCore.CoreName);
    }

    [Fact]
    public void REQ_CORE_003_power_on_state_has_zeroed_ram_and_frame_counter()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        for (int address = 0x0000; address <= 0x07FF; address++)
        {
            Assert.Equal(0, core.ReadMemory((ushort)address));
        }

        Assert.Equal(0, core.FrameCount);
        Assert.True(core.IsRunning);
    }

    [Fact]
    public void REQ_CORE_004_reset_is_a_power_cycle_and_clears_ram_and_prg_ram()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.WriteMemory(0x0300, 0x55);
        core.WriteMemory(0x6000, 0x55);
        Assert.Equal(0x55, core.ReadMemory(0x0300));
        Assert.Equal(0x55, core.ReadMemory(0x6000));

        core.RunFrame(0);
        Assert.Equal(1, core.FrameCount);

        core.Reset();

        // The reference clears PRG-RAM too (REQ-CORE-004, notes; DEC-02-Q3).
        Assert.Equal(0x00, core.ReadMemory(0x0300));
        Assert.Equal(0x00, core.ReadMemory(0x6000));
        Assert.Equal(0, core.FrameCount);
        Assert.Equal("Minimal.nes", core.RomName);
    }

    [Fact]
    public void REQ_CORE_004_reset_keeps_the_port_types()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        core.SetPortType(1, ControllerPortType.Zapper);
        core.Reset();

        Assert.Equal(ControllerPortType.Zapper, core.GetPortType(1));
    }

    [Fact]
    public void REQ_CORE_005_running_a_frame_with_no_rom_does_nothing()
    {
        var core = new NesCore();
        int callbacks = 0;
        core.VideoCallback = (in FrameBatch _) => callbacks++;

        for (int i = 0; i < 10; i++)
        {
            Assert.Equal(0, core.RunFrame(0));
        }

        Assert.Equal(0, callbacks);
        Assert.Equal(0, core.FrameCount);
        Assert.Equal(0, core.ReadAudio(new float[512]));
    }

    [Fact]
    public void REQ_CORE_005_closing_the_rom_stops_the_emulation()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        int callbacks = 0;
        core.VideoCallback = (in FrameBatch _) => callbacks++;
        core.RunFrame(0);
        Assert.Equal(1, callbacks);

        core.CloseRom();
        Assert.False(core.IsRunning);

        long frames = core.FrameCount;
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(1, callbacks);
        Assert.Equal(frames, core.FrameCount);
    }

    [Fact]
    public void REQ_CORE_006_frame_counter_counts_frames_and_is_host_writable()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        for (int i = 0; i < 100; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(100, core.FrameCount);

        core.FrameCount = 4242;
        Assert.Equal(4242, core.FrameCount);
    }

    [Fact]
    public void REQ_CORE_006_frame_counter_survives_a_savestate_round_trip()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Minimal.nes").Success);

        for (int i = 0; i < 100; i++)
        {
            core.RunFrame(0);
        }

        byte[] state = core.SaveState();

        for (int i = 0; i < 50; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(150, core.FrameCount);

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);
        Assert.Equal(100, core.FrameCount);
    }

    [Fact]
    public void REQ_CORE_001_two_instances_are_independent()
    {
        // Two cores, two different ROMs, run alternately: each must produce exactly what
        // it produces alone (REQ-ARCH-011).
        byte[] romA = TestRoms.Minimal(0x01);
        byte[] romB = TestRoms.Minimal(0x00, [0xA9, 0x37, 0x8D, 0x10, 0x03, 0x4C, 0x00, 0x80]);

        long[] aloneA = RunAlone(romA, 600);
        long[] aloneB = RunAlone(romB, 600);

        var coreA = new NesCore();
        var coreB = new NesCore();
        Assert.True(coreA.LoadRom(romA, "A.nes").Success);
        Assert.True(coreB.LoadRom(romB, "B.nes").Success);

        var interleavedA = new long[600];
        var interleavedB = new long[600];
        for (int frame = 0; frame < 600; frame++)
        {
            interleavedA[frame] = coreA.RunFrame((ushort)frame);
            interleavedB[frame] = coreB.RunFrame((ushort)(frame ^ 0xFFFF));
        }

        Assert.Equal(aloneA, interleavedA);
        Assert.Equal(aloneB, interleavedB);
        Assert.Equal(0x37, coreB.ReadMemory(0x0310));
        Assert.Equal(0x00, coreA.ReadMemory(0x0310));
    }

    [Fact]
    public void REQ_ARCH_002_the_assembly_has_no_platform_dependency()
    {
        string[] forbidden =
        [
            "Silk.NET", "ImGui", "OpenTK", "PresentationCore", "System.Windows", "System.Drawing",
        ];

        string[] offenders = typeof(NesCore).Assembly
            .GetReferencedAssemblies()
            .Select(static name => name.Name ?? string.Empty)
            .Where(name => forbidden.Any(prefix => name.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
            .ToArray();

        Assert.Empty(offenders);
    }

    private static long[] RunAlone(byte[] rom, int frames)
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(rom, "Alone.nes").Success);

        var cycles = new long[frames];
        for (int frame = 0; frame < frames; frame++)
        {
            cycles[frame] = core.RunFrame((ushort)frame);
        }

        return cycles;
    }
}
