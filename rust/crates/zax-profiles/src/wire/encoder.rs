//! The canonical writer of the profile messages (SPEC-03 REQ-PROF-010 to REQ-PROF-013 and
//! REQ-PROF-051): one pass, shared objects named by reference after their first occurrence,
//! group-encoded cells and a field written out of order.

use std::collections::HashMap;
use std::sync::Arc;

use super::schema;
use super::writer::ProtoWriter;
use super::cipher;
use crate::layout::{is_present, FieldMask, FieldPresence};
use crate::pattern::{ProfilePattern, RecordedPatternSlot};
use crate::profile::GameProfile;
use crate::script::ProfileScript;
use crate::settings::{LayerSettings, LayerSettingsSet, LightSettings, ProfileSettings};
use crate::tile::ProfileTile;
use crate::values::{ArgbColor, Mat4F, Vec2I, Vec3F, Vec4F};
use crate::variant::{PatternVariant, TexturePixels, VariantTexture};

pub(super) struct Encoder {
    writer: ProtoWriter,
    presence: FieldPresence,
    /// Reference keys by object address: the objects of a profile that are shared.
    keys: HashMap<usize, i32>,
    next_key: i32,
}

/// Capacity hint of the output: 4 KiB plus half a kilobyte per pattern, at most 32 MiB.
fn estimate_capacity(profile: &GameProfile) -> usize {
    let patterns = profile.permanent_patterns.len() + profile.temporary_patterns.len();
    (4096 + patterns.saturating_mul(512)).clamp(4096, 32 * 1024 * 1024)
}

impl Encoder {
    pub(super) fn new(profile: &GameProfile, presence: FieldPresence) -> Self {
        Self {
            writer: ProtoWriter::with_capacity(estimate_capacity(profile)),
            presence,
            keys: HashMap::new(),
            next_key: 1,
        }
    }

    pub(super) fn write(mut self, profile: &GameProfile) -> Vec<u8> {
        self.write_root(profile);
        self.writer.into_bytes()
    }

    /// Q-010: without recorded fields ("a profile built in memory") every scalar is written.
    fn allowed(&self, recorded: Option<FieldMask>, field: u32) -> bool {
        is_present(self.presence, recorded, field)
    }

    fn write_root(&mut self, profile: &GameProfile) {
        let recorded = profile.recorded_fields;
        self.optional_string(recorded, schema::root::TYPE, profile.signature.as_deref());
        self.optional_string(recorded, schema::root::VER, profile.version_text.as_deref());

        for script in &profile.scripts {
            let scope = self.writer.begin_nested(schema::root::SCRIPTS);
            self.write_script(script);
            self.writer.end_nested(scope);
        }
        for pattern in &profile.permanent_patterns {
            self.envelope(schema::root::PATTERNS, pattern, Self::write_pattern);
        }
        for pattern in &profile.temporary_patterns {
            self.envelope(schema::root::TEMP_PATTERNS, pattern, Self::write_pattern);
        }
        if let Some(settings) = &profile.settings {
            if self.allowed(recorded, schema::root::SETTINGS) {
                let scope = self.writer.begin_nested(schema::root::SETTINGS);
                self.write_settings(settings);
                self.writer.end_nested(scope);
            }
        }

        self.optional_string(recorded, schema::root::ID, profile.id.as_deref());
        self.optional_string(recorded, schema::root::TITLE, profile.title.as_deref());
        self.optional_string(recorded, schema::root::MAKER, profile.author.as_deref());
        self.optional_string(recorded, schema::root::GAME_DEV, profile.publisher.as_deref());
        self.optional_string(recorded, schema::root::DES, profile.description.as_deref());
    }

    fn write_script(&mut self, script: &ProfileScript) {
        let recorded = script.recorded_fields;
        self.optional_string(recorded, schema::script::NAME, script.name.as_deref());
        self.bool_field(recorded, schema::script::ENABLE, script.enabled);
        self.bool_field(recorded, schema::script::ERROR, script.stored_error_flag);
        self.optional_string(recorded, schema::script::SOURCE, script.source.as_deref());
        // Field 5 repeats the name (REQ-PROF-038); both fields are written from the model.
        self.optional_string(recorded, schema::script::NAME_AGAIN, script.name.as_deref());
        self.optional_string(recorded, schema::script::TAG_STR, script.tag_string.as_deref());
    }

