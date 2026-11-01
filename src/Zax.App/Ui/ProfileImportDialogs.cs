using System.Globalization;
using Zax.Profiles.Catalogue;

namespace Zax.App.Ui;

/// <summary>Which of the dialogs of AMD-056 REQ-UI-214 an import calls for.</summary>
internal enum ProfileImportDialogKind
{
    /// <summary>
    /// No dialog: the identifier was unknown, so the import went through and shows neither
    /// notification nor dialog — only the imported entry, selected in the box of REQ-UI-201.
    /// </summary>
    None,

    /// <summary>
    /// The file was refused for one of the causes of the table of REQ-UI-212, or the write
    /// failed after the confirmation. One button.
    /// </summary>
    Refused,

    /// <summary>
    /// The identifier belongs to a built-in profile: nothing was imported, and the dialog says
    /// so. One button.
    /// </summary>
    AlreadyBuiltIn,

    /// <summary>
    /// The identifier belongs to an existing user profile: the replacement happens only if the
    /// first button is pressed. Two buttons, the first destructive.
    /// </summary>
    ConfirmReplacement,
}

/// <summary>
/// One of the dialogs of AMD-056 REQ-UI-214, as a UI-independent value: what kind it is, its
/// title, its body and its buttons. <see cref="ProfileImportDialogs.Decide"/> builds it from a
/// <see cref="ProfileImportPlan"/>, which is what lets every criterion of the requirement be
/// checked without an ImGui frame.
/// </summary>
/// <param name="Kind">Which dialog, <see cref="ProfileImportDialogKind.None"/> for none.</param>
/// <param name="Title">Its title, in the language in force (AMD-036 §1).</param>
/// <param name="Body">Its body, in the language in force.</param>
/// <param name="PrimaryLabel">
/// Label of the first button: the one that acts, and the destructive one of the replacement.
/// Empty when there is no dialog.
/// </param>
/// <param name="CancelLabel">
/// Label of the second button, or <see langword="null"/> when the dialog has only one — which
/// the two one-button rows of the table have.
/// </param>
/// <param name="Destructive">
/// Whether <paramref name="PrimaryLabel"/> destroys something, which only the replacement does.
/// AMD-056 §3 leaves its presentation free (AMD-010 §4); this is what the drawing reads to
/// colour it.
/// </param>
internal sealed record ProfileImportDialog(
    ProfileImportDialogKind Kind,
    string Title,
    string Body,
    string PrimaryLabel,
    string? CancelLabel,
    bool Destructive = false)
{
    /// <summary>The answer for a plan that needs no dialog at all.</summary>
    public static ProfileImportDialog None { get; } = new(
        ProfileImportDialogKind.None, string.Empty, string.Empty, string.Empty, null);

    /// <summary>Whether anything is to be shown.</summary>
    public bool IsShown => Kind != ProfileImportDialogKind.None;

    /// <summary>Whether pressing the first button is what carries the import out.</summary>
    public bool NeedsConfirmation => Kind == ProfileImportDialogKind.ConfirmReplacement;
}

/// <summary>
/// The decision of AMD-056 REQ-UI-214: which dialog an import puts up, and with what words.
/// </summary>
/// <remarks>
/// <para>
/// It is a pure function of the plan <see cref="UserProfileFolder.Inspect"/> read out of the
/// file, so the four rows of the requirement's table — refusal, built-in identifier, existing
/// user profile, unknown identifier — are decided and checked with no window open. The
/// <b>form</b> of the dialogs stays free (AMD-056 §3, AMD-010 §4); only these texts and these
/// buttons are fixed.
/// </para>
/// <para>
/// Every text is an entry of the table of AMD-036, so the dialogs follow the interface
/// language. The English entries are the ones the amendment writes.
/// </para>
/// </remarks>
internal static class ProfileImportDialogs
{
    /// <summary>
    /// The dialog for <paramref name="plan"/>. <see cref="ProfileImportDialog.None"/> when the
    /// identifier was unknown: an import that goes through shows nothing.
    /// </summary>
    public static ProfileImportDialog Decide(ProfileImportPlan plan)
    {
        ArgumentNullException.ThrowIfNull(plan);

        return plan.Outcome switch
        {
            // Row 1. « <fichier> » is the *name* of the chosen file, extension kept, folder
            // dropped; the cause text carries the full path itself for the third cause, which
            // is why the same file can be named twice in one body.
            ProfileImportOutcome.Rejected => Refused(FileNameOf(plan.SourcePath), plan.Message),

            // Row 2. « <titre> » is the Title of the *imported file*, falling back to the file
            // name with its extension — not to the name without it, which is what the library
            // entry would have taken (REQ-PERS-034).
            ProfileImportOutcome.AlreadyBuiltIn => new ProfileImportDialog(
                ProfileImportDialogKind.AlreadyBuiltIn,
                UiStrings.Get(UiKeys.DialogImportBuiltInTitle),
                Format(
                    UiKeys.DialogImportBuiltInBody,
                    DialogTitleOf(plan),
                    FileNameOf(plan.SourcePath)),
                UiStrings.Get(UiKeys.ButtonOk),
                null),

            // Row 3. « <nom> » is the displayed title of the existing entry, a rename included;
            // `Replaced.Title` is that title (LibraryGameCatalog.InspectProfileImport).
            ProfileImportOutcome.Replaced => new ProfileImportDialog(
                ProfileImportDialogKind.ConfirmReplacement,
                Format(UiKeys.DialogImportReplaceTitle, plan.Replaced?.Title ?? plan.Title),
                Format(UiKeys.DialogImportReplaceBody, FileNameOf(plan.SourcePath)),
                UiStrings.Get(UiKeys.ButtonReplace),
                UiStrings.Get(UiKeys.ButtonCancel),
                Destructive: true),

            // Row 4: an unknown identifier. Nothing is shown.
            _ => ProfileImportDialog.None,
        };
    }

    /// <summary>
    /// The refusal dialog of row 1, which the failure of a write after the confirmation also
    /// uses — with the message of its own error in place of a cause of REQ-UI-084.
    /// </summary>
    public static ProfileImportDialog Refused(string fileName, string cause)
        => new(
            ProfileImportDialogKind.Refused,
            UiStrings.Get(UiKeys.DialogImportRefusedTitle),
            // The cause is produced in English by the profile reader, which knows nothing of
            // the text table (AMD-036 §1): it is translated where it is shown, here. A
            // file-system error has no entry and comes through as it is.
            Format(UiKeys.DialogImportRefusedBody, fileName, UiNotifications.Translate(cause)),
            UiStrings.Get(UiKeys.ButtonOk),
            null);

    /// <summary>
    /// « &lt;titre&gt; » of row 2: field 8 of the file, or the file name with its extension.
    /// </summary>
    private static string DialogTitleOf(ProfileImportPlan plan)
        => string.IsNullOrEmpty(plan.Header.Title)
            ? FileNameOf(plan.SourcePath)
            : plan.Header.Title!;

    /// <summary>« &lt;fichier&gt; »: the name of the chosen file, extension and all.</summary>
    private static string FileNameOf(string path)
    {
        string name = Path.GetFileName(path);
        return name.Length > 0 ? name : path;
    }

    private static string Format(string key, params object?[] values)
        => string.Format(CultureInfo.CurrentCulture, UiStrings.Get(key), values);
}
