using Zax.App.Loop;
using Zax.App.Ui;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.App.Tests;

/// <summary>
/// AMD-053 seen from the interface: the four commands §3 point 17 adds to the model of
/// REQ-UI-205, the favourites and the search of REQ-UI-210 and REQ-UI-211 in the game/profile
/// box of REQ-UI-201, the import notification of REQ-UI-212, and the labels of §4.
/// </summary>
[Collection(ImGuiCollection.Name)]
public sealed class Amd053LibraryTests
{
    /// <summary>
    /// §3 point 17: the four commands are in the model under their stable names, so a key, a pad
    /// button or a test reaches them exactly like a menu entry (REQ-UI-205).
    /// </summary>
    [Fact] // REQ-UI-205, REQ-UI-210, REQ-UI-211, REQ-LOOP-076
    public void The_four_commands_join_the_model()
    {
        using UiHarness harness = new("Arkanoid");

        foreach (string command in (string[])
            [ZaxCommands.ToggleFavorite,
             ZaxCommands.FilterFavorites,
             ZaxCommands.SearchLibrary,
             ZaxCommands.ResumeLast])
        {
            Assert.NotNull(harness.Host.Commands.Find(command));
            Assert.False(string.IsNullOrWhiteSpace(harness.Host.Commands.Find(command)!.Label));
        }

        Assert.Equal("favorite.toggle", ZaxCommands.ToggleFavorite);
        Assert.Equal("library.filter_favorites", ZaxCommands.FilterFavorites);
        Assert.Equal("library.search", ZaxCommands.SearchLibrary);
        Assert.Equal("resume.last", ZaxCommands.ResumeLast);

        // With no resume point there is nothing to resume.
        Assert.False(harness.Host.ResumeAvailable);
        Assert.False(harness.Host.Commands.Find(ZaxCommands.ResumeLast)!.CanExecute(default));
    }

    /// <summary>
    /// REQ-UI-210: <c>favorite.toggle</c> marks the entry it designates, the mark is persisted
    /// in <c>library.json</c> at once, and <c>library.filter_favorites</c> is a checked state.
    /// </summary>
    [Fact] // REQ-UI-210
    public void REQ_UI_210_the_commands_mark_and_filter()
    {
        using UiHarness harness = new("Arkanoid", "Digdug");
        string id = harness.Library.Entries.First(static e => e.DisplayTitle == "Digdug").Id;

        harness.Run(ZaxCommands.ToggleFavorite, CommandArgs.Of(id));

        Assert.True(harness.Catalog.IsFavorite(id));
        Assert.Contains("\"favorite\": true", File.ReadAllText(harness.Paths.LibraryFile), StringComparison.Ordinal);

        harness.Run(ZaxCommands.ToggleFavorite, CommandArgs.Of(id));
        Assert.False(harness.Catalog.IsFavorite(id));

        Assert.False(harness.Host.FavoritesOnly);
        harness.Run(ZaxCommands.FilterFavorites);
        Assert.True(harness.Host.FavoritesOnly);
        Assert.True(harness.Host.Commands.Find(ZaxCommands.FilterFavorites)!.IsChecked);
    }

    /// <summary>
    /// REQ-UI-210 and REQ-UI-211 in the box of REQ-UI-201: the search filters as it is typed,
    /// ignoring case and diacritics, and combines with the favourites filter.
    /// </summary>
    [Fact] // REQ-UI-201, REQ-UI-210, REQ-UI-211
    public void REQ_UI_211_the_chooser_searches_and_filters()
    {
        using UiHarness harness = new("Super Mario Bros.", "Mario Bros.", "Pokémon", "Arkanoid");
        ProfileChooserModel chooser = harness.Ui.Chooser;

        chooser.Search = "mario";
        Assert.Equal(
            ["Mario Bros.", "Super Mario Bros."],
            chooser.Filtered.Select(static choice => choice.Title).Order(StringComparer.Ordinal));

        chooser.Search = "pokemon";
        Assert.Equal(["Pokémon"], chooser.Filtered.Select(static choice => choice.Title));

        chooser.Search = string.Empty;
        Assert.Equal(harness.Library.Entries.Count, chooser.Filtered.Count);

        string id = harness.Library.Entries.First(static e => e.DisplayTitle == "Mario Bros.").Id;
        harness.Run(ZaxCommands.ToggleFavorite, CommandArgs.Of(id));

        chooser.FavoritesOnly = true;
        chooser.Search = "mario";
        Assert.Equal(["Mario Bros."], chooser.Filtered.Select(static choice => choice.Title));
        Assert.True(chooser.Filtered[0].Favorite);
    }

