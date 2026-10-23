//! Binding and frame instances (SPEC-04 §3.8), order and script queries (§3.9), injected
//! shapes and the pinned frame (§3.10), frame management (§3.12), hooks (§3.14) and the output
//! contract of SPEC-04B §3.8.

use std::sync::mpsc;

use rstest::rstest;
use zax_engine3d::draw_list::{MaterialClass, SceneContainer};
use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::frame::FrameInstanceId;
use zax_engine3d::geometry::builder::GeometryProgress;
use zax_engine3d::instance::{InstanceRef, PatternInstance, RenderStyle};
use zax_engine3d::palette::Rgb24;
use zax_engine3d::script::is_linked;
use zax_engine3d::settings::{RenderAlgorithm, ZAlignment, ZLayer};
use zax_engine3d::vectors::{Cell, Rect, Vec2, Vec3};
use zax_profiles::enums::{ProfileLayer, TextureFormat};

use super::support::{
    assert_close, cells, host, img, instances, permanent, recorder, recorder_mut, Batch,
    PatternOptions, ProfileBuilder, RecordingGeometryBuilder, RecordingScriptHost,
};

fn one_tile_engine(depth: i32, layer: ProfileLayer, tags: Option<&str>, instance_count: usize) -> ConversionEngine {
    ProfileBuilder::new()
        .pattern_with(
            1,
            1,
            &cells(&[img::a()]),
            PatternOptions {
                depth,
                layer,
                tags: tags.map(str::to_owned),
                instance_count,
                ..PatternOptions::default()
            },
        )
        .load()
}

fn default_engine() -> ConversionEngine {
    one_tile_engine(3, ProfileLayer::L1, None, 1)
}

/// Converts a batch of one tile at (16, 32) and returns the handle of its first instance.
fn one_tile_frame(engine: &mut ConversionEngine) -> FrameInstanceId {
    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    batch.convert(engine, 0).instances()[0]
}

fn first_reference(engine: &ConversionEngine, index: usize) -> InstanceRef {
    InstanceRef {
        pattern: permanent(engine, 0),
        index,
    }
}

#[test]
fn req_e3d_190_every_parameter_is_copied_and_the_custom_palette_is_reset() {
    let mut engine = one_tile_engine(3, ProfileLayer::L1, Some("t"), 1);
    let reference = first_reference(&engine, 0);
    let source = engine.registries_mut().0.instance_mut(reference).unwrap();
    source.offset = Vec3::new(1.0, 2.0, 3.0);
    source.scale = Vec3::new(2.0, 2.0, 2.0);
    source.rotation = Vec3::new(10.0, 20.0, 30.0);
    source.alpha = 0.5;
    source.custom_palette = 9;
    source.empty_color = 4;
    source.deform_amplitude = Vec3::new(0.0, 0.1, 0.0);
    source.set_custom_data(5, 7.0);

    let id = one_tile_frame(&mut engine);
    let instance = engine.converting_frame().instance(id);

    assert_eq!(Vec3::new(1.0, 2.0, 3.0), instance.offset);
    assert_eq!(Vec3::new(2.0, 2.0, 2.0), instance.scale);
    assert_eq!(Vec3::new(10.0, 20.0, 30.0), instance.rotation);
    assert_eq!(0.5, instance.alpha);
    assert_eq!(4, instance.empty_color);
    assert_eq!(7.0, instance.custom_data(5));
    assert_eq!(["t"], instance.tags());
    assert_eq!(reference, instance.reference());

    // The custom palette comes back to "none" every frame.
    assert_eq!(-1, instance.custom_palette);
}

#[test]
fn req_e3d_191_a_hidden_instance_stays_in_the_frame_but_is_not_drawn() {
    let mut engine = default_engine();
    engine.set_geometry(Box::new(RecordingGeometryBuilder::default()));

    let id = one_tile_frame(&mut engine);
    engine.converting_frame_mut().instance_mut(id).set_enable(false);

    assert_eq!(1, engine.converting_frame().instances().len());
    assert_eq!(RenderStyle::Hidden, engine.converting_frame().instance(id).style);
    assert!(engine.build_draw_list_of(FrameRole::Converting).tiles().is_empty());
}

