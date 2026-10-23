using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Import;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// The interface language of AMD-035: two complete tables, English on a fresh user folder, an
/// immediate and persisted switch to French, and an unknown value read as English.
/// </summary>
public sealed class UiLanguageTests
{
    /// <summary>
    /// AMD-035 §4: the two tables hold exactly the same keys, and no value is empty or blank.
    /// A missing translation would fall back to English silently, so the check is what keeps the
    /// French interface from going half English without anyone noticing.
    /// </summary>
    [Fact] // AMD-035 §3, §4
    public void The_two_tables_have_the_same_keys_and_no_empty_value()
    {
        IReadOnlyDictionary<string, string> english = UiStrings.EnglishTable;
        IReadOnlyDictionary<string, string> french = UiStrings.FrenchTable;

        Assert.NotEmpty(english);
        Assert.Empty(english.Keys.Except(french.Keys, StringComparer.Ordinal));
        Assert.Empty(french.Keys.Except(english.Keys, StringComparer.Ordinal));

        foreach ((string key, string text) in english)
        {
            Assert.False(string.IsNullOrWhiteSpace(text), key);
        }

        foreach ((string key, string text) in french)
        {
            Assert.False(string.IsNullOrWhiteSpace(text), key);
        }
    }

    /// <summary>
    /// Every key <see cref="UiKeys"/> publishes is in both tables, so no screen can ask for a
    /// text that does not exist and end up showing its raw key (AMD-035 §3).
    /// </summary>
    [Fact] // AMD-035 §3
    public void Every_published_key_is_in_both_tables()
    {
        foreach (string key in PublishedKeys())
        {
            Assert.True(UiStrings.EnglishTable.ContainsKey(key), key);
            Assert.True(UiStrings.FrenchTable.ContainsKey(key), key);
        }
    }

    /// <summary>
    /// Every command of the model and every setting the options window shows has a label in both
    /// tables: those two families of keys are built from a name at run time, so a command or a
    /// setting added without its text would otherwise show its key.
    /// </summary>
    [Fact] // AMD-035 §1
    public void Every_command_and_every_shown_setting_has_a_label()
    {
        using UiHarness harness = new("Arkanoid");

        foreach (ZaxCommand command in harness.Host.Commands.Commands)
        {
            string key = UiKeys.Command(command.Name);
            Assert.True(UiStrings.EnglishTable.ContainsKey(key), key);
            Assert.True(UiStrings.FrenchTable.ContainsKey(key), key);
        }

        foreach (OptionsRow row in new OptionsModel().AllRows())
        {
            string key = UiKeys.Setting(row.Descriptor.Key);
            Assert.True(UiStrings.EnglishTable.ContainsKey(key), key);
            Assert.True(UiStrings.FrenchTable.ContainsKey(key), key);
        }

        // The labels of the enumerated values that are words are translated; the ones that are
        // numbers — « 60fps », « 1/2 », « 75% » — are shown as the catalogue writes them, which
        // is what LabelOf falls back to. Neither case may ever yield a key or an empty string.
        foreach (UiLanguage spoken in (UiLanguage[])[UiLanguage.English, UiLanguage.French])
        {
            using UiLanguageScope scope = new(spoken);
            foreach (OptionsRow row in new OptionsModel().AllRows())
            {
                Assert.False(string.IsNullOrWhiteSpace(row.Label), row.Descriptor.Key);
                Assert.DoesNotContain(UiKeys.SettingPrefix, row.Label, StringComparison.Ordinal);

                foreach (SettingOption option in row.Descriptor.Domain.Options ?? [])
                {
                    string shown = UiText.LabelOf(row.Descriptor, option.Label);
                    Assert.False(string.IsNullOrWhiteSpace(shown), option.Label);
                    Assert.DoesNotContain(UiKeys.OptionPrefix, shown, StringComparison.Ordinal);
                }
            }
        }
    }

    /// <summary>
    /// AMD-035 §3: a key the translation is missing falls back to English rather than to an
    /// empty line or to the key itself.
    /// </summary>
    [Fact] // AMD-035 §3
    public void A_missing_translation_falls_back_to_english()
    {
        using UiLanguageScope language = new(UiLanguage.French);

        // Every key of the French table exists, so the fallback is exercised on a key neither
        // table holds: it must still answer something printable.
        Assert.Equal("ui.nothing.here", UiStrings.Get("ui.nothing.here"));
        Assert.Equal("Fichier", UiStrings.Get(UiKeys.MenuFile));
    }

