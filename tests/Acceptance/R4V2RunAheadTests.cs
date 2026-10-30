using System.Numerics;
using Zax.App;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Nes.State;
using Zax.Profiles;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 verification of the lot <c>v2-3-run-ahead</c>: AMD-041 (REQ-LOOP-064 to
/// REQ-LOOP-068, REQ-CORE-121, REQ-LOOP-060 amended, REQ-ARCH-012 and REQ-ARCH-013 amended)
/// and the <c>RunAheadFrames</c> row of AMD-040 REQ-LUA-114.
/// </summary>
/// <remarks>
/// <para>
/// Everything here is written from the amendment text alone and observes the program from the
/// outside: the settings file, the Lua results, the core's own state, the image the core
/// composes and the frames the conversion publishes. The desktop loop is driven through
/// <see cref="HostDriver"/>, which reaches <c>EmulationHost</c> by reflection, so no production
/// file is touched.
/// </para>
/// <para>
/// The test ROM of REQ-LOOP-065 is <see cref="R4RunAheadRom.Counter"/>. It follows the
/// convention the criterion needs (N-41-2 of the second R2 reading): <b>the counter an image
/// shows is the one the frame had at its beginning</b>. Its NMI handler runs at the very start
/// of an emulated frame — the core stops a frame at the first dot of the vertical blanking, so
/// the interrupt is serviced by the next call — and it writes the universal backdrop before
/// incrementing. Rendering stays off, so the whole 256 × 240 image is that backdrop colour and
/// the "image of the frame" is one byte: the five low bits are the counter, bit 5 is the state
/// of button A as read during that very frame.
/// </para>
/// </remarks>
public sealed class R4V2RunAheadTests
{
    /// <summary>Zero-page address of the counter of the test ROM (REQ-LOOP-065).</summary>
    private const ushort CounterAddress = 0x10;

    /// <summary>Zero-page address holding the byte the running frame put on its image.</summary>
    private const ushort DisplayedAddress = 0x12;

    /// <summary>Bit 5 of the displayed byte: button A as the frame read it.</summary>
    private const int ButtonABit = 0x20;

    // =====================================================================
    // REQ-LOOP-064 — the setting
    // =====================================================================

    /// <summary>
    /// REQ-LOOP-064 — an empty user root gives Run-ahead = 1, the key is <c>emu.run_ahead</c>,
    /// its domain is the four whole values 0 to 3 of AMD-041 §3 and its scope is U.
    /// </summary>
    [Fact] // REQ-LOOP-064
    public void REQ_LOOP_064_an_empty_user_root_gives_one_frame_of_run_ahead()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        new SettingsStore(root.Paths).Load(settings, DateTimeOffset.UnixEpoch);

        Assert.Equal(1, settings.RunAheadFrames);
        Assert.Equal(1, settings.Model.GetInt("emu.run_ahead"));

        SettingDescriptor descriptor = SettingsCatalogue.Get("emu.run_ahead");
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.Equal(1, descriptor.DefaultValue.AsInt);
        Assert.Equal("RunAheadFrames", descriptor.LuaPath);
    }

    /// <summary>
    /// REQ-LOOP-064 — "toute valeur hors de [0, 3] est ramenée dans l'intervalle": the written
    /// value is clamped, never replaced by the default, whatever the way in.
    /// </summary>
    [Theory] // REQ-LOOP-064
    [InlineData(-7, 0)]
    [InlineData(-1, 0)]
    [InlineData(0, 0)]
    [InlineData(1, 1)]
    [InlineData(2, 2)]
    [InlineData(3, 3)]
    [InlineData(4, 3)]
    [InlineData(7, 3)]
    [InlineData(int.MaxValue, 3)]
    public void REQ_LOOP_064_a_value_outside_zero_to_three_is_brought_back(int written, int expected)
    {
        AppSettings settings = new();
        settings.RunAheadFrames = written;
        Assert.Equal(expected, settings.RunAheadFrames);
    }

    /// <summary>
    /// REQ-LOOP-064 — "la valeur survit à un redémarrage": the setting is written to the user
    /// settings file and read back by a second instance over the same root.
    /// </summary>
    [Fact] // REQ-LOOP-064
    public void REQ_LOOP_064_the_value_survives_a_restart()
    {
        using TemporaryRoot root = new();

        AppSettings first = new();
        SettingsStore store = new(root.Paths);
        store.Load(first, DateTimeOffset.UnixEpoch);
        first.RunAheadFrames = 3;
        store.Save(first);

        Assert.Contains(
            "\"emu.run_ahead\": 3",
            File.ReadAllText(root.Paths.SettingsFile),
            StringComparison.Ordinal);

        AppSettings second = new();
        new SettingsStore(root.Paths).Load(second, DateTimeOffset.UnixEpoch);
        Assert.Equal(3, second.RunAheadFrames);
    }

    /// <summary>
    /// REQ-LOOP-064 and AMD-041 §3 — the four values carry the labels of the amendment, in
    /// English and in French, the value 1 being the recommended one.
    /// </summary>
    [Fact] // REQ-LOOP-064
    public void REQ_LOOP_064_the_four_values_carry_the_labels_of_the_amendment()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get("emu.run_ahead");
        IReadOnlyList<SettingOption> options = descriptor.Domain.Options
            ?? throw new InvalidOperationException("the Run-ahead setting has no listed values.");

        Assert.Equal(4, options.Count);
        Assert.Equal([0d, 1d, 2d, 3d], options.Select(static option => option.Value));

        Assert.Equal("Run-ahead", Ui("setting.emu.run_ahead", english: true));
        Assert.Equal("Run-ahead", Ui("setting.emu.run_ahead", english: false));

        string[] english = ["Off", "1 frame (Recommended)", "2 frames", "3 frames"];
        string[] french = ["Désactivé", "1 frame (recommandé)", "2 frames", "3 frames"];
        for (int value = 0; value <= 3; value++)
        {
            string label = options[value].Label;
            Assert.Equal(english[value], UiOption("emu.run_ahead", label, english: true));
            Assert.Equal(french[value], UiOption("emu.run_ahead", label, english: false));
        }
    }

    /// <summary>
    /// REQ-LOOP-064 and AMD-040 REQ-LUA-114 — <c>Setting.RunAheadFrames</c> reads the setting,
    /// writes it, and a write is brought back into [0, 3]: the criterion of both requirements.
    /// </summary>
    [Fact] // REQ-LOOP-064, REQ-LUA-114
    public void REQ_LUA_114_run_ahead_frames_is_read_and_written_from_lua()
    {
        using LuaRun run = new LuaProfile().Script("verify", "-- nothing").Start();

        Assert.Equal("number", run.TypeOf("Setting.RunAheadFrames"));
        Assert.Equal(1d, run.Num("Setting.RunAheadFrames"));

        run.Do("Setting.RunAheadFrames = 7");
        Assert.Equal(3d, run.Num("Setting.RunAheadFrames"));

        run.Do("Setting.RunAheadFrames = -1");
        Assert.Equal(0d, run.Num("Setting.RunAheadFrames"));

        run.Do("Setting.RunAheadFrames = 2");
        Assert.Equal(2d, run.Num("Setting.RunAheadFrames"));
        Assert.Equal(2, run.Settings.Model.GetInt("emu.run_ahead"));
    }

    /// <summary>
    /// REQ-LOOP-064 — the setting is modifiable during the game and takes effect at the next
    /// real frame: with the test ROM the displayed counter jumps from <c>c</c> to <c>c + 2</c>
    /// on the very first real frame that follows the change.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-064
    public void REQ_LOOP_064_a_change_during_the_game_applies_at_the_next_real_frame()
    {
        using HostDriver host = Open(0, R4RunAheadRom.Counter());
        Step(host, 8);

        NesCore core = host.Session!.Core;
        int counter = core.ReadMemory(CounterAddress);

        // With N = 0 the image is the one of the frame that has just run: the counter it had at
        // its beginning, one less than the counter now.
        Assert.Equal((counter - 1) & 0x1F, Displayed(core));

        host.Settings.RunAheadFrames = 2;
        Step(host, 1);

        // One real frame later the image is two frames ahead, and the core has advanced by one.
        Assert.Equal((counter + 2) & 0x1F, Displayed(core));
        Assert.Equal((counter + 1) & 0xFF, core.ReadMemory(CounterAddress));
    }

    // =====================================================================
    // REQ-CORE-121 — the in-memory state of the core
    // =====================================================================

    /// <summary>
    /// REQ-CORE-121, criterion — save, emulate 100 frames, restore, emulate the same 100 frames
    /// with the same inputs: images and RAM identical byte for byte.
    /// </summary>
    [Fact] // REQ-CORE-121
    public void REQ_CORE_121_a_restore_replays_a_hundred_frames_byte_for_byte()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(R4RunAheadRom.Counter(), "counter.nes").Success);
        core.Reset();
        for (int warm = 0; warm < 5; warm++)
        {
            core.RunFrame(0);
        }

        CoreState state = new();
        core.CaptureState(state);
        long frameAtCapture = core.FrameCount;

        (List<byte[]> firstImages, List<byte[]> firstRam) = Replay(core, 100);

        core.RestoreState(state);
        Assert.Equal(frameAtCapture, core.FrameCount);

        (List<byte[]> secondImages, List<byte[]> secondRam) = Replay(core, 100);

        for (int frame = 0; frame < 100; frame++)
        {
            Assert.True(
                firstImages[frame].AsSpan().SequenceEqual(secondImages[frame]),
                $"image of frame {frame} differs after the restore.");
            Assert.True(
                firstRam[frame].AsSpan().SequenceEqual(secondRam[frame]),
                $"RAM after frame {frame} differs after the restore.");
        }
    }

    /// <summary>
    /// REQ-CORE-121 — the state carries the frame counter and every component the requirement
    /// lists, its size is bounded, and the ROM image is not part of it.
    /// </summary>
    [Fact] // REQ-CORE-121
    public void REQ_CORE_121_the_state_is_bounded_and_holds_no_rom_image()
    {
        byte[] image = R4RunAheadRom.Counter();
        NesCore core = new();
        Assert.True(core.LoadRom(image, "counter.nes").Success);
        core.Reset();
        for (int frame = 0; frame < 20; frame++)
        {
            core.RunFrame(0x0001);
        }

        CoreState state = new();
        core.CaptureState(state);

        Assert.False(state.IsEmpty);
        Assert.True(
            state.Length <= core.MaxCoreStateSize(),
            $"state of {state.Length} bytes over the announced bound {core.MaxCoreStateSize()}.");

        // "L'image de la ROM, qui reste chargée, n'en fait pas partie": the bound itself is the
        // savestate bound minus the ROM, and 40 KiB of PRG + CHR do not fit in the state.
        Assert.True(core.MaxCoreStateSize() < core.MaxStateSize());
        Assert.True(
            state.Length < 32 * 1024,
            $"the state of {state.Length} bytes is large enough to hold the 32 KiB of PRG.");

        // Mapper, CPU, PPU, APU, inputs, RAM and the frame counter all come back.
        core.FrameCount = 12_345;
        core.WriteMemory(0x0400, 0x5A);
        core.RestoreState(state);
        Assert.Equal(20, core.FrameCount);
        Assert.NotEqual(0x5A, core.ReadMemory(0x0400));
    }

    /// <summary>
    /// REQ-CORE-121 — "une restauration qui laisse intacte la sortie audio déjà produite":
    /// the samples the host has not read yet survive the in-memory restore, unlike a savestate.
    /// </summary>
    [Fact] // REQ-CORE-121
    public void REQ_CORE_121_a_restore_leaves_the_audio_already_produced_alone()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(R4RunAheadRom.Counter(), "counter.nes").Success);
        core.Reset();
        core.RunFrame(0);

        // Everything produced so far is read away, so only the frame that follows the capture
        // can leave samples behind. ReadAudio is all or nothing (REQ-CORE-071), so it is asked
        // for small chunks.
        Assert.True(DrainAudio(core) > 0);

        CoreState state = new();
        core.CaptureState(state);

        // The displayed frame of REQ-LOOP-065 step 4 produces sound; step 5 must not undo it.
        core.RunFrame(0);
        core.RestoreState(state);

        int kept = DrainAudio(core);
        Assert.True(
            kept > 100,
            $"the restore left only {kept} samples of the sound the displayed frame produced.");

        // A savestate restore, which REQ-CORE-074 makes drop the pending audio, is the contrast.
        core.RunFrame(0);
        byte[] block = core.SaveState(includeRomImage: false);
        Assert.True(core.TryLoadState(block, 0, out _, out _, out _));
        Assert.Equal(0, DrainAudio(core));
    }

    // =====================================================================
    // REQ-LOOP-065 — the five steps of a real frame
    // =====================================================================

    /// <summary>
    /// REQ-LOOP-065, criterion (vector) — Run-ahead = 2: just before the real frame the ROM
    /// counter is <c>k</c>; A pressed at that frame; the displayed image shows <c>k + 2</c> with
    /// A pressed; after step 5 the counter is <c>k + 1</c>, the one of the real frame that has
    /// just run.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-065
    public void REQ_LOOP_065_the_vector_of_the_amendment()
    {
        using HostDriver host = Open(2, R4RunAheadRom.Counter());
        Step(host, 8);

        NesCore core = host.Session!.Core;
        int k = core.ReadMemory(CounterAddress);
        long coreFrames = core.FrameCount;

        Step(host, 1, InputAction.A);

        Assert.Equal(((k + 2) & 0x1F) | ButtonABit, Displayed(core));
        Assert.Equal((k + 1) & 0xFF, core.ReadMemory(CounterAddress));
        Assert.Equal(coreFrames + 1, core.FrameCount);

        // REQ-LOOP-060 (a) and REQ-LOOP-065: the inputs are read once and every one of the
        // N + 1 emulated frames gets the same E — the real frame saw A pressed too, and it is
        // its byte that survives step 5.
        Assert.Equal((k & 0x1F) | ButtonABit, core.ReadMemory(DisplayedAddress));
    }

    /// <summary>
    /// REQ-LOOP-065 — the displayed frame is N frames ahead for every N, and with N = 0 the
    /// frame is emulated once and it is the displayed one.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostTheory] // REQ-LOOP-065
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    public void REQ_LOOP_065_the_displayed_frame_is_n_frames_ahead(int runAhead)
    {
        using HostDriver host = Open(runAhead, R4RunAheadRom.Counter());
        Step(host, 8);

        NesCore core = host.Session!.Core;
        for (int frame = 0; frame < 6; frame++)
        {
            int before = core.ReadMemory(CounterAddress);
            long coreBefore = core.FrameCount;

            Step(host, 1);

            Assert.Equal((before + runAhead) & 0x1F, Displayed(core));
            Assert.Equal((before + 1) & 0xFF, core.ReadMemory(CounterAddress));
            Assert.Equal(coreBefore + 1, core.FrameCount);
        }
    }

    /// <summary>
    /// REQ-LOOP-065, step 1 and step 3 — the frames that are not displayed produce no sound:
    /// one real frame yields one frame of audio whatever N is.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostTheory] // REQ-LOOP-065, REQ-LOOP-067
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(3)]
    public void REQ_LOOP_067_one_real_frame_yields_one_frame_of_audio(int runAhead)
    {
        using HostDriver host = Open(runAhead, R4RunAheadRom.Counter());
        Step(host, 4);

        NesCore core = host.Session!.Core;
        DrainAudio(core);

        const int frames = 60;
        int samples = 0;
        for (int frame = 0; frame < frames; frame++)
        {
            Step(host, 1);
            samples += DrainAudio(core);
        }

        double perFrame = (double)samples / frames;
        double expected = (double)core.SampleRate / core.Timing.NominalFramesPerSecond;
        Assert.True(
            Math.Abs(perFrame - expected) < 0.05 * expected,
            $"N = {runAhead}: {perFrame:F1} samples per real frame instead of about {expected:F1}.");
    }

    // =====================================================================
    // REQ-LOOP-060 (amended) and REQ-LOOP-067 — what sees the displayed frame
    // =====================================================================

    /// <summary>
    /// REQ-LOOP-060 (c) amended and REQ-LOOP-067 — the 2D → 3D conversion receives one frame
    /// per real frame and it is the displayed one: the palette the conversion publishes is the
    /// one of the image two frames ahead.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-060, REQ-LOOP-067
    public void REQ_LOOP_060_the_conversion_receives_the_displayed_frame()
    {
        using HostDriver host = Open(2, R4RunAheadRom.Counter());
        Step(host, 8);

        NesCore core = host.Session!.Core;
        for (int frame = 0; frame < 5; frame++)
        {
            int before = core.ReadMemory(CounterAddress);
            int counterBefore = host.Session!.FrameCounter;

            Step(host, 1);

            // Entry 0 of the published palette is the universal backdrop of the frame that was
            // converted; the test ROM makes it the counter of that frame.
            Zax.Engine3D.ConversionFrame published = host.Session!.Engine.PublishedFrame!;
            Assert.Equal((before + 2) & 0x1F, published.Palette[0]);

            // One converted frame per real frame, and the sequence is contiguous.
            Assert.Equal(counterBefore + 1, host.Session!.FrameCounter);
        }
    }

    /// <summary>
    /// REQ-LOOP-067, criterion — a script that increments a counter in <c>Update</c> grows by 1
    /// per real frame whatever N is: the frame hooks are called neither for the real frame nor
    /// for the frames in advance that are not displayed.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostTheory] // REQ-LOOP-067
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    public void REQ_LOOP_067_the_frame_hooks_run_once_per_real_frame(int runAhead)
    {
        GameProfile profile = HookProfile();
        using HostDriver host = HostDriver.Create(profile);
        host.Settings.RunAheadFrames = runAhead;

        string rom = host.WriteRom(R4RunAheadRom.Counter());
        host.Launch(host.EntryIdByTitle("RunAhead-probe"), rom);
        Drain(host);
        Step(host, 1);

        var scripts = host.Session!.Scripts;
        int updates = (int)scripts.EvaluateNumber("return updates");
        int lates = (int)scripts.EvaluateNumber("return lates");
        int seen = (int)scripts.EvaluateNumber("return lastframe");

        const int frames = 40;
        Step(host, frames);

        // One call per real frame, never one for the real frame or for a frame in advance that
        // is not displayed — whatever N is.
        Assert.Equal(frames, (int)scripts.EvaluateNumber("return updates") - updates);
        Assert.Equal(frames, (int)scripts.EvaluateNumber("return lates") - lates);

        // The global `frame` the hooks see is the displayed frame's number, and the sequence of
        // displayed frames is contiguous: one per real frame.
        Assert.Equal(frames, (int)scripts.EvaluateNumber("return lastframe") - seen);
    }

    /// <summary>
    /// REQ-LOOP-067, criterion — with Run-ahead = 2, <c>Script:ReadMem</c> inside the frame
    /// hooks reads the RAM of the <b>displayed</b> frame, so the counter it reports is the one
    /// the image shows, <c>k + 2</c>.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-067
    public void REQ_LOOP_067_read_mem_in_a_hook_sees_the_displayed_frame()
    {
        GameProfile profile = HookProfile();
        using HostDriver host = HostDriver.Create(profile);
        host.Settings.RunAheadFrames = 2;

        string rom = host.WriteRom(R4RunAheadRom.Counter());
        host.Launch(host.EntryIdByTitle("RunAhead-probe"), rom);
        Drain(host);
        Step(host, 8);

        NesCore core = host.Session!.Core;
        var scripts = host.Session!.Scripts;

        for (int frame = 0; frame < 5; frame++)
        {
            int k = core.ReadMemory(CounterAddress);
            Step(host, 1);

            // What the hook read is the byte the displayed frame put on its image: k + 2.
            Assert.Equal((k + 2) & 0x1F, (int)scripts.EvaluateNumber("return seendisplayed") & 0x1F);

            // The core itself is back on the real frame: k + 1.
            Assert.Equal((k + 1) & 0xFF, core.ReadMemory(CounterAddress));
            Assert.Equal(k & 0x1F, core.ReadMemory(DisplayedAddress) & 0x1F);
        }
    }

    // =====================================================================
    // REQ-LOOP-066 — what follows the real frame
    // =====================================================================

    /// <summary>
    /// REQ-LOOP-066, criterion — Run-ahead = 3, quick save then quick load: the core's frame
    /// counter is the one of the real frame of the save.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-066
    public void REQ_LOOP_066_a_quick_save_carries_the_real_frame()
    {
        using HostDriver host = Open(3, R4RunAheadRom.Counter());
        StepTo(host, 120);

        NesCore core = host.Session!.Core;
        long savedFrame = core.FrameCount;
        int savedCounter = core.ReadMemory(CounterAddress);

        // 120 real frames, 480 emulated ones: the core's counter follows the real frame.
        Assert.Equal(120, savedFrame);

        host.Run(ZaxCommands.QuickSave);
        Step(host, 45);
        Assert.Equal(savedFrame + 45, host.Session!.Core.FrameCount);

        // No frame runs between the load and the reading: the core is exactly where the save
        // left it, on its real frame.
        host.Run(ZaxCommands.QuickLoad);

        NesCore reloaded = host.Session!.Core;
        Assert.Equal(savedFrame, reloaded.FrameCount);
        Assert.Equal(savedCounter, reloaded.ReadMemory(CounterAddress));
    }

    /// <summary>
    /// REQ-LOOP-066, criterion — the rollback history always captures a snapshot at every
    /// multiple of 60 of the <b>real</b> counter, whatever N is.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-066, REQ-LOOP-030
    public void REQ_LOOP_066_the_history_follows_the_real_counter()
    {
        // Snapshots at the real counters 0, 60, 120, 180 and 240; the one of 300 is taken at the
        // start of the next frame.
        Assert.Equal(5, Snapshots(0));
        Assert.Equal(5, Snapshots(3));

        static int Snapshots(int runAhead)
        {
            using HostDriver host = Open(runAhead, R4RunAheadRom.Counter());
            StepTo(host, 300);
            return host.Rollback.Count;
        }
    }

    /// <summary>
    /// REQ-LOOP-066, last sentence — "charger un état, faire un reset ou changer de jeu reprend
    /// la frame suivante avec le même N": after a quick load and after a reset the displayed
    /// frame is still N frames ahead.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-066
    public void REQ_LOOP_066_a_load_or_a_reset_carries_on_with_the_same_n()
    {
        using HostDriver host = Open(2, R4RunAheadRom.Counter());
        StepTo(host, 30);
        host.Run(ZaxCommands.QuickSave);
        Step(host, 10);
        host.Run(ZaxCommands.QuickLoad);

        Assert.Equal(2, host.Settings.RunAheadFrames);
        AheadByTwo(host);

        host.Run(ZaxCommands.Reset);
        Assert.Equal(2, host.Settings.RunAheadFrames);
        AheadByTwo(host);

        static void AheadByTwo(HostDriver host)
        {
            NesCore core = host.Session!.Core;
            Step(host, 4);
            int before = core.ReadMemory(CounterAddress);
            Step(host, 1);
            Assert.Equal((before + 2) & 0x1F, Displayed(core));
            Assert.Equal((before + 1) & 0xFF, core.ReadMemory(CounterAddress));
        }
    }

    // =====================================================================
    // REQ-LOOP-068 — the Zapper
    // =====================================================================

    /// <summary>
    /// REQ-LOOP-068, criterion (auto) — a ROM that shows a lit screen and counts the light the
    /// Zapper sees gives the same results with Run-ahead = 1 as with Run-ahead = 0: every
    /// emulated frame renders its pixels and detects the light on its own.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-068
    public void REQ_LOOP_068_the_zapper_gives_the_same_result_with_run_ahead()
    {
        (int light, int position) withoutRunAhead = ZapperRun(0);
        (int light, int position) withRunAhead = ZapperRun(1);

        Assert.True(withoutRunAhead.position >= 0, "the aim never landed on the NES image.");
        Assert.Equal(withoutRunAhead.position, withRunAhead.position);
        Assert.True(withoutRunAhead.light > 0, "the ROM never saw the light without run-ahead.");
        Assert.Equal(withoutRunAhead.light, withRunAhead.light);
    }

    /// <summary>
    /// REQ-LOOP-068, second sentence — with no Zapper plugged in, a frame that is not displayed
    /// may skip its pixels, but "tout le reste du comportement du cœur (temps, drapeau de
    /// sprite 0, IRQ) reste exact". The ROM measures, in CPU iterations, the delay from the
    /// start of the vertical blanking to the sprite 0 hit of the next frame: the figure, and the
    /// whole state of the core, are the same at N = 0, 1, 2 and 3.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-068
    public void REQ_LOOP_068_sprite_zero_and_timing_stay_exact_without_pixels()
    {
        (byte[] State, int Hit) reference = Sprite0Run(0);
        Assert.True(reference.Hit > 0, "the ROM never saw the sprite 0 hit.");

        for (int runAhead = 1; runAhead <= 3; runAhead++)
        {
            (byte[] State, int Hit) measured = Sprite0Run(runAhead);
            Assert.Equal(reference.Hit, measured.Hit);
            Assert.True(
                reference.State.AsSpan().SequenceEqual(measured.State),
                $"the core state after the sprite 0 ROM differs at N = {runAhead}.");
        }
    }

    // =====================================================================
    // AMD-041 §4 — libretro
    // =====================================================================

    /// <summary>
    /// AMD-041 §4 — in libretro the host provides run-ahead from the serialisation: the setting
    /// is not a core option there.
    /// </summary>
    [Fact] // REQ-LOOP-064
    public void AMD_041_the_setting_is_not_exposed_to_libretro()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get("emu.run_ahead");
        Assert.False(descriptor.IsLibretroExposed);
        Assert.DoesNotContain(
            SettingsCatalogue.All.Where(static d => d.IsLibretroExposed),
            d => d.Key == "emu.run_ahead");
    }

    // =====================================================================
    // REQ-ARCH-012 and REQ-ARCH-013 (amended)
    // =====================================================================

    /// <summary>
    /// REQ-ARCH-012 amended — "la suite des états du cœur réels ne dépend pas de N": the same
    /// ROM and the same input sequence leave the very same core state after the same number of
    /// real frames, for N = 0, 1, 2 and 3.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-ARCH-012
    public void REQ_ARCH_012_the_real_core_states_do_not_depend_on_n()
    {
        byte[] reference = CoreTrace(0);
        for (int runAhead = 1; runAhead <= 3; runAhead++)
        {
            Assert.True(
                reference.AsSpan().SequenceEqual(CoreTrace(runAhead)),
                $"the sequence of real core states differs at N = {runAhead}.");
        }
    }

    /// <summary>
    /// REQ-ARCH-012 amended — the frames, tiles and conversion results are only comparable at
    /// equal N: at a different N the published frame of the same real frame is a different one.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-ARCH-012
    public void REQ_ARCH_012_the_displayed_sequence_depends_on_n()
    {
        int[] zero = DisplayedTrace(0);
        int[] one = DisplayedTrace(1);
        int[] three = DisplayedTrace(3);

        Assert.Equal(zero, DisplayedTrace(0));
        Assert.NotEqual(zero, one);
        Assert.NotEqual(one, three);

        // And the shift is exactly N frames.
        for (int frame = 0; frame < zero.Length; frame++)
        {
            Assert.Equal((zero[frame] + 1) & 0x1F, one[frame]);
            Assert.Equal((zero[frame] + 3) & 0x1F, three[frame]);
        }
    }

    // =====================================================================
    // helpers
    // =====================================================================

    /// <summary>
    /// Runs the frames the host's clock already owes, so that one tick of one period runs
    /// exactly one real frame afterwards. A launch and a command both tick the clock while they
    /// wait for the load, which leaves a backlog (REQ-LOOP-012).
    /// </summary>
    private static void Drain(HostDriver host)
    {
        for (int tick = 0; tick < 32 && host.Tick(0.0) > 0; tick++)
        {
            // the backlog, consumed one burst at a time
        }
    }

    /// <summary>Runs exactly <paramref name="frames"/> real frames through the host's clock.</summary>
    private static void Step(HostDriver host, int frames, InputAction actions = InputAction.None)
    {
        double period = 1.0 / Math.Max(1.0, host.Pacer.EffectiveRate);

        // Whatever the clock already owed first, then one frame per period.
        int ran = host.Tick(0.0, actions);
        for (int tick = 0; tick < frames && ran < frames; tick++)
        {
            ran += host.Tick(period, actions);
        }

        Assert.Equal(frames, ran);
    }

    /// <summary>Runs real frames until the core's frame counter reaches <paramref name="target"/>.</summary>
    private static void StepTo(HostDriver host, long target)
    {
        long now = host.Session!.Core.FrameCount;
        Assert.True(now <= target, $"the core is already at frame {now}.");
        Step(host, (int)(target - now));
        Assert.Equal(target, host.Session!.Core.FrameCount);
    }

    /// <summary>Launches the "2D Mode" entry on a ROM and settles the clock.</summary>
    private static HostDriver Open(int runAhead, byte[] rom)
    {
        HostDriver host = HostDriver.Create();
        host.Settings.RunAheadFrames = runAhead;
        host.Launch2DMode(rom);
        Drain(host);
        return host;
    }

    /// <summary>
    /// Reads every sample the core holds and gives their number back. <c>ReadAudio</c> is all
    /// or nothing (REQ-CORE-071), so the queue is emptied in chunks of one sample.
    /// </summary>
    private static int DrainAudio(NesCore core)
    {
        float[] one = new float[1];
        int read = 0;
        while (core.ReadAudio(one) > 0)
        {
            read++;
        }

        return read;
    }

    /// <summary>The byte the whole image of the last displayed frame carries.</summary>
    private static int Displayed(NesCore core)
    {
        ReadOnlySpan<byte> pixels = core.PixelFrame;
        byte first = pixels[0];
        Assert.Equal(first, pixels[(120 * 256) + 128]);
        Assert.Equal(first, pixels[^1]);
        return first;
    }

    /// <summary>The images and the 2 KiB of RAM after each of <paramref name="frames"/> frames.</summary>
    private static (List<byte[]> Images, List<byte[]> Ram) Replay(NesCore core, int frames)
    {
        List<byte[]> images = [];
        List<byte[]> ram = [];
        for (int frame = 0; frame < frames; frame++)
        {
            // A changing input sequence, so the replay is not trivially identical.
            core.RunFrame((ushort)((frame % 3) == 0 ? 0x01 : 0x00));
            images.Add(core.PixelFrame.ToArray());

            byte[] bytes = new byte[0x800];
            for (int address = 0; address < bytes.Length; address++)
            {
                bytes[address] = core.ReadMemory((ushort)address);
            }

            ram.Add(bytes);
        }

        return (images, ram);
    }

    /// <summary>
    /// The real core state after 90 real frames at <paramref name="runAhead"/>: the 2 KiB of
    /// RAM, the frame counter and the whole in-memory state of REQ-CORE-121.
    /// </summary>
    private static byte[] CoreTrace(int runAhead)
    {
        using HostDriver host = Open(runAhead, R4RunAheadRom.Counter());
        StepTo(host, 10);

        for (int frame = 0; frame < 90; frame++)
        {
            Step(host, 1, (frame % 4) == 0 ? InputAction.A : InputAction.None);
        }

        NesCore core = host.Session!.Core;
        CoreState state = new();
        core.CaptureState(state);
        return state.AsSpan().ToArray();
    }

    /// <summary>
    /// Plays the sprite 0 ROM for 60 real frames at <paramref name="runAhead"/> and gives back
    /// the whole state of the core and the delay the ROM measured to the sprite 0 hit.
    /// </summary>
    private static (byte[] State, int Hit) Sprite0Run(int runAhead)
    {
        using HostDriver host = Open(runAhead, R4RunAheadRom.Sprite0());
        StepTo(host, 60);

        NesCore core = host.Session!.Core;
        CoreState state = new();
        core.CaptureState(state);
        int hit = core.ReadMemory(0x32) | (core.ReadMemory(0x33) << 8);
        return (state.AsSpan().ToArray(), hit);
    }

    /// <summary>The displayed byte of each of 10 real frames at <paramref name="runAhead"/>.</summary>
    private static int[] DisplayedTrace(int runAhead)
    {
        using HostDriver host = Open(runAhead, R4RunAheadRom.Counter());
        Step(host, 8);

        int[] trace = new int[10];
        for (int frame = 0; frame < trace.Length; frame++)
        {
            Step(host, 1);
            trace[frame] = Displayed(host.Session!.Core) & 0x1F;
        }

        return trace;
    }

    /// <summary>
    /// Plays the Zapper ROM for 30 real frames with the pointer at the centre of the render
    /// target, and gives back what the ROM counted and where the aim landed.
    /// </summary>
    private static (int Light, int Position) ZapperRun(int runAhead)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "RunAhead-zapper";
        profile.Scripts.Add(new ProfileScript(
            "verify",
            true,
            "function Start() Setting:SetInputType(0, 1) end",
            "*"));

        using HostDriver host = HostDriver.Create(profile);
        host.Settings.RunAheadFrames = runAhead;

        string rom = host.WriteRom(R4RunAheadRom.Zapper());
        host.Launch(host.EntryIdByTitle("RunAhead-zapper"), rom);
        Drain(host);
        Assert.True(host.ZapperActive);

        InputSnapshot snapshot = new(
            InputAction.None,
            InputAction.None,
            new Vector2(320f, 240f),
            new Vector2(640f, 480f),
            LeftButton: false,
            RightButton: false);

        double period = 1.0 / Math.Max(1.0, host.Pacer.EffectiveRate);
        int ran = 0;
        for (int tick = 0; tick < 128 && ran < 10; tick++)
        {
            ran += host.Tick(period, snapshot);
        }

        Assert.Equal(10, ran);

        NesCore core = host.Session!.Core;
        int light = core.ReadMemory(0x20) | (core.ReadMemory(0x21) << 8);
        ZapperPosition aim = host.Zapper.Position;
        int position = aim.X is >= 0 and < 256 && aim.Y is >= 0 and < 240
            ? (aim.Y * 256) + aim.X
            : -1;
        return (light, position);
    }

    /// <summary>A profile whose script counts its frame hooks and reads the core's memory.</summary>
    private static GameProfile HookProfile()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "RunAhead-probe";
        profile.Scripts.Add(new ProfileScript(
            "verify",
            true,
            """
            updates = 0
            lates = 0
            lastframe = -1
            seendisplayed = -1
            seencounter = -1
            function Update()
              updates = updates + 1
              lastframe = frame
              seendisplayed = Script:ReadMem(0x12)
              seencounter = Script:ReadMem(0x10)
            end
            function LateUpdate()
              lates = lates + 1
            end
            """,
            "*"));
        return profile;
    }

    /// <summary>A string of the interface, in one of the two languages of AMD-035.</summary>
    private static string Ui(string key, bool english)
    {
        IReadOnlyDictionary<string, string> table = UiTable(english);
        return table.TryGetValue(key, out string? text) ? text : key;
    }

    /// <summary>
    /// The label an interface shows for the domain value <paramref name="label"/> of
    /// <paramref name="settingKey"/>: the entry of this setting when there is one, the shared
    /// entry otherwise, the raw domain label as a last resort.
    /// </summary>
    private static string UiOption(string settingKey, string label, bool english)
    {
        IReadOnlyDictionary<string, string> table = UiTable(english);
        if (table.TryGetValue("option." + settingKey + "." + label, out string? scoped))
        {
            return scoped;
        }

        return table.TryGetValue("option." + label, out string? shared) ? shared : label;
    }

    /// <summary>The string table of one language, read off <c>UiStrings</c> by reflection.</summary>
    private static IReadOnlyDictionary<string, string> UiTable(bool english)
    {
        Type strings = typeof(AppOptions).Assembly.GetType("Zax.App.Ui.UiStrings", true)!;
        Type language = typeof(AppOptions).Assembly.GetType("Zax.App.Ui.UiLanguage", true)!;
        object value = Enum.Parse(language, english ? "English" : "French");
        return (IReadOnlyDictionary<string, string>)strings
            .GetMethod("TableOf", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)!
            .Invoke(null, [value])!;
    }
}

