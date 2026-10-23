//! Multi-tile shapes, multiple 3D objects, the mesh lifetime of SPEC-04B §3.7 and the
//! determinism REQ-ARCH-012 asks of the geometry.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::sync::Arc;

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::geometry::builder::{GeometryBuilder, PatternGeometryBuilder};
use zax_engine3d::image::TileImageKey;
use zax_engine3d::instance::{InstanceRef, RenderStyle};
use zax_engine3d::settings::{ConversionSettings, Geo3DType, RenderAlgorithm, ZLayer};
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::{MeshAlgorithm, ProfileLayer, VariantGeometry};
use zax_profiles::values::Vec4F;

use super::support::geo::{self, rectangles};
use super::support::{
    assert_close, cells, img, permanent, Batch, PatternOptions, ProfileBuilder, RecordingScriptHost,
};

fn with_depth(depth: i32) -> PatternOptions {
    PatternOptions {
        depth,
        ..PatternOptions::default()
    }
}

fn geometry_engine() -> ConversionEngine {
    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine
}

fn builder(engine: &ConversionEngine) -> &GeometryBuilder {
    engine.geometry_builder().unwrap()
}

fn builder_mut(engine: &mut ConversionEngine) -> &mut GeometryBuilder {
    engine.geometry_builder_mut().unwrap()
}

fn instance_ref(engine: &ConversionEngine, pattern: usize, index: usize) -> InstanceRef {
    InstanceRef {
        pattern: permanent(engine, pattern),
        index,
    }
}

fn convert_and_draw(engine: &mut ConversionEngine, batch: &Batch, counter: i32) -> FrameDrawList {
    batch.convert(engine, counter);
    engine.build_draw_list_of(FrameRole::Converting).clone()
}

fn redraw(engine: &mut ConversionEngine) -> FrameDrawList {
    engine.build_draw_list_of(FrameRole::Converting).clone()
}

#[test]
fn req_e3d_540_a_shape_draws_one_mesh_per_tile_and_shares_the_repeated_ones() {
    // Four tiles drawn, two distinct meshes: three cells carry the same image.
    let mut engine = geometry_engine();
    engine
        .load_profile(
            &ProfileBuilder::new()
                .pattern_with(2, 2, &cells(&[img::a(), img::a(), img::a(), img::b()]), with_depth(2))
                .build(),
            true,
        )
        .unwrap();

    let mut batch = Batch::new();
    batch
        .bg(img::a(), 0, 0, 0)
        .bg(img::a(), 8, 0, 0)
        .bg(img::a(), 0, 8, 0)
        .bg(img::b(), 8, 8, 0);
    let list = convert_and_draw(&mut engine, &batch, 0);

    assert_eq!(4, list.tiles().len());

    // REQ-E3D-540: one mesh per distinct 2D tile. Q-067 adds that what is *drawn* is the
    // variant of that mesh without the walls the neighbours cover, so the criterion is read on
    // the mesh of the image, which is the one the object owns.
    let mut meshes = HashSet::new();
    for tile in list.tiles() {
        let reference = engine.converting_frame().instance(tile.instance).reference();
        let mesh = builder_mut(&mut engine)
            .mesh_of(reference, tile.tile.image(), 0)
            .unwrap();
        meshes.insert(Arc::as_ptr(&mesh));
    }

    assert_eq!(2, meshes.len());
    assert_eq!(1, builder(&engine).object_count());
}

#[test]
fn req_e3d_541_each_3d_object_of_a_pattern_gets_its_own_geometry() {
    let mut engine = geometry_engine();

    let mut profile = ProfileBuilder::new();
    profile.pattern_with(
        1,
        1,
        &cells(&[img::a()]),
        PatternOptions {
            depth: 3,
            instance_count: 2,
            ..PatternOptions::default()
        },
    );
    let variant = profile.variant(0, 1);
    variant.geometry = VariantGeometry::Cube;
    variant.layer = ProfileLayer::L2;
    variant.depth = 4;
    engine.load_profile(&profile.build(), true).unwrap();

    let pattern = engine.patterns().get(permanent(&engine, 0)).unwrap();
    assert_eq!(2, pattern.instances().len());
    assert_eq!(2, builder(&engine).object_count());

    let first = builder(&engine).of(instance_ref(&engine, 0, 0)).unwrap();
    let second = builder(&engine).of(instance_ref(&engine, 0, 1)).unwrap();
    assert!(!std::ptr::eq(first, second));
    assert_eq!(3, first.build_depth());
    assert_eq!(4, second.build_depth());

    // The Default object is a prism of six rectangles, the CUBE object a box of twelve.
    let image = pattern.images()[0];
    assert_eq!(6, rectangles(first.mesh(&image).unwrap()));
    assert_eq!(12, rectangles(second.mesh(&image).unwrap()));

    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    let list = convert_and_draw(&mut engine, &batch, 0);
    assert_eq!(2, list.tiles().len());
    assert_ne!(list.tiles()[0].transform, list.tiles()[1].transform);
}

