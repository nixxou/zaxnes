//! The decision of AMD-056 REQ-UI-214: which dialogue the import of a profile puts up, and with
//! what words.
//!
//! It is a pure function of the plan [`zax_profiles::catalogue::UserProfileFolder::inspect`] reads
//! out of the file, so the four rows of the requirement's table — refusal, built-in identifier,
//! existing user profile, unknown identifier — are decided and checked with no window open. The
//! **form** of the dialogues stays free (AMD-056 §3, AMD-010 §4); only these texts and these
//! buttons are fixed.
//!
//! Every text is an entry of the table of AMD-036, so the dialogues follow the interface language.
//! The English entries are the ones the amendment writes.

use std::path::Path;

use zax_profiles::catalogue::{ProfileImportOutcome, ProfileImportPlan};

use super::{keys, notifications, strings};

/// Which of the dialogues of AMD-056 REQ-UI-214 an import calls for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileImportDialogKind {
    /// The file was refused for one of the causes of the table of REQ-UI-212, or the write failed
    /// after the confirmation. One button.
    Refused,
    /// The identifier belongs to a built-in profile: nothing was imported, and the dialogue says
    /// so. One button.
    AlreadyBuiltIn,
    /// The identifier belongs to an existing user profile: the replacement happens only if the
    /// first button is pressed. Two buttons, the first destructive.
    ConfirmReplacement,
}

/// One of the dialogues of AMD-056 REQ-UI-214, as a value the drawing reads: what kind it is, its
/// title, its body and its buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileImportDialog {
    /// Which dialogue.
    pub kind: ProfileImportDialogKind,
    /// Its title, in the language in force (AMD-036 §1).
    pub title: String,
    /// Its body, in the language in force.
    pub body: String,
    /// Label of the first button: the one that acts, and the destructive one of the replacement.
    pub primary_label: String,
    /// Label of the second button, or none when the dialogue has only one — which the two
    /// one-button rows of the table have.
    pub cancel_label: Option<String>,
    /// Whether the first button destroys something, which only the replacement does. AMD-056 §3
    /// leaves its presentation free (AMD-010 §4); this is what the drawing reads to colour it.
    pub destructive: bool,
}

impl ProfileImportDialog {
    /// Whether pressing the first button is what carries the import out.
    #[must_use]
    pub fn needs_confirmation(&self) -> bool {
        self.kind == ProfileImportDialogKind::ConfirmReplacement
    }
}

/// The dialogue for `plan`; none when the identifier was unknown, an import that goes through
/// showing neither notification nor dialogue (row 4 of the table).
#[must_use]
pub fn decide(plan: &ProfileImportPlan) -> Option<ProfileImportDialog> {
    match plan.outcome {
        // Row 1. « <fichier> » is the *name* of the chosen file, extension kept, folder dropped;
        // the cause text carries the full path itself for the third cause, which is why the same
        // file can be named twice in one body.
        ProfileImportOutcome::Rejected => {
            Some(refused(&file_name_of(&plan.source_path), &plan.message))
        }
        // Row 2. « <titre> » is the Title of the *imported file*, falling back to the file name
        // with its extension — not to the name without it, which is what the library entry would
        // have taken (REQ-PERS-034).
        ProfileImportOutcome::AlreadyBuiltIn => Some(ProfileImportDialog {
            kind: ProfileImportDialogKind::AlreadyBuiltIn,
            title: strings::get(keys::DIALOG_IMPORT_BUILT_IN_TITLE),
            body: strings::format(
                &strings::get(keys::DIALOG_IMPORT_BUILT_IN_BODY),
                &[&dialog_title_of(plan), &file_name_of(&plan.source_path)],
            ),
            primary_label: strings::get(keys::BUTTON_OK),
            cancel_label: None,
            destructive: false,
        }),
        // Row 3. « <nom> » is the displayed title of the existing entry, a rename included.
        ProfileImportOutcome::Replaced => Some(ProfileImportDialog {
            kind: ProfileImportDialogKind::ConfirmReplacement,
            title: strings::format(
                &strings::get(keys::DIALOG_IMPORT_REPLACE_TITLE),
                &[plan
                    .replaced
                    .as_ref()
                    .map_or(plan.title.as_str(), |replaced| replaced.title.as_str())],
            ),
            body: strings::format(
                &strings::get(keys::DIALOG_IMPORT_REPLACE_BODY),
                &[&file_name_of(&plan.source_path)],
            ),
            primary_label: strings::get(keys::BUTTON_REPLACE),
            cancel_label: Some(strings::get(keys::BUTTON_CANCEL)),
            destructive: true,
        }),
        // Row 4: an unknown identifier. Nothing is shown.
        ProfileImportOutcome::Added => None,
    }
}

/// The refusal dialogue of row 1, which the failure of a write after the confirmation also uses —
/// with the message of its own error in place of a cause of REQ-UI-084.
#[must_use]
pub fn refused(file_name: &str, cause: &str) -> ProfileImportDialog {
    ProfileImportDialog {
        kind: ProfileImportDialogKind::Refused,
        title: strings::get(keys::DIALOG_IMPORT_REFUSED_TITLE),
        // The cause is produced in English by the profile reader, which knows nothing of the text
        // table (AMD-036 §1): it is translated where it is shown, here. A file-system error has no
        // entry and comes through as it is.
        body: strings::format(
            &strings::get(keys::DIALOG_IMPORT_REFUSED_BODY),
            &[file_name, &notifications::translate(cause)],
        ),
        primary_label: strings::get(keys::BUTTON_OK),
        cancel_label: None,
        destructive: false,
    }
}

/// « <titre> » of row 2: field 8 of the file, or the file name with its extension.
fn dialog_title_of(plan: &ProfileImportPlan) -> String {
    match plan.header.title.as_deref() {
        Some(title) if !title.is_empty() => title.to_owned(),
        _ => file_name_of(&plan.source_path),
    }
}

/// « <fichier> »: the name of the chosen file, extension and all.
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