#[test]
fn req_e3d_192_the_instance_palette_falls_back_to_the_shape_palette() {
    let mut engine = default_engine();
    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 2);
    let id = batch.convert(&mut engine, 0).instances()[0];
    let instance = engine.converting_frame_mut().instance_mut(id);

    assert_eq!(2, instance.palette_table());

    instance.custom_palette = 9;
    assert_eq!(9, instance.palette_table());
}

#[test]
fn req_e3d_205_the_placement_box_and_the_anchor() {
    let mut engine = default_engine();
    let id = one_tile_frame(&mut engine);
    let instance = engine.converting_frame().instance(id);

    assert_eq!(Vec3::new(16.0, 208.0, 0.0), instance.box_min(engine.settings()));
    assert_eq!(Vec3::new(24.0, 216.0, 3.0), instance.box_max(engine.settings()));
    assert_eq!(Vec3::new(20.0, 212.0, 1.5), instance.pos);
    assert_eq!(Vec3::new(20.0, 212.0, 1.5), instance.center);
    assert_eq!(Vec3::new(16.0, 208.0, -1.5), instance.bl());
    assert_eq!(Vec3::new(23.0, 215.0, 1.5), instance.tr());
}

#[test]
fn req_e3d_205_the_effective_layer_of_an_unidentified_instance() {
    let mut engine = one_tile_engine(3, ProfileLayer::Unidentified, None, 1);
    let id = one_tile_frame(&mut engine);
    let layer = engine.converting_frame().instance(id).effective_layer(engine.settings());
    assert_eq!(ZLayer::L1, layer);

    engine.settings_mut().default_layer = ZLayer::L3;
    let layer = engine.converting_frame().instance(id).effective_layer(engine.settings());
    assert_eq!(ZLayer::L3, layer);

    // A sprite shape always falls back to L1.
    let mut sprite_engine = one_tile_engine(3, ProfileLayer::Unidentified, None, 1);
    sprite_engine.settings_mut().default_layer = ZLayer::L3;
    let mut batch = Batch::new();
    batch.sprite(img::a(), 16, 32);
    let id = batch.convert(&mut sprite_engine, 0).instances()[0];
    let sprite = sprite_engine.converting_frame().instance(id);
    assert_eq!(ZLayer::L1, sprite.effective_layer(sprite_engine.settings()));
}

#[test]
fn req_e3d_205_the_layer_offset_and_alignment_move_the_depth_interval() {
    let mut engine = one_tile_engine(4, ProfileLayer::L2, None, 1);
    let id = one_tile_frame(&mut engine);

    let interval = |engine: &ConversionEngine| engine.converting_frame().instance(id).depth_interval(engine.settings());
    assert_eq!((12.0, 16.0), interval(&engine));

    engine.settings_mut().layer_mut(ZLayer::L2).offset = 0.0;
    engine.settings_mut().layer_mut(ZLayer::L2).alignment = ZAlignment::Center;
    assert_eq!((-2.0, 2.0), interval(&engine));

    engine.settings_mut().layer_mut(ZLayer::L2).alignment = ZAlignment::Back;
    assert_eq!((-4.0, 0.0), interval(&engine));
}

#[test]
fn req_e3d_206_the_exposed_derived_quantities() {
    let mut engine = default_engine();
    let id = one_tile_frame(&mut engine);
    let frame = engine.converting_frame();
    let shape = frame.shape(id.shape);
    let instance = frame.instance(id);

    assert_eq!(Vec2::new(16.0, 208.0), shape.bottom_left());
    assert_eq!(Vec2::new(23.0, 215.0), shape.top_right());
    assert_eq!(Vec3::new(16.0, 209.0, -1.5), instance.bottom_left());
    assert_eq!(Vec3::new(23.0, 216.0, 1.5), instance.top_right());
    assert_eq!(Vec3::new(16.0, 208.0, -1.5), instance.bl());
    assert_eq!(Vec3::new(8.0, 8.0, 3.0), instance.size());
    assert_eq!(Cell::new(8, 8), shape.pixel_size());
    assert_eq!(0, instance.index());
}

#[test]
fn req_e3d_207_the_age_of_a_shape_is_always_one() {
    assert_eq!(1, zax_engine3d::frame::FrameShape::AGE);
}

