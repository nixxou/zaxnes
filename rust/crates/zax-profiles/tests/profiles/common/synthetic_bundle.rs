//! A builder of `UnityFS` bundles in every layout the reader supports (AMD-030 §4), so the
//! reader is tested without a 3dSen installation.

use zax_profiles::import::{UnityCompression, BUNDLE_SIGNATURE, TEXT_ASSET_CLASS_ID};

use super::lz4_encoder;

/// Class identifier of the `AssetBundle` object, which the import ignores.
pub const ASSET_BUNDLE_CLASS_ID: i32 = 142;

pub const ENGINE_VERSION: &str = "2019.4.41f2";

#[derive(Clone)]
#[allow(clippy::struct_excessive_bools)] // one switch per layout variant, as in the C# options
pub struct SyntheticBundleOptions {
    pub format_version: u32,
    pub info_compression: UnityCompression,
    pub block_compression: UnityCompression,
    pub block_size: usize,
    pub info_at_end: bool,
    pub file_list: bool,
    pub serialized_version: u32,
    pub big_endian: bool,
    pub type_tree: bool,
    pub node_path: String,
    pub info_compression_code: Option<u32>,
    pub block_compression_code: Option<u16>,
    pub block_uncompressed_size_code: Option<u32>,
    pub node_offset_delta: i64,
    pub node_size_delta: i64,
}

impl Default for SyntheticBundleOptions {
    fn default() -> Self {
        Self {
            format_version: 7,
            info_compression: UnityCompression::None,
            block_compression: UnityCompression::None,
            block_size: usize::MAX,
            info_at_end: false,
            file_list: true,
            serialized_version: 21,
            big_endian: false,
            type_tree: true,
            node_path: "CAB-0123456789abcdef0123456789abcdef".to_owned(),
            info_compression_code: None,
            block_compression_code: None,
            block_uncompressed_size_code: None,
            node_offset_delta: 0,
            node_size_delta: 0,
        }
    }
}

/// The numeric code of a compression, as the flags store it.
pub fn compression_code(compression: UnityCompression) -> u32 {
    match compression {
        UnityCompression::None => 0,
        UnityCompression::Lzma => 1,
        UnityCompression::Lz4 => 2,
        UnityCompression::Lz4Hc => 3,
        UnityCompression::Unknown(code) => code,
    }
}

pub fn build(assets: &[(&str, Vec<u8>)], options: &SyntheticBundleOptions) -> Vec<u8> {
    wrap(&build_serialized_file(assets, options), options)
}

pub fn build_serialized_file(
    assets: &[(&str, Vec<u8>)],
    options: &SyntheticBundleOptions,
) -> Vec<u8> {
    // 1. The object data, each object aligned on 8 bytes inside the data area.
    let mut data = ByteWriter::new(options.big_endian);
    let mut table: Vec<(usize, usize, i32)> = Vec::new();
    for (name, script) in assets {
        data.align_to(8, 0);
        let start = data.len();
        data.i32(i32::try_from(name.len()).unwrap());
        data.bytes(name.as_bytes());
        data.align_to(4, start);
        data.i32(i32::try_from(script.len()).unwrap());
        data.bytes(script);
        table.push((start, data.len() - start, 0));
    }

    // The AssetBundle object of class 142, which AMD-030 §4.4 says to ignore: a plain name.
    data.align_to(8, 0);
    let bundle_start = data.len();
    let bundle_name = "defaultprofiles_assets_all";
    data.i32(i32::try_from(bundle_name.len()).unwrap());
    data.bytes(bundle_name.as_bytes());
    data.align_to(4, bundle_start);
    table.push((bundle_start, data.len() - bundle_start, 1));

    let objects = data.into_bytes();

    // 2. The metadata, in the announced byte order.
    let mut metadata = ByteWriter::new(options.big_endian);
    metadata.c_string(ENGINE_VERSION);
    metadata.i32(19); // Target platform: StandaloneWindows64.
    metadata.byte(u8::from(options.type_tree));
    metadata.i32(2);
    write_type(&mut metadata, TEXT_ASSET_CLASS_ID, options);
    write_type(&mut metadata, ASSET_BUNDLE_CLASS_ID, options);
    metadata.i32(i32::try_from(table.len()).unwrap());

    // 3. The header, big-endian, then the metadata, then the data at an aligned offset.
    let header_size = if options.serialized_version >= 22 {
        20 + 28
    } else {
        20
    };

    // The object table is aligned on 4 bytes from the start of the file (AMD-030 §4.3); the
    // padding is computed here and written as part of the metadata.
    let object_table_start = align(header_size + metadata.len(), 4);
    let object_record_size = if options.serialized_version >= 22 {
        24
    } else {
        20
    };
    let metadata_end = object_table_start + table.len() * object_record_size;
    let data_offset = align(metadata_end, 16);

    let mut file = ByteWriter::new(true);
    file.u32(u32::try_from(metadata_end - header_size).unwrap());
    file.u32(u32::try_from(data_offset + objects.len()).unwrap());
    file.u32(options.serialized_version);
    file.u32(u32::try_from(data_offset).unwrap());
    file.byte(u8::from(options.big_endian));
    file.bytes(&[0, 0, 0]);

    if options.serialized_version >= 22 {
        // Q-071: the public layout, a u32 metadata size, an i64 file size, an i64 data offset
        // and one reserved i64.
        file.u32(u32::try_from(metadata_end - header_size).unwrap());
        file.i64(i64::try_from(data_offset + objects.len()).unwrap());
        file.i64(i64::try_from(data_offset).unwrap());
        file.i64(0);
    }

    file.bytes(&metadata.into_bytes());
    file.pad_to(object_table_start);

    file.big_endian = options.big_endian;
    for (start, size, type_index) in &table {
        file.i64(0x1000 + i64::try_from(*start).unwrap());
        if options.serialized_version >= 22 {
            file.i64(i64::try_from(*start).unwrap());
        } else {
            file.u32(u32::try_from(*start).unwrap());
        }
        file.u32(u32::try_from(*size).unwrap());
        file.i32(*type_index);
    }

    file.pad_to(data_offset);
    file.bytes(&objects);
    file.into_bytes()
}