    /// <summary>
    /// REQ-UI-212: an accepted import names the profile's title and, when it is not empty, its
    /// author; an empty author is left out of the notification.
    /// </summary>
    [Fact] // REQ-UI-212
    public void REQ_UI_212_an_accepted_import_names_the_title_and_the_author()
    {
        using UiLanguageScope english = new(UiLanguage.English);
        using UiHarness harness = new("Arkanoid");

        string both = WriteProfile(harness, "both.3dn", "aaaaaaaaaaaa", "Foo", "Bar");
        UserProfile imported = harness.Catalog.ImportProfile(both).Profile!;

        Assert.Equal("Foo", imported.Title);
        Assert.Equal("Bar", imported.Header.Author);
        Assert.Equal(
            "Profile imported: Foo, by Bar",
            UiText.ProfileImported(imported.Title, imported.Header.Author));

        string alone = WriteProfile(harness, "alone.3dn", "bbbbbbbbbbbb", "Foo", null);
        UserProfile second = harness.Catalog.ImportProfile(alone).Profile!;

        // An empty author is left out of the notification.
        Assert.Equal(
            "Profile imported: Foo",
            UiText.ProfileImported(second.Title, second.Header.Author));

        // And the title falls back to the file name without its extension (REQ-PERS-034).
        string plain = WriteProfile(harness, "Baz.3dn", "cccccccccccc", null, null);
        Assert.Equal("Baz", harness.Catalog.ImportProfile(plain).Profile!.Title);
    }

    /// <summary>
    /// REQ-UI-212: a refused import shows the text of REQ-UI-084 that matches the cause, at
    /// normal severity and for four seconds (REQ-PERS-041 point 1).
    /// </summary>
    [Fact] // REQ-UI-212, REQ-UI-084, REQ-PERS-041
    public void REQ_UI_212_a_refused_import_shows_the_text_of_its_cause()
    {
        using UiHarness harness = new("Arkanoid");

        string old = WriteProfile(harness, "old.3dn", "dddddddddddd", null, null, version: "1");
        ProfileImportResult refused = harness.Catalog.ImportProfile(old);

        Assert.Equal(ProfileImportOutcome.Rejected, refused.Outcome);
        Assert.Equal("3dn format version 1  is not supported", refused.Message);

        // REQ-UI-084 with REQ-PERS-041 point 1: normal severity, four seconds.
        harness.Host.Notifications.Show(refused.Message);
        Assert.Equal(NotificationSeverity.Normal, harness.Host.Notifications.Current!.Value.Severity);
        Assert.Equal(NotificationCenter.DefaultSeconds, harness.Host.Notifications.Current!.Value.Seconds);
    }

    /// <summary>
    /// AMD-053 §4 with AMD-035: the five labels of the table exist in both languages, with the
    /// exact wordings the amendment fixes.
    /// </summary>
    [Fact] // AMD-035, AMD-036
    public void The_five_labels_of_the_table_are_in_both_languages()
    {
        (string Key, string English, string French)[] labels =
        [
            (UiKeys.WelcomeResume, "Resume", "Reprendre"),
            (UiKeys.FieldFavorite, "Favorite", "Favori"),
            (UiKeys.FieldFavoritesOnly, "Favorites only", "Favoris seulement"),
            ("option.Favorites first", "Favorites first", "Favoris d'abord"),
            (UiKeys.FieldSearch, "Search", "Rechercher"),
        ];

        foreach ((string key, string english, string french) in labels)
        {
            Assert.Equal(english, UiStrings.EnglishTable[key]);
            Assert.Equal(french, UiStrings.FrenchTable[key]);
        }
    }

    /// <summary>
    /// REQ-LOOP-076: the welcome screen of REQ-UI-207 lays out with the <c>Resume</c> entry when
    /// one is offered, and without it when none is — one real Dear ImGui frame either way.
    /// </summary>
    [Fact] // REQ-LOOP-076, REQ-UI-207
    public void REQ_LOOP_076_the_welcome_screen_draws_the_resume_entry()
    {
        using ImGuiHarness imgui = new();
        using UiHarness harness = new("Arkanoid");

        Assert.False(harness.Host.ResumeAvailable);
        imgui.Frames(2, () => harness.Ui.Draw(1f / 60f));

        // A resume point is written by an exit made during a game.
        harness.Launch(harness.TwoDimensionalModeId);
        harness.Host.Shutdown();

        Assert.True(harness.Host.Slots.Find(Persistence.SaveSlotIndex.ResumeSlotId)!.IsOccupied);
        imgui.Frames(2, () => harness.Ui.Draw(1f / 60f));
    }

    private static string WriteProfile(
        UiHarness harness,
        string name,
        string? id,
        string? title,
        string? author,
        string? version = null)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        profile.Author = author;
        if (version is not null)
        {
            profile.VersionText = version;
        }

        string path = Path.Combine(harness.Paths.Root, name);
        File.WriteAllBytes(path, ProfileFormat.Save(profile));
        return path;
    }
}
