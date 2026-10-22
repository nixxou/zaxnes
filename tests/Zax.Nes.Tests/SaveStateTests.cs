namespace Zax.Nes.Tests;

/// <summary>
/// Savestates: self-containment, partial restore, failure modes and determinism
/// (REQ-CORE-090 to REQ-CORE-093, REQ-ARCH-012, REQ-ARCH-013).
/// </summary>
public sealed class SaveStateTests
{
    /// <summary>A ROM whose loop stirs work RAM, so that state differences are visible.</summary>
    private static byte[] CounterProgram() =>
    [
        0xE6, 0x20, // INC $20
        0xA5, 0x20, // LDA $20
        0x8D, 0x00, 0x03, // STA $0300
        0x4C, 0x00, 0x80, // JMP $8000
    ];

    private static NesCore LoadedCore(string name = "Minimal.nes")
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, CounterProgram()), name).Success);
        return core;
    }

    [Fact]
    public void REQ_CORE_090_a_state_block_is_self_contained()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 30; i++)
        {
            core.RunFrame(0);
        }

        byte[] state = core.SaveState();
        byte[] expectedRam = Snapshot(core);

        // Carry on in a reference run.
        var reference = new NesCore();
        Assert.True(reference.TryLoadState(state, 0, out _, out _, out string? err1), err1);
        for (int i = 0; i < 20; i++)
        {
            reference.RunFrame(0);
        }

        byte[] referenceRam = Snapshot(reference);

        // Now close the ROM and restore without reloading it: emulation must resume
        // identically (REQ-CORE-090).
        core.CloseRom();
        Assert.False(core.IsRunning);

        Assert.True(core.TryLoadState(state, 0, out _, out string romName, out string? err2), err2);
        Assert.True(core.IsRunning);
        Assert.Equal("Minimal.nes", romName);
        Assert.Equal(expectedRam, Snapshot(core));

        for (int i = 0; i < 20; i++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(referenceRam, Snapshot(core));
    }

    [Fact]
    public void REQ_CORE_091_restoring_at_an_offset_reports_the_bytes_consumed()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        byte[] state = core.SaveState();
        byte[] framed = [.. new byte[7], .. state, .. new byte[100]];

        Assert.True(core.TryLoadState(framed, 7, out int consumed, out string romName, out string? error), error);

        Assert.Equal(state.Length, consumed);
        Assert.Equal("Minimal.nes", romName);
    }

    [Fact]
    public void REQ_CORE_091_a_truncated_block_fails_and_leaves_the_core_stopped()
    {
        NesCore core = LoadedCore();
        core.RunFrame(0);

        byte[] state = core.SaveState();
        byte[] halved = state[..(state.Length / 2)];

        Assert.False(core.TryLoadState(halved, 0, out int consumed, out string romName, out string? error));

        Assert.Equal(0, consumed);
        Assert.Equal(string.Empty, romName);
        Assert.NotNull(error);
        Assert.False(core.IsRunning);

        // And a frame on a stopped core still does nothing (REQ-CORE-005).
        Assert.Equal(0, core.RunFrame(0));
    }

    [Fact]
    public void REQ_CORE_091_a_block_with_the_wrong_magic_fails()
    {
        NesCore core = LoadedCore();
        byte[] state = core.SaveState();
        state[0] ^= 0xFF;

        Assert.False(core.TryLoadState(state, 0, out _, out _, out string? error));
        Assert.Contains("magic", error, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void REQ_CORE_120_a_block_from_another_core_is_refused_with_the_spec_message()
    {
        NesCore core = LoadedCore();
        byte[] state = core.SaveState();

        // The core name is stored as a length-prefixed UTF-8 string right after the magic
        // and the version; corrupt its first character.
        int nameOffset = 4 + 4 + 4;
        state[nameOffset] = (byte)'X';

        Assert.False(core.TryLoadState(state, 0, out _, out string romName, out string? error));

        Assert.Equal(NesCore.ForeignCoreMessage, error);
        Assert.Equal("Save state used a different core.", error);
        Assert.Equal(string.Empty, romName);
    }

    [Fact]
    public void REQ_ACC_041_random_bytes_never_crash_the_restore()
    {
        var random = new Random(99);
        NesCore core = LoadedCore();

        for (int attempt = 0; attempt < 300; attempt++)
        {
            var junk = new byte[random.Next(0, 512)];
            random.NextBytes(junk);
            bool ok = core.TryLoadState(junk, 0, out _, out _, out string? error);
            Assert.True(!ok || error is null);
        }
    }

    [Fact]
    public void REQ_CORE_091_an_offset_outside_the_array_fails_cleanly()
    {
        NesCore core = LoadedCore();

        Assert.False(core.TryLoadState(new byte[10], 50, out _, out _, out string? error));
        Assert.NotNull(error);
    }

    [Fact]
    public void REQ_CORE_092_save_and_restore_at_frame_500_does_not_change_the_run()
    {
        // 1000 frames of fixed pseudo-random input, hashed; then the same run with a save
        // and restore at frame 500 (REQ-CORE-092).
        ulong straight = RunAndHash(saveRestoreAtFrame: -1);
        ulong interrupted = RunAndHash(saveRestoreAtFrame: 500);

        Assert.Equal(straight, interrupted);
    }

    [Fact]
    public void REQ_CORE_092_the_same_inputs_give_the_same_frames_memory_and_audio()
    {
        // Two instances, same ROM, same input sequence: batches, memory and samples must
        // match exactly (REQ-CORE-092, REQ-ARCH-012).
        NesCore first = LoadedCore();
        NesCore second = LoadedCore();

        var random = new Random(777);
        var words = new ushort[300];
        for (int i = 0; i < words.Length; i++)
        {
            words[i] = (ushort)random.Next(0, 0x10000);
        }

        var firstAudio = new List<float>();
        var secondAudio = new List<float>();

        foreach (ushort word in words)
        {
            Assert.Equal(first.RunFrame(word), second.RunFrame(word));
            Assert.Equal(Snapshot(first), Snapshot(second));

            var buffer = new float[512];
            while (first.ReadAudio(buffer) != 0)
            {
                firstAudio.AddRange(buffer);
            }

            while (second.ReadAudio(buffer) != 0)
            {
                secondAudio.AddRange(buffer);
            }
        }

        Assert.NotEmpty(firstAudio);
        Assert.Equal(firstAudio, secondAudio);
    }

    [Fact]
    public void REQ_CORE_092_saving_and_restoring_immediately_changes_nothing()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 17; i++)
        {
            core.RunFrame((ushort)i);
        }

        byte[] before = Snapshot(core);
        byte[] state = core.SaveState();

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);

        Assert.Equal(before, Snapshot(core));
        Assert.Equal(state, core.SaveState());
    }

    [Fact]
    public void REQ_ARCH_013_a_savestate_round_trip_replays_the_same_frames()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 40; i++)
        {
            core.RunFrame(0);
        }

        byte[] state = core.SaveState();

        var first = new List<byte[]>();
        for (int i = 0; i < 25; i++)
        {
            core.RunFrame((ushort)(i * 7));
            first.Add(Snapshot(core));
        }

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);

        for (int i = 0; i < 25; i++)
        {
            core.RunFrame((ushort)(i * 7));
            Assert.Equal(first[i], Snapshot(core));
        }
    }

    [Fact]
    public void REQ_ARCH_013_the_announced_size_bounds_the_real_one()
    {
        NesCore core = LoadedCore();
        int announced = core.MaxStateSize();

        for (int i = 0; i < 20; i++)
        {
            core.RunFrame((ushort)i);
            Assert.True(core.SaveState().Length <= announced);

            // The bound stays valid for the whole session.
            Assert.Equal(announced, core.MaxStateSize());
        }
    }

    [Fact]
    public void REQ_CORE_093_a_lightweight_block_leaves_the_rom_image_out()
    {
        NesCore core = LoadedCore();
        core.RunFrame(0);

        byte[] full = core.SaveState();
        byte[] light = core.SaveState(includeRomImage: false);

        Assert.True(light.Length < full.Length - (32 * 1024));

        // It still restores, as long as the ROM is loaded.
        Assert.True(core.TryLoadState(light, 0, out _, out _, out string? error), error);
        Assert.True(core.IsRunning);
    }

    [Fact]
    public void REQ_CORE_093_a_lightweight_block_is_refused_when_no_rom_is_loaded()
    {
        NesCore core = LoadedCore();
        core.RunFrame(0);
        byte[] light = core.SaveState(includeRomImage: false);

        var empty = new NesCore();
        Assert.False(empty.TryLoadState(light, 0, out _, out _, out string? error));
        Assert.Contains("no ROM image", error, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void REQ_CORE_090_saving_without_a_rom_throws()
    {
        var core = new NesCore();
        Assert.Throws<InvalidOperationException>(() => core.SaveState());
    }

    [Fact]
    public void REQ_CORE_014_the_stored_rom_name_comes_back_after_loading_another_rom()
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Test.nes").Success);
        core.RunFrame(0);

        byte[] state = core.SaveState();

        Assert.True(core.LoadRom(TestRoms.Minimal(0x00), "Other.nes").Success);
        Assert.Equal("Other.nes", core.RomName);

        Assert.True(core.TryLoadState(state, 0, out _, out string romName, out string? error), error);
        Assert.Equal("Test.nes", romName);
        Assert.Equal("Test.nes", core.RomName);
    }

    [Fact]
    public void REQ_CORE_074_restoring_a_state_drops_the_pending_audio()
    {
        NesCore core = LoadedCore();
        core.RunFrame(0);
        byte[] state = core.SaveState();

        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.Apu.AvailableSamples > 512);

        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);

        Assert.Equal(0, core.Apu.AvailableSamples);
        Assert.Equal(0, core.ReadAudio(new float[512]));
    }

    [Fact]
    public void REQ_CORE_074_reset_drops_the_pending_audio()
    {
        NesCore core = LoadedCore();
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.Apu.AvailableSamples > 0);

        core.Reset();
        Assert.Equal(0, core.Apu.AvailableSamples);
    }

    [Fact]
    public void REQ_CORE_090_the_region_is_part_of_the_state()
    {
        NesCore core = LoadedCore();
        core.Region = NesRegion.Pal;
        core.RunFrame(0);

        byte[] state = core.SaveState();

        core.Region = NesRegion.Ntsc;
        Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);

        Assert.Equal(NesRegion.Pal, core.Region);
    }

    private static ulong RunAndHash(int saveRestoreAtFrame)
    {
        NesCore core = LoadedCore();
        var random = new Random(20261020);
        ulong hash = 1469598103934665603UL;

        byte[]? state = null;
        for (int frame = 0; frame < 1000; frame++)
        {
            var word = (ushort)random.Next(0, 0x10000);
            core.RunFrame(word);

            if (frame == saveRestoreAtFrame)
            {
                state = core.SaveState();
                Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);
            }

            foreach (byte value in Snapshot(core))
            {
                hash = (hash ^ value) * 1099511628211UL;
            }
        }

        return hash;
    }

    /// <summary>Work RAM plus a few machine registers, as a comparable blob.</summary>
    private static byte[] Snapshot(NesCore core)
    {
        var blob = new byte[NesBus.RamSize + 8];
        core.Bus.Ram.CopyTo(blob);

        blob[NesBus.RamSize] = core.Cpu.A;
        blob[NesBus.RamSize + 1] = core.Cpu.X;
        blob[NesBus.RamSize + 2] = core.Cpu.Y;
        blob[NesBus.RamSize + 3] = core.Cpu.S;
        blob[NesBus.RamSize + 4] = core.Cpu.P;
        blob[NesBus.RamSize + 5] = (byte)core.Cpu.PC;
        blob[NesBus.RamSize + 6] = (byte)(core.Cpu.PC >> 8);
        blob[NesBus.RamSize + 7] = (byte)core.FrameCount;
        return blob;
    }
}
