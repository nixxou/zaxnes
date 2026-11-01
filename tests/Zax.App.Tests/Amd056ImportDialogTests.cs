using Zax.App.Ui;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.App.Tests;

/// <summary>
/// AMD-056: the import of a profile as 3dSen 2.0 presents it — the texts of REQ-UI-084 reworded
/// by §2, and the four dialogs of REQ-UI-214 with the one case that shows nothing at all.
/// </summary>
/// <remarks>
/// Each criterion is checked twice over: once on <see cref="ProfileImportDialogs.Decide"/>, the
/// pure decision the amendment's table describes, and once on the whole path
/// <c>ZaxUi.ImportProfile</c> takes — inspect, dialog, answer, write — which is what tells
/// whether the writing really waits for the answer. No ImGui frame is needed for either.
/// </remarks>
[Collection(ImGuiCollection.Name)]
public sealed class Amd056ImportDialogTests
{
    /// <summary>
    /// REQ-UI-214, first criterion: importing any text file named <c>junk.3dn</c> puts up
    /// <c>Cannot import this file</c>, whose body is the file <b>name</b> then the third text of
    /// REQ-UI-084 — which carries the full path itself — and the library does not change.
    /// </summary>
    [Fact] // REQ-UI-214, REQ-UI-084, REQ-UI-212
    public void REQ_UI_214_a_junk_file_is_refused_with_the_text_of_its_cause()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Arkanoid");

        int before = harness.Library.Entries.Count;
        string junk = Path.Combine(harness.Paths.Root, "junk.3dn");
        File.WriteAllText(junk, "not a profile, just some text");

        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(junk);
        ProfileImportDialog dialog = ProfileImportDialogs.Decide(plan);

        Assert.Equal(ProfileImportOutcome.Rejected, plan.Outcome);
        Assert.Equal(ProfileImportDialogKind.Refused, dialog.Kind);
        Assert.Equal("Cannot import this file", dialog.Title);
        Assert.Equal(
            $"junk.3dn: {junk} has an obsolete or wrong profile format. "
            + "This profile is not compatible with the current version of Zaxnes.",
            dialog.Body);

        // One button, « OK », and nothing behind it.
        Assert.Equal("OK", dialog.PrimaryLabel);
        Assert.Null(dialog.CancelLabel);
        Assert.False(dialog.NeedsConfirmation);

        // The whole path: the box is up with those words, the library is untouched, and the
        // refusal is logged (REQ-PERS-041).
        harness.Ui.ImportProfile(junk);

        Assert.True(harness.Ui.Confirm.IsOpen);
        Assert.True(harness.Ui.State.Dialog);
        Assert.Equal("Cannot import this file", harness.Ui.Confirm.Title);
        Assert.Equal(dialog.Body, harness.Ui.Confirm.Message);
        Assert.False(harness.Ui.Confirm.HasCancel);
        Assert.Equal("OK", harness.Ui.Confirm.ConfirmLabel);
        Assert.Equal(before, harness.Library.Entries.Count);
        Assert.Contains(harness.Log, line => line.Contains("was refused", StringComparison.Ordinal));

        // No notification: REQ-UI-214 replaces the one REQ-PERS-041 point 1 used to raise.
        Assert.Null(harness.Host.Notifications.Current);