#[test]
fn req_e3d_542_changing_a_frame_instance_moves_it_without_remeshing_it() {
    let mut engine = geometry_engine();
    engine
        .load_profile(&ProfileBuilder::new().pattern_with(1, 1, &cells(&[img::a()]), with_depth(4)).build(), true)
        .unwrap();

    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    let first = batch.convert(&mut engine, 0).instances()[0];
    let builds = builder(&engine).build_count();

    let before = redraw(&mut engine).tiles()[0].clone();
    let mesh = before.mesh.clone().unwrap();
    assert_close(4.0, mesh.bounds_max().z - mesh.bounds_min().z, 3);

    // A script makes the instance deeper: the placement box follows, the mesh does not.
    engine.converting_frame_mut().instance_mut(first).depth = 16;
    engine.calculate_positions();
    let list = redraw(&mut engine);
    assert_eq!(1, list.tiles().len());
    let after = &list.tiles()[0];

    assert!(Arc::ptr_eq(&mesh, after.mesh.as_ref().unwrap()));
    assert_close(4.0, mesh.bounds_max().z - mesh.bounds_min().z, 3);
    assert_ne!(before.anchor.z, after.anchor.z);
    assert_eq!(builds, builder(&engine).build_count());
}

#[test]
fn req_e3d_543_a_hidden_or_unready_object_is_not_drawn() {
    let mut engine = geometry_engine();
    builder_mut(&mut engine).defer_transfer = true;
    engine
        .load_profile(&ProfileBuilder::new().pattern_with(1, 1, &cells(&[img::a()]), with_depth(2)).build(), true)
        .unwrap();

    let mut batch = Batch::new();
    batch.bg(img::a(), 16, 32, 0);
    let frame = batch.convert(&mut engine, 0);

    // REQ-E3D-601: the shape is recognised and visible to scripts, but not drawn until its
    // meshes have been handed over.
    assert_eq!(1, frame.instances().len());
    assert!(redraw(&mut engine).tiles().is_empty());

    assert_eq!(1, builder_mut(&mut engine).commit());
    assert_eq!(1, redraw(&mut engine).tiles().len());

    let id = permanent(&engine, 0);
    engine.pattern_mut(id).unwrap().instances_mut()[0].style = RenderStyle::Hidden;
    assert!(convert_and_draw(&mut engine, &batch, 2).tiles().is_empty());
}

#[test]
fn req_e3d_600_a_mass_build_reports_its_progress_in_two_halves() {
    let mut engine = geometry_engine();
    builder_mut(&mut engine).defer_transfer = true;

    let mut profile = ProfileBuilder::new();
    for n in 0..4 {
        profile.pattern_with(1, 1, &[Some(img::full(n))], with_depth(2));
    }

    engine.load_profile(&profile.build(), true).unwrap();

    // Four objects built, none transferred: (4 + 0) / (2 × 4) = 0.5.
    assert_eq!(4, builder(&engine).build_count());
    assert_close(0.5, builder(&engine).progress().value(), 3);
    assert_close(0.5, engine.load_progress(), 3);

    assert_eq!(4, builder_mut(&mut engine).commit());
    assert_close(1.0, builder(&engine).progress().value(), 3);
    assert_close(1.0, engine.load_progress(), 3);

    // Changing the global algorithm rebuilds every object, with its own progress.
    engine.set_render_algorithm(RenderAlgorithm::Marching);
    assert_eq!(8, builder(&engine).build_count());
    assert_close(0.5, builder(&engine).progress().value(), 3);
    builder_mut(&mut engine).commit();
    assert_close(1.0, builder(&engine).progress().value(), 3);

    for &id in engine.patterns().permanent() {
        let reference = InstanceRef { pattern: id, index: 0 };
        assert_eq!(RenderAlgorithm::Marching, builder(&engine).of(reference).unwrap().algorithm());
    }
}

#[test]
fn req_e3d_500_an_object_with_its_own_algorithm_ignores_the_setting() {
    let mut engine = geometry_engine();

    let mut profile = ProfileBuilder::new();
    profile.pattern_with(
        1,
        1,
        &cells(&[img::a()]),
        PatternOptions {
            depth: 2,
            instance_count: 2,
            ..PatternOptions::default()
        },
    );
    profile.variant(0, 1).mesh_algorithm = MeshAlgorithm::Marching;
    engine.load_profile(&profile.build(), true).unwrap();

    let algorithm = |engine: &ConversionEngine, index| builder(engine).of(instance_ref(engine, 0, index)).unwrap().algorithm();
    assert_eq!(RenderAlgorithm::Greedy, algorithm(&engine, 0));
    assert_eq!(RenderAlgorithm::Marching, algorithm(&engine, 1));

    engine.set_render_algorithm(RenderAlgorithm::GreedyMulticolor);
    assert_eq!(RenderAlgorithm::GreedyMulticolor, algorithm(&engine, 0));
    assert_eq!(RenderAlgorithm::Marching, algorithm(&engine, 1));
}