/// <summary>
/// The two test ROMs the verifier builds from the amendment text: the counter ROM of
/// REQ-LOOP-065 and a Zapper ROM for REQ-LOOP-068. Both are NROM images of 32 KiB of PRG and
/// 8 KiB of CHR, and both leave rendering off, so the whole image is the universal backdrop
/// colour and one byte describes it entirely.
/// </summary>
public static class R4RunAheadRom
{
    /// <summary>
    /// The ROM of REQ-LOOP-065: its image carries, in the five low bits, the value its counter
    /// had at the <b>beginning</b> of the frame, and in bit 5 the state of button A as read
    /// during that frame.
    /// </summary>
    /// <remarks>
    /// The core stops an emulated frame at the first dot of the vertical blanking, so the NMI
    /// the blanking raises is serviced at the very start of the next emulated frame — before its
    /// 240 visible lines. The handler therefore writes the backdrop of the frame that is about
    /// to be drawn, and only then increments the counter: the convention the criterion of
    /// REQ-LOOP-065 needs.
    /// </remarks>
    public static byte[] Counter()
    {
        List<byte> reset =
        [
            0x78,                   // SEI
            0xD8,                   // CLD
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x01, 0x20,       // STA $2001      rendering off
            0x85, 0x10,             // STA $10        counter
            0x85, 0x11,             // STA $11        button A, shifted
            0x85, 0x12,             // STA $12        byte put on the image
            0xA9, 0x80,             // LDA #$80
            0x8D, 0x00, 0x20,       // STA $2000      NMI on
        ];

        int self = 0x8000 + reset.Count;
        reset.AddRange([0x4C, (byte)self, (byte)(self >> 8)]);

        byte[] nmi =
        [
            0xA9, 0x01,             // LDA #$01
            0x8D, 0x16, 0x40,       // STA $4016      strobe high
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x16, 0x40,       // STA $4016      strobe low
            0xAD, 0x16, 0x40,       // LDA $4016      first read: button A in bit 0
            0x29, 0x01,             // AND #$01
            0x0A, 0x0A, 0x0A, 0x0A, 0x0A,   // ASL A x5   -> bit 5
            0x85, 0x11,             // STA $11
            0xA5, 0x10,             // LDA $10
            0x29, 0x1F,             // AND #$1F
            0x05, 0x11,             // ORA $11
            0x85, 0x12,             // STA $12
            0xAD, 0x02, 0x20,       // LDA $2002      reset the address latch
            0xA9, 0x3F,             // LDA #$3F
            0x8D, 0x06, 0x20,       // STA $2006
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x06, 0x20,       // STA $2006      VRAM address = $3F00
            0xA5, 0x12,             // LDA $12
            0x8D, 0x07, 0x20,       // STA $2007      universal backdrop = the byte
            0xE6, 0x10,             // INC $10
            0x40,                   // RTI
        ];

        return Build([.. reset], nmi);
    }