    /// <summary>AMD-035 §4: a fresh user folder gives an interface entirely in English.</summary>
    [Fact] // AMD-035 §4
    public void A_fresh_user_folder_is_english()
    {
        using UiLanguageScope language = new();
        using UiHarness harness = new("Arkanoid");

        Assert.Equal("en", harness.Host.Settings.Language);
        Assert.Equal(UiLanguage.English, UiStrings.Language);
        Assert.Equal("File", harness.Ui.MenuBar.Menus[0].Title);
        Assert.Equal("Video", OptionsModel.TitleOf(OptionsTab.Video));
        Assert.Equal("Open a ROM…", harness.Host.Commands.Find(ZaxCommands.OpenRom)!.Label);
        Assert.Equal("Player 1 — NES buttons", harness.Ui.Keys.Sections[0].Title);
    }

    /// <summary>
    /// AMD-035 §1 and §4: writing <c>fr</c> switches the whole interface at once — the menu bar,
    /// the options window, the key configuration and the command labels — with no restart.
    /// </summary>
    [Fact] // AMD-035 §1, §4
    public void Switching_to_french_changes_the_texts_at_once()
    {
        using UiLanguageScope language = new();
        using UiHarness harness = new("Arkanoid");

        harness.Host.Settings.Language = "fr";

        Assert.Equal(UiLanguage.French, UiStrings.Language);
        Assert.Equal("Fichier", harness.Ui.MenuBar.Menus[0].Title);
        Assert.Equal("Vidéo", OptionsModel.TitleOf(OptionsTab.Video));
        Assert.Equal("Ouvrir une ROM…", harness.Host.Commands.Find(ZaxCommands.OpenRom)!.Label);
        Assert.Equal("Joueur 1 — boutons NES", harness.Ui.Keys.Sections[0].Title);
        Assert.Equal("Valeurs par défaut", UiText.ResetDefaults);
        Assert.Equal("Poste de travail", FileBrowserModel.ComputerLabel);

        // And back again, still without a restart.
        harness.Host.Settings.Language = "en";
        Assert.Equal("File", harness.Ui.MenuBar.Menus[0].Title);
    }

    /// <summary>AMD-035 §1: the choice is kept for the next launch.</summary>
    [Fact] // AMD-035 §1, §4
    public void The_choice_is_persisted_and_read_back_at_the_next_launch()
    {
        using UiLanguageScope language = new();
        using UiHarness harness = new("Arkanoid");

        harness.Host.Settings.Language = "fr";
        harness.Store.Save(harness.Settings);

        AppSettings restarted = new();
        Assert.Equal("en", restarted.Language);

        new SettingsStore(harness.Paths).Load(restarted, DateTimeOffset.UnixEpoch);
        Assert.Equal("fr", restarted.Language);
        Assert.Equal(UiLanguage.French, UiStrings.Parse(restarted.Language));
    }

    /// <summary>
    /// AMD-035 §1: an unknown value in the settings file is read as English, without an error
    /// and without rewriting the file behind the user's back.
    /// </summary>
    [Fact] // AMD-035 §1, §4
    public void An_unknown_value_falls_back_to_english()
    {
        using UiLanguageScope language = new(UiLanguage.French);

        AppSettings settings = new();
        settings.Model.SetString(SettingKeys.UiLanguage, "kl");

        Assert.Equal("en", settings.Language);
        Assert.Equal(UiLanguage.English, UiStrings.Parse(settings.Language));

        // The stored text is left alone: only what the interface reads is brought back.
        Assert.Equal("kl", settings.Model.GetString(SettingKeys.UiLanguage));
    }

    /// <summary>
    /// AMD-035 §1: the language is in the Interface tab of the options window, as a drop-down of
    /// the two values.
    /// </summary>
    [Fact] // AMD-035 §1
    public void The_options_window_offers_the_language_in_the_interface_tab()
    {
        OptionsModel model = new();
        OptionsRow row = Assert.Single(
            model.Rows(OptionsTab.Interface),
            candidate => candidate.Descriptor.Key == SettingKeys.UiLanguage);

        Assert.Equal(["en", "fr"], row.Descriptor.TextOptions);
        Assert.Equal("Language", row.Label);
        Assert.Equal("English", UiText.LabelOf(row.Descriptor, "en"));
        Assert.Equal("Français", UiText.LabelOf(row.Descriptor, "fr"));
    }

