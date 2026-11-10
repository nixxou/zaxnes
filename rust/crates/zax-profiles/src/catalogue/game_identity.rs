//! The generation of a profile and the game it designates (AMD-065 REQ-PROF-085, REQ-PROF-086).
//!
//! 3dSen 2.1 writes its profiles in another format (a "3dh", AMD-062 REQ-PROF-070) and gives the
//! same game another identifier in 45 cases out of 101. Both files are kept on the disk, but the
//! library shows one entry per game, the 2.1 one (REQ-PROF-087), which first asks what « the same
//! game » is.

use std::path::Path;

use super::profile_catalogue::CatalogueEntry;
use super::user_folder::UserProfile;
use crate::format::ProfileHeader;
use crate::hash;

/// Which generation of 3dSen wrote a profile (REQ-PROF-085).
///
/// Read at the **content**, never at the extension: the import from an installation writes a 3dh
/// under the name `<name>.3dn` (AMD-062 §8), and a 3dn named `.3dh` is read just as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfileGeneration {
    /// A 3dn: 1.x and 2.0 (SPEC-03).
    #[default]
    Earlier,
    /// A 3dh (REQ-PROF-070): 2.1.
    TwoPointOne,
}

impl ProfileGeneration {
    /// The generation a profile header says: a `"3dh"` signature is the envelope of a 2.1 profile.
    #[must_use]
    pub fn of_header(header: &ProfileHeader) -> Self {
        if header.is_hash_envelope() {
            Self::TwoPointOne
        } else {
            Self::Earlier
        }
    }

    /// The generation the bytes of a profile file say, without decoding anything.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        if hash::envelope::is_hash_profile(bytes) {
            Self::TwoPointOne
        } else {
            Self::Earlier
        }
    }

    /// True for generation 2.1, the one that replaces the others (REQ-PROF-087).
    #[must_use]
    pub const fn is_two_point_one(self) -> bool {
        matches!(self, Self::TwoPointOne)
    }
}

/// What decides whether two profiles designate the same game (REQ-PROF-086), with the generation
/// of the profile it was read from (REQ-PROF-085).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameIdentity {
    /// The `Id` read **in the file** — field 7 of a 3dn, field 7 of a 3dh envelope — empty when
    /// the file carries none. An identifier derived from the content (AMD-037) is deliberately
    /// **not** one of these: REQ-PROF-086 point 1 excludes it, two formats of one game having
    /// different bytes and therefore different derived identifiers.
    file_id: String,
    /// The profile name, normalised as [`normalised_name`]: the file name without extension,
    /// case folded, the apostrophe counting as `_`. Empty for an entry without a file.
    name: String,
    generation: ProfileGeneration,
}

impl GameIdentity {
    /// The identity of the profile of file `path` carrying `file_id` (empty when it carries none).
    #[must_use]
    pub fn new(file_id: &str, path: &Path, generation: ProfileGeneration) -> Self {
        Self {
            file_id: file_id.to_owned(),
            name: normalised_name(path),
            generation,
        }
    }

    /// The identity of a user profile (REQ-PROF-062, the two folders of AMD-033). The `Id` comes
    /// from the header, so a profile whose library identifier was derived from its content
    /// (AMD-037) carries no `Id` here, as REQ-PROF-086 point 1 asks.
    #[must_use]
    pub fn of_user(profile: &UserProfile) -> Self {
        Self::new(
            profile.header.id.as_deref().unwrap_or_default(),
            &profile.file_path,
            ProfileGeneration::of_header(&profile.header),
        )
    }

    /// The identity of the built-in profile of `entry` (REQ-PROF-060). The name is the one `file`
    /// names in the catalogue and needs no reading; `header` is what its file carries, for the
    /// callers that have read it (`ProfileCatalogue::read_built_in_profiles`). Without it only
    /// condition 2 of REQ-PROF-086 can be decided, which is still right — it just cannot pair the
    /// 56 games whose two formats agree on an `Id` and disagree on the file name.
    #[must_use]
    pub fn of_catalogue(entry: &CatalogueEntry, header: Option<&ProfileHeader>) -> Self {
        Self::new(
            header
                .and_then(|header| header.id.as_deref())
                .unwrap_or_default(),
            Path::new(&entry.file),
            header.map_or(ProfileGeneration::Earlier, ProfileGeneration::of_header),
        )
    }

    /// The generation of the profile this identity was read from.
    #[must_use]
    pub const fn generation(&self) -> ProfileGeneration {
        self.generation
    }

    /// The `Id` the file carries, empty when it carries none.
    #[must_use]
    pub fn file_id(&self) -> &str {
        &self.file_id
    }