    /// <summary>
    /// The ROM of REQ-LOOP-068: a white screen, and a 16-bit counter at <c>$20</c> that grows
    /// by one at every read of <c>$4017</c> whose bit 3 is clear, that is at every read where
    /// the Zapper sees light.
    /// </summary>
    public static byte[] Zapper()
    {
        List<byte> program =
        [
            0x78,                   // SEI
            0xD8,                   // CLD
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x01, 0x20,       // STA $2001      rendering off
            0x85, 0x20,             // STA $20
            0x85, 0x21,             // STA $21
            0xAD, 0x02, 0x20,       // LDA $2002
            0xA9, 0x3F,             // LDA #$3F
            0x8D, 0x06, 0x20,       // STA $2006
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x06, 0x20,       // STA $2006      VRAM address = $3F00
            0xA9, 0x30,             // LDA #$30       white
            0x8D, 0x07, 0x20,       // STA $2007
        ];

        int loop = 0x8000 + program.Count;
        program.AddRange(
        [
            0xAD, 0x17, 0x40,       // LDA $4017      the Zapper port
            0x29, 0x08,             // AND #$08       bit 3: 0 when light is seen
            0xD0, 0x06,             // BNE skip
            0xE6, 0x20,             // INC $20
            0xD0, 0x02,             // BNE skip
            0xE6, 0x21,             // INC $21
            0x4C, (byte)loop, (byte)(loop >> 8),     // skip: JMP loop
        ]);

        return Build([.. program]);
    }

