//! Builds synthetic "3dh" envelopes and payloads, for the branches AMD-062 cannot test on the
//! 101 built-in files: envelope versions 1 and 2, text entries of field 4, legacy digests,
//! malformed manifests (REQ-PROF-072, notes), tiles that carry their planes anyway and the
//! `textureKind` values REQ-PROF-074 refuses.

use zax_profiles::hash::tile_id::{self, TileId};

use super::proto_builder::ProtoBuilder;

/// Field numbers of the payload messages, written here by hand: the production writer is private
/// to `zax-profiles`, and these shapes are not ones it produces.
mod field {
    pub const ROOT_TYPE: u32 = 1;
    pub const ROOT_VER: u32 = 2;
    pub const ROOT_PATTERNS: u32 = 4;
    pub const ENVELOPE_NEW_KEY: u32 = 2;
    pub const ENVELOPE_MESSAGE: u32 = 10;
    pub const PATTERN_GRID: u32 = 1;
    pub const PATTERN_TILE_SIZE: u32 = 3;
    pub const PATTERN_VARIANTS: u32 = 10;
    pub const CELL_PLANES: u32 = 1;
    pub const TILE_LOW_PLANE: u32 = 1;
    pub const TILE_HIGH_PLANE: u32 = 2;
    pub const TILE_FLIP: u32 = 3;
    pub const TILE_HASH_TEXT: u32 = 4;
    pub const TILE_HASH_ID: u32 = 5;
    pub const VARIANT_SIZE_Z: u32 = 1;
    pub const VARIANT_TEX: u32 = 19;
    pub const VARIANT_XOR_TEX: u32 = 24;
    pub const VARIANT_TEXTURE_KIND: u32 = 25;
    pub const TEX_RECT_WIDTH: u32 = 1;
    pub const TEX_RECT_HEIGHT: u32 = 2;
    pub const TEX_RECT_PAYLOAD: u32 = 10;
    pub const TEX_PAYLOAD_DATA: u32 = 1;
    pub const TEX_PAYLOAD_FORMAT: u32 = 2;
    pub const TEX_PAYLOAD_EXTRA: u32 = 3;
    pub const VALUE_X: u32 = 1;
    pub const VALUE_Y: u32 = 2;
}

/// How a tile of a synthetic payload names itself.
#[derive(Debug, Clone)]
pub enum TileName {
    /// Field 5, the sixteen bytes of an identifier (REQ-PROF-075).
    Identifier(TileId),
    /// Field 5 with another number of bytes, which no manifest can hold.
    RawIdentifier(Vec<u8>),
    /// Field 4, a string (REQ-PROF-076).
    Text(String),
    /// Neither: an ordinary tile.
    None,
}

/// One cell of a synthetic pattern.
#[derive(Debug, Clone)]
pub struct TileSpec {
    pub name: TileName,
    /// The planes as the file stores them (masked), when the tile carries them anyway.
    pub stored_planes: Option<(i64, i64)>,
    pub flip: i32,
    pub write_flip: bool,
}

impl Default for TileSpec {
    fn default() -> Self {
        Self {
            name: TileName::None,
            stored_planes: None,
            flip: 0,
            write_flip: true,
        }
    }
}

impl TileSpec {
    /// A tile waiting for its planes, named by its identifier in field 5.
    pub fn named(id: TileId) -> Self {
        Self {
            name: TileName::Identifier(id),
            ..Self::default()
        }
    }

    /// A tile waiting for its planes, named by a string in field 4.
    pub fn text(key: &str) -> Self {
        Self {
            name: TileName::Text(key.to_owned()),
            ..Self::default()
        }
    }

    pub fn with_flip(mut self, flip: i32) -> Self {
        self.flip = flip;
        self
    }

    pub fn with_stored_planes(mut self, low: i64, high: i64) -> Self {
        self.stored_planes = Some((low, high));
        self
    }
}

/// A `TexRect` of a synthetic payload.
#[derive(Debug, Clone)]
pub struct TextureSpec {
    pub width: i32,
    pub height: i32,
    pub format: i32,
    pub data: Vec<i32>,
    pub extra: Vec<i32>,
    /// False to leave the `TexPayload` out altogether.
    pub write_payload: bool,
}

impl TextureSpec {
    pub fn indexed(width: i32, height: i32, data: Vec<i32>) -> Self {
        Self {
            width,
            height,
            format: 0,
            data,
            extra: Vec::new(),
            write_payload: true,
        }
    }
}

