//! The import of the profiles shipped with a 3dSen installation (AMD-030): a `UnityFS` bundle
//! reader with LZ4 decompression, and the writer of the `.3dn` files.

mod asset_reader;
mod bundle;
mod importer;
mod installation;
mod lz4;
mod profile_bundle;
mod serialized_file;
mod text_asset;

pub use bundle::{
    UnityBundle, UnityBundleFile, UnityCompression, MAXIMUM_FORMAT_VERSION, MINIMUM_FORMAT_VERSION,
    SIGNATURE as BUNDLE_SIGNATURE,
};
pub use importer::{
    ThreeDSenImportError, ThreeDSenImportSummary, ThreeDSenImporter, ThreeDSenProfileOutcome, ThreeDSenProfileResult,
};
pub use installation::{
    expected_layout, find_bundle, is_bundle_name, locate_bundle, BUNDLE_EXTENSION, BUNDLE_PREFIX,
    PLATFORMS,
};
pub use lz4::decode as decode_lz4_block;
pub use profile_bundle::read_text_assets;
pub use serialized_file::{UnityObject, UnitySerializedFile};
pub use text_asset::{UnityTextAsset, CLASS_ID as TEXT_ASSET_CLASS_ID};

/// A bundle, a serialized file or an asset is not what the reader supports or is damaged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct UnityAssetFormatError {
    message: String,
}

impl UnityAssetFormatError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The description of what is wrong.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
