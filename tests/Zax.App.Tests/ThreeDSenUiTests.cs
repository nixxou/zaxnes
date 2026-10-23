using Zax.App.Loop;
using Zax.App.Ui;
using Zax.Profiles;
using Zax.Profiles.Import;
using Zax.Profiles.Library;
using Zax.Profiles.Tests;

namespace Zax.App.Tests;

/// <summary>
/// « Fichier → Importer depuis 3dSen… » of AMD-030 §2: the folder picker, the summary message and
/// the library that refreshes without a restart.
/// </summary>
public sealed class ThreeDSenUiTests
{
    [Fact] // AMD-030 §2: « ouvre un sélecteur de dossier »
    public void The_menu_entry_opens_the_picker_in_folder_mode()
    {
        using UiHarness harness = new("Arkanoid");

        harness.Run(ZaxCommands.ImportThreeDSen);

        Assert.True(harness.Ui.State.FileBrowser);
        Assert.True(harness.Ui.Browser.AllowFolders);
        Assert.Equal(UiText.ImportThreeDSenTitle, harness.Ui.Browser.Title);
        Assert.Equal(UiText.ImportThreeDSenHint, harness.Ui.Browser.Hint);

        // §3: the bundle can be named directly, so the dialog offers its extension.
        Assert.Equal(
            FileBrowserModel.ThreeDSenFilters.Select(static f => f.Label),
            harness.Ui.Browser.Filters.Select(static f => f.Label));
        Assert.Equal(["bundle"], harness.Ui.Browser.Filter);
    }

    [Fact] // AMD-030 §2: the dialog answers with the folder being shown.
    public void The_picker_answers_with_a_folder()
    {
        using UiHarness harness = new();
        FileBrowserModel browser = new();
        browser.Open(
            UiText.ImportThreeDSenTitle,
            FileBrowserModel.ThreeDSenFilters,
            harness.Paths.Root,
            allowFolders: true);

        Assert.True(browser.CanConfirm);
        Assert.True(browser.Confirm());
        Assert.Equal(harness.Paths.Root, browser.Result);
    }

    [Fact] // AMD-030 §2: a selected sub-folder is the answer too.
    public void A_selected_folder_is_the_answer()
    {
        using UiHarness harness = new();
        string child = Path.Combine(harness.Paths.Root, "3dSen");
        Directory.CreateDirectory(child);

        FileBrowserModel browser = new();
        browser.Open(
            UiText.ImportThreeDSenTitle,
            FileBrowserModel.ThreeDSenFilters,
            harness.Paths.Root,
            allowFolders: true);
        browser.Selected = browser.Entries.Single(
            entry => entry.Kind == FileEntryKind.Folder && entry.Name == "3dSen");

        Assert.True(browser.Confirm());
        Assert.Equal(child, browser.Result);
    }

    [Fact] // REQ-UI-104: the ROM dialog still refuses to answer with a folder.
    public void The_rom_dialog_still_only_answers_with_a_file()
    {
        using UiHarness harness = new();
        FileBrowserModel browser = new();
        browser.Open(UiText.OpenRomTitle, FileBrowserModel.RomFilters, harness.Paths.Root);

        Assert.False(browser.AllowFolders);
        Assert.False(browser.CanConfirm);
        Assert.False(browser.Confirm());
        Assert.Null(browser.Result);
    }

