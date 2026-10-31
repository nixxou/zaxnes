using System.Security.Cryptography;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;

namespace Zax.App.Tests;

/// <summary>
/// AMD-053 — the resume point of REQ-LOOP-076 and the command line of REQ-UI-213, driven through
/// the real emulation loop. The favourites, the search and the import inspection are in
/// <see cref="Amd053LibraryTests"/>; the model-level criteria are in
/// <c>Zax.Acceptance.Tests.Amd053Tests</c>.
/// </summary>
public sealed class Amd053Tests
{
    // ---------------------------------------------------------- REQ-LOOP-076

    /// <summary>
    /// REQ-LOOP-076, first criterion: quitting during a game then relaunching offers the resume
    /// entry, and accepting it restores the state — the same core fingerprint as the savestate
    /// the exit wrote (REQ-LOOP-060).
    /// </summary>
    [Fact]
    public void REQ_LOOP_076_quitting_during_a_game_offers_the_resume_point()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);
        string fingerprint;

        using (LoopHarness first = Open(root, rom))
        {
            first.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
            first.RunFrames(120);
            Assert.Equal(EmulationState.Running, first.Host.State);

            fingerprint = Fingerprint(first);
            first.Host.Shutdown();

            SaveSlotRecord slot = first.Host.Slots.Find(SaveSlotIndex.ResumeSlotId)!;
            Assert.True(slot.IsOccupied);
            Assert.Equal(Game, slot.GameId);
            Assert.True(File.Exists(first.Paths.SlotStateFile(SaveSlotIndex.ResumeSlotId)));
        }

        using LoopHarness second = Open(root, rom);

        Assert.True(second.Host.ResumeAvailable);
        Assert.Equal(Game, second.Host.ResumeGameTitle);

        second.Run(ZaxCommands.ResumeLast);

        Assert.Equal(EmulationState.Running, second.Host.State);
        Assert.Equal(fingerprint, Fingerprint(second));
    }

    /// <summary>
    /// REQ-LOOP-076, second criterion: quitting from the Stopped state empties the slot, deletes
    /// its file and leaves the entry unoffered at the next launch. Rule 1 wins over rule 2, so
    /// ignoring the entry and then quitting without playing does clear the resume point.
    /// </summary>
    [Fact]
    public void REQ_LOOP_076_quitting_from_stopped_clears_the_resume_point()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using (LoopHarness first = Open(root, rom))
        {
            first.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
            first.RunFrames(60);
            first.Host.Shutdown();
            Assert.True(first.Host.Slots.Find(SaveSlotIndex.ResumeSlotId)!.IsOccupied);
        }

        // Second run: the entry is offered, ignored, and the program quits from Stopped.
        using (LoopHarness second = Open(root, rom))
        {
            Assert.True(second.Host.ResumeAvailable);
            Assert.Equal(EmulationState.Stopped, second.Host.State);
            second.Host.Shutdown();
        }

        using LoopHarness third = Open(root, rom);

        Assert.False(third.Host.ResumeAvailable);
        Assert.False(third.Host.Slots.Find(SaveSlotIndex.ResumeSlotId)!.IsOccupied);
        Assert.False(File.Exists(third.Paths.SlotStateFile(SaveSlotIndex.ResumeSlotId)));
    }

    /// <summary>
    /// REQ-LOOP-076, third criterion: at an exit made during a game, exactly one <c>Save</c>
    /// hook per active script runs before the capture, then exactly one <c>End</c> hook per
    /// active script, in that order (steps 1 to 3).
    /// </summary>
    [Fact]
    public void REQ_LOOP_076_the_exit_runs_save_then_end_once_per_script()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        GameProfile profile = GameProfile.CreateEmpty();
        foreach (string name in (string[])["a", "b"])
        {
            profile.Scripts.Add(new ProfileScript(
                name,
                true,
                $"function Save() print('Save {name}') end\n"
                    + $"function End() print('End {name}') end",
                "*"));
        }

        using LoopHarness harness = Open(root, rom, profile);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
        harness.RunFrames(10);
        harness.ScriptLines.Clear();

        harness.Host.Shutdown();

        Assert.Equal(["Save a", "Save b", "End a", "End b"], harness.ScriptLines);
    }

    /// <summary>
    /// REQ-LOOP-076, fourth criterion: a truncated resume file is not offered at the next start,
    /// its file is deleted, no notification is shown and the welcome screen of REQ-UI-207 — the
    /// Stopped state with no current game — is what the program comes up on.
    /// </summary>
    [Fact]
    public void REQ_LOOP_076_a_truncated_resume_file_is_discarded_without_a_notification()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using (LoopHarness first = Open(root, rom))
        {
            first.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
            first.RunFrames(60);
            first.Host.Shutdown();
        }

        // The file keeps a valid signature and version; only the length of a section is wrong,
        // which is what the whole-file check of §4.5.4 catches.
        string state = Path.Combine(root.Path, "saves", SaveSlotIndex.ResumeSlotId + ".state");
        byte[] truncated = File.ReadAllBytes(state);
        File.WriteAllBytes(state, truncated[..(truncated.Length - 64)]);

        using LoopHarness second = Open(root, rom);

        Assert.False(second.Host.ResumeAvailable);
        Assert.False(File.Exists(state));
        Assert.Null(second.Host.Notifications.Current);
        Assert.Equal(EmulationState.Stopped, second.Host.State);
        Assert.Null(second.Host.CurrentGameId);
        Assert.Contains(second.Log, line => line.Contains("resume point was discarded", StringComparison.Ordinal));
    }

    /// <summary>
    /// REQ-LOOP-076 and REQ-LOOP-052 as §3 points 1 and 2 amend them: the exit writes no
    /// numbered slot and no quick slot — only the resume slot.
    /// </summary>
    [Fact]
    public void REQ_LOOP_052_the_exit_writes_no_numbered_slot_and_no_quick_slot()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using LoopHarness harness = Open(root, rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
        harness.RunFrames(30);
        harness.Host.Shutdown();

        foreach (SaveSlotRecord slot in harness.Host.Slots.Slots)
        {
            if (string.Equals(slot.Id, SaveSlotIndex.ResumeSlotId, StringComparison.Ordinal))
            {
                Assert.True(slot.IsOccupied);
                continue;
            }

            Assert.False(slot.IsOccupied);
            Assert.False(File.Exists(harness.Paths.SlotStateFile(slot.Id)));
        }
    }

    /// <summary>
    /// Q-088, the point 53-N2 of the R2 review: <c>-noresume</c> suspends the resume slot in
    /// full for one run — it is neither read, nor written, nor erased — so quitting from Stopped
    /// under that option leaves the point of a previous session alone.
    /// </summary>
    [Fact]
    public void Q_088_noresume_neither_reads_writes_nor_erases_the_resume_slot()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using (LoopHarness first = Open(root, rom))
        {
            first.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
            first.RunFrames(60);
            first.Host.Shutdown();
        }

        long written = new FileInfo(Path.Combine(root.Path, "saves", "resume.state")).Length;

        using (LoopHarness suspended = Open(root, rom, suspendResume: true))
        {
            Assert.False(suspended.Host.ResumeAvailable);

            // Playing and quitting under -noresume writes nothing into the slot either.
            suspended.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
            suspended.RunFrames(30);
            suspended.Run(ZaxCommands.Stop);
            suspended.Host.Shutdown();
        }

        Assert.Equal(written, new FileInfo(Path.Combine(root.Path, "saves", "resume.state")).Length);

        using LoopHarness back = Open(root, rom);
        Assert.True(back.Host.ResumeAvailable);
    }

    /// <summary>
    /// REQ-PERS-052 as §3 point 5 extends it: the resume slot shows in no Load or Save list, and
    /// the save and load commands refuse it — only <c>resume.last</c> reads it.
    /// </summary>
    [Fact]
    public void REQ_PERS_052_the_resume_slot_is_out_of_the_save_and_load_commands()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using LoopHarness harness = Open(root, rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
        harness.RunFrames(30);

        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(SaveSlotIndex.ResumeSlotId));
        Assert.False(harness.Host.Slots.Find(SaveSlotIndex.ResumeSlotId)!.IsOccupied);

        Assert.False(SaveSlotIndex.IsUserSlotId(SaveSlotIndex.ResumeSlotId));
        Assert.True(SaveSlotIndex.IsValidSlotId(SaveSlotIndex.ResumeSlotId));
    }

    // ---------------------------------------------------------- REQ-UI-213

    /// <summary>
    /// REQ-UI-213: the three options are accepted with a single dash and with their double-dash
    /// aliases, <c>-slot</c> takes the 36 slots and the quick slot, and <c>-slot</c> implies
    /// <c>-noresume</c>.
    /// </summary>
    [Fact]
    public void REQ_UI_213_the_command_line_takes_the_three_options_and_their_aliases()
    {
        AppOptions dashed = AppOptions.Parse(["-id=10", "-slot=36", "-noresume", "-emustats"]);
        Assert.Equal("10", dashed.GameId);
        Assert.Equal("36", dashed.Slot);
        Assert.True(dashed.NoResume);
        Assert.True(dashed.EmuStats);

        AppOptions aliases = AppOptions.Parse(["--id=10", "--slot=quick", "--no-resume", "--emu-stats"]);
        Assert.Equal("10", aliases.GameId);
        Assert.Equal("quick", aliases.Slot);
        Assert.True(aliases.NoResume);
        Assert.True(aliases.EmuStats);

        // -slot implies -noresume, whatever the order.
        Assert.True(AppOptions.Parse(["-slot=3"]).NoResume);

        // A value outside the domain is ignored and the program starts normally; the command
        // line is not held to the ten slots the menus expose (AMD-010 REQ-UI-208).
        AppOptions outside = AppOptions.Parse(["-slot=99"]);
        Assert.Null(outside.Slot);
        Assert.False(outside.NoResume);
        Assert.Single(outside.Notices);

        Assert.Equal("20", AppOptions.Parse(["-slot=20"]).Slot);
        Assert.Null(AppOptions.Parse(["-slot=0"]).Slot);
        Assert.Null(AppOptions.Parse(["-slot=37"]).Slot);
    }

    /// <summary>
    /// REQ-UI-213, first criterion: <c>-id=&lt;game&gt; -slot=3</c> with slot 3 held by that
    /// game starts the game and then loads the slot, whose core fingerprint is the savestate's.
    /// The slot is read <b>after</b> step 7 of REQ-LOOP-021 — the state having gone to Running —
    /// which REQ-LOOP-007 is what defers it.
    /// </summary>
    [Fact]
    public void REQ_UI_213_a_slot_of_the_same_game_is_loaded_after_the_launch()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using LoopHarness harness = Open(root, rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
        harness.RunFrames(90);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));
        string saved = Fingerprint(harness);

        harness.RunFrames(90);
        Assert.NotEqual(saved, Fingerprint(harness));

        StartupRequest request = StartupLaunch.Decide(
            Game, "3", harness.Host.Slots, harness.Catalog.Exists, harness.Log.Add);
        Assert.Equal(new StartupRequest(Game, "3"), request);

        // What the host does with it: the launch first, the slot behind it.
        harness.Host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(request.GameId!));
        harness.Host.Enqueue(ZaxCommands.LoadSlot, CommandArgs.Of(request.SlotId!));
        harness.Host.ApplyCommands();
        harness.Settle();
        harness.Host.ApplyCommands();
        harness.Settle();

        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal(saved, Fingerprint(harness));
    }

    /// <summary>
    /// REQ-UI-213, second criterion: a slot of another game starts that game and <c>-id</c> has
    /// no effect; an empty slot is ignored, the game of <c>-id</c> is still started, and the
    /// fact is journalled; a slot outside the domain is ignored altogether.
    /// </summary>
    [Fact]
    public void REQ_UI_213_an_empty_slot_or_a_slot_of_another_game()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using LoopHarness harness = Open(root, rom);
        harness.Catalog.Add("b", rom);

        // Slot 5 is written by the game B.
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("b"));
        harness.RunFrames(30);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(5));

        List<string> log = [];
        StartupRequest other = StartupLaunch.Decide(
            Game, "5", harness.Host.Slots, harness.Catalog.Exists, log.Add);
        Assert.Equal(new StartupRequest(null, "5"), other);
        Assert.Contains(log, line => line.Contains("has no effect", StringComparison.Ordinal));

        log.Clear();
        StartupRequest empty = StartupLaunch.Decide(
            Game, "7", harness.Host.Slots, harness.Catalog.Exists, log.Add);
        Assert.Equal(new StartupRequest(Game, null), empty);
        Assert.Contains(log, line => line.Contains("-slot=7 is empty", StringComparison.Ordinal));

        // And an unknown -id is ignored, the program starting normally (REQ-UI-007).
        log.Clear();
        StartupRequest unknown = StartupLaunch.Decide(
            "no-such-game", null, harness.Host.Slots, harness.Catalog.Exists, log.Add);
        Assert.Equal(new StartupRequest(null, null), unknown);
        Assert.Contains(log, line => line.Contains("matches no library entry", StringComparison.Ordinal));
    }

    /// <summary>
    /// REQ-UI-213, third criterion: <c>-emustats</c> writes exactly one line per emulated frame
    /// — 600 ± 1 over ten NTSC seconds — into the log file, and nothing onto the parent console:
    /// a run that carries the option is not one of the runs that borrow it (AMD-032 §5).
    /// </summary>
    [Fact]
    public void REQ_UI_213_emustats_writes_one_line_per_emulated_frame()
    {
        using TemporaryRoot root = new();
        string rom = WriteRom(root);

        using LoopHarness harness = Open(root, rom, frameStats: true);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of(Game));
        harness.FrameStats.Clear();

        // Ten seconds of NTSC at the nominal cadence.
        int frames = 0;
        for (int tick = 0; tick < 600; tick++)
        {
            frames += harness.Tick(1.0 / 60.0);
        }

        Assert.InRange(frames, 599, 601);
        Assert.Equal(frames, harness.FrameStats.Count);
        Assert.All(harness.FrameStats, line => Assert.Contains(" ms", line, StringComparison.Ordinal));

        // The console is never borrowed for a normal launch, -emustats included.
        Assert.False(Program.PrintsAndExits(AppOptions.Parse(["-emustats"])));

        // The sink really is the log file of the user folder.
        using RunLog log = RunLog.Open(harness.Paths);
        foreach (string line in harness.FrameStats)
        {
            log.Write(line);
        }

        Assert.Equal(harness.FrameStats.Count, log.CountLines());
    }

    // ------------------------------------------------------------- helpers

    private const string Game = "10";

    private static string WriteRom(TemporaryRoot root)
    {
        string path = Path.Combine(root.Path, "game.nes");
        File.WriteAllBytes(path, SyntheticRom.Counter());
        return path;
    }

    private static LoopHarness Open(
        TemporaryRoot root,
        string rom,
        GameProfile? profile = null,
        bool suspendResume = false,
        bool frameStats = false)
    {
        FakeCatalog catalog = new();
        catalog.Add(Game, rom, profile);

        LoopHarness harness = LoopHarness.Create(
            catalog,
            root: root.Path,
            suspendResume: suspendResume,
            frameStats: frameStats);
        harness.KeepRoot = true;
        harness.Thumbnail = new byte[EmulationHost.ThumbnailWidth * EmulationHost.ThumbnailHeight * 3];
        return harness;
    }

    private static string Fingerprint(LoopHarness harness)
        => Convert.ToHexString(SHA1.HashData(
            harness.Host.Session!.Core.SaveState(includeRomImage: false)));
}
