//! The text assets of a 3dSen profile bundle (AMD-030 §4).

use super::bundle::UnityBundle;
use super::serialized_file::UnitySerializedFile;
use super::text_asset::{UnityTextAsset, CLASS_ID};
use super::UnityAssetFormatError;

const RESOURCE_EXTENSIONS: [&str; 2] = [".resS", ".resource"];

/// Reads every `TextAsset` of a bundle, in the order of its files and objects.
///
/// # Errors
///
/// The bytes are not a readable bundle, or a file or text asset in it is damaged.
pub fn read_text_assets(bundle: &[u8]) -> Result<Vec<UnityTextAsset>, UnityAssetFormatError> {
    let container = UnityBundle::read(bundle)?;

    let mut assets = Vec::new();
    for file in &container.files {
        if is_resource_node(&file.path) {
            continue;
        }

        let serialized = UnitySerializedFile::read(&file.data)?;
        for object in &serialized.objects {
            // AMD-030 §4.4: the AssetBundle object (142) and anything else are ignored.
            if object.class_id == CLASS_ID {
                assets.push(UnityTextAsset::read(object, serialized.is_big_endian)?);
            }
        }
    }
    Ok(assets)
}

fn is_resource_node(path: &str) -> bool {
    let lower = path.to_lowercase();
    RESOURCE_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(&extension.to_lowercase()))
}