#[test]
fn req_e3d_602_no_runtime_change_ever_rebuilds_a_mesh() {
    let mut engine = geometry_engine();
    engine
        .load_profile(&ProfileBuilder::new().pattern_with(1, 1, &cells(&[img::a()]), with_depth(4)).build(), true)
        .unwrap();

    let builds = builder(&engine).build_count();
    let reference = instance_ref(&engine, 0, 0);
    let image = engine.images().by_id(0).unwrap();
    let mesh = builder_mut(&mut engine).mesh_of(reference, &image, 0).unwrap();

    for counter in 1..=3u8 {
        let mut batch = Batch::new();
        batch
            .palette(1, 0x10 + counter)
            .bg(img::a(), 16 + i32::from(counter), 32, 0);
        let id = batch.convert(&mut engine, i32::from(counter)).instances()[0];

        let value = f32::from(counter);
        let instance = engine.converting_frame_mut().instance_mut(id);
        instance.offset = Vec3::new(value, 0.0, 0.0);
        instance.scale = Vec3::new(1.0 + value, 1.0, 1.0);
        instance.rotation = Vec3::new(0.0, 0.0, 10.0 * value);
        instance.alpha = 0.5;
        instance.layer = ZLayer::L3;
        instance.set_tag_string("!cast");
        instance.deform_amplitude = Vec3::new(0.0, 0.1, 0.0);
        instance.custom_palette = i32::from(counter);
        engine.calculate_positions();

        let list = redraw(&mut engine);
        assert_eq!(1, list.tiles().len());
        assert!(Arc::ptr_eq(&mesh, list.tiles()[0].mesh.as_ref().unwrap()));
    }

    assert_eq!(builds, builder(&engine).build_count());
    assert_close(1.0, builder(&engine).progress().value(), 3);
}

#[test]
fn req_e3d_600_learning_a_pattern_builds_only_its_geometry() {
    let mut engine = geometry_engine();
    engine
        .load_profile(&ProfileBuilder::new().pattern_with(1, 1, &cells(&[img::a()]), with_depth(4)).build(), true)
        .unwrap();

    let builds = builder(&engine).build_count();

    // An unknown sprite in the middle of the screen is learned (REQ-E3D-161) with the 3D
    // defaults of REQ-E3D-453: Default, depth 3, layer L1.
    let mut batch = Batch::new();
    batch.sprite(img::u(), 64, 64);
    let list = convert_and_draw(&mut engine, &batch, 0);

    assert_eq!(builds + 1, builder(&engine).build_count());
    assert_eq!(2, engine.patterns().permanent().len());

    let learned = engine.patterns().instance(instance_ref(&engine, 1, 0)).unwrap();
    assert_eq!(3, learned.depth);
    assert_eq!(ZLayer::L1, learned.layer);
    assert_eq!(Geo3DType::Default, learned.geometry);

    assert_eq!(1, list.tiles().len());
    assert_eq!(6, rectangles(list.tiles()[0].mesh.as_ref().unwrap()));
}

#[test]
fn req_e3d_603_releasing_a_pattern_frees_its_meshes_and_nothing_caps_a_frame() {
    let pair = ProfileBuilder::new()
        .pattern_with(2, 1, &cells(&[img::a(), img::b()]), with_depth(2))
        .build();
    let mut engine = geometry_engine();
    engine.load_profile(&pair, true).unwrap();

    assert_eq!(1, builder(&engine).object_count());
    engine.unload();
    assert_eq!(0, builder(&engine).object_count());

    // 1 500 instanced shapes over 3 000 tiles all reach the draw list (REQ-E3D-603).
    let mut crowd = geometry_engine();
    crowd.load_profile(&pair, true).unwrap();

    let pattern = permanent(&crowd, 0);
    Batch::new().convert(&mut crowd, 0);
    for n in 0..1500u16 {
        let position = Vec3::new(f32::from(n % 200), f32::from(n / 200), 0.0);
        crowd.add_new_shape(pattern, position, 4).unwrap();
    }

    crowd.calculate_positions();
    assert_eq!(3000, redraw(&mut crowd).tiles().len());
}

fn draw_at(engine: &mut ConversionEngine, x: i32, y: i32, counter: i32) -> FrameDrawList {
    let mut batch = Batch::new();
    batch.bg(img::a(), x, y, 0);
    convert_and_draw(engine, &batch, counter)
}