#[test]
fn req_e3d_220_scriptable_instances_are_the_segmentation_ones_in_order() {
    let mut engine = one_tile_engine(3, ProfileLayer::L1, None, 2);
    let pattern = permanent(&engine, 0);

    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(move |state| {
            state
                .create_new_shape(pattern, Vec3::new(100.0, 100.0, 0.0), 5)
                .unwrap();
        })),
        ..RecordingScriptHost::default()
    }));

    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0).bg(img::a(), 100, 100, 0);
    let frame = batch.convert(&mut engine, 1);

    // Two segmented shapes of two instances each, plus the injected shape.
    assert_eq!(3, frame.shapes().len());
    assert_eq!(6, frame.instances().len());
    assert_eq!(4, frame.scriptable_instances().len());
    let shapes = frame.shapes().to_vec();
    let instance = |shape: usize, index: usize| FrameInstanceId {
        shape: shapes[shape],
        index,
    };
    assert_eq!(Some(instance(0, 0)), engine.get_shape(0));
    assert_eq!(Some(instance(0, 1)), engine.get_shape(1));
    assert_eq!(Some(instance(1, 0)), engine.get_shape(2));

    // A shape created by a script is not returned by the tag queries.
    let mut all = Vec::new();
    engine.collect_shapes_with_tag("*", &mut all);
    assert_eq!(4, all.len());
    assert!(!all.contains(&instance(2, 0)));
}

#[test]
fn req_e3d_231_add_new_shape_places_the_tiles_at_the_grid_origin() {
    let mut engine = default_engine();
    let pattern = permanent(&engine, 0);

    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(move |state| {
            state
                .add_new_shape(pattern, Vec3::new(40.0, 100.0, 0.0), 5)
                .unwrap();
        })),
        ..RecordingScriptHost::default()
    }));

    let frame = Batch::new().convert(&mut engine, 1);

    let id = frame.shapes()[0];
    let tile = frame.shape_tiles(id).next().unwrap();
    assert_eq!(0, tile.x());
    assert_eq!(0, tile.y());
    assert_eq!(Vec3::new(44.0, 104.0, 1.5), frame.shape(id).instances()[0].pos);
}

#[test]
fn req_e3d_232_insert_shape_equals_create_new_shape_shifted_by_the_bottom_left() {
    let mut engine = default_engine();
    let pattern = permanent(&engine, 0);
    assert_eq!(Vec2::new(0.0, 0.0), engine.patterns().get(pattern).unwrap().bottom_left());

    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(move |state| {
            state.insert_shape(pattern, Vec3::new(10.0, 20.0, 0.0), 5).unwrap();
            state
                .create_new_shape(pattern, Vec3::new(10.0, 20.0, 0.0), 5)
                .unwrap();
        })),
        ..RecordingScriptHost::default()
    }));

    let frame = Batch::new().convert(&mut engine, 1);

    assert_eq!(2, frame.shapes().len());
    let first = frame.shape_tiles(frame.shapes()[0]).next().unwrap();
    let second = frame.shape_tiles(frame.shapes()[1]).next().unwrap();
    assert_eq!(second.x(), first.x());
    assert_eq!(second.y(), first.y());
}

#[test]
fn req_e3d_230_with_an_instance_only_that_instance_is_enabled() {
    let mut engine = one_tile_engine(3, ProfileLayer::L1, None, 3);
    let second = first_reference(&engine, 1);

    let (sender, receiver) = mpsc::channel();
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(move |state| {
            let returned = state
                .create_new_shape_from_instance(second, Vec3::new(0.0, 0.0, 0.0), 5)
                .unwrap();
            sender.send(returned).unwrap();
        })),
        ..RecordingScriptHost::default()
    }));

    let frame = Batch::new().convert(&mut engine, 1);

    let id = frame.shapes()[0];
    let shape = frame.shape(id);
    assert_eq!(FrameInstanceId { shape: id, index: 1 }, receiver.recv().unwrap());
    assert!(!shape.instances()[0].enable());
    assert!(shape.instances()[1].enable());
    assert!(!shape.instances()[2].enable());
}