    fn write_pattern(&mut self, pattern: &ProfilePattern) {
        let recorded = pattern.recorded_fields;
        if let Some(grid) = &pattern.grid {
            if !grid.cells().is_empty() && self.allowed(recorded, schema::pattern::GRID) {
                for cell in grid.cells() {
                    self.writer.write_group_start(schema::pattern::GRID);
                    if let Some(tile) = cell {
                        self.envelope(schema::cell::ENVELOPE, tile, Self::write_cell);
                    }
                    self.writer.write_group_end(schema::pattern::GRID);
                }
            }
        }

        if self.allowed(recorded, schema::pattern::CONSTRAINT_SIZE) {
            self.write_vec2i(schema::pattern::CONSTRAINT_SIZE, pattern.constraint_size);
        }
        if let Some(tile_size) = pattern.tile_size {
            if self.allowed(recorded, schema::pattern::TILE_SIZE) {
                self.write_vec2i(schema::pattern::TILE_SIZE, tile_size);
            }
        }
        self.bool_field(recorded, schema::pattern::PERMANENT, pattern.permanent);
        self.i32_field(recorded, schema::pattern::PIXEL_COUNT, pattern.pixel_count);
        self.i32_field(recorded, schema::pattern::COLOR_FLAGS, pattern.color_flags.bits());
        self.i32_field(recorded, schema::pattern::TILE_DENSITY, pattern.tile_density.raw());
        self.i32_field(recorded, schema::pattern::PIXEL_DENSITY, pattern.pixel_density.raw());
        for variant in &pattern.variants {
            self.envelope(schema::pattern::VARIANTS, variant, Self::write_variant);
        }
        self.bool_field(recorded, schema::pattern::SELF_CONNECTED, pattern.self_connected);
        self.bool_field(recorded, schema::pattern::SKIP_GEO, pattern.skip_geometry);
    }

    fn write_cell(&mut self, tile: &ProfileTile) {
        let recorded = tile.recorded_fields;
        let scope = self.writer.begin_nested(schema::cell::PLANES);
        if self.allowed(recorded, schema::tile_planes::LOW_PLANE) {
            self.writer
                .write_i64(schema::tile_planes::LOW_PLANE, cipher::encode(tile.low_plane));
        }
        if self.allowed(recorded, schema::tile_planes::HIGH_PLANE) {
            self.writer
                .write_i64(schema::tile_planes::HIGH_PLANE, cipher::encode(tile.high_plane));
        }
        self.i32_field(recorded, schema::tile_planes::FLIP, tile.flip.bits());
        self.writer.end_nested(scope);
    }

    fn write_variant(&mut self, variant: &PatternVariant) {
        use schema::variant as v;
        let recorded = variant.recorded_fields;
        self.i32_field(recorded, v::SIZE_Z, variant.depth);
        self.i32_field(recorded, v::GEO, variant.geometry.raw());
        self.bool_field(recorded, v::ANTI_SHADOW, variant.anti_shadow);
        self.bool_field(recorded, v::FLIP, variant.flip);
        self.bool_field(recorded, v::SOLID, variant.solid);
        self.bool_field(recorded, v::FULL_WARP, variant.full_warp);
        self.i32_field(recorded, v::LAYER, variant.layer.raw());
        self.optional_vec3(recorded, v::SCALE, variant.scale);
        self.optional_vec3(recorded, v::ROT, variant.rotation);
        self.optional_vec3(recorded, v::OFFSET, variant.offset);
        self.optional_vec3(recorded, v::PIVOT, variant.pivot);
        self.f32_field(recorded, v::ALPHA, variant.alpha);
        self.i32_field(recorded, v::MESH_ALGORITHM, variant.mesh_algorithm.raw());
        self.optional_vec3(recorded, v::DELTA_AMP, variant.deform_amplitude);
        self.optional_vec3(recorded, v::DELTA_PIVOT, variant.deform_pivot);
        self.optional_mat4(recorded, v::DELTA_SPEED, variant.deform_speed);
        self.optional_mat4(recorded, v::DELTA_OFFSET, variant.deform_offset);
        self.i32_field(recorded, v::RENDER_STYLE, variant.render_style.raw());
        if let Some(texture) = &variant.texture {
            if self.allowed(recorded, v::TEX) {
                let scope = self.writer.begin_nested(v::TEX);
                self.write_texture(texture);
                self.writer.end_nested(scope);
            }
        }
        self.optional_string(recorded, v::TAG_STR, variant.tag_string.as_deref());
        self.i32_field(recorded, v::EMPTY_COLOR, variant.stored_empty_color);
    }