/// One `ShapeParams` of a synthetic payload.
#[derive(Debug, Clone, Default)]
pub struct VariantSpec {
    pub depth: i32,
    /// Field 25, left out when none.
    pub texture_kind: Option<i32>,
    /// Field 24.
    pub xor_texture: Option<TextureSpec>,
    /// Field 19, which a "3dh" variant does not carry.
    pub texture: Option<TextureSpec>,
}

impl VariantSpec {
    pub fn of_kind(kind: i32) -> Self {
        Self {
            texture_kind: Some(kind),
            ..Self::default()
        }
    }

    pub fn with_xor_texture(mut self, texture: TextureSpec) -> Self {
        self.xor_texture = Some(texture);
        self
    }
}

/// One pattern of a synthetic payload.
#[derive(Debug, Clone)]
pub struct PatternSpec {
    pub tile_size: (i32, i32),
    /// Cells row by row; a `None` entry is an empty cell.
    pub cells: Vec<Option<TileSpec>>,
    pub variants: Vec<VariantSpec>,
}

impl Default for PatternSpec {
    fn default() -> Self {
        Self {
            tile_size: (1, 1),
            cells: Vec::new(),
            variants: vec![VariantSpec::of_kind(1)],
        }
    }
}

impl PatternSpec {
    /// A pattern of `cells.len()` by 1 cells.
    pub fn row(cells: Vec<Option<TileSpec>>) -> Self {
        Self {
            tile_size: (i32::try_from(cells.len()).unwrap(), 1),
            cells,
            ..Self::default()
        }
    }

    pub fn with_variants(mut self, variants: Vec<VariantSpec>) -> Self {
        self.variants = variants;
        self
    }
}

/// The payload of a synthetic "3dh" profile (REQ-PROF-074).
#[derive(Debug, Clone)]
pub struct PayloadSpec {
    pub signature: Option<String>,
    pub version: Option<String>,
    pub patterns: Vec<PatternSpec>,
}

impl Default for PayloadSpec {
    fn default() -> Self {
        Self {
            signature: Some("3dn".to_owned()),
            version: Some("2".to_owned()),
            patterns: Vec::new(),
        }
    }
}

impl PayloadSpec {
    pub fn of(patterns: Vec<PatternSpec>) -> Self {
        Self {
            patterns,
            ..Self::default()
        }
    }

    /// The bytes of this payload. Reference keys are handed out in the order the decoder meets
    /// them (REQ-PROF-011), so the messages are built in document order.
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoBuilder::new();
        if let Some(signature) = &self.signature {
            writer.string(field::ROOT_TYPE, signature);
        }
        if let Some(version) = &self.version {
            writer.string(field::ROOT_VER, version);
        }

        let mut next_key = 1;
        for pattern in &self.patterns {
            let key = next_key;
            next_key += 1;
            let body = encode_pattern(pattern, &mut next_key);
            writer.message(field::ROOT_PATTERNS, &envelope(key, &body));
        }

        writer.into_bytes()
    }
}

/// A reference envelope that carries a new object under `key` (REQ-PROF-011).
fn envelope(key: i32, message: &ProtoBuilder) -> ProtoBuilder {
    let mut writer = ProtoBuilder::new();
    writer.int32(field::ENVELOPE_NEW_KEY, key);
    writer.message(field::ENVELOPE_MESSAGE, message);
    writer
}

fn encode_pattern(spec: &PatternSpec, next_key: &mut i32) -> ProtoBuilder {
    let mut writer = ProtoBuilder::new();
    for cell in &spec.cells {
        writer.group_start(field::PATTERN_GRID);
        if let Some(tile) = cell {
            let key = *next_key;
            *next_key += 1;
            let mut body = ProtoBuilder::new();
            body.message(field::CELL_PLANES, &encode_tile(tile));
            // Inside the group, field 1 again: the reference envelope of the cell.
            writer.message(field::PATTERN_GRID, &envelope(key, &body));
        }
        writer.group_end(field::PATTERN_GRID);
    }

    let mut size = ProtoBuilder::new();
    size.int32(field::VALUE_X, spec.tile_size.0);
    size.int32(field::VALUE_Y, spec.tile_size.1);
    writer.message(field::PATTERN_TILE_SIZE, &size);

    for variant in &spec.variants {
        let key = *next_key;
        *next_key += 1;
        writer.message(
            field::PATTERN_VARIANTS,
            &envelope(key, &encode_variant(variant)),
        );
    }

    writer
}

