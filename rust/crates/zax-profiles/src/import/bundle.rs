//! The `UnityFS` bundle container (AMD-030 §4.1, §4.2): header, information block, data blocks
//! and the table of the files they hold.

use super::asset_reader::AssetReader;
use super::lz4;
use super::UnityAssetFormatError;

type Result<T> = std::result::Result<T, UnityAssetFormatError>;

/// Compression of the information block or of a data block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnityCompression {
    /// Stored.
    None,
    /// LZMA, which this reader does not support.
    Lzma,
    /// LZ4.
    Lz4,
    /// LZ4 high compression: the same block format as LZ4.
    Lz4Hc,
    /// A code the format does not define.
    Unknown(u32),
}

impl UnityCompression {
    fn of_flags(flags: u32) -> Self {
        match flags & COMPRESSION_MASK {
            0 => Self::None,
            1 => Self::Lzma,
            2 => Self::Lz4,
            3 => Self::Lz4Hc,
            other => Self::Unknown(other),
        }
    }
}

/// One file of a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnityBundleFile {
    /// Path inside the bundle.
    pub path: String,
    /// Node flags.
    pub flags: u32,
    /// The decompressed bytes.
    pub data: Vec<u8>,
}

/// A decoded `UnityFS` bundle.
#[derive(Debug, Clone)]
pub struct UnityBundle {
    /// Format version, 6 to 8.
    pub format_version: u32,
    /// Player version text.
    pub player_version: String,
    /// Engine version text.
    pub engine_version: String,
    /// The files of the bundle.
    pub files: Vec<UnityBundleFile>,
}

/// The signature every bundle starts with.
pub const SIGNATURE: &str = "UnityFS";
/// Oldest format version read.
pub const MINIMUM_FORMAT_VERSION: u32 = 6;
/// Newest format version read.
pub const MAXIMUM_FORMAT_VERSION: u32 = 8;

const COMPRESSION_MASK: u32 = 0x3F;
const FILE_LIST_FLAG: u32 = 0x40;
const INFO_AT_END_FLAG: u32 = 0x80;
const MAXIMUM_INFO_SIZE: u32 = 64 * 1024 * 1024;
const MAXIMUM_STREAM_SIZE: u64 = 1024 * 1024 * 1024;
const MAXIMUM_TABLE_LENGTH: i32 = 1 << 20;

struct Block {
    uncompressed: u32,
    compressed: u32,
    compression: UnityCompression,
}

struct Node {
    offset: i64,
    size: i64,
    flags: u32,
    path: String,
}

impl UnityBundle {
    /// Reads a bundle.
    ///
    /// # Errors
    ///
    /// The bytes are not a `UnityFS` bundle this reader supports (LZMA is not), or are
    /// truncated or inconsistent.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        // The header and the information block are big-endian whatever the serialized file
        // inside says (AMD-030 §4.1, §4.2).
        let mut reader = AssetReader::new(bytes, true);

        let signature = reader.read_null_terminated_string()?;
        if signature != SIGNATURE {
            return Err(UnityAssetFormatError::new(format!(
                "this is not a Unity asset bundle: the signature is \"{signature}\" and not \"{SIGNATURE}\"."
            )));
        }

        let format_version = reader.read_u32()?;
        if !(MINIMUM_FORMAT_VERSION..=MAXIMUM_FORMAT_VERSION).contains(&format_version) {
            return Err(UnityAssetFormatError::new(format!(
                "the bundle format version {format_version} is not supported; \
                 {MINIMUM_FORMAT_VERSION} to {MAXIMUM_FORMAT_VERSION} are."
            )));
        }

        let player_version = reader.read_null_terminated_string()?;
        let engine_version = reader.read_null_terminated_string()?;

        let total_size = reader.read_i64()?;
        let compressed_info_size = reader.read_u32()?;
        let uncompressed_info_size = reader.read_u32()?;
        let flags = reader.read_u32()?;