fn describe(list: &FrameDrawList) -> String {
    let mut text = String::new();
    for tile in list.tiles() {
        writeln!(
            text,
            "{:?} {} {} {:?}",
            tile.material,
            tile.alpha,
            tile.palette_table,
            tile.transform.to_row_major()
        )
        .unwrap();
        for vertex in tile.mesh.as_ref().unwrap().vertices() {
            writeln!(text, "  {} {} {}", vertex.position, vertex.normal, vertex.uv).unwrap();
        }
    }

    text
}

#[test]
fn req_e3d_643_the_clip_margins_never_touch_the_meshes() {
    let mut engine = geometry_engine();
    engine
        .load_profile(&ProfileBuilder::new().pattern_with(1, 1, &cells(&[img::a()]), with_depth(4)).build(), true)
        .unwrap();

    engine.settings_mut().clipping = ConversionSettings::DEFAULT_CLIPPING;
    let wide = describe(&draw_at(&mut engine, 64, 64, 1));

    engine.settings_mut().clipping = Vec4F::new(40.0, 40.0, 40.0, 40.0);
    let narrow = describe(&draw_at(&mut engine, 64, 64, 2));

    assert_eq!(wide, narrow);
}

fn build_all() -> String {
    let mut engine = geometry_engine();

    let tiles: [TileImageKey; 4] = [
        geo::solid(1),
        geo::tile(&["12345678"]),
        geo::tile(&["11111111", "1......1", "1......1", "11111111"]),
        geo::tile(&["1......."]),
    ];
    let kinds = [
        VariantGeometry::Default,
        VariantGeometry::VerticalCylinder,
        VariantGeometry::HorizontalCylinder,
        VariantGeometry::Cube,
    ];

    let mut profile = ProfileBuilder::new();
    for (n, tile) in (0i32..).zip(tiles) {
        profile.pattern_with(1, 1, &[Some(tile)], with_depth(2 + (2 * n)));
    }

    for (n, kind) in kinds.into_iter().enumerate() {
        let variant = profile.variant(n, 0);
        variant.geometry = kind;
        variant.solid = n % 2 == 0;
        variant.full_warp = n % 3 == 0;
    }

    engine.load_profile(&profile.build(), true).unwrap();

    let mut text = String::new();
    for &id in engine.patterns().permanent() {
        let count = engine.patterns().get(id).unwrap().instances().len();
        for index in 0..count {
            let built = builder(&engine).of(InstanceRef { pattern: id, index }).unwrap();
            writeln!(text, "{:?} {} {}", built.algorithm(), built.build_depth(), built.effective_depth()).unwrap();
            for mesh in built.meshes() {
                for vertex in mesh.vertices() {
                    writeln!(text, "  {} {} {}", vertex.position, vertex.normal, vertex.uv).unwrap();
                }

                for index in mesh.triangles() {
                    writeln!(text, "   {index}").unwrap();
                }
            }
        }
    }

    text
}

#[test]
fn req_arch_012_two_identical_builds_give_identical_geometry() {
    assert_eq!(build_all(), build_all());
}

#[test]
fn q_013_rebuilding_a_cylinder_does_not_shrink_it_again() {
    // REQ-E3D-452 writes D' back into sizeZ; a rebuild must still start from the depth the
    // profile asked for, or every algorithm change would thin the object down.
    let mut engine = geometry_engine();

    let mut profile = ProfileBuilder::new();
    profile.pattern_with(1, 1, &[Some(img::pixel(0, 0, false, false))], with_depth(8));
    profile.variant(0, 0).geometry = VariantGeometry::VerticalCylinder;
    engine.load_profile(&profile.build(), true).unwrap();

    let reference = instance_ref(&engine, 0, 0);
    let built = builder(&engine).of(reference).unwrap();
    assert_eq!(8, built.requested_depth());
    assert_eq!(1, built.effective_depth());
    assert_eq!(1, engine.patterns().instance(reference).unwrap().depth);

    engine.set_render_algorithm(RenderAlgorithm::Marching);
    let rebuilt = builder(&engine).of(reference).unwrap();
    assert_eq!(8, rebuilt.requested_depth());
    assert_eq!(1, rebuilt.effective_depth());

    // A BuildPattern hook that moves sizeZ still wins (REQ-E3D-455, REQ-E3D-322).
    engine.set_script_host(Box::new(RecordingScriptHost {
        on_build_pattern: Some(Box::new(|instance| instance.depth = 4)),
        ..RecordingScriptHost::default()
    }));
    engine.set_render_algorithm(RenderAlgorithm::Greedy);
    assert_eq!(4, builder(&engine).of(reference).unwrap().requested_depth());
}