#[test]
fn req_e3d_233_a_pinned_frame_restores_its_shapes_where_a_rectangle_overlaps() {
    let mut engine = default_engine();

    let mut first = Batch::new();
    first.bg(img::a(), 16, 32, 0).bg(img::a(), 160, 160, 0);
    let pinned = first.convert_and_publish(&mut engine, 1);
    assert_eq!(2, pinned.shapes().len());
    let pinned_shape = pinned.shape(pinned.shapes()[0]).clone();

    engine.pin_new_frame();
    assert!(engine.same_frame(FrameRole::Pinned, FrameRole::Published));

    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(|state| {
            state.restore_from_pinned_frame_rect(Rect::new(14.0, 30.0, 12.0, 12.0));
        })),
        ..RecordingScriptHost::default()
    }));

    let second = Batch::new().convert(&mut engine, 2);

    // The restored shape is the pinned frame's first shape, copied into this frame.
    assert_eq!(1, second.shapes().len());
    let restored_id = second.shapes()[0];
    let restored = second.shape(restored_id);
    assert_eq!(pinned_shape.pixel_start(), restored.pixel_start());
    assert_eq!(pinned_shape.pixel_end(), restored.pixel_end());
    assert_eq!(pinned_shape.tile_count(), restored.tile_count());
    assert_eq!(pinned_shape.reference_pattern(), restored.reference_pattern());
    assert!(restored.restored_from_pinned_frame());
    assert!(!restored.scriptable());
    assert!(second.scriptable_instances().is_empty());

    // Only the tiles whose 8 x 8 cell overlaps a rectangle are drawn.
    assert!(second.tile_overlaps_pinned_rects(second.shape_tiles(restored_id).next().unwrap()));
    assert_eq!(1, engine.build_draw_list_of(FrameRole::Converting).tiles().len());

    // The pinned frame itself is untouched by the new conversion.
    let pinned = engine.pinned_frame().unwrap();
    assert_eq!(2, pinned.shapes().len());
    assert_eq!(1, pinned.counter());
}

#[test]
fn req_e3d_233_without_a_pinned_frame_the_calls_do_nothing() {
    let mut engine = default_engine();
    Batch::new().convert(&mut engine, 1);

    engine.add_rect_from_pinned_frame(0.0, 0.0, 256.0, 248.0);
    assert!(engine.converting_frame().pinned_rects().is_empty());
    assert_eq!(0, engine.restore_from_pinned_frame());

    engine.release_pinned_frame();
    assert!(engine.pinned_frame().is_none());
}

#[test]
fn req_e3d_280_a_new_conversion_never_touches_the_published_or_pinned_frame() {
    let mut engine = default_engine();

    let mut first = Batch::new();
    first.bg(img::a(), 16, 32, 0);
    first.convert_and_publish(&mut engine, 1);
    engine.pin_new_frame();

    let mut batch = Batch::new();
    batch.bg(img::a(), 24, 32, 0);
    for counter in 2..8 {
        batch.convert(&mut engine, counter);
        assert!(!engine.same_frame(FrameRole::Converting, FrameRole::Published));
        assert!(!engine.same_frame(FrameRole::Converting, FrameRole::Pinned));
        engine.publish();
    }

    let published = engine.pinned_frame().unwrap();
    assert_eq!(1, published.shapes().len());
    assert_eq!(1, published.counter());
    assert_eq!(Cell::new(16, 32), published.shape(published.shapes()[0]).pixel_start());
}

#[test]
fn req_e3d_281_clearing_the_frames_empties_everything() {
    let mut engine = default_engine();
    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    batch.convert_and_publish(&mut engine, 5);
    engine.pin_new_frame();

    engine.clear_frames();

    assert!(engine.published_frame().is_none());
    assert!(engine.pinned_frame().is_none());
    assert!(engine.converting_frame().shapes().is_empty());
    assert_eq!(0, engine.converting_frame().counter());
}

#[test]
fn req_e3d_320_the_hooks_run_in_order_after_the_segmentation() {
    let mut engine = default_engine();
    let (sender, receiver) = mpsc::channel();
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update: Some(Box::new(move |state| {
            sender.send(state.converting_frame().shapes().len()).unwrap();
        })),
        ..RecordingScriptHost::default()
    }));

    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    batch.convert(&mut engine, 7);

    let host = host(&engine);
    assert_eq!(["globals(7)", "Update", "UpdateS[1]", "LateUpdate"], host.calls.as_slice());
    assert_eq!(1, receiver.recv().unwrap());
    assert_eq!(Some(7), host.last_frame);
    assert_close(7.0 / 60.0, host.last_time.unwrap(), 5);
}

