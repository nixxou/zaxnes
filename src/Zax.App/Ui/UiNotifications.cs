using System.Globalization;

namespace Zax.App.Ui;

/// <summary>
/// Translates, at display time, the notification and confirmation texts SPEC-10 §3.10 and
/// SPEC-09 fix word for word (AMD-036 §1, which closes Q-075).
/// </summary>
/// <remarks>
/// <para>
/// These texts are not written by the interface: they are produced by the emulation loop, by the
/// profile reader and by the settings files — modules that know nothing of
/// <see cref="UiStrings"/> and must keep producing the exact English the requirements fix, since
/// the console log and the command line stay in English (AMD-035 §2). The tables therefore hold
/// the English text as the entry and the translation beside it, and the message is translated
/// where it is shown, by looking its English text up.
/// </para>
/// <para>
/// Three of these texts carry a value — a format version, a path, a slot identifier — and are
/// held as a <c>{0}</c> pattern. <see cref="Translate"/> reads the value back out of the English
/// message and puts it into the translated pattern, so a translation may move it wherever the
/// sentence needs it.
/// </para>
/// </remarks>
internal static class UiNotifications
{
    /// <summary>
    /// The keys AMD-036 §1 moved into the tables, the fixed ones before the patterned ones so
    /// that a literal text can never be swallowed by a pattern.
    /// </summary>
    private static readonly string[] FixedKeys =
    [
        UiKeys.NotifyRollBack,
        UiKeys.NotifyFastForward,
        UiKeys.NotifyQuickSave,
        UiKeys.NotifyQuickLoad,
        UiKeys.NotifyCorruptedZip,
        UiKeys.NotifyUncompressionFailed,
        UiKeys.NotifyInvalidRom,
        UiKeys.NotifyGameMissing,
        UiKeys.NotifyForeignCore,
        UiKeys.NotifyNotA3dnFile,
        UiKeys.NotifySettingsUnreadable,
        UiKeys.NotifySettingsNewerVersion,
        UiKeys.NotifyGameListUnreadable,
        UiKeys.NotifyGameListNewerVersion,
        UiKeys.NotifyBindingsUnreadable,
        UiKeys.NotifyBindingsNewerVersion,
        UiKeys.ConfirmDeleteGame,
        UiKeys.ButtonYes,
        UiKeys.ButtonNo,
    ];

    /// <summary>The keys whose English entry carries a value.</summary>
    private static readonly string[] PatternKeys =
    [
        UiKeys.NotifyUnsupported3dnVersion,
        UiKeys.NotifyObsoleteProfileFormat,
        UiKeys.ConfirmDeleteSlot,
    ];

    /// <summary>
    /// <paramref name="text"/> in the language in force when it is one of the fixed texts, and
    /// <paramref name="text"/> itself otherwise — a script notification (REQ-UI-085), a file
    /// system message or a path has no entry and is shown as it comes.
    /// </summary>
    public static string Translate(string? text)
    {
        if (string.IsNullOrEmpty(text) || UiStrings.Language == UiLanguage.English)
        {
            return text ?? string.Empty;
        }

        foreach (string key in FixedKeys)
        {
            if (string.Equals(UiStrings.EnglishTable[key], text, StringComparison.Ordinal))
            {
                return UiStrings.Get(key);
            }
        }

        foreach (string key in PatternKeys)
        {
            if (TryRead(UiStrings.EnglishTable[key], text, out string value))
            {
                return string.Format(CultureInfo.InvariantCulture, UiStrings.Get(key), value);
            }
        }

        return text;
    }

    /// <summary>The import summary of AMD-030 §5 in the language in force (AMD-036 §1).</summary>
    public static string ThreeDSenSummary(int imported, int alreadyPresent, int failed)
        => string.Format(
            CultureInfo.InvariantCulture,
            UiStrings.Get(UiKeys.NotifyThreeDSenSummary),
            imported,
            alreadyPresent,
            failed);

    /// <summary>
    /// Reads the value <paramref name="text"/> holds where <paramref name="pattern"/> has its
    /// <c>{0}</c>, when the rest of the two match character for character.
    /// </summary>
    private static bool TryRead(string pattern, string text, out string value)
    {
        value = string.Empty;

        int hole = pattern.IndexOf("{0}", StringComparison.Ordinal);
        if (hole < 0)
        {
            return false;
        }

        string head = pattern[..hole];
        string tail = pattern[(hole + 3)..];
        if (text.Length < head.Length + tail.Length
            || !text.StartsWith(head, StringComparison.Ordinal)
            || !text.EndsWith(tail, StringComparison.Ordinal))
        {
            return false;
        }

        value = text[head.Length..(text.Length - tail.Length)];
        return true;
    }
}
