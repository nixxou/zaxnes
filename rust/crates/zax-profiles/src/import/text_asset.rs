//! The Unity `TextAsset` that carries one profile (AMD-030 §4.4).

use super::asset_reader::AssetReader;
use super::serialized_file::UnityObject;
use super::UnityAssetFormatError;

/// Class identifier of a `TextAsset`.
pub const CLASS_ID: i32 = 49;

/// A text asset: its name and its content, which for a profile is the `.3dn` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityTextAsset {
    /// The asset's `m_Name`.
    pub name: String,
    /// The asset's `m_Script`, unmodified (AMD-030 §5).
    pub script: Vec<u8>,
}

impl UnityTextAsset {
    /// A text asset from its name and content.
    #[must_use]
    pub fn new(name: &str, script: Vec<u8>) -> Self {
        Self {
            name: name.to_owned(),
            script,
        }
    }

    /// Reads a `TextAsset` object.
    ///
    /// # Errors
    ///
    /// The object is not a `TextAsset`, or its lengths do not fit its bytes.
    pub fn read(asset: &UnityObject<'_>, big_endian: bool) -> Result<Self, UnityAssetFormatError> {
        if asset.class_id != CLASS_ID {
            return Err(UnityAssetFormatError::new(format!(
                "object {} has class {} and not the {CLASS_ID} of a TextAsset.",
                asset.path_id, asset.class_id
            )));
        }

        let mut reader = AssetReader::new(asset.data, big_endian);

        let name_length = reader.read_i32()?;
        let name_length = usize::try_from(name_length)
            .ok()
            .filter(|length| *length <= reader.remaining())
            .ok_or_else(|| {
                UnityAssetFormatError::new(format!(
                    "the TextAsset {} announces a name of {name_length} bytes in an object of {}.",
                    asset.path_id,
                    asset.data.len()
                ))
            })?;
        let name = String::from_utf8_lossy(reader.read_bytes(name_length)?).into_owned();

        // AMD-030 §4.4: the 4-byte alignment that follows m_Name is counted from the start of
        // the object, which is where this reader starts.
        reader.align(4)?;

        let script_length = reader.read_i32()?;
        let script_length = usize::try_from(script_length)
            .ok()
            .filter(|length| *length <= reader.remaining())
            .ok_or_else(|| {
                UnityAssetFormatError::new(format!(
                    "the TextAsset \"{name}\" announces {script_length} bytes of content and only \
                     {} are left in an object of {}.",
                    reader.remaining(),
                    asset.data.len()
                ))
            })?;

        // The payload is handed over unmodified, which is what AMD-030 §5 requires of an
        // imported profile.
        Ok(Self {
            script: reader.read_bytes(script_length)?.to_vec(),
            name,
        })
    }
}