    /// <summary>
    /// The ROM of the second sentence of REQ-LOOP-068: background and sprites on, sprite 0 over
    /// an opaque background, and a 16-bit figure at <c>$32</c> giving the number of loop
    /// iterations between the start of the vertical blanking and the sprite 0 hit of the frame
    /// that follows. Nothing of it depends on the pixel image, only on the flag and on the time.
    /// </summary>
    public static byte[] Sprite0()
    {
        List<byte> program =
        [
            0x78,                   // SEI
            0xD8,                   // CLD
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x00, 0x20,       // STA $2000      no NMI, both tables at $0000
            0x8D, 0x01, 0x20,       // STA $2001      rendering off while VRAM is written

            0xAD, 0x02, 0x20,       // LDA $2002      reset the address latch
            0xA9, 0x3F,             // LDA #$3F
            0x8D, 0x06, 0x20,       // STA $2006
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x06, 0x20,       // STA $2006      VRAM address = $3F00
            0xA9, 0x0F,             // LDA #$0F       backdrop: black
            0x8D, 0x07, 0x20,       // STA $2007
            0xA9, 0x30,             // LDA #$30       entry 1: white
            0x8D, 0x07, 0x20,       // STA $2007

            0xAD, 0x02, 0x20,       // LDA $2002
            0xA9, 0x20,             // LDA #$20
            0x8D, 0x06, 0x20,       // STA $2006
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x06, 0x20,       // STA $2006      VRAM address = $2000
            0xA2, 0x04,             // LDX #$04
            0xA0, 0x00,             // LDY #$00
            0xA9, 0x01,             // LDA #$01       tile 1, opaque everywhere
            0x8D, 0x07, 0x20,       // fill: STA $2007
            0x88,                   //       DEY
            0xD0, 0xFA,             //       BNE fill
            0xCA,                   //       DEX
            0xD0, 0xF7,             //       BNE fill

            0xA9, 0x00,             // LDA #$00
            0x8D, 0x03, 0x20,       // STA $2003      OAM address = 0
            0xA9, 0x3C,             // LDA #$3C
            0x8D, 0x04, 0x20,       // STA $2004      sprite 0: Y = 60
            0xA9, 0x01,             // LDA #$01
            0x8D, 0x04, 0x20,       // STA $2004      tile 1
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x04, 0x20,       // STA $2004      attributes: in front, palette 0
            0xA9, 0x64,             // LDA #$64
            0x8D, 0x04, 0x20,       // STA $2004      X = 100

            0xA9, 0x1E,             // LDA #$1E
            0x8D, 0x01, 0x20,       // STA $2001      background and sprites on
        ];

        int main = 0x8000 + program.Count;
        program.AddRange(
        [
            0xAD, 0x02, 0x20,       // main: LDA $2002      wait for the blanking flag
            0x10, 0xFB,             //       BPL main
            0xA9, 0x00,             //       LDA #$00
            0x85, 0x30,             //       STA $30
            0x85, 0x31,             //       STA $31
            0xE6, 0x30,             // wait: INC $30
            0xD0, 0x02,             //       BNE poll
            0xE6, 0x31,             //       INC $31
            0xAD, 0x02, 0x20,       // poll: LDA $2002
            0x29, 0x40,             //       AND #$40       sprite 0 hit
            0xF0, 0xF3,             //       BEQ wait
            0xA5, 0x30,             //       LDA $30
            0x85, 0x32,             //       STA $32
            0xA5, 0x31,             //       LDA $31
            0x85, 0x33,             //       STA $33
            0x4C, (byte)main, (byte)(main >> 8),     // JMP main
        ]);

        return Build([.. program], nmi: null, opaqueTileOne: true);
    }

    private static byte[] Build(byte[] reset, byte[]? nmi = null, bool opaqueTileOne = false)
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2;
        image[5] = 1;
        image[6] = 0x01;

        const int prg = 16;
        reset.CopyTo(image, prg);

        int nmiVector = 0x8000;
        if (nmi is not null)
        {
            nmi.CopyTo(image, prg + 0x1000);
            nmiVector = 0x9000;
        }

        Vector(image, 0xFFFA, nmiVector);
        Vector(image, 0xFFFC, 0x8000);
        Vector(image, 0xFFFE, 0x8000);

        if (opaqueTileOne)
        {
            // Pattern 1 of table $0000: plane 0 all ones, plane 1 all zero, so every one of its
            // 64 dots has the colour 1 of its palette — never transparent.
            int chr = 16 + (32 * 1024);
            for (int row = 0; row < 8; row++)
            {
                image[chr + 16 + row] = 0xFF;
            }
        }

        return image;
    }

    private static void Vector(byte[] image, int address, int target)
    {
        int at = 16 + (address - 0x8000);
        image[at] = (byte)target;
        image[at + 1] = (byte)(target >> 8);
    }
}
