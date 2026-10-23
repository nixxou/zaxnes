namespace Zax.App.Ui;

/// <summary>The two interface languages of AMD-035 §1.</summary>
internal enum UiLanguage
{
    /// <summary>English, the default on a fresh user folder (AMD-035 §1).</summary>
    English,

    /// <summary>French.</summary>
    French,
}

/// <summary>
/// The two tables of interface texts of AMD-035: one key per text, one table per language, and
/// the English table as the fallback for a key a translation is missing.
/// </summary>
/// <remarks>
/// <para>
/// The language in force is a property of the running emulator, not of the machine: AMD-035 §1
/// forbids any detection of the system's language, so the only thing that sets it is the setting
/// <c>ui.language</c> — which the shell reads at start-up and writes here on every change, so the
/// change applies immediately (AMD-035 §4).
/// </para>
/// <para>
/// <see cref="Generation"/> counts the changes. The models that build a list of labels once —
/// the menu bar, the options window, the key configuration, the shortcut table — compare it with
/// the generation they were built for and rebuild themselves when it has moved, which is what
/// makes « the whole interface switches at once » true without a restart.
/// </para>
/// <para>
/// What stays out of the tables is what AMD-035 §2 leaves out — the console log, <c>--help</c>
/// and the command-line messages — and the titles and descriptions that come from a profile.
/// The texts SPEC-10 REQ-UI-082 to REQ-UI-084 and SPEC-09 fix word for word are <b>in</b> the
/// tables: AMD-036 §1 makes each of them the English entry, unchanged, with a French
/// translation beside it. <see cref="UiNotifications"/> translates them where they are shown.
/// </para>
/// </remarks>
internal static partial class UiStrings
{
    private static UiLanguage _language = UiLanguage.English;

    /// <summary>The language the interface is drawn in; English until the setting says otherwise.</summary>
    public static UiLanguage Language
    {
        get => _language;
        set
        {
            if (_language == value)
            {
                return;
            }

            _language = value;
            Generation++;
        }
    }

    /// <summary>
    /// How many times the language has changed since start-up. A model that caches labels keeps
    /// the value it built with and rebuilds when it differs.
    /// </summary>
    public static int Generation { get; private set; }

    /// <summary>The English table, which is also the fallback of every other language.</summary>
    public static IReadOnlyDictionary<string, string> EnglishTable => English;

    /// <summary>The French table.</summary>
    public static IReadOnlyDictionary<string, string> FrenchTable => French;

    /// <summary>The table of <paramref name="language"/>.</summary>
    public static IReadOnlyDictionary<string, string> TableOf(UiLanguage language)
        => language == UiLanguage.French ? French : English;

    /// <summary>
    /// The text of <paramref name="key"/> in the current language, the English one when the
    /// translation has no such key, and the key itself when neither table has it — never an
    /// empty string and never a blank (AMD-035 §3).
    /// </summary>
    public static string Get(string key)
    {
        ArgumentNullException.ThrowIfNull(key);

        if (TableOf(_language).TryGetValue(key, out string? text) && text.Length > 0)
        {
            return text;
        }

        return English.TryGetValue(key, out string? fallback) && fallback.Length > 0 ? fallback : key;
    }

    /// <summary>The <c>ui.language</c> text of <paramref name="language"/>: <c>en</c> or <c>fr</c>.</summary>
    public static string CodeOf(UiLanguage language)
        => language == UiLanguage.French ? Zax.Settings.SettingsCatalogue.LanguageFrench
            : Zax.Settings.SettingsCatalogue.LanguageEnglish;

    /// <summary>
    /// The language a <c>ui.language</c> text designates. Anything that is not <c>fr</c> — an
    /// empty value, a misspelling, a language this build does not have — is English (AMD-035 §1).
    /// </summary>
    public static UiLanguage Parse(string? code)
        => string.Equals(code?.Trim(), Zax.Settings.SettingsCatalogue.LanguageFrench, StringComparison.OrdinalIgnoreCase)
            ? UiLanguage.French
            : UiLanguage.English;
}