#[test]
fn req_e3d_001_the_position_takes_the_offset_a_hook_changed() {
    let mut engine = default_engine();
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update_shape: Some(Box::new(|instance| {
            instance.offset.y += 10.0;
            Ok(())
        })),
        ..RecordingScriptHost::default()
    }));

    let id = one_tile_frame(&mut engine);

    assert_eq!(222.0, engine.converting_frame().instance(id).pos.y);
}

#[test]
fn req_e3d_321_a_script_is_linked_by_star_or_by_an_equal_tag() {
    let engine = one_tile_engine(3, ProfileLayer::L1, Some("boss"), 1);
    let instance = engine.patterns().instance(first_reference(&engine, 0)).unwrap();

    assert!(is_linked(Some("enemy, boss"), instance));
    assert!(!is_linked(Some("enemy, Boss"), instance));
    assert!(is_linked(Some("*"), instance));
    assert!(is_linked(Some("ui,*"), instance));
    assert!(is_linked(Some("a*b"), instance));
    assert!(!is_linked(Some(""), instance));
}

#[test]
fn req_e3d_322_build_pattern_runs_before_every_construction() {
    let mut builder = ProfileBuilder::new();
    builder.pattern_with(
        1,
        1,
        &cells(&[img::a()]),
        PatternOptions {
            tags: Some("*".to_owned()),
            ..PatternOptions::default()
        },
    );
    let mut engine = ConversionEngine::new();
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_build_pattern: Some(Box::new(|instance| instance.depth = 8)),
        ..RecordingScriptHost::default()
    }));
    engine.set_geometry(Box::new(RecordingGeometryBuilder::default()));

    engine.load_profile(&builder.build(), true).unwrap();

    assert_eq!(1, host(&engine).built_patterns.len());
    let depth = |engine: &ConversionEngine, n: usize| {
        engine.patterns().get(permanent(engine, n)).unwrap().instances()[0].depth
    };
    assert_eq!(8, depth(&engine, 0));
    assert_eq!(1, recorder(&engine).built.len());

    // Learning a pattern runs the hook for it too, before its construction.
    let mut batch = Batch::new();
    batch.bg_cell(img::u(), 10, 10);
    batch.convert(&mut engine, 1);
    assert_eq!(2, host(&engine).built_patterns.len());
    assert_eq!(2, recorder(&engine).built.len());
    assert_eq!(8, depth(&engine, 1));
}

#[test]
fn req_e3d_323_the_pipeline_has_no_detect_shape_call_point() {
    // The hook is recognised by the runtime but never invoked, so the interface the pipeline
    // drives carries no such member (DEC-04-Q01).
    let source = include_str!("../../src/script.rs");
    assert!(!source.contains("fn detect_shape"));
    assert!(!source.contains("fn invoke_detect_shape"));
}

#[test]
fn req_e3d_324_a_hook_error_does_not_interrupt_the_conversion() {
    let mut engine = default_engine();
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_update_shape: Some(Box::new(|_| Err(zax_engine3d::script::ScriptError("lua error".to_owned())))),
        ..RecordingScriptHost::default()
    }));

    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    let frame = batch.convert(&mut engine, 1);

    assert_eq!(1, frame.script_error_count());
    assert_eq!(1, frame.shapes().len());
    assert_eq!(Vec3::new(20.0, 212.0, 1.5), instances(frame)[0].pos);

    super::support::host_mut(&mut engine).on_update_shape = None;
    let clean = batch.convert(&mut engine, 2);
    assert_eq!(0, clean.script_error_count());
}

#[test]
fn req_e3d_340_changing_the_global_algorithm_rebuilds_every_pattern() {
    let mut engine = ProfileBuilder::new()
        .row(&cells(&[img::a()]))
        .row(&cells(&[img::b()]))
        .load();
    engine.set_geometry(Box::new(RecordingGeometryBuilder::default()));

    engine.set_render_algorithm(RenderAlgorithm::Marching);
    assert_eq!(RenderAlgorithm::Marching, engine.settings().render_algorithm);
    assert_eq!(2, recorder(&engine).built.len());

    // Setting the same value again rebuilds nothing.
    engine.set_render_algorithm(RenderAlgorithm::Marching);
    assert_eq!(2, recorder(&engine).built.len());
}

