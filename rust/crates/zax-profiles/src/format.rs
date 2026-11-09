//! Loading and saving profiles, with the validation rules of SPEC-03 REQ-PROF-020 to
//! REQ-PROF-025 and REQ-PROF-033.

use crate::enums::ProfileFormatVersion;
use crate::hash;
use crate::layout::FieldPresence;
use crate::pattern::RecordedPatternSlot;
use crate::profile::{GameProfile, REQUIRED_SIGNATURE};
use crate::settings::LayerSettingsSet;
use crate::wire::{self, ProfileFormatError};

/// A decoded profile with what the encoder needs to write it back as it was.
#[derive(Debug, Clone)]
pub struct ProfileDocument {
    /// The profile.
    pub profile: GameProfile,
    /// Number of fields the decoder skipped because the schema does not know them.
    pub unknown_field_count: usize,
}

impl ProfileDocument {
    /// Writes the profile back, each node with the fields it was read with (REQ-PROF-051).
    #[must_use]
    pub fn save(&self) -> Vec<u8> {
        wire::encode(&self.profile, FieldPresence::AsRecorded)
    }
}

/// The identity fields of a profile, read without decoding the rest (REQ-PROF-060).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileHeader {
    /// Signature.
    pub signature: Option<String>,
    /// Field 2, `Ver`. SPEC-03 §4.1 calls the header the subset {1, 7, 8, 9, 10, 11}; the import
    /// of AMD-053 REQ-UI-212 needs one field more, because the cause it must name for a refused
    /// file is read from the version.
    pub version: Option<String>,
    /// Identifier.
    pub id: Option<String>,
    /// Title.
    pub title: Option<String>,
    /// Author.
    pub author: Option<String>,
    /// Publisher.
    pub publisher: Option<String>,
    /// Description.
    pub description: Option<String>,
}

impl ProfileHeader {
    /// True when this header was read from the envelope of a "3dh" profile, whose field 1 is
    /// `"3dh"` (AMD-062 REQ-PROF-070, REQ-PROF-073).
    #[must_use]
    pub fn is_hash_envelope(&self) -> bool {
        self.signature.as_deref() == Some(hash::envelope::SIGNATURE)
    }

    /// True when the signature is acceptable: absent, `"3dn"`, or `"3dh"`, which REQ-PROF-020 as
    /// amended by AMD-062 REQ-PROF-070 no longer refuses.
    #[must_use]
    pub fn has_acceptable_signature(&self) -> bool {
        self.signature
            .as_deref()
            .is_none_or(|signature| signature == REQUIRED_SIGNATURE)
            || self.is_hash_envelope()
    }

    /// True when the identifier is present and not empty.
    #[must_use]
    pub fn has_id(&self) -> bool {
        self.id.as_deref().is_some_and(|id| !id.is_empty())
    }

    /// How [`Self::version`] classifies under the rules of REQ-PROF-021. Field 2 being optional,
    /// an absent version is the current format. For a "3dh" envelope the field is the
    /// **envelope** version instead, whose supported values are `"1"`, `"2"` and `"5"`
    /// (REQ-PROF-072).
    #[must_use]
    pub fn format_version(&self) -> ProfileFormatVersion {
        if self.is_hash_envelope() {
            return if matches!(self.version.as_deref(), Some("1" | "2" | "5")) {
                ProfileFormatVersion::Current
            } else {
                ProfileFormatVersion::Unsupported
            };
        }

        crate::profile::version_rules::classify(self.version.as_deref())
    }

    /// True when the version is one this build refuses to load — `"1"`, and anything that
    /// reduces to it (REQ-PROF-021).
    #[must_use]
    pub fn has_unsupported_version(&self) -> bool {
        self.format_version() == ProfileFormatVersion::Unsupported
    }
}

/// Why a profile was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLoadErrorKind {
    /// The bytes are not a readable profile.
    Malformed,
    /// The signature is not `"3dn"`.
    WrongSignature,
    /// The version is not supported.
    UnsupportedVersion,
    /// The embedded settings are missing.
    MissingSettings,
    /// The embedded settings lack the light, the clipping or the layers.
    IncompleteSettings,
    /// The embedded settings do not hold six layers.
    WrongLayerCount,
    /// The "3dh" envelope has an unsupported version or no payload (AMD-062 REQ-PROF-072).
    UnsupportedHashEnvelope,
    /// The payload does not match the digest the envelope records (REQ-PROF-072).
    DamagedHashPayload,
    /// The manifest is malformed or holds the same string twice (REQ-PROF-072).
    InvalidHashManifest,
    /// A tile names a string outside the manifest, or a variant's `textureKind` is not usable
    /// (REQ-PROF-074).
    InvalidHashPayload,
}

/// A refused profile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ProfileLoadError {
    /// The reason.
    pub kind: ProfileLoadErrorKind,
    /// A description for the log.
    pub message: String,
}