fn encode_tile(spec: &TileSpec) -> ProtoBuilder {
    let mut writer = ProtoBuilder::new();
    if let Some((low, high)) = spec.stored_planes {
        writer.int64(field::TILE_LOW_PLANE, low);
        writer.int64(field::TILE_HIGH_PLANE, high);
    }
    if spec.write_flip {
        writer.int32(field::TILE_FLIP, spec.flip);
    }
    match &spec.name {
        TileName::Identifier(id) => writer.raw_bytes(field::TILE_HASH_ID, &id.to_binary()),
        TileName::RawIdentifier(bytes) => writer.raw_bytes(field::TILE_HASH_ID, bytes),
        TileName::Text(key) => writer.string(field::TILE_HASH_TEXT, key),
        TileName::None => {}
    }

    writer
}

fn encode_variant(spec: &VariantSpec) -> ProtoBuilder {
    let mut writer = ProtoBuilder::new();
    writer.int32(field::VARIANT_SIZE_Z, spec.depth);
    if let Some(texture) = &spec.texture {
        writer.message(field::VARIANT_TEX, &encode_texture(texture));
    }
    if let Some(texture) = &spec.xor_texture {
        writer.message(field::VARIANT_XOR_TEX, &encode_texture(texture));
    }
    if let Some(kind) = spec.texture_kind {
        writer.int32(field::VARIANT_TEXTURE_KIND, kind);
    }

    writer
}

fn encode_texture(spec: &TextureSpec) -> ProtoBuilder {
    let mut writer = ProtoBuilder::new();
    writer.int32(field::TEX_RECT_WIDTH, spec.width);
    writer.int32(field::TEX_RECT_HEIGHT, spec.height);
    if spec.write_payload {
        let mut payload = ProtoBuilder::new();
        for word in &spec.data {
            payload.int32(field::TEX_PAYLOAD_DATA, *word);
        }
        payload.int32(field::TEX_PAYLOAD_FORMAT, spec.format);
        for word in &spec.extra {
            payload.int32(field::TEX_PAYLOAD_EXTRA, *word);
        }
        writer.message(field::TEX_RECT_PAYLOAD, &payload);
    }

    writer
}

/// Every value of an envelope under the test's control, so that a refusal of REQ-PROF-072 can be
/// provoked (REQ-PROF-071 writes the fields in the order 1, 2, 3, 4, 6 to 12).
#[derive(Debug, Clone)]
pub struct EnvelopeSpec {
    pub signature: Option<String>,
    pub version: Option<String>,
    pub payload: Option<Vec<u8>>,
    pub hashes: Vec<String>,
    pub ids: Vec<TileId>,
    /// Field 12 written verbatim, for a table whose length is not a multiple of sixteen.
    pub raw_ids: Option<Vec<u8>>,
    pub write_payload_hash: bool,
    pub payload_hash_override: Option<String>,
    pub id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub description: Option<String>,
}

impl Default for EnvelopeSpec {
    fn default() -> Self {
        Self {
            signature: Some("3dh".to_owned()),
            version: Some("5".to_owned()),
            payload: Some(Vec::new()),
            hashes: Vec::new(),
            ids: Vec::new(),
            raw_ids: None,
            write_payload_hash: true,
            payload_hash_override: None,
            id: Some("SyntheticId".to_owned()),
            title: Some("Synthetic".to_owned()),
            author: None,
            publisher: None,
            description: None,
        }
    }
}

impl EnvelopeSpec {
    /// The bytes of this envelope.
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoBuilder::new();
        if let Some(signature) = &self.signature {
            writer.string(1, signature);
        }
        if let Some(version) = &self.version {
            writer.string(2, version);
        }
        if let Some(payload) = &self.payload {
            writer.raw_bytes(3, payload);
        }
        for hash in &self.hashes {
            writer.string(4, hash);
        }
        if self.write_payload_hash {
            let digest = self.payload_hash_override.clone().unwrap_or_else(|| {
                use sha2::{Digest, Sha256};
                let payload = self.payload.as_deref().unwrap_or_default();
                zax_profiles::hash::raw_tile::hex_lowercase(Sha256::digest(payload).as_slice())
            });
            writer.string(6, &digest);
        }
        if let Some(id) = &self.id {
            writer.string(7, id);
        }
        if let Some(title) = &self.title {
            writer.string(8, title);
        }
        if let Some(author) = &self.author {
            writer.string(9, author);
        }
        if let Some(publisher) = &self.publisher {
            writer.string(10, publisher);
        }
        if let Some(description) = &self.description {
            writer.string(11, description);
        }

        let table = self.raw_ids.clone().unwrap_or_else(|| {
            let mut table = Vec::with_capacity(self.ids.len() * tile_id::BYTE_COUNT);
            for id in &self.ids {
                table.extend_from_slice(&id.to_binary());
            }
            table
        });
        if !table.is_empty() {
            writer.raw_bytes(12, &table);
        }

        writer.into_bytes()
    }
}
