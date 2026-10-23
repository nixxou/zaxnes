//! The profile decoder: reads the messages of SPEC-03 §4 from a protobuf byte stream, rebuilds
//! the shared references of REQ-PROF-011 and records which fields each node carried.

use std::collections::HashMap;
use std::sync::Arc;

use super::reader::{ProtoReader, WireType};
use super::schema;
use super::{cipher, ProfileFormatError};
use crate::enums::{
    GlobalMeshAlgorithm, LayerAlignment, MeshAlgorithm, PatternColorFlags, PatternDensity,
    ProfileLayer, TextureFormat, TileFlip, TvRegionMode, VariantGeometry, VariantRenderStyle,
};
use crate::format::{ProfileDocument, ProfileHeader};
use crate::layout::PresenceRecorder;
use crate::pattern::{ProfilePattern, RecordedPatternSlot};
use crate::profile::GameProfile;
use crate::script::ProfileScript;
use crate::settings::{LayerSettings, LayerSettingsSet, LightSettings, ProfileSettings};
use crate::tile::{PatternGrid, ProfileTile};
use crate::values::{ArgbColor, Mat4F, Vec2I, Vec3F, Vec4F};
use crate::variant::{PatternVariant, TexturePixels, VariantTexture};

type Result<T> = std::result::Result<T, ProfileFormatError>;

/// An object registered under a reference key, ready to be named again by a later envelope.
enum Shared {
    Pattern(Arc<ProfilePattern>),
    Variant(Arc<PatternVariant>),
    Tile(Arc<ProfileTile>),
}

/// A message type that travels in reference envelopes.
trait Enveloped: Sized {
    /// Name used in the error message of a reference to the wrong kind of object.
    const NAME: &'static str;

    fn share(object: &Arc<Self>) -> Shared;
    fn shared(registered: &Shared) -> Option<Arc<Self>>;
}

impl Enveloped for ProfilePattern {
    const NAME: &'static str = "ProfilePattern";

    fn share(object: &Arc<Self>) -> Shared {
        Shared::Pattern(Arc::clone(object))
    }

    fn shared(registered: &Shared) -> Option<Arc<Self>> {
        match registered {
            Shared::Pattern(pattern) => Some(Arc::clone(pattern)),
            _ => None,
        }
    }
}

impl Enveloped for PatternVariant {
    const NAME: &'static str = "PatternVariant";

    fn share(object: &Arc<Self>) -> Shared {
        Shared::Variant(Arc::clone(object))
    }

    fn shared(registered: &Shared) -> Option<Arc<Self>> {
        match registered {
            Shared::Variant(variant) => Some(Arc::clone(variant)),
            _ => None,
        }
    }
}

impl Enveloped for ProfileTile {
    const NAME: &'static str = "ProfileTile";

    fn share(object: &Arc<Self>) -> Shared {
        Shared::Tile(Arc::clone(object))
    }

    fn shared(registered: &Shared) -> Option<Arc<Self>> {
        match registered {
            Shared::Tile(tile) => Some(Arc::clone(tile)),
            _ => None,
        }
    }
}

impl Shared {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Pattern(_) => ProfilePattern::NAME,
            Self::Variant(_) => PatternVariant::NAME,
            Self::Tile(_) => ProfileTile::NAME,
        }
    }
}

pub(super) struct Decoder<'a> {
    reader: ProtoReader<'a>,
    objects_by_key: HashMap<i32, Shared>,
    next_key: i32,
    unknown_field_count: usize,
}

/// Reads the header fields of the root message and skips the rest.
pub(super) fn read_header(bytes: &[u8]) -> Result<ProfileHeader> {
    let mut reader = ProtoReader::new(bytes);
    let mut header = ProfileHeader::default();
    while let Some((field, wire_type)) = reader.try_read_tag(reader.end())? {
        match field {
            schema::root::TYPE => header.signature = Some(reader.read_string()?),
            schema::root::ID => header.id = Some(reader.read_string()?),
            schema::root::TITLE => header.title = Some(reader.read_string()?),
            schema::root::MAKER => header.author = Some(reader.read_string()?),
            schema::root::GAME_DEV => header.publisher = Some(reader.read_string()?),
            schema::root::DES => header.description = Some(reader.read_string()?),
            _ => reader.skip(field, wire_type)?,
        }
    }
    Ok(header)
}