        // Its lone button only closes.
        harness.Ui.Confirm.Confirm();
        Assert.False(harness.Ui.Confirm.IsOpen);
        Assert.Equal(before, harness.Library.Entries.Count);
    }

    /// <summary>
    /// REQ-UI-214, second criterion: a profile whose <c>Ver</c> is <c>"v1"</c> is refused with a
    /// body ending in <c>3dn format version 1  is not supported</c> — the version sanitised as
    /// REQ-PROF-021 sanitises it (AMD-056 §2), and the two spaces of the original.
    /// </summary>
    [Fact] // REQ-UI-214, REQ-UI-084, REQ-PROF-021
    public void REQ_UI_214_a_refused_version_is_named_sanitised()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Arkanoid");

        string old = Write(harness, "old.3dn", "dddddddddddd", "Old", version: "v1");
        ProfileImportDialog dialog =
            ProfileImportDialogs.Decide(harness.Catalog.InspectProfileImport(old));

        Assert.Equal(ProfileImportDialogKind.Refused, dialog.Kind);
        Assert.EndsWith("3dn format version 1  is not supported", dialog.Body, StringComparison.Ordinal);
        Assert.Equal("old.3dn: 3dn format version 1  is not supported", dialog.Body);

        // AMD-056 §3, divergence (D): the refusal happens at import time, as REQ-UI-212 has it,
        // and not at launch time as the 2.0 does. Nothing was copied.
        Assert.False(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "old.3dn")));
    }

    /// <summary>
    /// REQ-UI-214, third criterion: a file carrying the identifier of a built-in profile of
    /// title « Foo » puts up <c>Already in Zaxnes</c> with the body the table writes, and
    /// nothing is copied.
    /// </summary>
    [Fact] // REQ-UI-214, REQ-PERS-030, REQ-PERS-041
    public void REQ_UI_214_a_built_in_identifier_imports_nothing()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Foo");

        // The harness numbers its built-in profiles g0, g1…; « Foo » is the first.
        GameLibraryEntry builtIn = harness.Library.Entries.Single(
            static entry => entry.Title == "Foo");
        Assert.Equal(GameProfileSource.BuiltIn, builtIn.Source);

        int before = harness.Library.Entries.Count;
        string path = Write(harness, "x.3dn", builtIn.Id, "Foo");

        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(path);
        ProfileImportDialog dialog = ProfileImportDialogs.Decide(plan);

        Assert.Equal(ProfileImportOutcome.AlreadyBuiltIn, plan.Outcome);
        Assert.Equal(ProfileImportDialogKind.AlreadyBuiltIn, dialog.Kind);
        Assert.Equal("Already in Zaxnes", dialog.Title);
        Assert.Equal("Foo is a built-in profile, so x.3dn was not imported.", dialog.Body);
        Assert.Equal("OK", dialog.PrimaryLabel);
        Assert.Null(dialog.CancelLabel);

        harness.Ui.ImportProfile(path);

        Assert.True(harness.Ui.Confirm.IsOpen);
        Assert.Equal("Already in Zaxnes", harness.Ui.Confirm.Title);
        Assert.False(harness.Ui.Confirm.HasCancel);
        Assert.Null(harness.Host.Notifications.Current);

        // Nothing copied, no entry gained, and the fact logged (REQ-PERS-041).
        Assert.False(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "x.3dn")));
        Assert.Equal(before, harness.Library.Entries.Count);
        Assert.Contains(
            harness.Log,
            line => line.Contains("identifier of a built-in profile", StringComparison.Ordinal));
    }

    /// <summary>
    /// REQ-UI-214, third criterion again: when the file carries no title of its own,
    /// « &lt;titre&gt; » falls back to the file <b>name with its extension</b> — not to the
    /// name without it, which is what the library entry would have taken (REQ-PERS-034).
    /// </summary>
    [Fact] // REQ-UI-214
    public void REQ_UI_214_a_built_in_identifier_without_a_title_is_named_by_its_file()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Foo");

        string id = harness.Library.Entries.Single(static entry => entry.Title == "Foo").Id;
        string path = Write(harness, "nameless.3dn", id, title: null);

        ProfileImportDialog dialog =
            ProfileImportDialogs.Decide(harness.Catalog.InspectProfileImport(path));

        Assert.Equal(
            "nameless.3dn is a built-in profile, so nameless.3dn was not imported.",
            dialog.Body);
    }

    /// <summary>
    /// REQ-UI-214, fourth criterion: a new version of an existing <b>user</b> profile asks
    /// <c>Replace &lt;name&gt;?</c>; <c>Cancel</c> leaves the file and the entry intact, and
    /// <c>Replace</c> replaces the file and keeps the ROM, the cover and the slots
    /// (REQ-PERS-041 point 3).
    /// </summary>
    [Fact] // REQ-UI-214, REQ-PERS-041
    public void REQ_UI_214_replacing_a_user_profile_is_confirmed_first()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Arkanoid");

        // A user profile is in the library first, under its own file.
        string first = Write(harness, "mine.3dn", "eeeeeeeeeeee", "Mine");
        Assert.Equal(ProfileImportOutcome.Added, harness.Catalog.ImportProfile(first).Outcome);

        string installed = Path.Combine(harness.Paths.ProfilesDirectory, "mine.3dn");
        Assert.True(File.Exists(installed));

        // REQ-UI-214 names the entry by its displayed title, a rename included.
        GameLibraryEntry entry = harness.Library.Entries.Single(
            static e => e.Source == GameProfileSource.User);
        entry.Title = "My Own Name";
        entry.RomPath = @"C:\roms\mine.nes";

        // A second file, same identifier, different content and a different name.
        string second = Write(harness, "mine-v2.3dn", "eeeeeeeeeeee", "Mine v2");

        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(second);
        ProfileImportDialog dialog = ProfileImportDialogs.Decide(plan);

        Assert.Equal(ProfileImportOutcome.Replaced, plan.Outcome);
        Assert.Equal(ProfileImportDialogKind.ConfirmReplacement, dialog.Kind);
        Assert.Equal("Replace My Own Name?", dialog.Title);
        Assert.Equal(
            "The profile will be replaced with the one in mine-v2.3dn. "
            + "The ROM, cover and save states will be kept.",
            dialog.Body);
        Assert.Equal("Replace", dialog.PrimaryLabel);
        Assert.Equal("Cancel", dialog.CancelLabel);
        Assert.True(dialog.Destructive);
        Assert.True(dialog.NeedsConfirmation);

        // « Cancel » changes nothing: the old file is still there and the new one is not.
        harness.Ui.ImportProfile(second);
        Assert.True(harness.Ui.Confirm.IsOpen);
        Assert.True(harness.Ui.Confirm.HasCancel);
        Assert.True(harness.Ui.Confirm.Destructive);

        harness.Ui.Confirm.Close();

        Assert.False(harness.Ui.Confirm.IsOpen);
        Assert.True(File.Exists(installed));
        Assert.False(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "mine-v2.3dn")));
        Assert.Equal("My Own Name", harness.Library.Find(entry.Id)!.Title);

        // « Replace » does it: the old file goes, the new one arrives, and the ROM is kept.
        harness.Ui.ImportProfile(second);
        Assert.True(harness.Ui.Confirm.IsOpen);
        harness.Ui.Confirm.Confirm();

        Assert.False(harness.Ui.Confirm.IsOpen);
        Assert.False(File.Exists(installed));
        Assert.True(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "mine-v2.3dn")));
        Assert.Equal(@"C:\roms\mine.nes", harness.Library.Find(entry.Id)!.RomPath);

        // A successful replacement shows nothing either (REQ-UI-214, last bullet).
        Assert.Null(harness.Host.Notifications.Current);
    }

    /// <summary>
    /// REQ-UI-214, fifth criterion: a profile of an unknown identifier shows neither
    /// notification nor dialog, the library gains one entry, and the imported entry becomes the
    /// selected one of the game/profile box of REQ-UI-201.
    /// </summary>
    [Fact] // REQ-UI-214, REQ-UI-201, REQ-UI-212
    public void REQ_UI_214_an_unknown_identifier_is_imported_in_silence()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Arkanoid");

        int before = harness.Library.Entries.Count;
        string path = Write(harness, "fresh.3dn", "ffffffffffff", "Fresh");

        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(path);
        Assert.Equal(ProfileImportOutcome.Added, plan.Outcome);
        Assert.False(ProfileImportDialogs.Decide(plan).IsShown);

        harness.Ui.ImportProfile(path);

        Assert.False(harness.Ui.Confirm.IsOpen);
        Assert.Null(harness.Host.Notifications.Current);
        Assert.Equal(before + 1, harness.Library.Entries.Count);
        Assert.Equal("Fresh", harness.Library.Find("ffffffffffff")!.Title);

        // The imported entry is the selection of the box of REQ-UI-201.
        Assert.Equal("ffffffffffff", harness.Ui.Chooser.SelectedId);

        // The fact is still logged (REQ-PERS-041).
        Assert.Contains(harness.Log, line => line.Contains("'Fresh' imported", StringComparison.Ordinal));
    }

    /// <summary>
    /// REQ-UI-214: « Si l'écriture échoue après la confirmation, le dialogue de refus s'affiche
    /// avec le message de l'erreur » — the error's own message, not the third text of
    /// REQ-UI-084, which would claim a readable and well-formed file was malformed.
    /// </summary>
    [Fact] // REQ-UI-214
    public void REQ_UI_214_a_write_that_fails_shows_the_message_of_its_error()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Arkanoid");

        string path = Write(harness, "blocked.3dn", "999999999999", "Blocked");
        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(path);

        // A file where the profile folder must be: creating the folder cannot succeed.
        Directory.CreateDirectory(harness.Paths.Root);
        if (Directory.Exists(harness.Paths.ProfilesDirectory))
        {
            Directory.Delete(harness.Paths.ProfilesDirectory, recursive: true);
        }

        File.WriteAllText(harness.Paths.ProfilesDirectory, "in the way");

        ProfileImportResult result = harness.Catalog.ApplyProfileImport(plan);

        Assert.Equal(ProfileImportOutcome.Rejected, result.Outcome);
        Assert.NotEmpty(result.Message);

        // The refusal dialog carries that message, and not a cause of REQ-UI-084.
        ProfileImportDialog dialog = ProfileImportDialogs.Refused("blocked.3dn", result.Message);
        Assert.Equal("Cannot import this file", dialog.Title);
        Assert.StartsWith("blocked.3dn: ", dialog.Body, StringComparison.Ordinal);
        Assert.DoesNotContain("obsolete or wrong profile format", dialog.Body, StringComparison.Ordinal);
    }

    /// <summary>
    /// AMD-056 §2 with AMD-036 §1: the titles, bodies and buttons of the three dialogs are
    /// entries of the text table, so in <c>fr</c> none of them is still English.
    /// </summary>
    [Fact] // REQ-UI-214, AMD-036
    public void REQ_UI_214_the_dialogs_follow_the_interface_language()
    {
        using UiHarness harness = new("Foo");

        string id = harness.Library.Entries.Single(static entry => entry.Title == "Foo").Id;
        string path = Write(harness, "x.3dn", id, "Foo");
        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(path);

        using UiLanguageScope french = new(UiLanguage.French);
        ProfileImportDialog dialog = ProfileImportDialogs.Decide(plan);

        Assert.Equal("Déjà dans Zaxnes", dialog.Title);
        Assert.Equal("Foo est un profil embarqué ; x.3dn n'a pas été importé.", dialog.Body);

        // The refusal body carries a cause produced in English by the profile reader, which is
        // translated where it is shown.
        ProfileImportDialog refused = ProfileImportDialogs.Refused(
            "junk.3dn", ProfileImportMessages.NotA3dnFile);

        Assert.Equal("Impossible d'importer ce fichier", refused.Title);
        Assert.Equal("junk.3dn : Ce fichier n'est pas un fichier 3dn", refused.Body);

        // And the two buttons of the replacement.
        Assert.Equal("Remplacer", UiStrings.Get(UiKeys.ButtonReplace));
        Assert.Equal("Annuler", UiStrings.Get(UiKeys.ButtonCancel));
    }

    /// <summary>
    /// AMD-056 REQ-UI-214: <see cref="UserProfileFolder.Inspect"/> writes <b>nothing</b>. The
    /// confirmation would be worthless otherwise.
    /// </summary>
    [Fact] // REQ-UI-214
    public void REQ_UI_214_inspecting_writes_nothing()
    {
        using UiHarness harness = new("Arkanoid");

        string path = Write(harness, "quiet.3dn", "111111111111", "Quiet");
        int before = harness.Library.Entries.Count;

        ProfileImportPlan plan = harness.Catalog.InspectProfileImport(path);

        Assert.True(plan.Writes);
        Assert.Equal(before, harness.Library.Entries.Count);
        Assert.False(File.Exists(Path.Combine(harness.Paths.ProfilesDirectory, "quiet.3dn")));

        // Inspecting twice is as harmless as inspecting once.
        Assert.Equal(plan, harness.Catalog.InspectProfileImport(path));
    }

    private static string Write(
        UiHarness harness,
        string name,
        string? id,
        string? title,
        string? version = null)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        if (version is not null)
        {
            profile.VersionText = version;
        }

        string path = Path.Combine(harness.Paths.Root, name);
        File.WriteAllBytes(path, ProfileFormat.Save(profile));
        return path;
    }
}
