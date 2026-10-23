//! Preparing a profile for export (SPEC-03 REQ-PROF-052).

use std::sync::Arc;

use crate::pattern::ProfilePattern;
use crate::profile::{GameProfile, CURRENT_VERSION_TEXT, REQUIRED_SIGNATURE};

/// Length of a generated profile identifier.
pub const IDENTIFIER_LENGTH: usize = 12;

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The system random source failed.
#[derive(Debug, thiserror::Error)]
#[error("no random identifier could be generated: {0}")]
pub struct IdentifierError(#[from] getrandom::Error);

/// A new profile identifier: nine random bytes in standard base64, twelve characters.
///
/// # Errors
///
/// The system random source failed.
pub fn new_identifier() -> Result<String, IdentifierError> {
    let mut entropy = [0_u8; 9];
    getrandom::fill(&mut entropy)?;
    let mut identifier = String::with_capacity(IDENTIFIER_LENGTH);
    for group in entropy.chunks_exact(3) {
        let bits = (u32::from(group[0]) << 16) | (u32::from(group[1]) << 8) | u32::from(group[2]);
        for shift in [18, 12, 6, 0] {
            let index = usize::try_from((bits >> shift) & 0x3F).unwrap_or(0);
            identifier.push(char::from(BASE64_ALPHABET[index]));
        }
    }
    Ok(identifier)
}

/// Brings a profile to the form the exporter writes: current signature and version, an
/// identifier, a title (the ROM file name when it is empty), scripts sorted and patterns filed
/// by their own `permanent` flag.
///
/// # Errors
///
/// A missing identifier could not be generated.
pub fn normalize(profile: &mut GameProfile, rom_file_name: Option<&str>) -> Result<(), IdentifierError> {
    profile.signature = Some(REQUIRED_SIGNATURE.to_owned());
    profile.version_text = Some(CURRENT_VERSION_TEXT.to_owned());

    if profile.id.as_deref().is_none_or(str::is_empty) {
        profile.id = Some(new_identifier()?);
    }

    if let (Some(title), Some(rom_file_name)) = (&profile.title, rom_file_name) {
        if title.is_empty() {
            profile.title = Some(rom_file_name.to_owned());
        }
    }

    profile.sort_scripts();
    split_patterns_by_family(profile);
    Ok(())
}

/// Refiles every pattern in the permanent or the temporary list by its own `permanent` flag,
/// keeping the order and the sharing.
pub fn split_patterns_by_family(profile: &mut GameProfile) {
    let (permanent, temporary): (Vec<Arc<ProfilePattern>>, Vec<Arc<ProfilePattern>>) = profile
        .all_patterns()
        .cloned()
        .partition(|pattern| pattern.permanent);
    profile.permanent_patterns = permanent;
    profile.temporary_patterns = temporary;
}