        if usize::try_from(total_size).ok() != Some(bytes.len()) {
            return Err(UnityAssetFormatError::new(format!(
                "the bundle announces {total_size} bytes and the file holds {}: \
                 it is truncated or not the file it claims to be.",
                bytes.len()
            )));
        }

        if flags & FILE_LIST_FLAG == 0 {
            return Err(UnityAssetFormatError::new(
                "the bundle's information block carries no file list (flag 0x40 is clear), \
                 so nothing can be extracted from it.",
            ));
        }

        // AMD-030 §4.1: from format version 7 on, what follows the header starts on a multiple
        // of 16 (the measured header ends at 50 and the information block starts at 64).
        if format_version >= 7 {
            reader.align(16)?;
        }

        if uncompressed_info_size > MAXIMUM_INFO_SIZE {
            return Err(UnityAssetFormatError::new(format!(
                "the bundle announces an information block of {uncompressed_info_size} bytes, \
                 more than the accepted {MAXIMUM_INFO_SIZE}."
            )));
        }

        let compressed_info_size = to_usize(compressed_info_size)?;
        let (compressed_info, data_start, data_end) = if flags & INFO_AT_END_FLAG == 0 {
            let info = reader.read_bytes(compressed_info_size)?;
            (info, reader.position(), bytes.len())
        } else {
            // The data blocks start right after the aligned header; the information block is
            // the tail of the file (AMD-030 §4.1 bit 0x80, §4.2 last paragraph).
            let data_start = reader.position();
            let info_start = bytes
                .len()
                .checked_sub(compressed_info_size)
                .filter(|start| *start >= data_start)
                .ok_or_else(|| {
                    UnityAssetFormatError::new(format!(
                        "the information block of {compressed_info_size} bytes does not fit at \
                         the end of a {}-byte bundle whose data start at {data_start}.",
                        bytes.len()
                    ))
                })?;
            (&bytes[info_start..], data_start, info_start)
        };

        let info = decompress(
            compressed_info,
            to_usize(uncompressed_info_size)?,
            UnityCompression::of_flags(flags),
            "the bundle's information block",
        )?;

        let files = read_info_and_blocks(&info, bytes, data_start, data_end)?;
        Ok(Self {
            format_version,
            player_version,
            engine_version,
            files,
        })
    }
}

fn read_info_and_blocks(
    info: &[u8],
    bytes: &[u8],
    data_start: usize,
    data_end: usize,
) -> Result<Vec<UnityBundleFile>> {
    let mut reader = AssetReader::new(info, true);

    // 16 bytes of hash, which nothing here needs (AMD-030 §4.2).
    reader.skip(16)?;

    let block_count = reader.read_i32()?;
    check_table_length(block_count, "data blocks")?;

    let mut blocks = Vec::new();
    let mut stream_size = 0_u64;
    let mut compressed_size = 0_u64;
    for _ in 0..block_count {
        let uncompressed = reader.read_u32()?;
        let compressed = reader.read_u32()?;
        let block_flags = u32::from(reader.read_u16()?);
        stream_size += u64::from(uncompressed);
        compressed_size += u64::from(compressed);
        blocks.push(Block {
            uncompressed,
            compressed,
            compression: UnityCompression::of_flags(block_flags),
        });
    }

    if stream_size > MAXIMUM_STREAM_SIZE {
        return Err(UnityAssetFormatError::new(format!(
            "the bundle's blocks announce {stream_size} bytes once decompressed, \
             more than the accepted {MAXIMUM_STREAM_SIZE}."
        )));
    }

    let available = data_end - data_start;
    if usize::try_from(compressed_size).map_or(true, |needed| needed > available) {
        return Err(UnityAssetFormatError::new(format!(
            "the bundle's {block_count} data blocks need {compressed_size} bytes from offset \
             {data_start} and only {available} are available: the file is truncated \
             or its block table is inconsistent."
        )));
    }

    let node_count = reader.read_i32()?;
    check_table_length(node_count, "file nodes")?;

    let mut nodes = Vec::new();
    for _ in 0..node_count {
        nodes.push(Node {
            offset: reader.read_i64()?,
            size: reader.read_i64()?,
            flags: reader.read_u32()?,
            path: reader.read_null_terminated_string()?,
        });
    }

    let stream = expand(&blocks, bytes, data_start, to_usize_wide(stream_size)?)?;

    nodes
        .into_iter()
        .map(|node| {
            let range = usize::try_from(node.offset)
                .ok()
                .zip(usize::try_from(node.size).ok())
                .and_then(|(offset, size)| Some(offset..offset.checked_add(size)?))
                .filter(|range| range.end <= stream.len())
                .ok_or_else(|| {
                    UnityAssetFormatError::new(format!(
                        "the bundle node \"{}\" claims {} bytes at offset {} of a stream of \
                         {} bytes: the file table is out of range.",
                        node.path,
                        node.size,
                        node.offset,
                        stream.len()
                    ))
                })?;
            Ok(UnityBundleFile {
                data: stream[range].to_vec(),
                path: node.path,
                flags: node.flags,
            })
        })
        .collect()
}