    fn write_texture(&mut self, texture: &VariantTexture) {
        let recorded = texture.recorded_fields;
        // The only field written out of order in the whole format (REQ-PROF-013).
        if let Some(pixels) = &texture.pixels {
            if self.allowed(recorded, schema::tex_rect::PAYLOAD) {
                let scope = self.writer.begin_nested(schema::tex_rect::PAYLOAD);
                self.write_texture_pixels(pixels);
                self.writer.end_nested(scope);
            }
        }
        self.i32_field(recorded, schema::tex_rect::WIDTH, texture.width);
        self.i32_field(recorded, schema::tex_rect::HEIGHT, texture.height);
    }

    fn write_texture_pixels(&mut self, pixels: &TexturePixels) {
        let recorded = pixels.recorded_fields;
        if self.allowed(recorded, schema::tex_payload::DATA) {
            for word in &pixels.data {
                self.writer.write_i32(schema::tex_payload::DATA, *word);
            }
        }
        self.i32_field(recorded, schema::tex_payload::FORMAT, pixels.format.raw());

        // Field 3 is left out when every value is zero. For a payload that came from a file the
        // recorded field set decides instead, so that a stored all-zero plane is written back.
        let write_extra = !pixels.extra.is_empty()
            && self.allowed(recorded, schema::tex_payload::EXTRA)
            && (self.presence == FieldPresence::AsRecorded || !pixels.extra_is_empty());
        if write_extra {
            for word in &pixels.extra {
                self.writer.write_i32(schema::tex_payload::EXTRA, *word);
            }
        }
    }