impl<'a> Decoder<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            reader: ProtoReader::new(data),
            objects_by_key: HashMap::new(),
            next_key: 1,
            unknown_field_count: 0,
        }
    }

    pub(super) fn read_document(mut self) -> Result<ProfileDocument> {
        let end = self.reader.end();
        let profile = self.read_root(end)?;
        Ok(ProfileDocument {
            profile,
            unknown_field_count: self.unknown_field_count,
        })
    }

    fn read_root(&mut self, limit: usize) -> Result<GameProfile> {
        let mut profile = GameProfile::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::root::TYPE => profile.signature = Some(self.reader.read_string()?),
                schema::root::VER => profile.version_text = Some(self.reader.read_string()?),
                schema::root::SCRIPTS => {
                    let end = self.reader.read_length_limit()?;
                    profile.scripts.push(self.read_script(end)?);
                }
                schema::root::PATTERNS => {
                    let end = self.reader.read_length_limit()?;
                    profile.permanent_patterns.push(self.read_envelope(end, Self::read_pattern)?);
                }
                schema::root::TEMP_PATTERNS => {
                    let end = self.reader.read_length_limit()?;
                    profile.temporary_patterns.push(self.read_envelope(end, Self::read_pattern)?);
                }
                schema::root::SETTINGS => {
                    let end = self.reader.read_length_limit()?;
                    profile.settings = Some(self.read_settings(end)?);
                }
                schema::root::ID => profile.id = Some(self.reader.read_string()?),
                schema::root::TITLE => profile.title = Some(self.reader.read_string()?),
                schema::root::MAKER => profile.author = Some(self.reader.read_string()?),
                schema::root::GAME_DEV => profile.publisher = Some(self.reader.read_string()?),
                schema::root::DES => profile.description = Some(self.reader.read_string()?),
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        profile.recorded_fields = present.finish(schema::root::MASK);
        Ok(profile)
    }

    fn read_script(&mut self, limit: usize) -> Result<ProfileScript> {
        let mut script = ProfileScript::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::script::NAME => script.name = Some(self.reader.read_string()?),
                schema::script::ENABLE => script.enabled = self.reader.read_bool()?,
                schema::script::ERROR => script.stored_error_flag = self.reader.read_bool()?,
                schema::script::SOURCE => script.source = Some(self.reader.read_string()?),
                schema::script::NAME_AGAIN => {
                    // Redundant copy of the name; field 1 is authoritative (REQ-PROF-038).
                    self.reader.read_string()?;
                }
                schema::script::TAG_STR => script.tag_string = Some(self.reader.read_string()?),
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        script.recorded_fields = present.finish(schema::script::MASK);
        Ok(script)
    }

    fn read_pattern(&mut self, limit: usize) -> Result<ProfilePattern> {
        let mut pattern = ProfilePattern::default();
        let mut cells: Option<Vec<Option<Arc<ProfileTile>>>> = None;
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::pattern::GRID if wire_type == WireType::StartGroup => {
                    let cell = self.read_cell_group(field)?;
                    cells.get_or_insert_with(Vec::new).push(cell);
                }
                schema::pattern::CONSTRAINT_SIZE => {
                    let end = self.reader.read_length_limit()?;
                    pattern.constraint_size = self.read_vec2i(end)?;
                }
                schema::pattern::TILE_SIZE => {
                    let end = self.reader.read_length_limit()?;
                    pattern.tile_size = Some(self.read_vec2i(end)?);
                }
                schema::pattern::PERMANENT => pattern.permanent = self.reader.read_bool()?,
                schema::pattern::PIXEL_COUNT => pattern.pixel_count = self.reader.read_i32()?,
                schema::pattern::COLOR_FLAGS => {
                    pattern.color_flags = PatternColorFlags::from_bits(self.reader.read_i32()?);
                }
                schema::pattern::TILE_DENSITY => {
                    pattern.tile_density = PatternDensity::from_raw(self.reader.read_i32()?);
                }
                schema::pattern::PIXEL_DENSITY => {
                    pattern.pixel_density = PatternDensity::from_raw(self.reader.read_i32()?);
                }
                schema::pattern::VARIANTS => {
                    let end = self.reader.read_length_limit()?;
                    pattern.variants.push(self.read_envelope(end, Self::read_variant)?);
                }
                schema::pattern::SELF_CONNECTED => pattern.self_connected = self.reader.read_bool()?,
                schema::pattern::SKIP_GEO => pattern.skip_geometry = self.reader.read_bool()?,
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        if let Some(cells) = cells {
            let size = pattern.tile_size.unwrap_or_default();
            pattern.grid = Some(PatternGrid::new(size.x, size.y, cells));
        }
        pattern.recorded_fields = present.finish(schema::pattern::MASK);
        Ok(pattern)
    }

    fn read_cell_group(&mut self, group_field: u32) -> Result<Option<Arc<ProfileTile>>> {
        let mut tile = None;
        while let Some((field, wire_type)) = self.reader.try_read_group_tag(group_field)? {
            if field == schema::cell::ENVELOPE && wire_type == WireType::Length {
                let end = self.reader.read_length_limit()?;
                tile = Some(self.read_envelope(end, Self::read_cell)?);
            } else {
                self.skip_unknown(field, wire_type)?;
            }
        }
        Ok(tile)
    }

    fn read_cell(&mut self, limit: usize) -> Result<ProfileTile> {
        let mut tile = None;
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            if field == schema::cell::PLANES && wire_type == WireType::Length {
                let end = self.reader.read_length_limit()?;
                tile = Some(self.read_tile_planes(end)?);
            } else {
                self.skip_unknown(field, wire_type)?;
            }
        }
        // A cell whose planes message is absent does not occur in any built-in profile; an
        // all-transparent tile is the faithful reading of "absent message, default value".
        Ok(tile.unwrap_or(ProfileTile::new(0, 0, TileFlip::NONE)))
    }

    fn read_tile_planes(&mut self, limit: usize) -> Result<ProfileTile> {
        let mut stored_low = 0;
        let mut stored_high = 0;
        let mut flip = 0;
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::tile_planes::LOW_PLANE => stored_low = self.reader.read_i64()?,
                schema::tile_planes::HIGH_PLANE => stored_high = self.reader.read_i64()?,
                schema::tile_planes::FLIP => flip = self.reader.read_i32()?,
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        let mut tile = ProfileTile::new(
            cipher::decode(stored_low),
            cipher::decode(stored_high),
            TileFlip::from_bits(flip),
        );
        tile.recorded_fields = present.finish(schema::tile_planes::MASK);
        Ok(tile)
    }

    fn read_variant(&mut self, limit: usize) -> Result<PatternVariant> {
        let mut variant = PatternVariant::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::variant::SIZE_Z => variant.depth = self.reader.read_i32()?,
                schema::variant::GEO => {
                    variant.geometry = VariantGeometry::from_raw(self.reader.read_i32()?);
                }
                schema::variant::ANTI_SHADOW => variant.anti_shadow = self.reader.read_bool()?,
                schema::variant::FLIP => variant.flip = self.reader.read_bool()?,
                schema::variant::SOLID => variant.solid = self.reader.read_bool()?,
                schema::variant::FULL_WARP => variant.full_warp = self.reader.read_bool()?,
                schema::variant::LAYER => {
                    variant.layer = ProfileLayer::from_raw(self.reader.read_i32()?);
                }
                schema::variant::SCALE => variant.scale = Some(self.read_vec3f_field()?),
                schema::variant::ROT => variant.rotation = Some(self.read_vec3f_field()?),
                schema::variant::OFFSET => variant.offset = Some(self.read_vec3f_field()?),
                schema::variant::PIVOT => variant.pivot = Some(self.read_vec3f_field()?),
                schema::variant::ALPHA => variant.alpha = self.reader.read_f32()?,
                schema::variant::MESH_ALGORITHM => {
                    variant.mesh_algorithm = MeshAlgorithm::from_raw(self.reader.read_i32()?);
                }
                schema::variant::DELTA_AMP => variant.deform_amplitude = Some(self.read_vec3f_field()?),
                schema::variant::DELTA_PIVOT => variant.deform_pivot = Some(self.read_vec3f_field()?),
                schema::variant::DELTA_SPEED => {
                    let end = self.reader.read_length_limit()?;
                    variant.deform_speed = Some(self.read_mat4f(end)?);
                }
                schema::variant::DELTA_OFFSET => {
                    let end = self.reader.read_length_limit()?;
                    variant.deform_offset = Some(self.read_mat4f(end)?);
                }
                schema::variant::RENDER_STYLE => {
                    variant.render_style = VariantRenderStyle::from_raw(self.reader.read_i32()?);
                }
                schema::variant::TEX => {
                    let end = self.reader.read_length_limit()?;
                    variant.texture = Some(self.read_texture(end)?);
                }
                schema::variant::TAG_STR => variant.tag_string = Some(self.reader.read_string()?),
                schema::variant::EMPTY_COLOR => {
                    // Stored as a signed byte (REQ-PROF-037).
                    let [low, ..] = self.reader.read_i32()?.to_le_bytes();
                    variant.stored_empty_color = i32::from(i8::from_le_bytes([low]));
                }
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        variant.recorded_fields = present.finish(schema::variant::MASK);
        Ok(variant)
    }

    fn read_texture(&mut self, limit: usize) -> Result<VariantTexture> {
        let mut texture = VariantTexture::new(0, 0, None);
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::tex_rect::WIDTH => texture.width = self.reader.read_i32()?,
                schema::tex_rect::HEIGHT => texture.height = self.reader.read_i32()?,
                schema::tex_rect::PAYLOAD => {
                    let end = self.reader.read_length_limit()?;
                    texture.pixels = Some(self.read_texture_pixels(end)?);
                }
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        texture.recorded_fields = present.finish(schema::tex_rect::MASK);
        Ok(texture)
    }

    fn read_texture_pixels(&mut self, limit: usize) -> Result<TexturePixels> {
        let mut pixels = TexturePixels::new(TextureFormat::Indexed, Vec::new(), Vec::new());
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::tex_payload::DATA => pixels.data.push(self.reader.read_i32()?),
                schema::tex_payload::FORMAT => {
                    pixels.format = TextureFormat::from_raw(self.reader.read_i32()?);
                }
                schema::tex_payload::EXTRA => {
                    // Stored as a signed 16-bit word.
                    let [b0, b1, ..] = self.reader.read_i32()?.to_le_bytes();
                    pixels.extra.push(i32::from(i16::from_le_bytes([b0, b1])));
                }
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        pixels.recorded_fields = present.finish(schema::tex_payload::MASK);
        Ok(pixels)
    }

    fn read_settings(&mut self, limit: usize) -> Result<ProfileSettings> {
        use schema::settings as s;
        let mut settings = ProfileSettings::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                s::LIGHT => {
                    let end = self.reader.read_length_limit()?;
                    settings.light = Some(self.read_light(end)?);
                }
                s::BG_COLOR => settings.use_nes_background_color = self.reader.read_bool()?,
                s::CUSTOM_BG_COLOR => {
                    let end = self.reader.read_length_limit()?;
                    settings.custom_background_color = Some(self.read_argb(end)?);
                }
                s::CLIPPING => {
                    let end = self.reader.read_length_limit()?;
                    settings.clipping = Some(self.read_vec4f(end)?);
                }
                s::MAP_ENABLE => settings.map_enable = self.reader.read_bool()?,
                s::LAYER => {
                    let end = self.reader.read_length_limit()?;
                    settings.layers = Some(self.read_layers(end)?);
                }
                s::GRAPHIC_QUALITY => settings.graphic_quality = self.reader.read_i32()?,
                s::VR_RUNTIME => settings.vr_runtime = self.reader.read_i32()?,
                s::VSYNC => settings.vsync = self.reader.read_i32()?,
                s::FRAME_RATE => settings.frame_rate = self.reader.read_i32()?,
                s::GAME_SPEED => settings.game_speed = self.reader.read_f32()?,
                s::VOLUME => settings.volume = self.reader.read_f32()?,
                s::RENDER_MODE => {
                    settings.mesh_algorithm = GlobalMeshAlgorithm::from_raw(self.reader.read_i32()?);
                }
                s::SAVED_PATTERN_3D => {
                    let end = self.reader.read_length_limit()?;
                    settings.saved_patterns.push(self.read_recorded_slot(end)?);
                }
                s::SAVE_ANIM_PATTERN_3D => {
                    let end = self.reader.read_length_limit()?;
                    settings.saved_anim_patterns.push(self.read_recorded_slot(end)?);
                }
                s::CONTROL_METHOD => settings.control_method = self.reader.read_i32()?,
                s::SHADOW => settings.shadow = self.reader.read_bool()?,
                s::SAME_BG_PALETTE => settings.same_background_palette = self.reader.read_bool()?,
                s::SPRITE_TOLERANCE => settings.sprite_tolerance = self.reader.read_i32()?,
                s::SCAN_DIRECTION => settings.scan_direction = self.reader.read_i32()?,
                s::SKYBOX => settings.skybox = self.reader.read_i32()?,
                s::CORE => settings.core = self.reader.read_i32()?,
                s::MIRRORING => settings.mirroring = self.reader.read_bool()?,
                s::EMPTY_TILE => settings.empty_tile = self.reader.read_bool()?,
                s::LOCAL_CAM => settings.local_camera = self.reader.read_bool()?,
                s::SCENE_RATIO => settings.scene_ratio = self.reader.read_f32()?,
                s::EMULATED_MENU_BUTTON => settings.emulated_menu_button = self.reader.read_bool()?,
                s::RENDER_METHOD => settings.render_method = self.reader.read_i32()?,
                s::RES_SCALE_INDEX => settings.resolution_scale_index = self.reader.read_i32()?,
                s::SKIP_FRAME => settings.skip_frame = self.reader.read_bool()?,
                s::BG_MUSIC => settings.background_music = self.reader.read_bool()?,
                s::ANTI_ALIAS => settings.anti_alias = self.reader.read_i32()?,
                s::AUTO_TV_MODE => settings.auto_tv_mode = self.reader.read_bool()?,
                s::TV_MODE => settings.tv_mode = TvRegionMode::from_raw(self.reader.read_i32()?),
                s::TV_BOX => settings.tv_box = self.reader.read_i32()?,
                s::SORT_OPTION => settings.sort_option = self.reader.read_i32()?,
                s::SAME_SP_PALETTE => settings.same_sprite_palette = self.reader.read_bool()?,
                s::FOV => settings.fov = self.reader.read_f32()?,
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        settings.recorded_fields = present.finish(schema::settings::MASK);
        Ok(settings)
    }

    fn read_light(&mut self, limit: usize) -> Result<LightSettings> {
        let mut light = LightSettings::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::light::DIRECTION => light.direction = Some(self.read_vec3f_field()?),
                schema::light::COLOR => {
                    let end = self.reader.read_length_limit()?;
                    light.color = Some(self.read_argb(end)?);
                }
                schema::light::INTENSITY => light.intensity = self.reader.read_f32()?,
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        light.recorded_fields = present.finish(schema::light::MASK);
        Ok(light)
    }

    fn read_layers(&mut self, limit: usize) -> Result<LayerSettingsSet> {
        let mut layers = LayerSettingsSet::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::layers::ITEMS => {
                    let end = self.reader.read_length_limit()?;
                    layers.layers.push(self.read_layer_params(end)?);
                }
                schema::layers::DEF_LAYER => {
                    layers.default_layer = ProfileLayer::from_raw(self.reader.read_i32()?);
                }
                schema::layers::AUTO => layers.auto = self.reader.read_bool()?,
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        layers.recorded_fields = present.finish(schema::layers::MASK);
        Ok(layers)
    }

    fn read_layer_params(&mut self, limit: usize) -> Result<LayerSettings> {
        let mut layer = LayerSettings::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::layer_params::OFFSET => layer.offset = self.reader.read_f32()?,
                schema::layer_params::ALIGNED => {
                    layer.aligned = LayerAlignment::from_raw(self.reader.read_i32()?);
                }
                schema::layer_params::CAST_SHADOW => layer.stored_cast_shadow = self.reader.read_bool()?,
                schema::layer_params::RECV_SHADOW => layer.stored_recv_shadow = self.reader.read_bool()?,
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        layer.recorded_fields = present.finish(schema::layer_params::MASK);
        Ok(layer)
    }

    fn read_recorded_slot(&mut self, limit: usize) -> Result<RecordedPatternSlot> {
        let mut slot = RecordedPatternSlot::default();
        let mut present = PresenceRecorder::default();
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            present.saw(field);
            match field {
                schema::recorded_slot::NAME => slot.name = Some(self.reader.read_string()?),
                schema::recorded_slot::PATTERN => {
                    // A direct message here: memorised slots carry no reference envelope.
                    let end = self.reader.read_length_limit()?;
                    slot.pattern = Some(self.read_pattern(end)?);
                }
                _ => {
                    self.skip_unknown(field, wire_type)?;
                    present.unknown(field);
                }
            }
        }
        slot.recorded_fields = present.finish(schema::recorded_slot::MASK);
        Ok(slot)
    }

    fn read_vec2i(&mut self, limit: usize) -> Result<Vec2I> {
        let (mut x, mut y) = (0, 0);
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            match field {
                schema::values::X => x = self.reader.read_i32()?,
                schema::values::Y => y = self.reader.read_i32()?,
                _ => self.skip_unknown(field, wire_type)?,
            }
        }
        Ok(Vec2I::new(x, y))
    }

    /// Reads the length prefix of a three-float message, then the message.
    fn read_vec3f_field(&mut self) -> Result<Vec3F> {
        let limit = self.reader.read_length_limit()?;
        let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            match field {
                schema::values::X => x = self.reader.read_f32()?,
                schema::values::Y => y = self.reader.read_f32()?,
                schema::values::Z => z = self.reader.read_f32()?,
                _ => self.skip_unknown(field, wire_type)?,
            }
        }
        Ok(Vec3F::new(x, y, z))
    }

    fn read_vec4f(&mut self, limit: usize) -> Result<Vec4F> {
        let (mut x, mut y, mut z, mut w) = (0.0, 0.0, 0.0, 0.0);
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            match field {
                schema::values::X => x = self.reader.read_f32()?,
                schema::values::Y => y = self.reader.read_f32()?,
                schema::values::Z => z = self.reader.read_f32()?,
                schema::values::W => w = self.reader.read_f32()?,
                _ => self.skip_unknown(field, wire_type)?,
            }
        }
        Ok(Vec4F::new(x, y, z, w))
    }

    fn read_mat4f(&mut self, limit: usize) -> Result<Mat4F> {
        let mut cells = [0.0_f32; Mat4F::CELL_COUNT];
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            let slot = field
                .checked_sub(schema::values::MAT4_FIRST_CELL)
                .and_then(|index| cells.get_mut(usize::try_from(index).ok()?));
            match slot {
                Some(cell) if wire_type == WireType::Fixed32 => *cell = self.reader.read_f32()?,
                _ => self.skip_unknown(field, wire_type)?,
            }
        }
        Ok(Mat4F::new(cells))
    }

    fn read_argb(&mut self, limit: usize) -> Result<ArgbColor> {
        let mut packed = 0;
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            if field == schema::values::ARGB_VALUE {
                packed = self.reader.read_i32()?;
            } else {
                self.skip_unknown(field, wire_type)?;
            }
        }
        Ok(ArgbColor::new(packed))
    }

    /// Reads one reference envelope: a new object under the next key, or a reference to an
    /// earlier one (REQ-PROF-011).
    fn read_envelope<T: Enveloped>(
        &mut self,
        limit: usize,
        read_message: fn(&mut Self, usize) -> Result<T>,
    ) -> Result<Arc<T>> {
        let mut message: Option<T> = None;
        let mut new_key: Option<i32> = None;
        let mut existing_key: Option<i32> = None;
        while let Some((field, wire_type)) = self.reader.try_read_tag(limit)? {
            match field {
                schema::envelope::EXISTING_KEY => existing_key = Some(self.reader.read_i32()?),
                schema::envelope::NEW_KEY => {
                    let key = self.reader.read_i32()?;
                    if key != self.next_key {
                        return Err(ProfileFormatError::new(format!(
                            "Reference key {key} is out of sequence; {} was expected.",
                            self.next_key
                        )));
                    }
                    new_key = Some(key);
                    // The key belongs to this object before the objects it contains, so the
                    // counter moves on before the payload is read (prefix order).
                    self.next_key += 1;
                }
                schema::envelope::MESSAGE => {
                    if new_key.is_none() {
                        return Err(ProfileFormatError::new(
                            "A reference envelope carries a message without a new key.",
                        ));
                    }
                    let end = self.reader.read_length_limit()?;
                    message = Some(read_message(self, end)?);
                }
                // Fields 3, 4 and 8 of the convention (dynamic type) never appear in a
                // profile; REQ-PROF-010 lets a reader skip them.
                _ => self.skip_unknown(field, wire_type)?,
            }
        }

        if let Some(key) = existing_key {
            if message.is_some() || new_key.is_some() {
                return Err(ProfileFormatError::new(format!(
                    "Reference envelope {key} both names an object and carries one."
                )));
            }
            let target = self.objects_by_key.get(&key).ok_or_else(|| {
                ProfileFormatError::new(format!("Reference to the unknown key {key}."))
            })?;
            return T::shared(target).ok_or_else(|| {
                ProfileFormatError::new(format!(
                    "Reference key {key} names a {}, not a {}.",
                    target.type_name(),
                    T::NAME
                ))
            });
        }

        let (Some(message), Some(key)) = (message, new_key) else {
            return Err(ProfileFormatError::new(
                "A reference envelope carries neither a key nor an object.",
            ));
        };
        let object = Arc::new(message);
        self.objects_by_key.insert(key, T::share(&object));
        Ok(object)
    }

    fn skip_unknown(&mut self, field: u32, wire_type: WireType) -> Result<()> {
        self.unknown_field_count += 1;
        self.reader.skip(field, wire_type)
    }
}