#[test]
fn req_e3d_340_the_per_instance_algorithm_wins_over_the_setting() {
    let mut instance = PatternInstance::new();
    instance.mesh_algorithm = 2;
    assert_eq!(RenderAlgorithm::Marching, instance.effective_algorithm(RenderAlgorithm::Greedy));
    assert_eq!(RenderAlgorithm::Marching, instance.effective_algorithm(RenderAlgorithm::GreedyMulticolor));

    instance.mesh_algorithm = 0;
    assert_eq!(RenderAlgorithm::Greedy, instance.effective_algorithm(RenderAlgorithm::Greedy));
    assert_eq!(RenderAlgorithm::Marching, instance.effective_algorithm(RenderAlgorithm::Marching));
}

#[test]
fn req_e3d_361_a_shape_without_geometry_is_kept_but_not_drawn() {
    let mut engine = default_engine();
    engine.set_geometry(Box::new(RecordingGeometryBuilder::default()));
    let reference = first_reference(&engine, 0);
    recorder_mut(&mut engine).not_ready.insert(reference);

    one_tile_frame(&mut engine);

    assert_eq!(1, engine.converting_frame().shapes().len());
    assert_eq!(1, engine.converting_frame().scriptable_instances().len());
    assert!(engine.build_draw_list_of(FrameRole::Converting).tiles().is_empty());
}

#[rstest]
#[case(0, 0, 0, 1.0)]
#[case(50, 0, 100, 0.25)]
#[case(100, 100, 100, 1.0)]
#[case(100, 0, 100, 0.5)]
fn req_e3d_362_the_loading_progress(#[case] built: usize, #[case] transferred: usize, #[case] to_build: usize, #[case] expected: f32) {
    let mut engine = default_engine();
    engine.set_geometry(Box::new(RecordingGeometryBuilder {
        reported_progress: GeometryProgress::new(built, transferred, to_build),
        ..RecordingGeometryBuilder::default()
    }));

    assert_close(expected, engine.load_progress(), 5);
}

#[test]
fn req_e3d_250_254_the_scene_background_colour() {
    let mut engine = default_engine();
    let mut batch = Batch::new();
    batch.palette(0, 0x21).bg(img::a(), 16, 32, 0);
    batch.convert_and_publish(&mut engine, 1);

    assert_eq!(Rgb24::new(0x38, 0xC0, 0xFC), engine.background_color(false));

    // A skybox, or the setting turned off, gives the custom colour.
    assert_eq!(engine.settings().custom_background_color, engine.background_color(true));
    engine.settings_mut().use_nes_background_color = false;
    engine.settings_mut().custom_background_color = Rgb24::new(1, 2, 3);
    assert_eq!(Rgb24::new(1, 2, 3), engine.background_color(false));
}

#[test]
fn req_e3d_252_a_palette_write_only_affects_the_frame_it_targets() {
    let mut engine = default_engine();
    engine.set_script_host(Box::new(RecordingScriptHost::default()));

    let mut batch = Batch::new();
    batch.palette(5, 0x16).bg(img::a(), 16, 32, 0);
    batch.convert_and_publish(&mut engine, 1);

    super::support::host_mut(&mut engine).on_update = Some(Box::new(|state| {
        assert!(state.set_color_index(5, 0x30, true));
    }));
    let current = batch.convert(&mut engine, 2);

    assert_eq!(Some(0x30), current.palette().get(5));
    assert_eq!(0x30, engine.frame_color_index(5, true));

    // The published frame keeps the core's value; the next frame rebuilds from the core.
    assert_eq!(0x16, engine.frame_color_index(5, false));
    engine.publish();
    super::support::host_mut(&mut engine).on_update = None;
    assert_eq!(Some(0x16), batch.convert(&mut engine, 3).palette().get(5));
}

