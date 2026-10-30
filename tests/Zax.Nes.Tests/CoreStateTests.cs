using Zax.Nes.Ppu;
using Zax.Nes.State;

namespace Zax.Nes.Tests;

/// <summary>
/// The in-memory state of the core and the output switches of one frame, which run-ahead is built
/// on (AMD-041 REQ-CORE-121, REQ-LOOP-065, REQ-LOOP-068).
/// </summary>
public sealed class CoreStateTests
{
    /// <summary>
    /// A ROM that stirs work RAM at every iteration, so two runs that diverged are visible in the
    /// first bytes of RAM.
    /// </summary>
    private static byte[] CounterProgram() =>
    [
        0xE6, 0x20,       // INC $20
        0xA5, 0x20,       // LDA $20
        0x8D, 0x00, 0x03, // STA $0300
        0xE6, 0x21,       // INC $21
        0x4C, 0x00, 0x80, // JMP $8000
    ];

    /// <summary>
    /// A core with the counter ROM, the picture turned on over a nametable of varied tiles and
    /// pulse 1 sounding, so that a frame produces an image, tile records and audio.
    /// </summary>
    private static NesCore Busy()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, CounterProgram()), "RunAhead.nes").Success);

        // Three frames of warm-up, as SPEC-02 §5.1 prescribes, before the PPU is written to.
        for (int frame = 0; frame < 3; frame++)
        {
            core.RunFrame(0);
        }

        // A nametable row of increasing tile indices and a palette that is not all black, so the
        // composed pixels differ from one column to the next.
        core.WriteMemory(0x2006, 0x20);
        core.WriteMemory(0x2006, 0x00);
        for (int tile = 0; tile < 64; tile++)
        {
            core.WriteMemory(0x2007, (byte)(tile + 1));
        }

        core.WriteMemory(0x2006, 0x3F);
        core.WriteMemory(0x2006, 0x00);
        foreach (byte entry in (byte[])[0x0F, 0x21, 0x11, 0x30])
        {
            core.WriteMemory(0x2007, entry);
        }

        core.WriteMemory(0x2001, 0x1E); // background and sprites on, no clipping
        core.WriteMemory(0x2000, 0x00);

        // Pulse 1 at a constant volume: the APU then has something to resample (REQ-CORE-073).
        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, 0xBF);
        core.WriteMemory(0x4002, 0x40);
        core.WriteMemory(0x4003, 0x08);

        return core;
    }

    /// <summary>Everything one frame leaves behind that the host can observe.</summary>
    private static byte[] Fingerprint(NesCore core)
    {
        byte[] blob = new byte[NesBus.RamSize + Ppu2C02.ScreenWidth * Ppu2C02.ScreenHeight + 16];
        core.Bus.Ram.CopyTo(blob);
        core.PixelFrame.CopyTo(blob.AsSpan(NesBus.RamSize));

        int tail = NesBus.RamSize + (Ppu2C02.ScreenWidth * Ppu2C02.ScreenHeight);
        blob[tail] = core.Cpu.A;
        blob[tail + 1] = core.Cpu.X;
        blob[tail + 2] = core.Cpu.Y;
        blob[tail + 3] = core.Cpu.S;
        blob[tail + 4] = core.Cpu.P;
        blob[tail + 5] = (byte)core.Cpu.PC;
        blob[tail + 6] = (byte)(core.Cpu.PC >> 8);
        BitConverter.TryWriteBytes(blob.AsSpan(tail + 7), core.FrameCount);
        return blob;
    }

    private static ushort InputOf(int frame) => (ushort)((frame * 37) & 0xFFFF);

    /// <summary>
    /// REQ-CORE-121 — save, emulate 100 frames, restore, emulate the same 100 frames with the
    /// same inputs: the images and the RAM are identical byte for byte, frame by frame.
    /// </summary>
    [Fact]
    public void REQ_CORE_121_a_restore_replays_a_hundred_frames_byte_for_byte()
    {
        NesCore core = Busy();
        CoreState state = new();
        core.CaptureState(state);

        List<byte[]> first = [];
        for (int frame = 0; frame < 100; frame++)
        {
            core.RunFrame(InputOf(frame));
            first.Add(Fingerprint(core));
        }

        core.RestoreState(state);

        List<byte[]> second = [];
        for (int frame = 0; frame < 100; frame++)
        {
            core.RunFrame(InputOf(frame));
            second.Add(Fingerprint(core));
        }

        Assert.Equal(100, first.Count);
        for (int frame = 0; frame < first.Count; frame++)
        {
            Assert.Equal(first[frame], second[frame]);
        }
    }

    /// <summary>
    /// REQ-CORE-121 — the whole machine comes back identical: a second capture of the restored
    /// state is the very same block of bytes.
    /// </summary>
    [Fact]
    public void REQ_CORE_121_the_restored_state_is_identical()
    {
        NesCore core = Busy();
        CoreState reference = new();
        core.CaptureState(reference);
        byte[] expected = reference.AsSpan().ToArray();

        for (int frame = 0; frame < 7; frame++)
        {
            core.RunFrame(InputOf(frame));
        }

        Assert.NotEqual(expected, ReCapture(core));

        core.RestoreState(reference);
        Assert.Equal(expected, ReCapture(core));

        static byte[] ReCapture(NesCore core)
        {
            CoreState again = new();
            core.CaptureState(again);
            return again.AsSpan().ToArray();
        }
    }

    /// <summary>
    /// REQ-CORE-121 — a restore leaves the audio already produced intact, unlike a savestate
    /// restore, which drops it (REQ-CORE-074).
    /// </summary>
    [Fact]
    public void REQ_CORE_121_a_restore_keeps_the_audio_already_produced()
    {
        NesCore core = Busy();
        core.RunFrame(0);

        CoreState state = new();
        core.CaptureState(state);

        core.RunFrame(0);
        int produced = core.Apu.AvailableSamples;
        Assert.True(produced > 0, "the frame produced no audio at all");

        core.RestoreState(state);

        // Step 5 of REQ-LOOP-065 undoes the frame, not the sound it produced: the queue is still
        // there, with the very samples the player is about to hear.
        Assert.Equal(produced, core.Apu.AvailableSamples);

        float[] kept = new float[produced];
        Assert.Equal(produced, core.ReadAudio(kept));
        Assert.Contains(kept, sample => sample != 0f);

        // And the savestate path still drops it (REQ-CORE-074, Q-006).
        core.RunFrame(0);
        Assert.True(core.Apu.AvailableSamples > 0);
        byte[] block = core.SaveState();
        Assert.True(core.TryLoadState(block, 0, out _, out _, out string? error), error);
        Assert.Equal(0, core.Apu.AvailableSamples);
    }

    /// <summary>
    /// REQ-CORE-121 — the ROM image is not part of the state and the size is bounded by what the
    /// core announces (REQ-ARCH-013).
    /// </summary>
    [Fact]
    public void REQ_CORE_121_the_state_excludes_the_rom_image_and_is_bounded()
    {
        NesCore core = Busy();
        CoreState state = new();
        core.CaptureState(state);

        Assert.False(state.IsEmpty);
        Assert.True(state.Length <= core.MaxCoreStateSize());

        // The portable block carries the 40 KiB image as well, so the in-memory state is smaller
        // by at least that much.
        byte[] portable = core.SaveState(includeRomImage: true);
        Assert.True(
            portable.Length - state.Length >= 40 * 1024,
            $"portable {portable.Length}, in memory {state.Length}");

        // The ROM stays loaded through a restore: the state carries none of it.
        core.RestoreState(state);
        Assert.True(core.IsRunning);
        Assert.Equal("RunAhead.nes", core.RomName);
    }

    /// <summary>
    /// REQ-CORE-121 — the same buffer serves every frame: two hundred captures and restores stay
    /// inside the announced bound and the core never leaves the real frame behind.
    /// </summary>
    /// <remarks>
    /// The length itself moves from one frame to the next: the tile records of REQ-CORE-030 are
    /// part of the PPU state and there are not always as many of them. What is fixed is the
    /// bound (REQ-ARCH-013).
    /// </remarks>
    [Fact]
    public void REQ_CORE_121_the_same_buffer_serves_every_frame()
    {
        NesCore core = Busy();
        CoreState state = new();

        for (int frame = 0; frame < 200; frame++)
        {
            long before = core.FrameCount;
            core.RunFrame(InputOf(frame));
            core.CaptureState(state);
            Assert.True(
                state.Length <= core.MaxCoreStateSize(),
                $"frame {frame}: {state.Length} bytes for a bound of {core.MaxCoreStateSize()}");

            core.RunFrame(InputOf(frame), FrameOutput.None);
            core.RestoreState(state);
            Assert.Equal(before + 1, core.FrameCount);
        }
    }

    /// <summary>REQ-CORE-121 — a capture or a restore without a ROM is refused, not half done.</summary>
    [Fact]
    public void REQ_CORE_121_a_stopped_core_has_no_state()
    {
        NesCore core = new();
        CoreState state = new();

        Assert.Throws<InvalidOperationException>(() => core.CaptureState(state));
        Assert.Throws<InvalidOperationException>(() => core.RestoreState(state));

        NesCore loaded = Busy();
        Assert.Throws<InvalidOperationException>(() => loaded.RestoreState(new CoreState()));
    }

    /// <summary>
    /// REQ-LOOP-065 — the emulation does not depend on what a frame outputs: the same frame run
    /// silent and invisible leaves the machine in exactly the same state.
    /// </summary>
    [Theory]
    [InlineData(FrameOutput.None)]
    [InlineData(FrameOutput.Pixels)]
    [InlineData(FrameOutput.Audio)]
    [InlineData(FrameOutput.Video)]
    public void REQ_LOOP_065_what_a_frame_outputs_never_changes_the_machine(FrameOutput output)
    {
        byte[] ordinary = StateAfter(FrameOutput.All);
        byte[] reduced = StateAfter(output);

        Assert.Equal(ordinary, reduced);

        static byte[] StateAfter(FrameOutput output)
        {
            NesCore core = Busy();
            for (int frame = 0; frame < 5; frame++)
            {
                core.RunFrame(InputOf(frame), output);
            }

            CoreState state = new();
            core.CaptureState(state);
            return state.AsSpan().ToArray();
        }
    }

    /// <summary>
    /// REQ-LOOP-065 and REQ-LOOP-067 — a frame that is not the displayed one hands no batch to
    /// the host, produces no sample, and leaves the pixel image alone; an ordinary frame does all
    /// three.
    /// </summary>
    [Fact]
    public void REQ_LOOP_065_a_hidden_frame_produces_nothing_for_the_host()
    {
        NesCore core = Busy();
        int batches = 0;
        core.VideoCallback = (in FrameBatch _) => batches++;

        core.RunFrame(0, FrameOutput.All);
        Assert.Equal(1, batches);
        Assert.True(core.Apu.AvailableSamples > 0);

        byte[] picture = core.PixelFrame.ToArray();
        core.ReadAudio(new float[core.Apu.AvailableSamples]);
        Assert.Equal(0, core.Apu.AvailableSamples);

        core.RunFrame(0, FrameOutput.None);

        Assert.Equal(1, batches);
        Assert.Equal(0, core.Apu.AvailableSamples);
        Assert.Equal(picture, core.PixelFrame.ToArray());

        // The switches belong to the frame: the next ordinary frame produces everything again.
        core.RunFrame(0);
        Assert.Equal(2, batches);
        Assert.True(core.Apu.AvailableSamples > 0);
    }

    /// <summary>
    /// REQ-LOOP-068 — a frame asked for its pixels writes them, whatever else it withholds: that
    /// is what a machine with a Zapper port gets for every emulated frame.
    /// </summary>
    [Fact]
    public void REQ_LOOP_068_a_hidden_frame_can_still_render_its_pixels()
    {
        NesCore reference = Busy();
        NesCore hidden = Busy();

        for (int frame = 0; frame < 4; frame++)
        {
            reference.RunFrame(InputOf(frame), FrameOutput.All);
            hidden.RunFrame(InputOf(frame), FrameOutput.Pixels);
        }

        Assert.Equal(reference.PixelFrame.ToArray(), hidden.PixelFrame.ToArray());

        // And the picture really was composed, rather than left uniformly blank.
        Assert.True(reference.PixelFrame.ToArray().Distinct().Count() > 1);
    }
}
