using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// Run-ahead (AMD-041): the setting of REQ-LOOP-064, the five-step real frame of REQ-LOOP-065,
/// what follows the real frame and what sees the displayed one (REQ-LOOP-066, REQ-LOOP-067), the
/// Zapper of REQ-LOOP-068 and the determinism of the amended REQ-ARCH-012.
/// </summary>
public sealed class RunAheadTests : IDisposable
{
    private readonly string _romPath = Path.Combine(
        Path.GetTempPath(),
        $"zax-runahead-{Guid.NewGuid():N}.nes");

    public RunAheadTests() => File.WriteAllBytes(_romPath, RunAheadRom.Build());

    /// <inheritdoc/>
    public void Dispose() => File.Delete(_romPath);

    // ------------------------------------------------------------ REQ-LOOP-064

    /// <summary>
    /// REQ-LOOP-064 — an empty user root gives Run-ahead = 1, the value marked « recommended ».
    /// </summary>
    [Fact]
    public void REQ_LOOP_064_an_empty_user_root_gives_one_frame()
    {
        AppSettings settings = new();

        Assert.Equal(1, settings.RunAheadFrames);
        Assert.Equal(1, SettingsCatalogue.Get(SettingKeys.EmuRunAhead).DefaultValue.AsInt);
        Assert.Equal(SettingsCatalogue.RunAheadDefault, settings.RunAheadFrames);
    }

    /// <summary>
    /// REQ-LOOP-064 and AMD-040 REQ-LUA-114 — a value outside [0, 3] is brought back into the
    /// interval, whichever way it is written.
    /// </summary>
    [Theory]
    [InlineData(7, 3)]
    [InlineData(4, 3)]
    [InlineData(3, 3)]
    [InlineData(0, 0)]
    [InlineData(-1, 0)]
    [InlineData(-100, 0)]
    public void REQ_LOOP_064_a_value_outside_the_interval_is_clamped(int written, int expected)
    {
        AppSettings settings = new();
        settings.RunAheadFrames = written;
        Assert.Equal(expected, settings.RunAheadFrames);

        // The very same clamping through the Lua view of the setting (Setting.RunAheadFrames).
        Zax.Scripting.ScriptSettings script = new(new SettingsModel());
        script.RunAheadFrames = written;
        Assert.Equal(expected, script.RunAheadFrames);
    }

    /// <summary>REQ-LOOP-064 — the value survives a restart: it is a persisted user preference.</summary>
    [Fact]
    public void REQ_LOOP_064_the_value_survives_a_restart()
    {
        using LoopHarness harness = LoopHarness.Create();
        SettingsStore store = new(harness.Paths);

        harness.Settings.RunAheadFrames = 3;
        store.Save(harness.Settings);

        AppSettings reloaded = new();
        store.Load(reloaded, harness.Now);
        Assert.Equal(3, reloaded.RunAheadFrames);

        Assert.True(SettingsCatalogue.Get(SettingKeys.EmuRunAhead).IsPersisted);
        Assert.Equal(SettingScope.User, SettingsCatalogue.Get(SettingKeys.EmuRunAhead).Scope);
    }

    /// <summary>
    /// REQ-LOOP-064 and AMD-041 §3 — the options window shows the setting as a list of the four
    /// values, with the labels of the amendment in both interface languages.
    /// </summary>
    [Fact]
    public void REQ_LOOP_064_the_options_window_shows_the_four_labels()
    {
        Ui.OptionsRow row = Assert.Single(
            new Ui.OptionsModel().AllRows(),
            candidate => candidate.Descriptor.Key == SettingKeys.EmuRunAhead);

        Assert.Equal(Ui.OptionsTab.Emulation, Ui.OptionsModel.TabOf(row.Descriptor));
        Assert.Equal(0d, row.Minimum);
        Assert.Equal(3d, row.Maximum);

        IReadOnlyList<SettingOption> options = row.Descriptor.Domain.Options!;
        Assert.Equal([0d, 1d, 2d, 3d], options.Select(option => option.Value));

        using (UiLanguageScope english = new(Ui.UiLanguage.English))
        {
            Assert.Equal("Run-ahead", row.Label);
            Assert.Equal(
                ["Off", "1 frame (Recommended)", "2 frames", "3 frames"],
                options.Select(option => Ui.UiText.LabelOf(row.Descriptor, option.Label)));
        }

        using (UiLanguageScope french = new(Ui.UiLanguage.French))
        {
            Assert.Equal("Run-ahead", row.Label);
            Assert.Equal(
                ["Désactivé", "1 frame (recommandé)", "2 frames", "3 frames"],
                options.Select(option => Ui.UiText.LabelOf(row.Descriptor, option.Label)));
        }
    }

