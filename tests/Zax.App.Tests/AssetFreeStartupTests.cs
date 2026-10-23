using Zax.App.Audio;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Render;
using Zax.Platform;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;
using Zax.Profiles.Wire;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// The publishable base of AMD-024: the application must run with <b>no asset folder at all</b>
/// (REQ-PUB-001), ship no asset file (REQ-PUB-002), synthesise its audio (REQ-PUB-005), take its
/// font from the machine (REQ-PUB-006) and carry the tile key in code (REQ-PUB-007).
/// </summary>
public sealed class AssetFreeStartupTests : IDisposable
{
    private readonly string _root = Path.Combine(
        Path.GetTempPath(),
        $"zax-p7a-{Guid.NewGuid():N}");

    /// <inheritdoc/>
    public void Dispose()
    {
        try
        {
            if (Directory.Exists(_root))
            {
                Directory.Delete(_root, recursive: true);
            }
        }
        catch (IOException)
        {
            // A throw-away folder that will not go is not a test failure.
        }
    }

    [Fact] // REQ-PUB-001 (Q-065)
    public void REQ_PUB_001_the_library_without_an_asset_folder_holds_the_two_generic_entries()
    {
        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, Paths());

        GameLibraryEntry[] entries = [.. catalog.Library.Entries];
        Assert.Equal(2, entries.Length);

        // « 2D Mode » keeps the identifier REQ-PROF-060 writes in clear, so a library.json from
        // an earlier version still finds its entry (REQ-PERS-035).
        Assert.Equal(BuiltInCatalogue.TwoDimensionalModeId, catalog.TwoDimensionalModeId);
        Assert.Equal("47", catalog.TwoDimensionalModeId);
        Assert.Equal(BuiltInCatalogue.DefaultThreeDimensionalId, catalog.DefaultThreeDimensionalId);

        GameLibraryEntry twoD = entries.Single(entry => entry.Is2DMode);
        GameLibraryEntry threeD = entries.Single(entry => entry.IsDefaultThreeDimensional);
        Assert.Equal(ProfileCatalogue.TwoDimensionalModeTitle, twoD.DisplayTitle);
        Assert.Equal(ProfileCatalogue.DefaultThreeDimensionalTitle, threeD.DisplayTitle);