#[test]
fn req_e3d_641_the_draw_list_carries_the_placement_and_the_flags() {
    let mut engine = default_engine();
    engine.set_geometry(Box::new(RecordingGeometryBuilder::default()));

    let mut batch = Batch::new();
    batch
        .palette(1, 0x16)
        .bg(img::a(), 16, 32, 1)
        .sprite_full(img::a(), 40, 40, 0, true);
    batch.convert(&mut engine, 3);

    let list = engine.build_draw_list_of(FrameRole::Converting).clone();

    assert_eq!(2, list.tiles().len());
    assert_eq!(3, list.frame_counter());
    assert_close(3.0 / 60.0, list.time(), 5);
    assert_eq!(Rgb24::new(0xFC, 0x38, 0x00), list.palette()[1]);

    let background = &list.tiles()[0];
    assert!(engine.converting_frame().instance(background.instance).background());
    assert!(!background.sprite);
    assert_eq!(SceneContainer::Game, background.container);
    assert_eq!(MaterialClass::Opaque, background.material);
    assert_eq!(1, background.palette_table);
    assert_eq!(TextureFormat::Indexed, background.texture_format);
    assert!(background.cast_shadow);
    assert!(background.receive_shadow);
    assert!(background.deform.is_none());
    assert!(background.mesh.is_some());

    // The tile centre of SPEC-04B REQ-E3D-405 and the transform of REQ-E3D-406.
    assert_eq!(Vec3::new(20.0, 212.0, 1.5), background.tile_center);
    assert_eq!(background.tile_center, background.transform.transform_point(Vec3::ZERO));

    let sprite = &list.tiles()[1];
    assert!(sprite.sprite);
    assert!(sprite.behind_background);
}

#[test]
fn req_e3d_642_material_classes_and_shadow_rules() {
    let mut engine = default_engine();
    let id = one_tile_frame(&mut engine);
    let first_tile = |engine: &mut ConversionEngine| engine.build_draw_list_of(FrameRole::Converting).tiles()[0].clone();
    let edit = |engine: &mut ConversionEngine, change: &dyn Fn(&mut zax_engine3d::frame::FrameInstance)| {
        change(engine.converting_frame_mut().instance_mut(id));
    };

    edit(&mut engine, &|instance| instance.alpha = 0.5);
    assert_eq!(MaterialClass::Transparent, first_tile(&mut engine).material);

    edit(&mut engine, &|instance| instance.style = RenderStyle::ShadowOnly);
    let shadow_only = first_tile(&mut engine);
    assert_eq!(MaterialClass::ShadowOnly, shadow_only.material);
    assert!(shadow_only.cast_shadow);

    edit(&mut engine, &|instance| {
        instance.style = RenderStyle::Normal;
        instance.alpha = 1.0;
        instance.layer = ZLayer::Ui;
    });
    let ui = first_tile(&mut engine);
    assert_eq!(MaterialClass::Interface, ui.material);
    assert_eq!(SceneContainer::Interface, ui.container);
    assert!(!ui.cast_shadow);
    assert!(!ui.receive_shadow);

    // A transparent instance casts no shadow (REQ-E3D-567).
    edit(&mut engine, &|instance| {
        instance.layer = ZLayer::L1;
        instance.alpha = 0.25;
    });
    assert!(!first_tile(&mut engine).cast_shadow);

    // !cast takes the shadow away, !recv the reception.
    edit(&mut engine, &|instance| {
        instance.alpha = 1.0;
        instance.set_tag_string("!cast");
    });
    assert!(!first_tile(&mut engine).cast_shadow);
    edit(&mut engine, &|instance| instance.set_tag_string("!recv"));
    assert!(!first_tile(&mut engine).receive_shadow);
}

#[test]
fn req_e3d_567_layer_permissions_decide_the_shadows() {
    let mut engine = one_tile_engine(3, ProfileLayer::L2, None, 1);
    one_tile_frame(&mut engine);

    let tile = engine.build_draw_list_of(FrameRole::Converting).tiles()[0].clone();
    assert!(!tile.cast_shadow);
    assert!(tile.receive_shadow);

    engine.settings_mut().shadow = false;
    let none = engine.build_draw_list_of(FrameRole::Converting).tiles()[0].clone();
    assert!(!none.cast_shadow);
    assert!(!none.receive_shadow);
}
