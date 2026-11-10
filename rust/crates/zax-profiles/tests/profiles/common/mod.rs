//! Helpers shared by the profile tests.

use zax_profiles::format::ProfileDocument;
use zax_profiles::hash::profile::HashProfileState;

pub mod fake_fs;
pub mod lz4_encoder;
pub mod model_comparer;
pub mod neutral_assets;
pub mod physical_fs;
pub mod proto_builder;
pub mod recording_log;
pub mod synthetic_bundle;
pub mod synthetic_hash_profile;
pub mod synthetic_rom;

/// The "3dh" state of a document the test read from a "3dh" file (AMD-062).
pub fn hash_state(document: &ProfileDocument) -> &HashProfileState {
    document
        .hash_state()
        .expect("the document was read from a \"3dh\" profile")
}
