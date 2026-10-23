//! The Unity serialized file (AMD-030 §4.3): the header, the type table and the object table of
//! the files inside a bundle.

use super::asset_reader::AssetReader;
use super::UnityAssetFormatError;

type Result<T> = std::result::Result<T, UnityAssetFormatError>;

/// One object of a serialized file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnityObject<'a> {
    /// Path identifier.
    pub path_id: i64,
    /// Class identifier.
    pub class_id: i32,
    /// The object's own bytes, a slice of the file.
    pub data: &'a [u8],
}

/// A serialized file.
#[derive(Debug, Clone)]
pub struct UnitySerializedFile<'a> {
    /// Format version, 17 to 22.
    pub version: u32,
    /// True when the file's own values are big-endian.
    pub is_big_endian: bool,
    /// Unity version text.
    pub unity_version: String,
    /// Target platform code.
    pub target_platform: i32,
    /// The objects.
    pub objects: Vec<UnityObject<'a>>,
}

/// Oldest version read.
pub const MINIMUM_VERSION: u32 = 17;
/// Newest version read.
pub const MAXIMUM_VERSION: u32 = 22;

const MONO_BEHAVIOUR_CLASS_ID: i32 = 114;
const MAXIMUM_TABLE_LENGTH: i32 = 1 << 20;
const MAXIMUM_STRING_BUFFER: i32 = 64 * 1024 * 1024;

impl<'a> UnitySerializedFile<'a> {
    /// Reads a serialized file.
    ///
    /// # Errors
    ///
    /// The bytes are not a serialized file of a supported version, or are truncated or
    /// inconsistent.
    pub fn read(bytes: &'a [u8]) -> Result<Self> {
        // AMD-030 §4.3: the header is big-endian, whatever order it then announces.
        let mut reader = AssetReader::new(bytes, true);

        let mut metadata_size = i64::from(reader.read_u32()?);
        let mut file_size = i64::from(reader.read_u32()?);
        let version = reader.read_u32()?;
        let mut data_offset = i64::from(reader.read_u32()?);
        let big_endian = reader.read_byte()? != 0;
        reader.skip(3)?;

        if !(MINIMUM_VERSION..=MAXIMUM_VERSION).contains(&version) {
            return Err(UnityAssetFormatError::new(format!(
                "the serialized file version {version} is not supported; \
                 {MINIMUM_VERSION} to {MAXIMUM_VERSION} are."
            )));
        }

        if version >= 22 {
            // Q-071: from version 22 the sizes and offsets are given again, the file size and
            // the data offset as 64-bit fields and the metadata size as a 32-bit one, followed
            // by one reserved 64-bit field. No measured file uses this branch (the installation
            // carries version 21), so the public layout is the one implemented.
            metadata_size = i64::from(reader.read_u32()?);
            file_size = reader.read_i64()?;
            data_offset = reader.read_i64()?;
            reader.skip(8)?;
        }

        let length = i64::try_from(bytes.len()).unwrap_or(i64::MAX);
        if file_size > length {
            return Err(UnityAssetFormatError::new(format!(
                "the serialized file announces {file_size} bytes and only {length} are there: \
                 it is truncated."
            )));
        }
        if data_offset < 0 || data_offset > length {
            return Err(UnityAssetFormatError::new(format!(
                "the serialized file puts its data at offset {data_offset} of {length} bytes."
            )));
        }
        if metadata_size < 0 {
            return Err(UnityAssetFormatError::new(format!(
                "the serialized file announces a metadata block of {metadata_size} bytes."
            )));
        }

        reader.big_endian = big_endian;

        let unity_version = reader.read_null_terminated_string()?;
        let target_platform = reader.read_i32()?;
        let enable_type_tree = reader.read_byte()? != 0;

        let type_count = reader.read_i32()?;
        check_table_length(type_count, "types")?;

        let mut class_ids = Vec::new();
        for _ in 0..type_count {
            class_ids.push(read_type(&mut reader, version, enable_type_tree)?);
        }

        let object_count = reader.read_i32()?;
        check_table_length(object_count, "objects")?;

        let mut objects = Vec::new();
        for index in 0..object_count {
            // AMD-030 §4.3: the object table is aligned on 4 bytes from the start of the file.
            // Each record is a multiple of 4 bytes long, so this only ever moves on the first
            // one, but it is what the format says.
            reader.align(4)?;

            let path_id = reader.read_i64()?;
            let byte_start = if version >= 22 {
                reader.read_i64()?
            } else {
                i64::from(reader.read_u32()?)
            };
            let byte_size = i64::from(reader.read_u32()?);
            let type_index = reader.read_i32()?;

            let class_id = usize::try_from(type_index)
                .ok()
                .and_then(|type_index| class_ids.get(type_index))
                .copied()
                .ok_or_else(|| {
                    UnityAssetFormatError::new(format!(
                        "object {index} of the serialized file points at type {type_index} of {type_count}."
                    ))
                })?;

            let start = data_offset + byte_start;
            let range = (byte_start >= 0 && byte_size >= 0 && start + byte_size <= length)
                .then(|| {
                    usize::try_from(start)
                        .ok()
                        .zip(usize::try_from(byte_size).ok())
                })
                .flatten()
                .map(|(start, size)| start..start + size)
                .ok_or_else(|| {
                    UnityAssetFormatError::new(format!(
                        "object {index} of the serialized file claims {byte_size} bytes at offset \
                         {start} of {length}: the object table is out of range."
                    ))
                })?;
            objects.push(UnityObject {
                path_id,
                class_id,
                data: &bytes[range],
            });
        }

        Ok(Self {
            version,
            is_big_endian: big_endian,
            unity_version,
            target_platform,
            objects,
        })
    }
}

fn read_type(reader: &mut AssetReader<'_>, version: u32, enable_type_tree: bool) -> Result<i32> {
    let class_id = reader.read_i32()?;
    reader.skip(1)?; // The stripped flag.
    reader.skip(2)?; // The script type index.

    if class_id == MONO_BEHAVIOUR_CLASS_ID {
        reader.skip(16)?; // The script GUID.
    }

    reader.skip(16)?; // The old type hash.

    // Q-071: the type dependencies of version 21 and later belong to the type-tree block in the
    // public format, so nothing more is read when the tree is absent.
    if !enable_type_tree {
        return Ok(class_id);
    }

    let node_count = reader.read_i32()?;
    check_table_length(node_count, "type-tree nodes")?;

    let string_buffer_size = reader.read_i32()?;
    if !(0..=MAXIMUM_STRING_BUFFER).contains(&string_buffer_size) {
        return Err(UnityAssetFormatError::new(format!(
            "a type tree announces a string buffer of {string_buffer_size} bytes."
        )));
    }

    let node_size = if version >= 19 { 32 } else { 24 };
    reader.skip(to_usize(node_count)? * node_size)?;
    reader.skip(to_usize(string_buffer_size)?)?;

    if version >= 21 {
        let dependency_count = reader.read_i32()?;
        check_table_length(dependency_count, "type dependencies")?;
        reader.skip(to_usize(dependency_count)? * 4)?;
    }

    Ok(class_id)
}

fn check_table_length(count: i32, what: &str) -> Result<()> {
    if (0..=MAXIMUM_TABLE_LENGTH).contains(&count) {
        Ok(())
    } else {
        Err(UnityAssetFormatError::new(format!(
            "the serialized file announces {count} {what}, which is not a usable count."
        )))
    }
}

fn to_usize(value: i32) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| UnityAssetFormatError::new(format!("{value} is not a usable length.")))
}
