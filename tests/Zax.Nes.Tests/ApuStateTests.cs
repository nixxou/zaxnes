namespace Zax.Nes.Tests;

/// <summary>
/// The APU's part of a savestate: all five channels, the frame counter and the output path
/// (REQ-CORE-074, REQ-CORE-090, REQ-CORE-092).
/// </summary>
public sealed class ApuStateTests
{
    private static NesCore SoundingCore()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Apu.nes").Success);

        // All five channels busy, so that every piece of channel state is non-trivial.
        core.WriteMemory(0x4000, 0x7F); // Pulse 1: duty 1, halt, envelope period 15.
        core.WriteMemory(0x4001, 0x8B); // Sweep enabled, period 0, negate, shift 3.
        core.WriteMemory(0x4002, 0x9A);
        core.WriteMemory(0x4003, 0xC9);

        core.WriteMemory(0x4004, 0xB6); // Pulse 2: duty 2, halt, constant volume 6.
        core.WriteMemory(0x4005, 0x08);
        core.WriteMemory(0x4006, 0x47);
        core.WriteMemory(0x4007, 0xA1);

        core.WriteMemory(0x4008, 0x55); // Triangle: control clear, linear reload 85.
        core.WriteMemory(0x400A, 0x37);
        core.WriteMemory(0x400B, 0x92);

        core.WriteMemory(0x400C, 0x3A); // Noise: halt, envelope period 10.
        core.WriteMemory(0x400E, 0x86); // Short mode, period index 6.
        core.WriteMemory(0x400F, 0x70);

        core.WriteMemory(0x4010, 0x4B); // DMC: loop, rate 11.
        core.WriteMemory(0x4011, 0x39);
        core.WriteMemory(0x4012, 0x10);
        core.WriteMemory(0x4013, 0x05);

        core.WriteMemory(0x4015, 0x1F); // Everything enabled.
        core.WriteMemory(0x4017, 0x00); // Four-step, frame IRQ enabled.

        for (int i = 0; i < 40_000; i++)
        {
            core.Bus.Tick();
        }

        // The samples produced while setting up are read off, so that a capture starts at the
        // same point in the stream whether or not a state was restored in between: restoring
        // drops what the host has not read yet (REQ-CORE-074).
        var one = new float[1];
        while (core.ReadAudio(one) != 0)
        {
        }

        return core;
    }

    private static float[] Capture(NesCore core, int cycles)
    {
        var samples = new List<float>();
        var block = new float[64];

        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            if (core.Apu.AvailableSamples >= block.Length)
            {
                core.ReadAudio(block);
                samples.AddRange(block);
            }
        }

        return [.. samples];
    }

    [Fact]
    public void REQ_CORE_090_the_apu_state_survives_a_save_and_restore()
    {
        NesCore core = SoundingCore();

        byte status = core.Apu.PeekStatus();
        double level = core.Apu.MixedLevel;
        byte[] state = core.SaveState();

        float[] expected = Capture(core, 60_000);
        Assert.NotEmpty(expected);
        Assert.Contains(expected, sample => sample != 0.0f);

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);

        Assert.Equal(status, core.Apu.PeekStatus());
        Assert.Equal(level, core.Apu.MixedLevel);

        float[] replayed = Capture(core, 60_000);

        Assert.Equal(expected, replayed);
    }

    [Fact]
    public void REQ_CORE_092_a_restored_state_replays_the_same_audio_in_a_fresh_instance()
    {
        NesCore source = SoundingCore();
        byte[] state = source.SaveState();
        float[] expected = Capture(source, 60_000);

        // A block is self-contained (REQ-CORE-090): a brand new instance that has never seen
        // the ROM has to produce the same samples.
        var restored = new NesCore();
        Assert.True(restored.TryLoadState(state, 0, out _, out string romName, out string? error), error);
        Assert.Equal("Apu.nes", romName);

        Assert.Equal(expected, Capture(restored, 60_000));
    }

    [Fact]
    public void REQ_CORE_074_restoring_a_state_drops_the_samples_waiting_to_be_read()
    {
        NesCore core = SoundingCore();
        byte[] state = core.SaveState();

        for (int frame = 0; frame < 10; frame++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.Apu.AvailableSamples > 512);

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);

        Assert.Equal(0, core.Apu.AvailableSamples);
        Assert.Equal(0, core.ReadAudio(new float[512]));

        core.RunFrame(0);
        Assert.Equal(512, core.ReadAudio(new float[512]));
    }

    [Fact]
    public void REQ_CORE_074_reset_drops_the_samples_and_restarts_from_silence()
    {
        NesCore core = SoundingCore();
        for (int frame = 0; frame < 10; frame++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.Apu.AvailableSamples > 512);

        core.Reset();

        Assert.Equal(0, core.Apu.AvailableSamples);
        Assert.Equal(0, core.ReadAudio(new float[512]));

        core.RunFrame(0);
        var buffer = new float[512];
        Assert.Equal(512, core.ReadAudio(buffer));
        Assert.All(buffer, sample => Assert.Equal(0.0f, sample));
    }

    [Fact]
    public void REQ_CORE_005_closing_the_rom_stops_the_audio()
    {
        NesCore core = SoundingCore();
        core.RunFrame(0);

        core.CloseRom();

        Assert.Equal(0, core.Apu.AvailableSamples);

        for (int frame = 0; frame < 10; frame++)
        {
            Assert.Equal(0, core.RunFrame(0));
        }

        Assert.Equal(0, core.Apu.AvailableSamples);
    }

    [Fact]
    public void REQ_ARCH_013_the_announced_state_size_covers_the_apu()
    {
        NesCore core = SoundingCore();

        Assert.True(core.SaveState().Length <= core.MaxStateSize());
    }
}