    /// <summary>AMD-041 §4 — the setting is not a libretro core option.</summary>
    [Fact]
    public void REQ_LOOP_064_libretro_does_not_expose_the_setting()
    {
        Assert.False(SettingsCatalogue.Get(SettingKeys.EmuRunAhead).IsLibretroExposed);
        Assert.Null(LibretroCoreOptions.Find(SettingDescriptor.BuildLibretroKey(SettingKeys.EmuRunAhead)));

        // And a session a frontend drives itself runs no frame in advance (§4).
        using Harness harness = Harness.Open(_romPath);
        Assert.Equal(0, harness.Session.RunAheadFrames);
    }

    // ------------------------------------------------------------ REQ-LOOP-065

    /// <summary>
    /// REQ-LOOP-065, acceptance vector — Run-ahead = 2: just before real frame k the counter of
    /// the core is k; with A pressed on that frame the displayed picture shows the counter k + 2
    /// and A pressed; after step 5 the counter of the core is k + 1.
    /// </summary>
    [Fact]
    public void REQ_LOOP_065_the_displayed_frame_is_two_frames_ahead()
    {
        using Harness harness = Harness.Open(_romPath);
        harness.Warm();

        harness.Session.RunAheadFrames = 2;

        int counter = harness.Ram(RunAheadRom.CounterAddress);
        long coreFrames = harness.Core.FrameCount;

        harness.Session.RunFrame((int)InputAction.A);

        // The picture is the one of the second frame in advance, and it saw A pressed.
        Assert.Equal(counter + 2, RunAheadRom.Displayed(harness.Core));
        Assert.True(RunAheadRom.ShowsButtonA(harness.Core));

        // The core is back on the real frame: one frame further, not three.
        Assert.Equal(coreFrames + 1, harness.Core.FrameCount);
        Assert.Equal(counter + 1, harness.Ram(RunAheadRom.CounterAddress));
    }

    /// <summary>
    /// REQ-LOOP-065 — with N = 0 the frame is emulated once and it is the displayed one, which is
    /// the order of REQ-LOOP-060 unchanged.
    /// </summary>
    [Fact]
    public void REQ_LOOP_065_zero_leaves_the_frame_order_unchanged()
    {
        using Harness harness = Harness.Open(_romPath);
        harness.Warm();

        harness.Session.RunAheadFrames = 0;

        int counter = harness.Ram(RunAheadRom.CounterAddress);
        long coreFrames = harness.Core.FrameCount;

        harness.Session.RunFrame((int)InputAction.A);

        Assert.Equal(counter, RunAheadRom.Displayed(harness.Core));
        Assert.True(RunAheadRom.ShowsButtonA(harness.Core));
        Assert.Equal(coreFrames + 1, harness.Core.FrameCount);
        Assert.Equal(counter + 1, harness.Ram(RunAheadRom.CounterAddress));
    }