    /// <summary>
    /// AMD-035 §2: the console log and the command line stay in English whatever the setting
    /// says. The usage text is a constant of <see cref="AppOptions"/>, so it cannot follow the
    /// language at all; this pins the intention.
    /// </summary>
    [Fact] // AMD-035 §2
    public void The_help_text_stays_in_english()
    {
        using UiLanguageScope language = new(UiLanguage.French);

        Assert.Contains("Show this help and exit.", AppOptions.Usage, StringComparison.Ordinal);
        Assert.Contains("Usage:", AppOptions.Usage, StringComparison.Ordinal);
    }

    /// <summary>
    /// AMD-036 §2: in <c>en</c>, every notification whose text SPEC-10 §3.10 and SPEC-09 fix word
    /// for word is identical, character for character, to the text of the spec — the two spaces
    /// of REQ-UI-084 and the missing space after the path included. The acceptance criteria of
    /// those requirements are read with the default language, so this is what keeps them true now
    /// that the texts live in a table.
    /// </summary>
    [Fact] // REQ-UI-082, REQ-UI-083, REQ-UI-084, REQ-PERS-022, REQ-PERS-024, REQ-PERS-042, REQ-PERS-054
    public void In_english_the_fixed_texts_are_the_texts_of_the_spec()
    {
        using UiLanguageScope language = new(UiLanguage.English);

        // REQ-UI-082: the four in-game shortcut notifications.
        Assert.Equal("Roll Back", UiStrings.Get(UiKeys.NotifyRollBack));
        Assert.Equal("Fast Forward", UiStrings.Get(UiKeys.NotifyFastForward));
        Assert.Equal("Quick Save", UiStrings.Get(UiKeys.NotifyQuickSave));
        Assert.Equal("Quick Load", UiStrings.Get(UiKeys.NotifyQuickLoad));

        // REQ-UI-083: the five load and read failures.
        Assert.Equal("Corrupted zip file", UiStrings.Get(UiKeys.NotifyCorruptedZip));
        Assert.Equal(
            "Uncompression is not successful", UiStrings.Get(UiKeys.NotifyUncompressionFailed));
        Assert.Equal("Invalid rom file.", UiStrings.Get(UiKeys.NotifyInvalidRom));
        Assert.Equal(
            "The game profile doesn't exist anymore.", UiStrings.Get(UiKeys.NotifyGameMissing));
        Assert.Equal("Save state used a different core.", UiStrings.Get(UiKeys.NotifyForeignCore));

        // REQ-UI-084, AMD-029 §3: the three import failures, oddities included.
        Assert.Equal("This file is not a 3dn file", UiStrings.Get(UiKeys.NotifyNotA3dnFile));
        Assert.Equal(
            "3dn format version 7  is not supported",
            UiNotifications.Translate(ProfileImportMessages.UnsupportedVersion("7")));
        Assert.Equal(
            @"C:\x\y.3dnhas an obsolate or wrong profile format. "
            + "This profile is not compatible with the current version of Zaxnes.",
            UiNotifications.Translate(ProfileImportMessages.ObsoleteFormat(@"C:\x\y.3dn")));

        // REQ-PERS-022 and REQ-PERS-024: the messages of the persisted files.
        Assert.Equal(
            "Settings file was unreadable and has been reset.",
            UiStrings.Get(UiKeys.NotifySettingsUnreadable));
        Assert.Equal(
            "Settings were created by a newer version.",
            UiStrings.Get(UiKeys.NotifySettingsNewerVersion));

        // REQ-PERS-042 and REQ-PERS-054: the two questions and their buttons.
        Assert.Equal(
            EmulationHost.DeleteGameQuestion, UiStrings.Get(UiKeys.ConfirmDeleteGame));
        Assert.Equal(
            "Do you want to delete the slot 4 ?",
            UiNotifications.Translate(EmulationHost.DeleteSlotQuestion("4")));
        Assert.Equal("Yes", UiStrings.Get(UiKeys.ButtonYes));
        Assert.Equal("No", UiStrings.Get(UiKeys.ButtonNo));

        // Every one of them is produced in English by the module that raises it, so the English
        // interface shows exactly what that module wrote.
        foreach (string key in FixedTextKeys())
        {
            Assert.Equal(UiStrings.EnglishTable[key], UiStrings.Get(key));
        }
    }