    fn write_settings(&mut self, settings: &ProfileSettings) {
        use schema::settings as s;
        let r = settings.recorded_fields;
        if let Some(light) = &settings.light {
            if self.allowed(r, s::LIGHT) {
                let scope = self.writer.begin_nested(s::LIGHT);
                self.write_light(light);
                self.writer.end_nested(scope);
            }
        }
        self.bool_field(r, s::BG_COLOR, settings.use_nes_background_color);
        if let Some(color) = settings.custom_background_color {
            if self.allowed(r, s::CUSTOM_BG_COLOR) {
                self.write_argb(s::CUSTOM_BG_COLOR, color);
            }
        }
        if let Some(clipping) = settings.clipping {
            if self.allowed(r, s::CLIPPING) {
                self.write_vec4(s::CLIPPING, clipping);
            }
        }
        self.bool_field(r, s::MAP_ENABLE, settings.map_enable);
        if let Some(layers) = &settings.layers {
            if self.allowed(r, s::LAYER) {
                let scope = self.writer.begin_nested(s::LAYER);
                self.write_layers(layers);
                self.writer.end_nested(scope);
            }
        }
        self.i32_field(r, s::GRAPHIC_QUALITY, settings.graphic_quality);
        self.i32_field(r, s::VR_RUNTIME, settings.vr_runtime);
        self.i32_field(r, s::VSYNC, settings.vsync);
        self.i32_field(r, s::FRAME_RATE, settings.frame_rate);
        self.f32_field(r, s::GAME_SPEED, settings.game_speed);
        self.f32_field(r, s::VOLUME, settings.volume);
        self.i32_field(r, s::RENDER_MODE, settings.mesh_algorithm.raw());
        self.slots(r, s::SAVED_PATTERN_3D, &settings.saved_patterns);
        self.slots(r, s::SAVE_ANIM_PATTERN_3D, &settings.saved_anim_patterns);
        self.i32_field(r, s::CONTROL_METHOD, settings.control_method);
        self.bool_field(r, s::SHADOW, settings.shadow);
        self.bool_field(r, s::SAME_BG_PALETTE, settings.same_background_palette);
        self.i32_field(r, s::SPRITE_TOLERANCE, settings.sprite_tolerance);
        self.i32_field(r, s::SCAN_DIRECTION, settings.scan_direction);
        self.i32_field(r, s::SKYBOX, settings.skybox);
        self.i32_field(r, s::CORE, settings.core);
        self.bool_field(r, s::MIRRORING, settings.mirroring);
        self.bool_field(r, s::EMPTY_TILE, settings.empty_tile);
        self.bool_field(r, s::LOCAL_CAM, settings.local_camera);
        self.f32_field(r, s::SCENE_RATIO, settings.scene_ratio);
        self.bool_field(r, s::EMULATED_MENU_BUTTON, settings.emulated_menu_button);
        self.i32_field(r, s::RENDER_METHOD, settings.render_method);
        self.i32_field(r, s::RES_SCALE_INDEX, settings.resolution_scale_index);
        self.bool_field(r, s::SKIP_FRAME, settings.skip_frame);
        self.bool_field(r, s::BG_MUSIC, settings.background_music);
        self.i32_field(r, s::ANTI_ALIAS, settings.anti_alias);
        self.bool_field(r, s::AUTO_TV_MODE, settings.auto_tv_mode);
        self.i32_field(r, s::TV_MODE, settings.tv_mode.raw());
        self.i32_field(r, s::TV_BOX, settings.tv_box);
        self.i32_field(r, s::SORT_OPTION, settings.sort_option);
        self.bool_field(r, s::SAME_SP_PALETTE, settings.same_sprite_palette);
        self.f32_field(r, s::FOV, settings.fov);
    }

    fn slots(&mut self, recorded: Option<FieldMask>, field: u32, slots: &[RecordedPatternSlot]) {
        if !self.allowed(recorded, field) {
            return;
        }
        for slot in slots {
            let scope = self.writer.begin_nested(field);
            self.write_recorded_slot(slot);
            self.writer.end_nested(scope);
        }
    }

    fn write_light(&mut self, light: &LightSettings) {
        let recorded = light.recorded_fields;
        self.optional_vec3(recorded, schema::light::DIRECTION, light.direction);
        if let Some(color) = light.color {
            if self.allowed(recorded, schema::light::COLOR) {
                self.write_argb(schema::light::COLOR, color);
            }
        }
        self.f32_field(recorded, schema::light::INTENSITY, light.intensity);
    }

    fn write_layers(&mut self, layers: &LayerSettingsSet) {
        let recorded = layers.recorded_fields;
        if self.allowed(recorded, schema::layers::ITEMS) {
            for layer in &layers.layers {
                let scope = self.writer.begin_nested(schema::layers::ITEMS);
                self.write_layer_params(layer);
                self.writer.end_nested(scope);
            }
        }
        self.i32_field(recorded, schema::layers::DEF_LAYER, layers.default_layer.raw());
        self.bool_field(recorded, schema::layers::AUTO, layers.auto);
    }

    fn write_layer_params(&mut self, layer: &LayerSettings) {
        let recorded = layer.recorded_fields;
        self.f32_field(recorded, schema::layer_params::OFFSET, layer.offset);
        self.i32_field(recorded, schema::layer_params::ALIGNED, layer.aligned.raw());
        self.bool_field(recorded, schema::layer_params::CAST_SHADOW, layer.stored_cast_shadow);
        self.bool_field(recorded, schema::layer_params::RECV_SHADOW, layer.stored_recv_shadow);
    }