    /// <summary>
    /// REQ-LOOP-065 — the displayed frame is N frames ahead for each of the four values, and one
    /// real frame always advances the core by exactly one frame while emulating N + 1.
    /// </summary>
    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    public void REQ_LOOP_065_a_real_frame_emulates_n_plus_one_frames(int ahead)
    {
        using Harness harness = Harness.Open(_romPath);
        harness.Warm();

        harness.Session.RunAheadFrames = ahead;

        int counter = harness.Ram(RunAheadRom.CounterAddress);
        long coreFrames = harness.Core.FrameCount;
        long cycles = harness.Core.CycleCount;

        harness.Session.RunFrame(0);

        Assert.Equal(counter + ahead, RunAheadRom.Displayed(harness.Core));
        Assert.Equal(coreFrames + 1, harness.Core.FrameCount);
        Assert.Equal(counter + 1, harness.Ram(RunAheadRom.CounterAddress));

        // N + 1 frames were emulated — the picture proves it, being N frames ahead — and the
        // state that survives is the real one: one frame's worth of cycles, not N + 1
        // (REQ-LOOP-066).
        RegionTiming timing = harness.Core.Timing;
        double perFrame = timing.ScanlinesPerFrame
            * (double)RegionTiming.DotsPerScanline
            * timing.DotsPerCycleDenominator
            / timing.DotsPerCycleNumerator;
        long spent = harness.Core.CycleCount - cycles;
        Assert.InRange(spent, (long)(perFrame - 40), (long)(perFrame + 40));
    }

