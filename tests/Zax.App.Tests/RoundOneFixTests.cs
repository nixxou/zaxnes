using Zax.App;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// Regressions for the non-conformities the independent round-1 verification reported, so that
/// each correction has a test of the implementers' own beside the verifier's acceptance test.
/// </summary>
[Collection(ImGuiCollection.Name)]
public sealed class RoundOneFixTests
{
    // ------------------------------------------------- REQ-SCN-013, end of a game

    /// <summary>
    /// SPEC-06 REQ-SCN-013: the end of a game resets the view, goes back to TPV and puts the
    /// smoothing time back to 0.4 s. Round 1 found the branch missing in the host.
    /// </summary>
    [Fact] // REQ-SCN-013
    public void REQ_SCN_013_stopping_a_game_resets_the_view_and_returns_to_tpv()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        SceneCamera camera = harness.Scene.Camera;
        camera.SetMode(CameraMode.Fpv);
        camera.Current.Distance = 42f;
        camera.Current.Angles = new System.Numerics.Vector3(45f, 45f, 0f);
        camera.SmoothTime = 2f;
        harness.Host.Settings.GameSpeed = 4;

        harness.Run(ZaxCommands.Stop);

        Assert.Equal(CameraMode.Tpv, camera.Mode);
        Assert.Equal(SceneConstants.DefaultAngles.X, camera.Current.Angles.X, 3);
        Assert.Equal(SceneConstants.DefaultAngles.Y, camera.Current.Angles.Y, 3);
        Assert.Equal(
            SceneConstants.DefaultDistance(harness.Scene.Camera.Fov),
            camera.Current.Distance,
            3);
        Assert.Equal(CriticalSmoothing.DefaultSmoothTime, camera.SmoothTime, 4);
        Assert.Equal(0, harness.Host.Settings.GameSpeed);
    }

    // ------------------------------------------------- REQ-AUD-022, menu music

    /// <summary>
    /// SPEC-11 REQ-AUD-022: the menu music comes back after a game. Round 1 found the track left
    /// suspended for the rest of the session.
    /// </summary>
    [Fact] // REQ-AUD-022
    public void REQ_AUD_022_the_menu_music_leaves_suspension_when_a_game_ends()
    {
        using LoopHarness harness = LoopHarness.Create();
        Audio.AudioMixer mixer = harness.Mixer;
        mixer.MenuMusicEnabled = true;

        // In the menu the track plays; entering the game fades it out and suspends it.
        mixer.EmulationAudible = true;
        mixer.Advance(5.0);
        Assert.True(mixer.MusicSuspended);

        // Back to the menu: the track is playable again and its gain climbs back.
        mixer.EmulationAudible = false;
        Assert.False(mixer.MusicSuspended);

        mixer.Advance(5.0);
        Assert.False(mixer.MusicSuspended);
        Assert.True(mixer.MusicGain > 0f);
    }

    // ------------------------------------------- REQ-INP-101 / REQ-UI-004, Menu

    /// <summary>
    /// SPEC-08 REQ-INP-101 and SPEC-10 REQ-UI-004 case 1: a short press on Menu made while a
    /// load is in progress is <b>ignored</b>, not deferred — the game must not pause the instant
    /// the load ends.
    /// </summary>
    [Fact] // REQ-INP-101, REQ-UI-004
    public void REQ_INP_101_a_menu_press_during_a_load_is_dropped()
    {
        using UiHarness harness = new("Arkanoid");
        string rom = harness.WriteRom();
        harness.Store.Games.SetProfileFor(Path.GetFullPath(rom), "g0");

        harness.Host.Enqueue(ZaxCommands.OpenRom, CommandArgs.Of(rom));
        harness.Host.ApplyCommands();
        Assert.True(harness.Host.IsLoading);

        // Pressed and released while the loading indicator is up.
        harness.Keyboard.Press(Silk.NET.Input.Key.Escape);
        harness.Tick(0.05);
        harness.Keyboard.Release(Silk.NET.Input.Key.Escape);
        harness.Tick(0.05);

        Assert.Equal(0, harness.Host.PendingCommands);

        harness.Settle();
        harness.Tick();
        harness.Tick();

        Assert.Equal(EmulationState.Running, harness.Host.State);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-004 cases 2 and 3: a modal window takes the Menu press before the
    /// emulation does. The file browser closes as with « Cancel » and the emulation keeps
    /// running; the confirmation box E8 swallows the press and stays up (REQ-UI-110).
    /// </summary>
    [Fact] // REQ-UI-004, REQ-UI-110
    public void REQ_UI_004_a_modal_window_takes_the_menu_press_before_the_emulation()
    {
        using UiHarness harness = new("Arkanoid");
        harness.Launch("g0");
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // Case 3: the browser is open, Escape closes it and the game keeps running.
        harness.Host.Enqueue(ZaxCommands.OpenRom);
        harness.Host.ApplyCommands();
        Assert.True(harness.Ui.State.FileBrowser);
        Assert.True(harness.Host.ModalWindowOpen);

        Escape(harness);
        Assert.False(harness.Ui.State.FileBrowser);
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // Case 2: a confirmation box is open, Escape does nothing at all.
        harness.Host.Enqueue(ZaxCommands.QuickSave);
        harness.Host.ApplyCommands();
        harness.Host.Enqueue(ZaxCommands.DeleteSlot, CommandArgs.Of(SaveSlotIndex.QuickSlotId));
        harness.Host.ApplyCommands();
        Assert.True(harness.Ui.State.Dialog);

        Escape(harness);
        Assert.True(harness.Ui.State.Dialog);
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // Case 4, for contrast: with no modal window the press pauses the game.
        harness.Ui.Confirm.Close();
        harness.Ui.State.Dialog = false;
        Escape(harness);
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        static void Escape(UiHarness harness)
        {
            harness.Keyboard.Press(Silk.NET.Input.Key.Escape);
            harness.Tick(0.05);
            harness.Keyboard.Release(Silk.NET.Input.Key.Escape);
            harness.Tick(0.05);
            harness.Host.ApplyCommands();
            harness.Tick();
        }
    }

    // --------------------------------- REQ-PERS-042 / REQ-UI-037, delete a game

    /// <summary>
    /// SPEC-09 REQ-PERS-042: deleting a user game stops the emulation when it is the current
    /// game, empties every slot of that game including the quick slot, and removes the profile
    /// file, the cover art and the library entry.
    /// </summary>
    [Fact] // REQ-PERS-042, REQ-UI-037
    public void REQ_PERS_042_a_user_game_is_deleted_with_its_states_its_file_and_its_cover()
    {
        using UiHarness harness = new("Arkanoid");
        string profileFile = Path.Combine(harness.Paths.ProfilesDirectory, "mine.3dn");
        Directory.CreateDirectory(harness.Paths.ProfilesDirectory);
        File.WriteAllBytes(profileFile, ProfileFormat.Save(EmptyProfile("mine", "Mine")));
        harness.Library.AddUserProfile(new UserProfile("mine", "Mine", profileFile, default));

        harness.Launch("mine");
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));
        harness.Run(ZaxCommands.QuickSave);
        Assert.True(harness.Host.Slots.Find("3")!.IsOccupied);
        Assert.True(harness.Host.Slots.Find("quick")!.IsOccupied);

        // A cover, so that its file can be shown to go too.
        string cover = GameArt.CoverFile(harness.Paths, "Mine", "mine");
        Directory.CreateDirectory(harness.Paths.CoversDirectory);
        File.WriteAllBytes(cover, Zax.Render.Assets.PngImage.EncodeRgb(2, 2, new byte[2 * 2 * 3]));
        harness.Catalog.SetCover("mine", Path.GetFileName(cover));

        harness.Run(ZaxCommands.DeleteGame, CommandArgs.Of("mine"));

        // The interface asked first, with the exact text of SPEC-10 REQ-UI-037.
        Assert.True(harness.Ui.State.Dialog);
        Assert.Equal("Do you want to delete the game and its save states ?", harness.Ui.Confirm.Message);
        Assert.Equal("Mine", harness.Ui.Confirm.Title);
        Assert.Equal("Yes", harness.Ui.Confirm.ConfirmLabel);
        Assert.Equal("No", harness.Ui.Confirm.CancelLabel);

        harness.Ui.Confirm.Confirm();

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Null(harness.Host.CurrentGameId);
        Assert.False(harness.Host.Slots.Find("3")!.IsOccupied);
        Assert.False(harness.Host.Slots.Find("quick")!.IsOccupied);
        Assert.False(File.Exists(harness.Paths.SlotStateFile("3")));
        Assert.False(File.Exists(harness.Paths.SlotStateFile("quick")));
        Assert.False(File.Exists(profileFile));
        Assert.False(File.Exists(cover));
        Assert.Null(harness.Library.Find("mine"));
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-042 and SPEC-10 REQ-UI-037: a built-in profile and the « 2D Mode » entry
    /// are never deletable, and « No » changes nothing.
    /// </summary>
    [Fact] // REQ-PERS-042, REQ-UI-037
    public void REQ_PERS_042_a_built_in_profile_is_never_deletable_and_no_changes_nothing()
    {
        using UiHarness harness = new("Arkanoid");
        string profileFile = Path.Combine(harness.Paths.ProfilesDirectory, "mine.3dn");
        Directory.CreateDirectory(harness.Paths.ProfilesDirectory);
        File.WriteAllBytes(profileFile, ProfileFormat.Save(EmptyProfile("mine", "Mine")));
        harness.Library.AddUserProfile(new UserProfile("mine", "Mine", profileFile, default));

        Assert.False(harness.Host.RequestDeleteGame("g0"));
        Assert.False(harness.Host.RequestDeleteGame(harness.TwoDimensionalModeId));
        Assert.False(harness.Host.DeleteGame("g0"));
        Assert.NotNull(harness.Library.Find("g0"));

        Assert.True(harness.Host.RequestDeleteGame("mine"));
        harness.Ui.Confirm.Close();
        Assert.NotNull(harness.Library.Find("mine"));
        Assert.True(File.Exists(profileFile));
    }

    /// <summary>
    /// SPEC-10 REQ-UI-037 and REQ-UI-052: both deletions are reachable from the menu bar, which
    /// AMD-010 §3 keeps among the mandatory functions.
    /// </summary>
    [Fact] // REQ-UI-037, REQ-UI-052
    public void REQ_UI_037_the_menu_bar_offers_both_deletions()
    {
        using UiHarness harness = new("Arkanoid");
        harness.Launch("g0");
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));

        IReadOnlyList<MenuItem> rows = [.. harness.Ui.MenuBar.AllItems()];

        Assert.Contains(rows, row => row.Command == ZaxCommands.DeleteGame);
        Assert.Contains(
            rows,
            row => row.Command == ZaxCommands.DeleteSlot
                && row.Label.Contains("2", StringComparison.Ordinal));
        Assert.All(
            rows.Where(static row => row.Command is not null),
            row => Assert.NotEmpty(row.Label));
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-054 and SPEC-10 REQ-UI-052: deleting a slot asks for confirmation with
    /// the exact text, titled with the game of the slot.
    /// </summary>
    [Fact] // REQ-UI-052, REQ-PERS-054
    public void REQ_UI_052_deleting_a_slot_asks_the_documented_question()
    {
        using UiHarness harness = new("Arkanoid");
        harness.Launch("g0");
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(5));
        Assert.True(harness.Host.Slots.Find("5")!.IsOccupied);

        harness.Run(ZaxCommands.DeleteSlot, CommandArgs.Of(5));

        Assert.True(harness.Ui.State.Dialog);
        Assert.Equal("Do you want to delete the slot 5 ?", harness.Ui.Confirm.Message);
        Assert.Equal("Arkanoid", harness.Ui.Confirm.Title);

        // « No » keeps the slot.
        harness.Ui.Confirm.Close();
        Assert.True(harness.Host.Slots.Find("5")!.IsOccupied);

        harness.Run(ZaxCommands.DeleteSlot, CommandArgs.Of(5));
        harness.Ui.Confirm.Confirm();
        Assert.False(harness.Host.Slots.Find("5")!.IsOccupied);
        Assert.False(File.Exists(harness.Paths.SlotStateFile("5")));
    }

    // -------------------------------------- REQ-PERS-036 / REQ-PERS-074, covers

    /// <summary>
    /// SPEC-09 REQ-PERS-074: a file name derived from user data has every forbidden character
    /// replaced by <c>_</c> and carries the profile identifier, which is what makes it unique.
    /// </summary>
    [Fact] // REQ-PERS-074
    public void REQ_PERS_074_a_derived_cover_name_is_sanitised_and_carries_the_identifier()
    {
        Assert.Equal("A_B_C__3dsen_42.png", GameArt.CoverFileName("A/B:C?", "42"));
        Assert.Equal("A_B_3dsen_42.png", GameArt.CoverFileName("A_B", "42"));
        Assert.DoesNotContain('\\', GameArt.SanitiseFileName(@"a\b"));
        Assert.Equal("a_b", GameArt.SanitiseFileName(@"a\b"));
        Assert.Equal("a_b_c", GameArt.SanitiseFileName("a<b>c"));

        // The identifier is what distinguishes two games of the same title.
        Assert.NotEqual(GameArt.CoverFileName("Same", "1"), GameArt.CoverFileName("Same", "2"));
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-036: an imported image is shrunk to fit 256 × 192 with its proportions
    /// kept — the factor is <c>max(w / 256, h / 192)</c> — and written as PNG without alpha.
    /// </summary>
    [Fact] // REQ-PERS-036
    public void REQ_PERS_036_an_imported_cover_is_reduced_to_fit_the_documented_size()
    {
        Assert.Equal((192, 192), GameArt.FittedSize(1024, 1024));
        Assert.Equal((256, 48), GameArt.FittedSize(1024, 192));
        Assert.Equal((64, 48), GameArt.FittedSize(64, 48));

        using TemporaryRoot root = new();
        UserDataPaths paths = UserDataPaths.Resolve(root.Path);

        string source = Path.Combine(root.Path, "art.png");
        byte[] pixels = new byte[1024 * 1024 * 3];
        Array.Fill(pixels, (byte)0x40);
        File.WriteAllBytes(source, Zax.Render.Assets.PngImage.EncodeRgb(1024, 1024, pixels));

        string? name = GameArt.ImportCover(paths, "A/B", "7", source, out string? message);

        Assert.Null(message);
        Assert.Equal("A_B_3dsen_7.png", name);

        Zax.Render.Assets.ImageData written = Zax.Render.Assets.PngImage.Load(
            Path.Combine(paths.CoversDirectory, name!));
        Assert.Equal(192, written.Width);
        Assert.Equal(192, written.Height);
        Assert.Equal(0x40, written.Pixel(10, 10).R);
        Assert.Equal(255, written.Pixel(10, 10).A);
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-036: a file this build cannot decode is refused with a message instead
    /// of leaving a broken cover behind (Q-050).
    /// </summary>
    [Fact] // REQ-PERS-036, Q-050
    public void REQ_PERS_036_an_undecodable_cover_is_refused()
    {
        using TemporaryRoot root = new();
        UserDataPaths paths = UserDataPaths.Resolve(root.Path);

        string source = Path.Combine(root.Path, "art.jpg");
        File.WriteAllBytes(source, [0xFF, 0xD8, 0xFF, 0xE0, 0, 0]);

        Assert.Null(GameArt.ImportCover(paths, "T", "1", source, out string? message));
        Assert.Equal(GameArt.UnsupportedSourceMessage, message);
        Assert.Empty(Directory.GetFiles(paths.CoversDirectory));
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-036, « Create game art »: the pause capture becomes the cover of the
    /// current game, and the library entry records its file name.
    /// </summary>
    [Fact] // REQ-PERS-036
    public void REQ_PERS_036_the_pause_capture_can_become_the_cover()
    {
        using UiHarness harness = new("Arkanoid");
        harness.Launch("g0");
        harness.Run(ZaxCommands.CreateGameArt);

        string? cover = harness.Library.Find("g0")!.Cover;
        Assert.Equal("Arkanoid_3dsen_g0.png", cover);
        Assert.True(File.Exists(Path.Combine(harness.Paths.CoversDirectory, cover!)));
    }

    // ---------------------------------------------- REQ-LOOP-054, one instance

    /// <summary>
    /// SPEC-07 REQ-LOOP-054: one instance at a time on a given user root. A second guard on the
    /// same root does not own it; a guard on another root does.
    /// </summary>
    [Fact] // REQ-LOOP-054
    public void REQ_LOOP_054_a_second_instance_on_the_same_root_does_not_own_it()
    {
        using TemporaryRoot first = new();
        using TemporaryRoot second = new();

        SingleInstance owner = SingleInstance.Acquire(first.Path);
        Assert.True(owner.IsOwner);

        // A named mutex is owned by a thread, so a second launch is simulated on another one:
        // the real second launch is another process, which is the same thing to the kernel.
        Assert.False(AcquireElsewhere(first.Path));
        Assert.True(AcquireElsewhere(second.Path));

        // The name depends on the root alone, and is stable across spellings of it.
        Assert.Equal(
            SingleInstance.NameFor(first.Path),
            SingleInstance.NameFor(first.Path + Path.DirectorySeparatorChar));
        Assert.NotEqual(SingleInstance.NameFor(first.Path), SingleInstance.NameFor(second.Path));

        // Releasing the guard lets the next instance in.
        owner.Dispose();
        Assert.True(AcquireElsewhere(first.Path));

        static bool AcquireElsewhere(string root)
        {
            bool owned = false;
            Thread thread = new(() =>
            {
                using SingleInstance guard = SingleInstance.Acquire(root);
                owned = guard.IsOwner;
            });

            thread.Start();
            thread.Join();
            return owned;
        }
    }

    // -------------------------------------------------- REQ-UI-207, the triangle

    /// <summary>
    /// AMD-010 REQ-UI-207 with AMD-024 REQ-PUB-006: « Fichier ▸ Ouvrir une ROM » must read as the
    /// amendment writes it, with the <b>system</b> font the publishable build loads at run time.
    /// Round 1 saw « Fichier ? Ouvrir une ROM », because neither bundled font carried U+25B8. The
    /// string stays the amendment's; the atlas is what gains the glyph.
    /// </summary>
    [Fact] // REQ-UI-207, REQ-PUB-006
    public unsafe void REQ_UI_207_the_triangle_of_the_welcome_hint_is_in_the_font_atlas()
    {
        Assert.Contains('▸', UiText.WelcomeHint);
        Assert.Equal("Fichier ▸ Ouvrir une ROM", UiText.WelcomeHint);

        nint context = ImGuiNET.ImGui.CreateContext();
        try
        {
            ImGuiNET.ImGui.SetCurrentContext(context);
            if (!UiFonts.Load())
            {
                // No candidate face is installed; REQ-PUB-006 then allows the built-in font and
                // its ASCII substitutes, which carry no accent to check.
                return;
            }

            Assert.NotNull(UiFonts.BodyFile);

            // Building the atlas is what fills the glyph index tables the remap works on, which
            // the application gets from the ImGui controller.
            ImGuiNET.ImGui.GetIO().Fonts.GetTexDataAsRGBA32(out nint _, out int _, out int _);
            UiFonts.RemapMissingGlyphs();

            foreach (ImGuiNET.ImFontPtr font in (ImGuiNET.ImFontPtr[])[UiFonts.Body!.Value, UiFonts.Heading!.Value])
            {
                // Every character of the hint resolves to a real glyph rather than to the
                // missing-glyph box: the accents of AMD-010 and the triangle of REQ-UI-207,
                // whether the face carries U+25B8 itself or one of the substitutes.
                foreach (char letter in UiText.WelcomeHint)
                {
                    Assert.NotEqual(nint.Zero, (nint)font.FindGlyphNoFallback(letter).NativePtr);
                }

                foreach (char accented in "ÉÈÊËÀÂÇÎÏÔÙÛÜæœ«»…")
                {
                    Assert.NotEqual(nint.Zero, (nint)font.FindGlyphNoFallback(accented).NativePtr);
                }
            }
        }
        finally
        {
            // The statics of UiFonts point into this atlas; they must not outlive the context,
            // or the next test that pushes the heading font would walk freed memory.
            UiFonts.Load(builtInOnly: true);
            ImGuiNET.ImGui.DestroyContext(context);
        }
    }

    private static GameProfile EmptyProfile(string id, string title)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        ProfileExport.Normalize(profile);
        return profile;
    }
}