pub fn wrap(serialized: &[u8], options: &SyntheticBundleOptions) -> Vec<u8> {
    // The data blocks: the stream is the single file node, cut into blocks.
    let block_size = options.block_size.min(serialized.len().max(1));
    let mut blocks: Vec<(usize, Vec<u8>)> = serialized
        .chunks(block_size)
        .map(|plain| (plain.len(), store(plain, options.block_compression)))
        .collect();
    if blocks.is_empty() {
        blocks.push((0, Vec::new()));
    }

    // The information block.
    let mut info = ByteWriter::new(true);
    info.bytes(&[0; 16]); // The hash, ignored by the reader.
    info.i32(i32::try_from(blocks.len()).unwrap());
    for (uncompressed, stored) in &blocks {
        info.u32(
            options
                .block_uncompressed_size_code
                .unwrap_or_else(|| u32::try_from(*uncompressed).unwrap()),
        );
        info.u32(u32::try_from(stored.len()).unwrap());
        info.u16(options.block_compression_code.unwrap_or_else(|| {
            u16::try_from(compression_code(options.block_compression)).unwrap()
        }));
    }

    info.i32(1);
    info.i64(options.node_offset_delta);
    info.i64(i64::try_from(serialized.len()).unwrap() + options.node_size_delta);
    info.u32(4);
    info.c_string(&options.node_path);

    let plain_info = info.into_bytes();
    let stored_info = store(&plain_info, options.info_compression);

    let mut flags = options
        .info_compression_code
        .unwrap_or_else(|| compression_code(options.info_compression));
    if options.file_list {
        flags |= 0x40;
    }
    if options.info_at_end {
        flags |= 0x80;
    }

    let mut header = ByteWriter::new(true);
    header.c_string(BUNDLE_SIGNATURE);
    header.u32(options.format_version);
    header.c_string("5.x.x");
    header.c_string(ENGINE_VERSION);
    header.i64(0); // Patched below, once the whole length is known.
    header.u32(u32::try_from(stored_info.len()).unwrap());
    header.u32(u32::try_from(plain_info.len()).unwrap());
    header.u32(flags);

    let size_field = header.len() - 20;
    if options.format_version >= 7 {
        header.align_to(16, 0);
    }

    if !options.info_at_end {
        header.bytes(&stored_info);
    }
    for (_, stored) in &blocks {
        header.bytes(stored);
    }
    if options.info_at_end {
        header.bytes(&stored_info);
    }

    let mut bundle = header.into_bytes();
    let length = i64::try_from(bundle.len()).unwrap();
    bundle[size_field..size_field + 8].copy_from_slice(&length.to_be_bytes());
    bundle
}

fn store(plain: &[u8], compression: UnityCompression) -> Vec<u8> {
    match compression {
        UnityCompression::Lz4 | UnityCompression::Lz4Hc => lz4_encoder::compress(plain),

        // LZMA is never produced: the bundles that claim it carry stored bytes and an
        // overridden compression code, which is exactly the refusal AMD-030 §4.1 asks for.
        UnityCompression::None | UnityCompression::Lzma | UnityCompression::Unknown(_) => {
            plain.to_vec()
        }
    }
}

const NODE_COUNT: usize = 3;

fn write_type(metadata: &mut ByteWriter, class_id: i32, options: &SyntheticBundleOptions) {
    metadata.i32(class_id);
    metadata.byte(0); // Not a stripped type.
    metadata.i16(-1); // No script type index.
    metadata.bytes(&[0; 16]); // The old type hash.

    if !options.type_tree {
        return;
    }

    let strings = b"m_Name\0m_Script\0";
    metadata.i32(i32::try_from(NODE_COUNT).unwrap());
    metadata.i32(i32::try_from(strings.len()).unwrap());
    let node_size = if options.serialized_version >= 19 {
        32
    } else {
        24
    };
    metadata.bytes(&vec![0; NODE_COUNT * node_size]);
    metadata.bytes(strings);

    if options.serialized_version >= 21 {
        metadata.i32(1);
        metadata.i32(class_id);
    }
}

fn align(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}

struct ByteWriter {
    bytes: Vec<u8>,
    big_endian: bool,
}

impl ByteWriter {
    fn new(big_endian: bool) -> Self {
        Self {
            bytes: Vec::new(),
            big_endian,
        }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn c_string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
        self.bytes.push(0);
    }

    fn i16(&mut self, value: i16) {
        let encoded = if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        self.bytes(&encoded);
    }

    fn u16(&mut self, value: u16) {
        let encoded = if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        self.bytes(&encoded);
    }

    fn i32(&mut self, value: i32) {
        let encoded = if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        self.bytes(&encoded);
    }

    fn u32(&mut self, value: u32) {
        let encoded = if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        self.bytes(&encoded);
    }

    fn i64(&mut self, value: i64) {
        let encoded = if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        self.bytes(&encoded);
    }

    fn align_to(&mut self, alignment: usize, origin: usize) {
        self.pad_to(origin + align(self.bytes.len() - origin, alignment));
    }

    fn pad_to(&mut self, position: usize) {
        if self.bytes.len() < position {
            self.bytes.resize(position, 0);
        }
    }
}
