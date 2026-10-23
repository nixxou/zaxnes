//! Deep comparison of two decoded profiles, including which objects are shared: an object
//! shared in one model must be shared the same way in the other (REQ-PROF-011).

use std::collections::HashMap;
use std::sync::Arc;

use zax_profiles::pattern::{ProfilePattern, RecordedPatternSlot};
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_profiles::settings::ProfileSettings;
use zax_profiles::variant::PatternVariant;

type Pairs = HashMap<usize, usize>;

pub fn assert_equal(expected: &GameProfile, actual: &GameProfile, source: &str) {
    let mut pairs = Pairs::new();

    assert_eq!(expected.signature, actual.signature);
    assert_eq!(expected.version_text, actual.version_text);
    assert_eq!(expected.id, actual.id);
    assert_eq!(expected.title, actual.title);
    assert_eq!(expected.author, actual.author);
    assert_eq!(expected.publisher, actual.publisher);
    assert_eq!(expected.description, actual.description);

    assert_eq!(expected.scripts.len(), actual.scripts.len());
    for (expected, actual) in expected.scripts.iter().zip(&actual.scripts) {
        assert_script(expected, actual);
    }

    assert_pattern_list(&expected.permanent_patterns, &actual.permanent_patterns, &mut pairs, &format!("{source} patterns"));
    assert_pattern_list(&expected.temporary_patterns, &actual.temporary_patterns, &mut pairs, &format!("{source} temp patterns"));

    assert_eq!(expected.settings.is_none(), actual.settings.is_none());
    if let (Some(expected), Some(actual)) = (&expected.settings, &actual.settings) {
        assert_settings(expected, actual, &mut pairs, source);
    }
}

fn assert_script(expected: &ProfileScript, actual: &ProfileScript) {
    assert_eq!(expected.name, actual.name);
    assert_eq!(expected.enabled, actual.enabled);
    assert_eq!(expected.stored_error_flag, actual.stored_error_flag);
    assert_eq!(expected.source, actual.source);
    assert_eq!(expected.tag_string, actual.tag_string);
}

fn assert_pattern_list(
    expected: &[Arc<ProfilePattern>],
    actual: &[Arc<ProfilePattern>],
    pairs: &mut Pairs,
    location: &str,
) {
    assert_eq!(expected.len(), actual.len());
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        if assert_same_sharing(Arc::as_ptr(expected) as usize, Arc::as_ptr(actual) as usize, pairs, &format!("{location}[{index}]")) {
            assert_pattern(expected, actual, pairs, &format!("{location}[{index}]"));
        }
    }
}

fn assert_pattern(expected: &ProfilePattern, actual: &ProfilePattern, pairs: &mut Pairs, location: &str) {
    assert_eq!(expected.constraint_size, actual.constraint_size);
    assert_eq!(expected.tile_size, actual.tile_size);
    assert_eq!(expected.permanent, actual.permanent);
    assert_eq!(expected.pixel_count, actual.pixel_count);
    assert_eq!(expected.color_flags, actual.color_flags);
    assert_eq!(expected.tile_density, actual.tile_density);
    assert_eq!(expected.pixel_density, actual.pixel_density);
    assert_eq!(expected.self_connected, actual.self_connected);
    assert_eq!(expected.skip_geometry, actual.skip_geometry);

    assert_eq!(expected.grid.is_none(), actual.grid.is_none());
    if let (Some(expected_grid), Some(actual_grid)) = (&expected.grid, &actual.grid) {
        assert_eq!(expected_grid.width(), actual_grid.width());
        assert_eq!(expected_grid.height(), actual_grid.height());
        assert_eq!(expected_grid.cells().len(), actual_grid.cells().len());
        for (index, (expected_cell, actual_cell)) in expected_grid.cells().iter().zip(actual_grid.cells()).enumerate() {
            assert_eq!(expected_cell.is_none(), actual_cell.is_none());
            if let (Some(expected_cell), Some(actual_cell)) = (expected_cell, actual_cell) {
                let here = format!("{location} cell {index}");
                if assert_same_sharing(Arc::as_ptr(expected_cell) as usize, Arc::as_ptr(actual_cell) as usize, pairs, &here) {
                    assert_eq!(expected_cell, actual_cell);
                }
            }
        }
    }

    assert_eq!(expected.variants.len(), actual.variants.len());
    for (index, (expected, actual)) in expected.variants.iter().zip(&actual.variants).enumerate() {
        let here = format!("{location} variant {index}");
        if assert_same_sharing(Arc::as_ptr(expected) as usize, Arc::as_ptr(actual) as usize, pairs, &here) {
            assert_variant(expected, actual);
        }
    }
}

fn assert_variant(expected: &PatternVariant, actual: &PatternVariant) {
    assert_eq!(expected.depth, actual.depth);
    assert_eq!(expected.geometry, actual.geometry);
    assert_eq!(expected.anti_shadow, actual.anti_shadow);
    assert_eq!(expected.flip, actual.flip);
    assert_eq!(expected.solid, actual.solid);
    assert_eq!(expected.full_warp, actual.full_warp);
    assert_eq!(expected.layer, actual.layer);
    assert_eq!(expected.scale, actual.scale);
    assert_eq!(expected.rotation, actual.rotation);
    assert_eq!(expected.offset, actual.offset);
    assert_eq!(expected.pivot, actual.pivot);
    assert_eq!(expected.alpha.to_bits(), actual.alpha.to_bits());
    assert_eq!(expected.mesh_algorithm, actual.mesh_algorithm);
    assert_eq!(expected.deform_amplitude, actual.deform_amplitude);
    assert_eq!(expected.deform_pivot, actual.deform_pivot);
    assert_eq!(expected.deform_speed, actual.deform_speed);
    assert_eq!(expected.deform_offset, actual.deform_offset);
    assert_eq!(expected.render_style, actual.render_style);
    assert_eq!(expected.tag_string, actual.tag_string);
    assert_eq!(expected.stored_empty_color, actual.stored_empty_color);

    assert_eq!(expected.texture.is_none(), actual.texture.is_none());
    if let (Some(expected), Some(actual)) = (&expected.texture, &actual.texture) {
        assert_eq!(expected.width, actual.width);
        assert_eq!(expected.height, actual.height);
        assert_eq!(expected.pixels.is_none(), actual.pixels.is_none());
        if let (Some(expected), Some(actual)) = (&expected.pixels, &actual.pixels) {
            assert_eq!(expected.format, actual.format);
            assert_eq!(expected.data, actual.data);
            assert_eq!(expected.extra, actual.extra);
        }
    }
}

