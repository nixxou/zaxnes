using Zax.Nes;
using Zax.Nes.Rom;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-02 §3.1 to §3.4: lifecycle, ROM loading, regions,
/// frame boundaries and input word.
/// </summary>
public sealed class CoreInterfaceTests
{
    [Fact] // REQ-CORE-001
    public void Two_instances_are_independent()
    {
        byte[] romA = SpecRom.Build(SpecRom.Mirroring.Vertical);
        byte[] romB = SpecRom.Build(SpecRom.Mirroring.Horizontal);

        static List<int> Solo(byte[] image)
        {
            NesCore core = new();
            core.LoadRom(image, "solo.nes");
            List<int> counts = [];
            core.VideoCallback = (in FrameBatch b) => counts.Add(b.Records.Length);
            for (int i = 0; i < 600; i++)
            {
                core.RunFrame((ushort)(i & 0xFF));
            }

            return counts;
        }

        List<int> soloA = Solo(romA);
        List<int> soloB = Solo(romB);

        NesCore a = new();
        NesCore b = new();
        a.LoadRom(romA, "a.nes");
        b.LoadRom(romB, "b.nes");
        List<int> alternatedA = [];
        List<int> alternatedB = [];
        a.VideoCallback = (in FrameBatch x) => alternatedA.Add(x.Records.Length);
        b.VideoCallback = (in FrameBatch x) => alternatedB.Add(x.Records.Length);
        for (int i = 0; i < 600; i++)
        {
            a.RunFrame((ushort)(i & 0xFF));
            b.RunFrame((ushort)(i & 0xFF));
        }

        Assert.Equal(soloA, alternatedA);
        Assert.Equal(soloB, alternatedB);
    }

    [Fact] // REQ-CORE-002
    public void Every_operation_of_the_table_is_exercised()
    {
        NesCore core = new();                                    // initialise
        core.VideoCallback = (in FrameBatch _) => { };           // register the video callback
        core.ZapperPositionSource = _ => new ZapperPosition(1, 2); // Zapper position source
        Assert.Equal(new VideoInfo(33, 31, 128), NesCore.VideoInfo); // video info
        Assert.True(core.LoadRom(SpecRom.Build(), "Ops.nes").Success); // load a ROM
        core.SetPortType(1, ControllerPortType.Zapper);          // port device type
        Assert.Equal(ControllerPortType.Zapper, core.GetPortType(1));
        core.Region = NesRegion.Pal;                             // choose the region
        core.Speed = 1.5;                                        // speed factor
        core.RunFrame(0x0001);                                   // run a frame with an input word
        Assert.True(core.ReadAudio(new float[256]) is 0 or 256); // read audio
        core.WriteMemory(0x0012, 0x42);                          // write a memory byte
        Assert.Equal(0x42, core.ReadMemory(0x0012));             // read a memory byte
        byte[] state = core.SaveState();                         // save the state
        core.FrameCount = 7;                                     // frame counter (write)
        Assert.Equal(7, core.FrameCount);                        // frame counter (read)
        Assert.True(core.TryLoadState(state, 0, out _, out _, out _)); // restore the state
        Assert.False(string.IsNullOrEmpty(NesCore.CoreName));    // core name
        core.Reset();                                            // reset
        core.CloseRom();                                         // close the ROM
        Assert.False(core.IsRunning);                            // release
    }