        // Neither names a file, and neither can be deleted (REQ-UI-037).
        Assert.All(entries, entry =>
        {
            Assert.Equal(string.Empty, entry.ProfilePath);
            Assert.False(entry.CanDelete);
            Assert.True(entry.IsGeneric);
        });
    }

    [Fact] // REQ-PUB-001 (Q-065)
    public void REQ_PUB_001_both_generic_entries_resolve_to_the_empty_profile_built_in_code()
    {
        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, Paths());

        foreach (string id in (string[])
            [BuiltInCatalogue.TwoDimensionalModeId, BuiltInCatalogue.DefaultThreeDimensionalId])
        {
            Assert.True(catalog.Exists(id));
            ResolvedGame? game = catalog.Resolve(id);
            Assert.NotNull(game);

            // The profile has no pattern at all: the default geometry of SPEC-04 is what the
            // 3D entry converts with, and the 2D entry shows the pixel frame.
            Assert.Empty(game.Profile.PermanentPatterns);
            Assert.Empty(game.Profile.TemporaryPatterns);
        }

        Assert.True(catalog.Resolve(BuiltInCatalogue.TwoDimensionalModeId)!.Is2DMode);
        Assert.False(catalog.Resolve(BuiltInCatalogue.DefaultThreeDimensionalId)!.Is2DMode);
        Assert.True(catalog.Resolve(BuiltInCatalogue.DefaultThreeDimensionalId)!.IsDefaultThreeDimensional);
    }

    [Fact] // REQ-PUB-001 (Q-065)
    public void REQ_PUB_001_the_default_3D_entry_converts_in_3D_and_the_2D_entry_does_not()
    {
        // REQ-SCN-055 makes a profile with no pattern the 2D game; the default-3D entry asks for
        // the other reading of the same empty profile.
        SceneState sceneTwoD = new();
        using GameSession twoD = GameSession.Create(
            GameProfile.CreateEmpty(), "2D Mode", sceneTwoD, twoDimensionalMode: null);
        Assert.True(twoD.TwoDimensionalMode);

        SceneState sceneThreeD = new();
        using GameSession threeD = GameSession.Create(
            GameProfile.CreateEmpty(), "3D", sceneThreeD, twoDimensionalMode: false);
        Assert.False(threeD.TwoDimensionalMode);
    }

    [Fact] // REQ-PUB-001
    public void REQ_PUB_001_a_3dn_profile_still_imports_with_no_asset_folder()
    {
        UserDataPaths paths = Paths();
        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, paths);

        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = "verifier-1";
        profile.Title = "Un jeu importé";
        ProfileExport.Normalize(profile);

        string file = Path.Combine(_root, "imported.3dn");
        File.WriteAllBytes(file, ProfileFormat.Save(profile));

        ProfileImportResult result = catalog.ImportProfile(file);
        Assert.Equal(ProfileImportOutcome.Added, result.Outcome);

        // The entry is there, deletable, and resolves to the profile that was imported.
        GameLibraryEntry entry = catalog.Library.Find("verifier-1")!;
        Assert.Equal("Un jeu importé", entry.DisplayTitle);
        Assert.True(entry.CanDelete);
        Assert.Equal("verifier-1", catalog.Resolve("verifier-1")!.Id);

        // And the two generic entries are still there beside it.
        Assert.Equal(3, catalog.Library.Entries.Count);
    }

    [Fact] // REQ-PUB-001
    public void REQ_PUB_001_the_asset_folder_is_never_discovered_only_named()
    {
        // AMD-024: nothing in the repository or the release may depend on an asset folder, so
        // the folder is not looked for. Only --assets names one.
        Assert.Null(NeutralAssets.Locate());
        Assert.Null(NeutralAssets.Locate("  "));
        Assert.Null(NeutralAssets.Locate(Path.Combine(_root, "does-not-exist")));

        Directory.CreateDirectory(_root);
        Assert.Equal(Path.GetFullPath(_root), NeutralAssets.Locate(_root));
    }

    [Fact] // REQ-PUB-001 (Q-065)
    public void REQ_PUB_001_the_profile_chooser_offers_the_two_generic_entries_first()
    {
        LibraryGameCatalog catalog = LibraryGameCatalog.Open(assetRoot: null, Paths());
        ProfileChooserModel chooser = new(catalog.Library);

        Assert.Equal(2, chooser.All.Count);
        Assert.True(chooser.All[0].Is2DMode);
        Assert.True(chooser.All[1].IsDefault3D);
        Assert.Equal("no 3D conversion", chooser.All[0].SourceLabel);
        Assert.Equal("default 3D geometry", chooser.All[1].SourceLabel);
    }

    [Fact] // REQ-PUB-001
    public void REQ_PUB_001_the_command_line_resolves_the_generic_entries_without_assets()
    {
        List<string> log = [];

        // With nothing asked for, the 2D entry is what a launch falls back to (REQ-SCN-055).
        Assert.Equal(
            ProfileCatalogue.TwoDimensionalModeTitle,
            ProfileResolver.Resolve(null, null, log.Add).Title);

        // And the 3D entry is reachable by title.
        Assert.Equal(
            ProfileCatalogue.DefaultThreeDimensionalTitle,
            ProfileResolver.Resolve(ProfileCatalogue.DefaultThreeDimensionalTitle, null, log.Add).Title);

        // --list-profiles prints the two built-ins rather than nothing.
        Assert.Equal(2, ProfileResolver.List(null).Count);
    }

    [Fact] // REQ-PUB-002
    public void REQ_PUB_002_the_repository_carries_no_asset_file()
    {
        string? repository = FindRepositoryRoot();
        Assert.NotNull(repository);

        string[] forbidden =
            [".bytes", ".3dn", ".png", ".jpg", ".jpeg", ".obj", ".wav", ".ogg", ".mp3", ".ttf",
             ".otf", ".ico"];

        List<string> found = [];
        foreach (string path in Directory.EnumerateFiles(repository, "*", SearchOption.AllDirectories))
        {
            string relative = Path.GetRelativePath(repository, path);

            // Build output, the published release and the local test-ROM corpus are not the
            // repository: tests/roms holds public test ROMs and their reference recordings and
            // carries its own .gitignore, so nothing in it is committed either.
            if (relative.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}")
                || relative.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}")
                || relative.StartsWith("artifacts", StringComparison.Ordinal)
                || relative.StartsWith(Path.Combine("tests", "roms"), StringComparison.Ordinal)
                || relative.Contains("TestResults", StringComparison.Ordinal)
                || relative.StartsWith(".git", StringComparison.Ordinal))
            {
                continue;
            }

            if (forbidden.Contains(Path.GetExtension(path), StringComparer.OrdinalIgnoreCase))
            {
                found.Add(relative);
            }
        }

        // REQ-PUB-002 excepts the files « générés par le projet et listés dans
        // ASSETS-LICENSES.md ». Lot ICON-1 adds exactly one such family: the logo and the icon
        // made for Zaxnes, under branding/. Everything else must still be absent, and the
        // exception only counts while the notices file really carries the entry.
        string[] stowaways = [.. found.Where(static relative =>
            !relative.StartsWith("branding" + Path.DirectorySeparatorChar, StringComparison.Ordinal))];
        Assert.Empty(stowaways);

        if (found.Count > 0)
        {
            string notices = File.ReadAllText(Path.Combine(repository, "ASSETS-LICENSES.md"));
            Assert.Contains("branding/", notices, StringComparison.Ordinal);
            Assert.Contains("GPL-3.0", notices, StringComparison.Ordinal);
        }
    }

    [Fact] // REQ-PUB-007
    public void REQ_PUB_007_the_tile_encoding_key_is_a_code_constant()
    {
        // The key the imported .3dn files of REQ-PROF-030 are read with is interoperability
        // data, not an asset: it lives in the code and no file is consulted.
        Assert.Equal(0x000708FACD884A5AUL, TilePlaneCipher.Key);
        Assert.Equal(26, TilePlaneCipher.Rotation);

        // And it is still the bijection a round trip needs.
        foreach (ulong plane in (ulong[])[0UL, 1UL, 0xDEADBEEFCAFEF00DUL, ulong.MaxValue])
        {
            Assert.Equal(plane, TilePlaneCipher.Decode(TilePlaneCipher.Encode(plane)));
        }
    }

    [Fact] // REQ-PUB-005
    public void REQ_PUB_005_every_interface_event_yields_a_non_silent_synthesised_sound()
    {
        SynthAudioLibrary library = new(48_000);

        foreach (UiSound sound in Enum.GetValues<UiSound>())
        {
            WaveAudio? clip = library.Sound(sound);
            Assert.NotNull(clip);
            Assert.Equal(1, clip.Channels);
            Assert.Equal(48_000, clip.SampleRate);

            // Non-silent, and roughly the length the table of SPEC-11 §2 gives.
            Assert.True(Peak(clip.Samples) > 0.05f, $"{sound} is silent");
            Assert.Equal(UiSoundSynth.Duration(sound), clip.Duration, 0.002);
        }

        // The four are distinct: no two events share a waveform (REQ-AUD-041).
        UiSound[] all = Enum.GetValues<UiSound>();
        for (int i = 0; i < all.Length; i++)
        {
            for (int j = i + 1; j < all.Length; j++)
            {
                Assert.NotEqual(library.Sound(all[i])!.Samples, library.Sound(all[j])!.Samples);
            }
        }
    }

    [Fact] // REQ-PUB-005 (revised 2026-10-21)
    public void REQ_PUB_005_the_publishable_build_carries_no_menu_music()
    {
        // The revision of REQ-PUB-005 removes the menu music entirely: the three WAV tracks of
        // SPEC-11 §2 were copies taken from the original build and are not replaced.
        SynthAudioLibrary library = new(48_000);
        Assert.Equal(0, library.TrackCount);
        Assert.Null(library.LoadTrack(0));
        Assert.Null(library.LoadTrack(-1));
    }

    [Fact] // REQ-PUB-005 (revised 2026-10-21)
    public void REQ_PUB_005_nothing_plays_in_the_menus_but_the_interface_sounds()
    {
        SynthAudioLibrary library = new(48_000);
        AudioMixer mixer = new(48_000, library, new Random(11));
        float[] stereo = new float[AudioMixer.BlockFrames * 2];

        // Not running, that is on the welcome screen or in the menus: the output stays silent,
        // whatever the music gain climbs to, because there is no track to start.
        for (int block = 0; block < 400; block++)
        {
            mixer.Fill(stereo);
            Assert.Equal(0f, Peak(stereo));
        }

        Assert.Equal(-1, mixer.CurrentTrack);
        Assert.Equal(0, mixer.TrackChanges);

        // The interface sounds are still there, and still audible (REQ-AUD-040, REQ-AUD-041).
        mixer.PlaySound(UiSound.Confirm);
        Assert.True(mixer.SoundPlaying);

        bool audible = false;
        for (int block = 0; block < 20 && !audible; block++)
        {
            mixer.Fill(stereo);
            audible = Peak(stereo) > 1e-5f;
        }

        Assert.True(audible, "the synthesised interface sounds must still be audible");
    }

    [Fact] // REQ-PUB-005 (revised 2026-10-21)
    public void REQ_PUB_005_the_background_music_setting_survives_but_is_no_longer_shown()
    {
        // The setting stays in the model so that an existing settings.json still reads — and
        // Lua still sees Setting.BGMusic — but the options window no longer offers it.
        Assert.Contains(
            SettingKeys.AudioBgMusic,
            SettingsCatalogue.All.Select(static descriptor => descriptor.Key));
        Assert.Contains(SettingKeys.AudioBgMusic, OptionsModel.ExcludedKeys);

        OptionsModel options = new();
        Assert.DoesNotContain(
            SettingKeys.AudioBgMusic,
            options.AllRows().Select(static row => row.Descriptor.Key));
    }

    [Fact] // REQ-PUB-006
    public void REQ_PUB_006_the_interface_font_comes_from_the_system()
    {
        (string? regular, string? bold) = SystemFonts.Locate();

        if (OperatingSystem.IsWindows())
        {
            // REQ-PUB-006 names Segoe UI and Arial; a Windows machine has at least one of them.
            Assert.NotNull(SystemFonts.Folder);
            Assert.NotNull(regular);
            Assert.True(File.Exists(regular));
            Assert.StartsWith(SystemFonts.Folder, regular, StringComparison.OrdinalIgnoreCase);

            if (bold is not null)
            {
                Assert.True(File.Exists(bold));
            }
        }

        // The file is read from the machine, never from the project: no font ships with it.
        Assert.True(SystemFonts.Supported);
    }

    [Fact] // REQ-PUB-006
    public void REQ_PUB_006_the_font_can_be_named_or_forced_back_to_the_built_in_one()
    {
        AppOptions options = AppOptions.Parse(["--font", "arial.ttf"]);
        Assert.Equal("arial.ttf", options.FontFile);
        Assert.False(options.BuiltInFont);

        Assert.True(AppOptions.Parse(["--builtin-font"]).BuiltInFont);

        // A name that exists in the system font folder resolves to its full path; an unknown one
        // falls through to the candidate list.
        if (OperatingSystem.IsWindows() && SystemFonts.Folder is { } folder)
        {
            string arial = Path.Combine(folder, "arial.ttf");
            if (File.Exists(arial))
            {
                Assert.Equal(arial, SystemFonts.Locate("arial.ttf").Regular);
            }
        }

        Assert.Equal(SystemFonts.Locate().Regular, SystemFonts.Locate("no-such-font-9f3a.ttf").Regular);
    }

    private UserDataPaths Paths() => UserDataPaths.Resolve(_root);

    private static float Peak(float[] samples)
    {
        float peak = 0f;
        foreach (float sample in samples)
        {
            peak = Math.Max(peak, Math.Abs(sample));
        }

        return peak;
    }

    /// <summary>Walks up from the test binaries looking for the solution file.</summary>
    private static string? FindRepositoryRoot()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            if (File.Exists(Path.Combine(at.FullName, "Zax.sln")))
            {
                return at.FullName;
            }

            at = at.Parent;
        }

        return null;
    }
}