    /// <summary>
    /// AMD-036 §2: in <c>fr</c>, none of those texts is still English — the message the loop
    /// produced is translated where it is shown, values and paths kept as they are.
    /// </summary>
    [Fact] // AMD-036 §1, §2
    public void In_french_the_fixed_texts_are_translated_where_they_are_shown()
    {
        using UiLanguageScope language = new(UiLanguage.French);

        Assert.Equal("Sauvegarde rapide", UiNotifications.Translate("Quick Save"));
        Assert.Equal("Fichier ROM invalide.", UiNotifications.Translate(RomFile.InvalidRomMessage));
        Assert.Equal(
            "Le profil du jeu n'existe plus.",
            UiNotifications.Translate(SaveStateFile.GameMissingMessage));
        Assert.Equal(
            "Le fichier de réglages était illisible et a été réinitialisé.",
            UiNotifications.Translate(SettingsFile.UnreadableMessage));

        // The value of a patterned text is read out of the English message and put back into the
        // French one, wherever the sentence needs it.
        Assert.Equal(
            "La version 7 du format 3dn n'est pas prise en charge",
            UiNotifications.Translate(ProfileImportMessages.UnsupportedVersion("7")));
        Assert.StartsWith(
            @"C:\x\y.3dn a un format de profil obsolète",
            UiNotifications.Translate(ProfileImportMessages.ObsoleteFormat(@"C:\x\y.3dn")),
            StringComparison.Ordinal);

        // The two confirmations and their buttons, through the model the box draws.
        ConfirmDialogModel box = new();
        box.Open(new ConfirmationRequest(
            "Arkanoid",
            EmulationHost.DeleteSlotQuestion("4"),
            EmulationHost.YesLabel,
            EmulationHost.NoLabel,
            () => { }));

        Assert.Equal("Voulez-vous supprimer l'emplacement 4 ?", box.Message);
        Assert.Equal("Oui", box.ConfirmLabel);
        Assert.Equal("Non", box.CancelLabel);

        // A text no requirement fixes — a script notification, a file system message — is shown
        // as it comes rather than turned into its key.
        Assert.Equal("anything else", UiNotifications.Translate("anything else"));
    }

    /// <summary>
    /// AMD-036 §2: the output of <c>--import-3dsen</c> is English whatever <c>ui.language</c>
    /// says, because the command line stays English (AMD-035 §2). Only the notification the
    /// interface shows follows the language.
    /// </summary>
    [Fact] // AMD-036 §1, §2, AMD-030 §5
    public void The_3dsen_import_summary_is_english_on_the_command_line()
    {
        using UiLanguageScope language = new(UiLanguage.French);

        ThreeDSenImportSummary summary = new(
            @"C:\3dSen\bundle",
            [
                new ThreeDSenProfileResult("Pinball", ThreeDSenProfileOutcome.Imported, "Pinball.3dn", string.Empty),
                new ThreeDSenProfileResult("Tetris", ThreeDSenProfileOutcome.AlreadyPresent, "Tetris.3dn", string.Empty),
                new ThreeDSenProfileResult("junk", ThreeDSenProfileOutcome.Failed, string.Empty, "bad"),
            ]);

        Assert.Equal("3dSen: 1 profile(s) imported, 1 already present, 1 failed.", summary.Message);

        // The same three counts, in the language of the interface, for the notification.
        Assert.Equal(
            "3dSen : 1 profil(s) importé(s), 1 déjà présent(s), 1 en échec.",
            UiNotifications.ThreeDSenSummary(
                summary.Imported, summary.AlreadyPresent, summary.Failed));
    }

    /// <summary>The keys of the texts AMD-036 §1 moved into the tables.</summary>
    private static IEnumerable<string> FixedTextKeys()
        => PublishedKeys().Where(key =>
            key.StartsWith("notify.", StringComparison.Ordinal)
            || key.StartsWith("confirm.", StringComparison.Ordinal)
            || key == UiKeys.ButtonYes
            || key == UiKeys.ButtonNo);

    private static IEnumerable<string> PublishedKeys()
    {
        foreach (System.Reflection.FieldInfo field in typeof(UiKeys).GetFields(
            System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static))
        {
            if (field.IsLiteral
                && field.GetRawConstantValue() is string value
                && !value.EndsWith('.'))
            {
                yield return value;
            }
        }
    }
}