fn assert_settings(expected: &ProfileSettings, actual: &ProfileSettings, pairs: &mut Pairs, location: &str) {
    assert_eq!(expected.light.is_none(), actual.light.is_none());
    if let (Some(expected), Some(actual)) = (&expected.light, &actual.light) {
        assert_eq!(expected.direction, actual.direction);
        assert_eq!(expected.color, actual.color);
        assert_eq!(expected.intensity.to_bits(), actual.intensity.to_bits());
    }

    assert_eq!(expected.use_nes_background_color, actual.use_nes_background_color);
    assert_eq!(expected.custom_background_color, actual.custom_background_color);
    assert_eq!(expected.clipping, actual.clipping);
    assert_eq!(expected.map_enable, actual.map_enable);

    assert_eq!(expected.layers.is_none(), actual.layers.is_none());
    if let (Some(expected), Some(actual)) = (&expected.layers, &actual.layers) {
        assert_eq!(expected.default_layer, actual.default_layer);
        assert_eq!(expected.auto, actual.auto);
        assert_eq!(expected.layers.len(), actual.layers.len());
        for (expected, actual) in expected.layers.iter().zip(&actual.layers) {
            assert_eq!(expected.offset.to_bits(), actual.offset.to_bits());
            assert_eq!(expected.aligned, actual.aligned);
            assert_eq!(expected.stored_cast_shadow, actual.stored_cast_shadow);
            assert_eq!(expected.stored_recv_shadow, actual.stored_recv_shadow);
        }
    }

    assert_eq!(expected.graphic_quality, actual.graphic_quality);
    assert_eq!(expected.vr_runtime, actual.vr_runtime);
    assert_eq!(expected.vsync, actual.vsync);
    assert_eq!(expected.frame_rate, actual.frame_rate);
    assert_eq!(expected.game_speed.to_bits(), actual.game_speed.to_bits());
    assert_eq!(expected.volume.to_bits(), actual.volume.to_bits());
    assert_eq!(expected.mesh_algorithm, actual.mesh_algorithm);
    assert_eq!(expected.control_method, actual.control_method);
    assert_eq!(expected.shadow, actual.shadow);
    assert_eq!(expected.same_background_palette, actual.same_background_palette);
    assert_eq!(expected.sprite_tolerance, actual.sprite_tolerance);
    assert_eq!(expected.scan_direction, actual.scan_direction);
    assert_eq!(expected.skybox, actual.skybox);
    assert_eq!(expected.core, actual.core);
    assert_eq!(expected.mirroring, actual.mirroring);
    assert_eq!(expected.empty_tile, actual.empty_tile);
    assert_eq!(expected.local_camera, actual.local_camera);
    assert_eq!(expected.scene_ratio.to_bits(), actual.scene_ratio.to_bits());
    assert_eq!(expected.emulated_menu_button, actual.emulated_menu_button);
    assert_eq!(expected.render_method, actual.render_method);
    assert_eq!(expected.resolution_scale_index, actual.resolution_scale_index);
    assert_eq!(expected.skip_frame, actual.skip_frame);
    assert_eq!(expected.background_music, actual.background_music);
    assert_eq!(expected.anti_alias, actual.anti_alias);
    assert_eq!(expected.auto_tv_mode, actual.auto_tv_mode);
    assert_eq!(expected.tv_mode, actual.tv_mode);
    assert_eq!(expected.tv_box, actual.tv_box);
    assert_eq!(expected.sort_option, actual.sort_option);
    assert_eq!(expected.same_sprite_palette, actual.same_sprite_palette);
    assert_eq!(expected.fov.to_bits(), actual.fov.to_bits());

    assert_slots(&expected.saved_patterns, &actual.saved_patterns, pairs, &format!("{location} saved patterns"));
    assert_slots(&expected.saved_anim_patterns, &actual.saved_anim_patterns, pairs, &format!("{location} saved anim patterns"));
}

fn assert_slots(expected: &[RecordedPatternSlot], actual: &[RecordedPatternSlot], pairs: &mut Pairs, location: &str) {
    assert_eq!(expected.len(), actual.len());
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        assert_eq!(expected.name, actual.name);
        assert_eq!(expected.pattern.is_none(), actual.pattern.is_none());
        if let (Some(expected), Some(actual)) = (&expected.pattern, &actual.pattern) {
            assert_pattern(expected, actual, pairs, &format!("{location}[{index}]"));
        }
    }
}

/// Records that `expected` is matched by `actual`. Returns true the first time; on a later
/// meeting of the same `expected` object it checks the very same `actual` object is there.
fn assert_same_sharing(expected: usize, actual: usize, pairs: &mut Pairs, location: &str) -> bool {
    if let Some(known) = pairs.get(&expected) {
        assert_eq!(*known, actual, "{location}: an object shared in one model is not shared in the other");
        return false;
    }
    pairs.insert(expected, actual);
    true
}