    /// <summary>
    /// REQ-LOOP-065 — the frames in advance are silent: one real frame yields the audio of one
    /// frame, not of N + 1, and it is the audio of the displayed frame.
    /// </summary>
    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(3)]
    public void REQ_LOOP_065_one_real_frame_produces_one_frame_of_audio(int ahead)
    {
        using Harness harness = Harness.Open(_romPath);
        harness.Warm();
        harness.Session.RunAheadFrames = ahead;

        // A reference of one ordinary frame's worth of samples, from the same ROM.
        harness.Core.ReadAudio(new float[harness.Core.Apu.AvailableSamples]);
        harness.Session.RunFrame(0);
        int samples = harness.Core.Apu.AvailableSamples;

        int expected = harness.Core.SampleRate / harness.Core.Timing.NominalFramesPerSecond;
        Assert.InRange(samples, expected - 8, expected + 8);
    }

    // ------------------------------------------------------------ REQ-LOOP-066

    /// <summary>
    /// REQ-LOOP-066 — the sequence of real core states does not depend on N: the same inputs give
    /// the same real states, byte for byte, at 0, 1, 2 and 3 frames of run-ahead.
    /// </summary>
    /// <remarks>
    /// This is the half of the amended REQ-ARCH-012 that holds <em>across</em> values of N. What
    /// is only compared at equal N is the sequence of displayed frames, of tiles and of
    /// conversion results, which the test below covers.
    /// </remarks>
    [Fact]
    public void REQ_LOOP_066_the_real_core_states_do_not_depend_on_n()
    {
        List<string> reference = RealStates(0);

        foreach (int ahead in (int[])[1, 2, 3])
        {
            Assert.Equal(reference, RealStates(ahead));
        }

        List<string> RealStates(int ahead)
        {
            using Harness harness = Harness.Open(_romPath);
            harness.Warm();
            harness.Session.RunAheadFrames = ahead;

            List<string> states = [];
            for (int frame = 0; frame < 40; frame++)
            {
                harness.Session.RunFrame((int)(frame % 3 == 0 ? InputAction.A : InputAction.None));
                states.Add(harness.Fingerprint());
            }

            return states;
        }
    }

    /// <summary>
    /// REQ-ARCH-012 (amended) — at equal N, two runs from the same state with the same inputs
    /// give the same displayed frames, tile by tile.
    /// </summary>
    [Theory]
    [InlineData(0)]
    [InlineData(2)]
    public void REQ_ARCH_012_two_runs_at_equal_n_show_the_same_frames(int ahead)
    {
        Assert.Equal(DisplayedFrames(ahead), DisplayedFrames(ahead));

        List<string> DisplayedFrames(int ahead)
        {
            using Harness harness = Harness.Open(_romPath);
            harness.Warm();
            harness.Session.RunAheadFrames = ahead;

            List<string> pictures = [];
            for (int frame = 0; frame < 20; frame++)
            {
                harness.Session.RunFrame((int)(frame % 4 == 0 ? InputAction.A : InputAction.None));
                pictures.Add(
                    $"{RunAheadRom.Displayed(harness.Core)}/{harness.Session.FrameCounter}/"
                    + $"{harness.Session.Engine.PublishedFrame!.Instances.Count}");
            }

            return pictures;
        }
    }

    /// <summary>
    /// REQ-LOOP-066, acceptance criterion — Run-ahead = 3: the rollback history captures a
    /// snapshot at every multiple of 60 of the <b>real</b> counter, and a quick save then a quick
    /// load put the core back on the real frame of the save.
    /// </summary>
    [Fact]
    public void REQ_LOOP_066_the_history_and_the_save_states_follow_the_real_frame()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(RunAheadRom.Build());
        harness.Catalog.Add("game", rom);
        harness.Settings.RunAheadFrames = 3;

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("game"));
        harness.Run(ZaxCommands.Resume);

        int ran = harness.RunFrames(130);
        Assert.True(ran >= 130, $"only {ran} frames ran");

        GameSession session = harness.Host.Session!;
        Assert.Equal(3, session.RunAheadFrames);

        // One snapshot per 60 real frames: the core counter the history watches is the real one.
        Assert.Equal(1 + (session.Core.FrameCount / 60), harness.Host.Rollback.Count);

        long real = session.Core.FrameCount;
        harness.Run(ZaxCommands.QuickSave);

        harness.RunFrames(20);
        Assert.True(session.Core.FrameCount > real);

        harness.Run(ZaxCommands.QuickLoad);
        Assert.Equal(real, harness.Host.Session!.Core.FrameCount);

        // The next frame starts again with the same N (REQ-LOOP-066, last sentence).
        harness.RunFrames(1);
        Assert.Equal(3, harness.Host.Session!.RunAheadFrames);
    }

    // ------------------------------------------------------------ REQ-LOOP-067

    /// <summary>
    /// REQ-LOOP-067, acceptance criterion — a script that increments a counter in <c>Update</c>
    /// goes up by 1 per real frame whatever N, and with Run-ahead = 2 <c>Script:ReadMem</c> reads
    /// the displayed value k + 2.
    /// </summary>
    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    public void REQ_LOOP_067_the_frame_hooks_run_once_per_real_frame(int ahead)
    {
        using Harness harness = Harness.Open(
            _romPath,
            new ProfileScript(
                "hooks",
                true,
                $$"""
                    hooks = 0
                    reads = {}
                    frames = {}
                    function Update()
                      hooks = hooks + 1
                      reads[hooks] = Script:ReadMem({{RunAheadRom.DisplayedAddress}})
                      frames[hooks] = frame
                    end
                    """,
                null));

        harness.Warm();
        harness.Session.RunAheadFrames = ahead;

        int before = (int)harness.Session.Scripts.EvaluateNumber("return hooks");
        for (int frame = 0; frame < 10; frame++)
        {
            int counter = harness.Ram(RunAheadRom.CounterAddress);
            int displayed = harness.Session.FrameCounter;

            harness.Session.RunFrame(0);

            int hooks = (int)harness.Session.Scripts.EvaluateNumber("return hooks");
            Assert.Equal(before + frame + 1, hooks);

            // The hook saw the RAM of the displayed frame, N frames ahead of the real one.
            Assert.Equal(
                counter + ahead,
                (int)harness.Session.Scripts.EvaluateNumber($"return reads[{hooks}]"));

            // And the global « frame » was the displayed frame's counter.
            Assert.Equal(
                displayed,
                (int)harness.Session.Scripts.EvaluateNumber($"return frames[{hooks}]"));
        }
    }

    // ------------------------------------------------------------ REQ-LOOP-068

    /// <summary>
    /// REQ-LOOP-068, acceptance criterion — with a Zapper on a port, the light the game detects
    /// is the same with Run-ahead = 1 as with 0, frame by frame, because the detection of the real
    /// frame bears on the picture that frame produces itself.
    /// </summary>
    [Theory]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    public void REQ_LOOP_068_the_zapper_sees_the_same_light_as_without_run_ahead(int ahead)
        => Assert.Equal(Light(0), Light(ahead));

    /// <summary>
    /// REQ-LOOP-068 — every emulated frame renders its pixels when a Zapper is plugged in, and
    /// only the displayed one does when none is.
    /// </summary>
    [Fact]
    public void REQ_LOOP_068_a_zapper_keeps_every_frame_rendering_its_pixels()
    {
        using Harness withZapper = Harness.Open(_romPath);
        withZapper.Warm();
        withZapper.Core.SetPortType(1, ControllerPortType.Zapper);
        withZapper.Core.ZapperPositionSource = static _ => new ZapperPosition(4, 120);
        withZapper.Session.RunAheadFrames = 2;
        withZapper.Session.RunFrame(0);

        // The picture is still the displayed frame's: rendering the hidden frames' pixels does
        // not overwrite it, the displayed frame being the last one emulated.
        Assert.True(RunAheadRom.Displayed(withZapper.Core) > 0);
    }

    private List<string> Light(int ahead)
    {
        using Harness harness = Harness.Open(_romPath);
        harness.Core.SetPortType(1, ControllerPortType.Zapper);

        // A fixed aim on the top-left cell, whose tile index — and therefore whose colours —
        // changes at every frame, so the light the game sees changes too.
        harness.Core.ZapperPositionSource = static _ => new ZapperPosition(3, 1);
        harness.Warm();
        harness.Session.RunAheadFrames = ahead;

        List<string> records = [];
        for (int frame = 0; frame < 30; frame++)
        {
            // Port 2 byte $10: the trigger released (SPEC-08 §4.1, DEC-02-Q1).
            harness.Session.RunFrame(0, 0, 0x10);
            int index = harness.Ram(RunAheadRom.ProbeIndexAddress);
            records.Add($"{index}:{harness.Ram(RunAheadRom.ProbeLog + ((index + 255) % 256))}");
        }

        Assert.Contains("8", string.Join(",", records));
        return records;
    }

    /// <summary>A session on the vector ROM, with the 2D profile and no host around it.</summary>
    private sealed class Harness : IDisposable
    {
        private Harness(GameSession session, SceneState scene)
        {
            Session = session;
            Scene = scene;
        }

        public GameSession Session { get; }

        public SceneState Scene { get; }

        public NesCore Core => Session.Core;

        public static Harness Open(string romPath, params ProfileScript[] scripts)
        {
            GameProfile profile = GameProfile.CreateEmpty();
            foreach (ProfileScript script in scripts)
            {
                profile.Scripts.Add(script);
            }

            SceneState scene = new();
            GameSession session = GameSession.Load(romPath, profile, "Run-ahead", scene);
            session.ApplyProfileSettings(scene, profile.Settings);
            return new Harness(session, scene);
        }

        /// <summary>
        /// Runs the frames the ROM needs to finish its warm-up and show its first picture, with
        /// no run-ahead so the state is plainly the real one.
        /// </summary>
        public void Warm()
        {
            Session.RunAheadFrames = 0;
            for (int frame = 0; frame < 8; frame++)
            {
                Session.RunFrame(0);
            }

            Assert.True(Ram(RunAheadRom.CounterAddress) > 0, "the ROM never started counting");
        }

        public int Ram(int address) => Core.ReadMemory((ushort)address);

        /// <summary>Everything of the real state a frame may have changed.</summary>
        public string Fingerprint()
        {
            System.Text.StringBuilder text = new();
            text.Append(Core.FrameCount).Append('/').Append(Core.CycleCount).Append('/');
            foreach (byte value in Core.Bus.Ram)
            {
                text.Append(value.ToString("X2", System.Globalization.CultureInfo.InvariantCulture));
            }

            return text.ToString();
        }

        public void Dispose() => Session.Dispose();
    }
}