impl ProfileLoadError {
    pub(crate) fn new(kind: ProfileLoadErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Where the loader and the catalogue report what they skip or refuse.
pub trait ProfileLog {
    /// A normal event.
    fn information(&self, message: &str);
    /// Something skipped or recovered.
    fn warning(&self, message: &str);
    /// Something refused.
    fn error(&self, message: &str);
}

/// A log that drops everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullProfileLog;

impl ProfileLog for NullProfileLog {
    fn information(&self, _message: &str) {}
    fn warning(&self, _message: &str) {}
    fn error(&self, _message: &str) {}
}

/// The log used when none is given.
pub static NULL_PROFILE_LOG: NullProfileLog = NullProfileLog;

/// Decodes `bytes` and applies the load rules. A refused profile is reported on `log` as
/// `<source>: <message>`.
///
/// # Errors
///
/// The reason the bytes are not an acceptable profile.
pub fn load(
    bytes: &[u8],
    log: Option<&dyn ProfileLog>,
    source_name: Option<&str>,
) -> Result<ProfileDocument, ProfileLoadError> {
    let result = load_core(bytes);
    if let Err(error) = &result {
        let source = source_name.unwrap_or("<profile>");
        log.unwrap_or(&NULL_PROFILE_LOG)
            .error(&format!("{source}: {}", error.message));
    }
    result
}

/// Encodes `profile`; `presence` says whether every scalar is written or only those a decoded
/// file carried (REQ-PROF-051).
#[must_use]
pub fn save(profile: &GameProfile, presence: FieldPresence) -> Vec<u8> {
    wire::encode(profile, presence)
}

/// Reads the identity fields of a profile; none, with a warning on `log`, when the bytes are
/// unreadable.
///
/// For a "3dh" profile the metadata are read from the **envelope**, fields 7 to 11, with the
/// same numbers and the same meaning as a 3dn header; the payload is not decoded for this
/// (REQ-PROF-073). A 3dh whose payload is damaged, and which the loader therefore refuses, still
/// has a readable header and so still appears in the list of games.
#[must_use]
pub fn load_header(
    bytes: &[u8],
    log: Option<&dyn ProfileLog>,
    source_name: Option<&str>,
) -> Option<ProfileHeader> {
    let decoded = if hash::envelope::is_hash_profile(bytes) {
        wire::hash_envelope::decode(bytes).map(|envelope| envelope.to_header())
    } else {
        wire::decode_header(bytes)
    };
    match decoded {
        Ok(header) => Some(header),
        Err(error) => {
            log.unwrap_or(&NULL_PROFILE_LOG).warning(&format!(
                "{}: unreadable profile header ({error}).",
                source_name.unwrap_or("<profile>")
            ));
            None
        }
    }
}

fn load_core(bytes: &[u8]) -> Result<ProfileDocument, ProfileLoadError> {
    let mut document = wire::decode(bytes).map_err(|error: ProfileFormatError| {
        ProfileLoadError::new(
            ProfileLoadErrorKind::Malformed,
            format!("not a readable profile ({error})"),
        )
    })?;
    let profile = &mut document.profile;

    if let Some(signature) = &profile.signature {
        if signature != REQUIRED_SIGNATURE {
            return Err(ProfileLoadError::new(
                ProfileLoadErrorKind::WrongSignature,
                format!("signature \"{signature}\" is not \"{REQUIRED_SIGNATURE}\""),
            ));
        }
    }

    if profile.format_version() == ProfileFormatVersion::Unsupported {
        return Err(ProfileLoadError::new(
            ProfileLoadErrorKind::UnsupportedVersion,
            format!(
                "format version \"{}\" is not supported",
                profile.version_text.as_deref().unwrap_or_default()
            ),
        ));
    }

    let Some(settings) = profile.settings.as_mut() else {
        return Err(ProfileLoadError::new(
            ProfileLoadErrorKind::MissingSettings,
            "the embedded settings are missing",
        ));
    };

    let (Some(_), Some(_), Some(layers)) = (&settings.light, &settings.clipping, &settings.layers)
    else {
        return Err(ProfileLoadError::new(
            ProfileLoadErrorKind::IncompleteSettings,
            "the embedded settings lack the light, the clipping or the layers",
        ));
    };

    if layers.layers.len() != LayerSettingsSet::LAYER_COUNT {
        return Err(ProfileLoadError::new(
            ProfileLoadErrorKind::WrongLayerCount,
            format!(
                "the embedded settings hold {} layers, not {}",
                layers.layers.len(),
                LayerSettingsSet::LAYER_COUNT
            ),
        ));
    }

    // DEC-03-Q03: a profile without the editor's memorised slots is accepted and the five
    // default slots are rebuilt. No built-in profile is in that case.
    if settings.saved_patterns.is_empty() {
        settings.saved_patterns = RecordedPatternSlot::default_shape_slots();
    }
    if settings.saved_anim_patterns.is_empty() {
        settings.saved_anim_patterns = RecordedPatternSlot::default_deform_slots();
    }

    profile.sort_scripts();
    Ok(document)
}