    fn write_recorded_slot(&mut self, slot: &RecordedPatternSlot) {
        let recorded = slot.recorded_fields;
        self.optional_string(recorded, schema::recorded_slot::NAME, slot.name.as_deref());
        if let Some(pattern) = &slot.pattern {
            if self.allowed(recorded, schema::recorded_slot::PATTERN) {
                let scope = self.writer.begin_nested(schema::recorded_slot::PATTERN);
                self.write_pattern(pattern);
                self.writer.end_nested(scope);
            }
        }
    }

    /// Writes `value` as an envelope: its first occurrence as a new key with the message, any
    /// later one as a reference to that key (REQ-PROF-011).
    fn envelope<T>(
        &mut self,
        field: u32,
        value: &Arc<T>,
        write_message: fn(&mut Self, &T),
    ) {
        let envelope = self.writer.begin_nested(field);
        let address = Arc::as_ptr(value) as usize;
        if let Some(existing_key) = self.keys.get(&address) {
            self.writer
                .write_i32(schema::envelope::EXISTING_KEY, *existing_key);
        } else {
            // A new object takes its key before the objects it contains (REQ-PROF-011).
            let key = self.next_key;
            self.next_key += 1;
            self.keys.insert(address, key);
            self.writer.write_i32(schema::envelope::NEW_KEY, key);
            let message = self.writer.begin_nested(schema::envelope::MESSAGE);
            write_message(self, value);
            self.writer.end_nested(message);
        }
        self.writer.end_nested(envelope);
    }

    fn optional_string(&mut self, recorded: Option<FieldMask>, field: u32, value: Option<&str>) {
        if let Some(value) = value {
            if self.allowed(recorded, field) {
                self.writer.write_string(field, value);
            }
        }
    }

    fn optional_vec3(&mut self, recorded: Option<FieldMask>, field: u32, value: Option<Vec3F>) {
        if let Some(vector) = value {
            if self.allowed(recorded, field) {
                let scope = self.writer.begin_nested(field);
                self.writer.write_f32(schema::values::X, vector.x);
                self.writer.write_f32(schema::values::Y, vector.y);
                self.writer.write_f32(schema::values::Z, vector.z);
                self.writer.end_nested(scope);
            }
        }
    }

    fn optional_mat4(&mut self, recorded: Option<FieldMask>, field: u32, value: Option<Mat4F>) {
        if let Some(matrix) = value {
            if self.allowed(recorded, field) {
                let scope = self.writer.begin_nested(field);
                for (cell, value) in (schema::values::MAT4_FIRST_CELL..).zip(matrix.cells()) {
                    self.writer.write_f32(cell, *value);
                }
                self.writer.end_nested(scope);
            }
        }
    }

    fn write_vec2i(&mut self, field: u32, value: Vec2I) {
        let scope = self.writer.begin_nested(field);
        self.writer.write_i32(schema::values::X, value.x);
        self.writer.write_i32(schema::values::Y, value.y);
        self.writer.end_nested(scope);
    }

    fn write_vec4(&mut self, field: u32, value: Vec4F) {
        let scope = self.writer.begin_nested(field);
        self.writer.write_f32(schema::values::X, value.x);
        self.writer.write_f32(schema::values::Y, value.y);
        self.writer.write_f32(schema::values::Z, value.z);
        self.writer.write_f32(schema::values::W, value.w);
        self.writer.end_nested(scope);
    }

    fn write_argb(&mut self, field: u32, value: ArgbColor) {
        let scope = self.writer.begin_nested(field);
        self.writer.write_i32(schema::values::ARGB_VALUE, value.packed);
        self.writer.end_nested(scope);
    }

    fn i32_field(&mut self, recorded: Option<FieldMask>, field: u32, value: i32) {
        if self.allowed(recorded, field) {
            self.writer.write_i32(field, value);
        }
    }

    fn bool_field(&mut self, recorded: Option<FieldMask>, field: u32, value: bool) {
        if self.allowed(recorded, field) {
            self.writer.write_bool(field, value);
        }
    }

    fn f32_field(&mut self, recorded: Option<FieldMask>, field: u32, value: f32) {
        if self.allowed(recorded, field) {
            self.writer.write_f32(field, value);
        }
    }
}