    [Fact] // AMD-030 §2: « un message donne le nombre de profils importés » + « apparaissent aussitôt »
    public void The_import_fills_the_library_and_reports_the_counts()
    {
        using UiHarness harness = new("Arkanoid");
        string install = WriteInstallation(
            harness,
            [("Pinball", Profile("PINBALL00001", "Pinball")),
             ("Tetris", Profile("TETRIS000001", "Tetris"))]);

        harness.Run(ZaxCommands.ImportThreeDSen, CommandArgs.Of(install));

        // AMD-036 §1: the notification is in the language of the interface, English by default.
        Assert.Equal(
            "3dSen: 2 profile(s) imported, 0 already present, 0 failed.",
            harness.Host.Notifications.Current!.Value.Text);
        Assert.Equal(NotificationSeverity.Normal, harness.Host.Notifications.Current!.Value.Severity);

        // The profiles are in the user's folder, under their own name.
        Assert.True(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "Pinball.3dn")));
        Assert.True(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "Tetris.3dn")));

        // §5: the title and the identifier shown are the profile's own.
        GameLibraryEntry entry = harness.Library.Find("PINBALL00001")!;
        Assert.Equal("Pinball", entry.DisplayTitle);
        Assert.Equal(GameProfileSource.User, entry.Source);
        Assert.NotNull(harness.Library.Find("TETRIS000001"));

        // The chooser of REQ-UI-201 is rebuilt, so the new profiles are pickable at once.
        Assert.Contains(harness.Ui.Chooser.All, row => row.Title == "Pinball");
    }

    [Fact] // AMD-030 §2: a second import adds nothing.
    public void A_second_import_reports_the_profiles_as_already_present()
    {
        using UiHarness harness = new("Arkanoid");
        string install = WriteInstallation(harness, [("Pinball", Profile("PINBALL00001", "Pinball"))]);

        harness.Run(ZaxCommands.ImportThreeDSen, CommandArgs.Of(install));
        harness.Run(ZaxCommands.ImportThreeDSen, CommandArgs.Of(install));

        Assert.Equal(
            "3dSen: 0 profile(s) imported, 1 already present, 0 failed.",
            harness.Host.Notifications.Current!.Value.Text);
        Assert.Single(Directory.GetFiles(harness.Paths.ProfilesDirectory, "*.3dn"));
    }

    [Fact] // AMD-030 §3: « l'interface le dit clairement et rappelle le chemin attendu »
    public void An_installation_without_a_bundle_is_reported_as_an_error()
    {
        using UiHarness harness = new("Arkanoid");
        string install = Path.Combine(harness.Paths.Root, "not-3dsen");
        Directory.CreateDirectory(install);

        harness.Run(ZaxCommands.ImportThreeDSen, CommandArgs.Of(install));

        string message = harness.Host.Notifications.Current!.Value.Text;
        Assert.Equal(NotificationSeverity.Error, harness.Host.Notifications.Current!.Value.Severity);
        Assert.Contains("no 3dSen profile bundle was found", message, StringComparison.Ordinal);
        Assert.Contains(ThreeDSenInstallation.ExpectedLayout, message, StringComparison.Ordinal);
        Assert.False(Directory.Exists(harness.Paths.ProfilesDirectory)
            && Directory.GetFiles(harness.Paths.ProfilesDirectory, "*.3dn").Length > 0);
    }

    [Fact] // AMD-030 §5: a corrupt bundle is a message, not a crash, and writes nothing.
    public void A_corrupt_bundle_is_reported_as_an_error()
    {
        using UiHarness harness = new("Arkanoid");
        string install = WriteInstallation(harness, [("Pinball", Profile("PINBALL00001", "Pinball"))]);
        string bundle = Directory.GetFiles(install, "*.bundle", SearchOption.AllDirectories)[0];
        File.WriteAllBytes(bundle, File.ReadAllBytes(bundle)[..64]);

        harness.Run(ZaxCommands.ImportThreeDSen, CommandArgs.Of(install));

        Assert.Equal(NotificationSeverity.Error, harness.Host.Notifications.Current!.Value.Severity);
        Assert.False(Directory.Exists(harness.Paths.ProfilesDirectory)
            && Directory.GetFiles(harness.Paths.ProfilesDirectory, "*.3dn").Length > 0);
    }

    /// <summary>Writes a synthetic 3dSen installation inside the harness's temporary root.</summary>
    private static string WriteInstallation(
        UiHarness harness,
        IReadOnlyList<(string Name, byte[] Script)> assets)
    {
        string install = Path.Combine(harness.Paths.Root, "3dSen");
        string directory = Path.Combine(
            install, "3dSen_Data", "StreamingAssets", "aa", "StandaloneWindows64");
        Directory.CreateDirectory(directory);

        File.WriteAllBytes(
            Path.Combine(
                directory,
                ThreeDSenInstallation.BundlePrefix + "feedface"
                + ThreeDSenInstallation.BundleExtension),
            SyntheticUnityBundle.Build(
                assets,
                new SyntheticBundleOptions
                {
                    BlockCompression = UnityCompression.Lz4,
                    BlockSize = 128 * 1024,
                    InfoAtEnd = true,
                }));

        return install;
    }

    private static byte[] Profile(string id, string title)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        return ProfileFormat.Save(profile);
    }
}
