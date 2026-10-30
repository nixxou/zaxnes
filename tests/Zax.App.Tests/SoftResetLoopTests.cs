using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Ui;
using Zax.Profiles;

namespace Zax.App.Tests;

/// <summary>
/// The frontend side of the soft reset of AMD-044: the command of REQ-LOOP-072, its place in the
/// game menu, and the extended input action of REQ-INP-125.
/// </summary>
public sealed class SoftResetLoopTests
{
    /// <summary>
    /// A profile whose <c>Start</c> and <c>End</c> hooks count themselves and whose
    /// <c>Update</c> hook writes the frame number into <c>PData[1]</c>.
    /// </summary>
    private static GameProfile ScriptedProfile()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Scripts.Add(new ProfileScript("!prelude", true, "starts = 0; ends = 0", string.Empty));
        profile.Scripts.Add(new ProfileScript(
            "counter",
            true,
            """
            function Start() starts = starts + 1 end
            function End() ends = ends + 1 end
            function Update() SetPData(1, frame) end
            """,
            "*"));
        return profile;
    }

    private static LoopHarness LaunchScripted(out GameProfile profile)
    {
        profile = ScriptedProfile();
        LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter(), "soft.nes");
        harness.Catalog.Add("soft", rom, profile);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("soft"));
        Assert.Equal(EmulationState.Running, harness.Host.State);
        return harness;
    }

    /// <summary>
    /// AMD-044 REQ-LOOP-072 acceptance: in game, a soft reset empties the rollback history,
    /// leaves <c>PData</c> alone and calls neither <c>End</c> nor <c>Start</c>.
    /// </summary>
    [Fact]
    public void REQ_LOOP_072_a_soft_reset_keeps_the_profile_and_empties_the_history()
    {
        using LoopHarness harness = LaunchScripted(out _);

        harness.RunFrames(300);
        Assert.True(harness.Host.Rollback.Count > 0);
        Assert.Equal(299.0, harness.Host.Session!.Scripts.ReadPersistentData(2), 0);
        Assert.Equal(1.0, harness.Host.Session!.Scripts.EvaluateNumber("return starts"));

        harness.Run(ZaxCommands.SoftReset);

        // The history is emptied (REQ-LOOP-031) and the core restarts from its reset vector.
        Assert.Equal(0, harness.Host.Rollback.Count);
        Assert.Equal(0, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // PData is untouched and no hook ran: the profile was not relaunched.
        Assert.Equal(299.0, harness.Host.Session!.Scripts.ReadPersistentData(2), 0);
        Assert.Equal(1.0, harness.Host.Session!.Scripts.EvaluateNumber("return starts"));
        Assert.Equal(0.0, harness.Host.Session!.Scripts.EvaluateNumber("return ends"));
    }

    /// <summary>
    /// AMD-044 REQ-LOOP-072: the soft reset keeps the RAM, where the hard reset of REQ-LOOP-050
    /// relaunches the game and clears it. The ROM counts frames into <c>$0010</c>.
    /// </summary>
    [Fact]
    public void REQ_LOOP_072_a_soft_reset_keeps_the_ram_where_a_hard_reset_clears_it()
    {
        using LoopHarness harness = LaunchScripted(out _);

        harness.RunFrames(120);
        byte counter = harness.Host.Session!.Core.ReadMemory(0x0010);
        Assert.NotEqual(0, counter);

        harness.Run(ZaxCommands.SoftReset);
        Assert.NotEqual(0, harness.Host.Session!.Core.ReadMemory(0x0010));

        harness.Run(ZaxCommands.Reset);
        Assert.Equal(0, harness.Host.Session!.Core.ReadMemory(0x0010));
    }

    /// <summary>
    /// AMD-044 REQ-LOOP-072: the soft reset lifts the pause and resets the pacing clock, as the
    /// hard reset does for the rest of the program.
    /// </summary>
    [Fact]
    public void REQ_LOOP_072_a_soft_reset_lifts_the_pause()
    {
        using LoopHarness harness = LaunchScripted(out _);

        harness.RunFrames(120);
        harness.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        harness.Run(ZaxCommands.SoftReset);

        Assert.Equal(EmulationState.Running, harness.Host.State);
    }

    /// <summary>
    /// AMD-044 REQ-LOOP-072: the learned conversion state survives a soft reset, which is one of
    /// the two things that distinguish it from a relaunch.
    /// </summary>
    [Fact]
    public void REQ_LOOP_072_a_soft_reset_keeps_the_learned_conversion_state()
    {
        using LoopHarness harness = LaunchScripted(out _);
        harness.RunFrames(120);

        object before = harness.Host.Session!.Engine;

        harness.Run(ZaxCommands.SoftReset);

        // The session — and with it the engine holding the learned patterns — is the same
        // instance: nothing was unloaded and nothing was read from disk again.
        Assert.Same(before, harness.Host.Session!.Engine);
    }

    /// <summary>
    /// AMD-044 REQ-LOOP-072: the command does nothing outside a running game, and nothing while
    /// the loading indicator is up.
    /// </summary>
    [Fact]
    public void REQ_LOOP_072_the_command_is_unavailable_without_a_game()
    {
        using LoopHarness harness = LoopHarness.Create();

        ZaxCommand command = harness.Host.Commands.Find(ZaxCommands.SoftReset)!;
        Assert.False(command.CanExecute());

        harness.Run(ZaxCommands.SoftReset);
        Assert.Equal(EmulationState.Stopped, harness.Host.State);

        EmulationStateMachineTests.Launch(harness, "g1");
        Assert.True(harness.Host.Commands.Find(ZaxCommands.SoftReset)!.CanExecute());
    }

    /// <summary>
    /// AMD-044 §3: the two resets sit next to each other in the game menu, with the labels of
    /// the table — <c>Hard Reset</c> / <c>Soft Reset</c> in English, <c>Reset complet</c> /
    /// <c>Reset doux</c> in French.
    /// </summary>
    [Fact]
    public void REQ_LOOP_072_the_game_menu_offers_both_resets_with_the_amd_044_labels()
    {
        using UiLanguageScope scope = new();

        UiStrings.Language = UiLanguage.English;
        Assert.Equal("Hard Reset", UiStrings.Get(UiKeys.Command(ZaxCommands.Reset)));
        Assert.Equal("Soft Reset", UiStrings.Get(UiKeys.Command(ZaxCommands.SoftReset)));

        UiStrings.Language = UiLanguage.French;
        Assert.Equal("Reset complet", UiStrings.Get(UiKeys.Command(ZaxCommands.Reset)));
        Assert.Equal("Reset doux", UiStrings.Get(UiKeys.Command(ZaxCommands.SoftReset)));
    }

    /// <summary>
    /// AMD-044 REQ-INP-125: the action exists in the extended-action table with the next free
    /// code, <c>0x80000</c>, and has no default binding at all.
    /// </summary>
    [Fact]
    public void REQ_INP_125_the_soft_reset_action_has_the_next_free_code_and_no_default_binding()
    {
        Assert.Equal(0x80000, (int)InputAction.SoftReset);
        Assert.Equal("SoftReset", InputActions.NameOf(InputAction.SoftReset));
        Assert.Equal(InputAction.SoftReset, InputActions.Parse("SoftReset"));
        Assert.Contains(InputAction.SoftReset, InputActions.All);

        Assert.DoesNotContain(
            InputAction.SoftReset,
            DefaultBindings.PlayerOneKeyboard.Select(static entry => entry.Action));
        Assert.DoesNotContain(
            InputAction.SoftReset,
            DefaultBindings.StandardGamepadSet.Select(static entry => entry.Action));

        Assert.False(DefaultBindings.CreatePlayerOneKeyboard().Get(InputAction.SoftReset).IsBound);
        Assert.False(DefaultBindings.CreateGamepad().Get(InputAction.SoftReset).IsBound);
    }

    /// <summary>
    /// AMD-044 REQ-INP-125 acceptance: the action is bindable in the controls screen and, once
    /// bound, triggers the command of REQ-LOOP-072.
    /// </summary>
    [Fact]
    public void REQ_INP_125_the_bound_action_triggers_the_soft_reset()
    {
        using LoopHarness harness = LaunchScripted(out _);

        // The controls screen of REQ-UI-203 offers the action among the system hotkeys.
        KeyConfigModel keys = new(harness.Host.Commands);
        Assert.Contains(
            keys.AllRows(),
            row => row.Target.IsAction && row.Target.Action == InputAction.SoftReset);

        harness.RunFrames(300);
        Assert.True(harness.Host.Rollback.Count > 0);
        long before = harness.Host.Session!.Core.FrameCount;
        Assert.True(before > 0);

        // One rendered frame with the action held: the rising edge queues the command, which
        // the same Tick applies between two emulated frames, before the frames it then runs.
        harness.Tick(1.0 / 60.0, InputAction.SoftReset);

        Assert.True(
            harness.Host.Session!.Core.FrameCount < before,
            $"the core is still at frame {harness.Host.Session!.Core.FrameCount}.");

        // The history was emptied; the single snapshot left is the recapture of frame 0, taken
        // by the very frame that followed the reset (REQ-LOOP-030).
        Assert.InRange(harness.Host.Rollback.Count, 0, 1);

        // Holding the action does not reset the console again and again: it is an edge.
        harness.RunFrames(300, InputAction.SoftReset);
        Assert.True(harness.Host.Session!.Core.FrameCount >= 300);
    }
}