fn expand(
    blocks: &[Block],
    bytes: &[u8],
    data_start: usize,
    stream_size: usize,
) -> Result<Vec<u8>> {
    let mut stream = vec![0_u8; stream_size];
    let mut source = data_start;
    let mut written = 0;

    for (index, block) in blocks.iter().enumerate() {
        let compressed = to_usize(block.compressed)?;
        let uncompressed = to_usize(block.uncompressed)?;
        let input = &bytes[source..source + compressed];
        let target = &mut stream[written..written + uncompressed];

        let produced = decompress_into(
            input,
            target,
            block.compression,
            &format!("the data block {index}"),
        )?;
        if produced != uncompressed {
            return Err(UnityAssetFormatError::new(format!(
                "the data block {index} decodes to {produced} bytes and not the announced \
                 {uncompressed}."
            )));
        }

        source += compressed;
        written += uncompressed;
    }
    Ok(stream)
}

fn decompress(
    source: &[u8],
    uncompressed_size: usize,
    compression: UnityCompression,
    what: &str,
) -> Result<Vec<u8>> {
    let mut target = vec![0_u8; uncompressed_size];
    let produced = decompress_into(source, &mut target, compression, what)?;
    if produced != uncompressed_size {
        return Err(UnityAssetFormatError::new(format!(
            "{what} decodes to {produced} bytes and not the announced {uncompressed_size}."
        )));
    }
    Ok(target)
}

/// Decodes `source` into `target` and returns the number of bytes produced; a stored block is
/// copied and must be exactly as long as its target.
fn decompress_into(
    source: &[u8],
    target: &mut [u8],
    compression: UnityCompression,
    what: &str,
) -> Result<usize> {
    match compression {
        UnityCompression::None => {
            if source.len() != target.len() {
                return Err(UnityAssetFormatError::new(format!(
                    "{what} is stored uncompressed and announces {} bytes for {} decompressed ones.",
                    source.len(),
                    target.len()
                )));
            }
            target.copy_from_slice(source);
            Ok(source.len())
        }
        UnityCompression::Lz4 | UnityCompression::Lz4Hc => lz4::decode(source, target),
        UnityCompression::Lzma => Err(UnityAssetFormatError::new(format!(
            "{what} is compressed with LZMA, which this reader does not support; \
             only uncompressed, LZ4 and LZ4HC bundles can be read."
        ))),
        UnityCompression::Unknown(code) => Err(UnityAssetFormatError::new(format!(
            "{what} uses the unknown compression code {code}."
        ))),
    }
}

fn check_table_length(count: i32, what: &str) -> Result<()> {
    if (0..=MAXIMUM_TABLE_LENGTH).contains(&count) {
        Ok(())
    } else {
        Err(UnityAssetFormatError::new(format!(
            "the bundle announces {count} {what}, which is not a usable count."
        )))
    }
}

fn to_usize(value: u32) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| UnityAssetFormatError::new(format!("{value} does not fit in memory.")))
}

fn to_usize_wide(value: u64) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| UnityAssetFormatError::new(format!("{value} does not fit in memory.")))
}