    /// The normalised profile name, empty for an entry without a file.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// REQ-PROF-086: the two profiles designate the same game when they carry the same non-empty
    /// `Id`, or bear the same profile name.
    ///
    /// Both conditions ignore an absent value: the generic entries of the catalogue (Q-065) have
    /// neither an `Id` nor a file, and must designate no game but themselves.
    #[must_use]
    pub fn same_game(&self, other: &Self) -> bool {
        (!self.file_id.is_empty() && self.file_id == other.file_id)
            || (!self.name.is_empty() && self.name == other.name)
    }
}

/// The profile name of the file `path` as REQ-PROF-086 point 2 compares it: the file name without
/// its extension, case folded, the apostrophe `'` counting as `_`.
///
/// The apostrophe is what AMD-062 §1 measured between the two installations: 3dSen 2.1 names its
/// asset `Castlevania II - Simon's Quest (USA)` where the 1.x folder writes
/// `Castlevania II - Simon_s Quest (USA)`.
#[must_use]
pub fn normalised_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().replace('\'', "_").to_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(file_id: &str, file: &str, generation: ProfileGeneration) -> GameIdentity {
        GameIdentity::new(file_id, Path::new(file), generation)
    }

    /// REQ-PROF-086 point 2: the apostrophe and the case are ignored, the folder is not part of
    /// the name, and neither is the extension — whichever of the two it is.
    #[test]
    fn req_prof_086_the_profile_name_ignores_the_folder_the_extension_the_case_and_the_apostrophe()
    {
        let name = |file| normalised_name(Path::new(file));
        assert_eq!("hogan_s alley", name("downloads/Hogan's Alley.3dh"));
        assert_eq!("hogan_s alley", name("profiles/Hogan_s Alley.3dn"));
        assert_eq!("hogan_s alley", name("assets/profiles/HOGAN'S ALLEY.bytes"));
        assert_eq!(
            "castlevania ii - simon_s quest (usa)",
            name("Castlevania II - Simon's Quest (USA).3dn")
        );
        assert_eq!("", name(""));
    }

    /// REQ-PROF-086 point 1 wins over the name: the 56 games whose two formats agree on an `Id`
    /// are the same game even when nothing else matches.
    #[test]
    fn req_prof_086_the_same_file_id_is_the_same_game_whatever_the_name() {
        let plain = identity(
            "Jhj6E7iOysxT",
            "p/Super Mario Bros.3dn",
            ProfileGeneration::Earlier,
        );
        let hash = identity(
            "Jhj6E7iOysxT",
            "u/smb-renamed.3dn",
            ProfileGeneration::TwoPointOne,
        );
        assert!(plain.same_game(&hash));
        assert!(hash.same_game(&plain));
    }

    /// REQ-PROF-086 point 2 stands on its own: the 45 built-in profiles without an `Id` are
    /// recognised by their name alone.
    #[test]
    fn req_prof_086_the_same_name_is_the_same_game_without_any_file_id() {
        let plain = identity("", "p/Hogan_s Alley.3dn", ProfileGeneration::Earlier);
        let hash = identity(
            "3348a6f46bb7",
            "u/Hogan's Alley.3dn",
            ProfileGeneration::TwoPointOne,
        );
        assert!(plain.same_game(&hash));
        assert!(hash.same_game(&plain));
    }

    /// Two profiles that share neither are two games, and an identity with nothing at all — the
    /// generic entries of the catalogue — is only itself.
    #[test]
    fn req_prof_086_nothing_shared_is_not_the_same_game() {
        let left = identity("abc", "p/Tetris.3dn", ProfileGeneration::Earlier);
        let right = identity("def", "p/Tennis.3dn", ProfileGeneration::Earlier);
        assert!(!left.same_game(&right));

        let generic = GameIdentity::default();
        assert!(!generic.same_game(&generic.clone()));
        assert!(!generic.same_game(&left));
        assert!(!left.same_game(&generic));
    }

    /// REQ-PROF-085: the generation is read at the content. A 3dh envelope begins with its
    /// signature, which `of_bytes` recognises without decoding anything.
    #[test]
    fn req_prof_085_the_generation_is_read_at_the_content() {
        let mut envelope = hash::envelope::SIGNATURE_BYTES.to_vec();
        envelope.extend_from_slice(&[0x12, 0x01, b'5']);
        assert_eq!(
            ProfileGeneration::TwoPointOne,
            ProfileGeneration::of_bytes(&envelope)
        );
        let mut plain = vec![0x0a, 0x03];
        plain.extend_from_slice(b"3dn");
        assert_eq!(
            ProfileGeneration::Earlier,
            ProfileGeneration::of_bytes(&plain)
        );
        assert_eq!(ProfileGeneration::Earlier, ProfileGeneration::of_bytes(&[]));

        let header = ProfileHeader {
            signature: Some("3dh".to_owned()),
            ..ProfileHeader::default()
        };
        assert!(ProfileGeneration::of_header(&header).is_two_point_one());
        assert!(!ProfileGeneration::of_header(&ProfileHeader::default()).is_two_point_one());
    }
}