    [Fact] // REQ-CORE-003
    public void Loading_puts_the_machine_in_its_power_on_state()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(SpecRom.Build(), "Minimal.nes").Success);

        for (int address = 0x0000; address <= 0x07FF; address++)
        {
            Assert.Equal(0, core.ReadMemory((ushort)address));
        }

        Assert.Equal(0, core.FrameCount);
    }

    [Fact] // REQ-CORE-004
    public void Reset_is_a_power_cycle()
    {
        NesCore core = SpecRom.Warm();
        core.WriteMemory(0x0300, 0x55);
        core.WriteMemory(0x6000, 0x55);
        core.Reset();

        Assert.Equal(0, core.ReadMemory(0x0300));
        Assert.Equal(0, core.ReadMemory(0x6000));
        Assert.Equal(0, core.FrameCount);
    }

    [Fact] // REQ-CORE-005
    public void A_closed_core_does_nothing()
    {
        NesCore core = SpecRom.Warm();
        long before = core.FrameCount;
        core.CloseRom();

        int calls = 0;
        core.VideoCallback = (in FrameBatch _) => calls++;
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(0, calls);
        Assert.Equal(before, core.FrameCount);
        Assert.Equal(0, core.ReadAudio(new float[1]));
    }

    [Fact] // REQ-CORE-006
    public void Frame_counter_counts_frames_and_rides_in_the_savestate()
    {
        NesCore core = new();
        core.LoadRom(SpecRom.Build(), "Minimal.nes");
        for (int i = 0; i < 100; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(100, core.FrameCount);

        byte[] state = core.SaveState();
        for (int i = 0; i < 50; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(150, core.FrameCount);
        Assert.True(core.TryLoadState(state, 0, out _, out _, out _));
        Assert.Equal(100, core.FrameCount);
    }

    [Fact] // REQ-CORE-010
    public void An_ines_and_an_equivalent_nes20_header_both_load()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(SpecRom.Build(), "Minimal.nes").Success);

        byte[] nes20 = SpecRom.Build();
        nes20[7] = 0x08;   // NES 2.0 identifier in bits 2-3 of byte 7
        nes20[9] = 0x00;   // high nibbles of the PRG/CHR sizes
        Assert.True(new NesCore().LoadRom(nes20, "Minimal20.nes").Success);
    }

    [Fact] // REQ-CORE-011
    public void Loading_fails_on_a_bad_signature_a_truncated_file_and_an_unknown_mapper()
    {
        NesCore core = new();

        RomLoadResult noSignature = core.LoadRom(new byte[16], "zero.nes");
        Assert.False(noSignature.Success);
        Assert.False(core.IsRunning);

        byte[] full = SpecRom.Build();
        RomLoadResult truncated = core.LoadRom(full.AsSpan(0, full.Length - 1), "short.nes");
        Assert.False(truncated.Success);

        byte[] mapper200 = SpecRom.Build();
        mapper200[6] = (byte)((mapper200[6] & 0x0F) | 0x80);  // low nibble of 200 = 8
        mapper200[7] = 0xC0;                                   // high nibble = 12
        RomLoadResult unknown = core.LoadRom(mapper200, "m200.nes");
        Assert.False(unknown.Success);
        Assert.Contains("200", unknown.Error);
    }

    [Theory] // REQ-CORE-012, SPEC-02 §5.2
    [InlineData(new byte[] { 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 },
        0, (int)NametableMirroring.Vertical, 8 * 1024, 0, (int)NesRegion.Ntsc, false)]
    [InlineData(new byte[] { 0x08, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 },
        1, (int)NametableMirroring.Horizontal, 0, 32 * 1024, (int)NesRegion.Ntsc, true)]
    [InlineData(new byte[] { 0x02, 0x01, 0x41, 0x44, 0x69, 0x73, 0x6B, 0x44, 0x75, 0x64, 0x65, 0x21 },
        4, (int)NametableMirroring.Vertical, 8 * 1024, 0, (int)NesRegion.Ntsc, false)]
    [InlineData(new byte[] { 0x02, 0x00, 0x23, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 },
        2, (int)NametableMirroring.Vertical, 0, 32 * 1024, (int)NesRegion.Ntsc, true)]
    [InlineData(new byte[] { 0x02, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 },
        0, (int)NametableMirroring.FourScreen, 8 * 1024, 0, (int)NesRegion.Ntsc, false)]
    [InlineData(new byte[] { 0x02, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00 },
        0, (int)NametableMirroring.Horizontal, 8 * 1024, 0, (int)NesRegion.Dendy, false)]
    [InlineData(new byte[] { 0x02, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 },
        0, (int)NametableMirroring.Horizontal, 8 * 1024, 0, (int)NesRegion.Pal, false)]
    public void Header_vectors(byte[] tail, int mapper, int mirroring, int chrRom, int chrRam, int region, bool battery)
    {
        byte[] image = new byte[16];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        tail.CopyTo(image, 4);

        Assert.True(InesHeader.TryParse(image, out InesHeader header, out string? error), error);
        Assert.Equal(mapper, header.Mapper);
        Assert.Equal((NametableMirroring)mirroring, header.Mirroring);
        Assert.Equal(chrRom, header.ChrRomSize);
        Assert.Equal(chrRam, header.ChrRamSize);
        Assert.Equal((NesRegion)region, header.Region);
        Assert.Equal(battery, header.HasBattery);
    }

    [Fact] // REQ-CORE-013
    public void The_core_never_applies_the_detected_region_by_itself()
    {
        byte[] pal = SpecRom.Build();
        pal[9] = 0x01;

        NesCore core = new();
        RomLoadResult result = core.LoadRom(pal, "pal.nes");
        Assert.True(result.Success);
        Assert.Equal(NesRegion.Pal, result.Region);
        Assert.Equal(NesRegion.Ntsc, core.Region);
    }

    [Fact] // REQ-CORE-014
    public void The_rom_name_rides_in_the_savestate()
    {
        NesCore core = new();
        core.LoadRom(SpecRom.Build(), "Test.nes");
        byte[] state = core.SaveState();

        core.LoadRom(SpecRom.Build(SpecRom.Mirroring.Horizontal), "Other.nes");
        Assert.True(core.TryLoadState(state, 0, out _, out string name, out _));
        Assert.Equal("Test.nes", name);
        Assert.Equal("Test.nes", core.RomName);
    }

    [Theory] // REQ-CORE-015
    [InlineData(NesRegion.Ntsc, 29780.5, 0.5)]
    [InlineData(NesRegion.Pal, 33247.5, 0.5)]
    [InlineData(NesRegion.Dendy, 35464.0, 1.0)]
    public void Average_cpu_cycles_per_frame(NesRegion region, double expected, double tolerance)
    {
        NesCore core = new();
        core.LoadRom(SpecRom.Build(), "Timing.nes");
        core.Region = region;

        // Discard the first frames, which carry the transition of a region change.
        for (int i = 0; i < 4; i++)
        {
            core.RunFrame(0);
        }

        long start = core.CycleCount;
        for (int i = 0; i < 1000; i++)
        {
            core.RunFrame(0);
        }

        double average = (core.CycleCount - start) / 1000.0;
        Assert.InRange(average, expected - tolerance, expected + tolerance);
    }

    [Fact] // REQ-CORE-016
    public void Region_can_change_between_frames_without_resetting()
    {
        NesCore core = SpecRom.Warm();
        core.WriteMemory(0x0400, 0x5A);

        core.Region = NesRegion.Pal;
        for (int i = 0; i < 4; i++)
        {
            core.RunFrame(0);
        }

        long start = core.CycleCount;
        for (int i = 0; i < 500; i++)
        {
            core.RunFrame(0);
        }

        Assert.InRange((core.CycleCount - start) / 500.0, 33247.0, 33248.0);
        Assert.Equal(0x5A, core.ReadMemory(0x0400));
    }

    [Fact] // REQ-CORE-020
    public void A_frame_stops_at_the_start_of_vblank()
    {
        // NMI handler at $9000: INC $0010 then RTI; the main program loops at $8000.
        byte[] image = SpecRom.Build(nmi: [0xE6, 0x10, 0x40]);

        NesCore core = new();
        core.LoadRom(image, "Nmi.nes");
        for (int i = 0; i < 3; i++)
        {
            core.RunFrame(0);
        }

        core.WriteMemory(0x0010, 0);
        core.WriteMemory(0x2000, 0x80); // NMI enabled

        int calls = 0;
        core.VideoCallback = (in FrameBatch _) => calls++;
        const int frames = 40;
        for (int i = 0; i < frames; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(frames, calls);
        int counter = core.ReadMemory(0x0010);
        Assert.InRange(counter, frames - 1, frames);
    }

    [Fact] // REQ-CORE-021
    public void The_callback_fires_even_for_an_empty_batch()
    {
        NesCore core = SpecRom.Warm();
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x00);
        Assert.Empty(batch.Records);
    }

    /// <summary>The ten reads the on-ROM reader stored for a port, as bit 0 only.</summary>
    internal static int[] SerialBits(NesCore core, int port)
    {
        int at = port == 0 ? 0x0200 : 0x0210;
        return [.. Enumerable.Range(0, 10).Select(i => core.ReadMemory((ushort)(at + i)) & 1)];
    }

    [Fact] // REQ-CORE-022
    public void The_input_word_feeds_the_two_serial_ports()
    {
        NesCore core = SpecRom.Warm(program: SpecRom.PortReader);

        core.RunFrame(0x0081);
        Assert.Equal([1, 0, 0, 0, 0, 0, 0, 1], SerialBits(core, 0)[..8]);

        core.RunFrame(0x0800);
        Assert.Equal([0, 0, 0, 1, 0, 0, 0, 0], SerialBits(core, 1)[..8]);
    }

    [Fact] // REQ-CORE-022, note: the ninth read and beyond return 1
    public void Reads_past_the_eighth_return_one()
    {
        NesCore core = SpecRom.Warm(program: SpecRom.PortReader);
        core.RunFrame(0x0000);

        int[] port1 = SerialBits(core, 0);
        Assert.Equal(1, port1[8]);
        Assert.Equal(1, port1[9]);
    }

    [Fact] // REQ-CORE-024
    public void Port_type_switches_between_controller_and_zapper()
    {
        NesCore core = SpecRom.Warm(program: SpecRom.PortReader);
        Assert.Equal(ControllerPortType.Controller, core.GetPortType(0));
        Assert.Equal(ControllerPortType.Controller, core.GetPortType(1));

        core.ZapperPositionSource = _ => new ZapperPosition(-1, -1);
        core.SetPortType(1, ControllerPortType.Zapper);
        core.RunFrame(0x1000); // port 2 byte = 0x10, trigger released

        // All ten reads return the same Zapper byte: no serial register advances.
        for (int i = 0; i < 10; i++)
        {
            Assert.Equal(0x18, core.ReadMemory((ushort)(0x0210 + i)));
        }

        core.SetPortType(1, ControllerPortType.Controller);
        core.RunFrame(0x0800);
        Assert.Equal([0, 0, 0, 1, 0, 0, 0, 0], SerialBits(core, 1)[..8]);
    }
}
