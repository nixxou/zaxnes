//! The test vectors of SPEC-04B §5.5 — placement, colour and deformation — and the output
//! contract of §3.8, driven end to end through the pipeline of SPEC-04.

use std::collections::HashSet;

use zax_engine3d::draw_list::{DrawTile, FrameDrawList, MaterialClass, SceneContainer};
use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::grid;
use zax_engine3d::image::TileImageKey;
use zax_engine3d::instance::{PatternInstance, RenderStyle};
use zax_engine3d::palette::{nes_color, Rgb24};
use zax_engine3d::settings::{RenderAlgorithm, ZAlignment, ZLayer};
use zax_engine3d::vectors::{Mat4, Vec2, Vec3};
use zax_profiles::enums::{ProfileLayer, TextureFormat};
use zax_profiles::profile::GameProfile;
use zax_profiles::variant::{TexturePixels, VariantTexture};

use super::support::geo::{self, close, rectangles};
use super::support::{assert_close, cells, img, permanent, Batch, PatternOptions, ProfileBuilder};

fn profile(key: TileImageKey, depth: i32, layer: ProfileLayer, tags: Option<&str>) -> GameProfile {
    ProfileBuilder::new()
        .pattern_with(
            1,
            1,
            &[Some(key)],
            PatternOptions {
                depth,
                layer,
                tags: tags.map(str::to_owned),
                ..PatternOptions::default()
            },
        )
        .build()
}

fn engine_with(depth: i32, layer: ProfileLayer, tags: Option<&str>) -> ConversionEngine {
    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.load_profile(&profile(img::a(), depth, layer, tags), true).unwrap();
    engine
}

fn engine(depth: i32) -> ConversionEngine {
    engine_with(depth, ProfileLayer::L1, None)
}

fn reference(engine: &mut ConversionEngine) -> &mut PatternInstance {
    let id = permanent(engine, 0);
    &mut engine.pattern_mut(id).unwrap().instances_mut()[0]
}

fn draw_batch(engine: &mut ConversionEngine, batch: &Batch, counter: i32) -> FrameDrawList {
    batch.convert(engine, counter);
    engine.build_draw_list_of(FrameRole::Converting).clone()
}

fn draw(engine: &mut ConversionEngine, image: TileImageKey, x: i32, y: i32) -> FrameDrawList {
    let mut batch = Batch::new();
    batch.bg(image, x, y, 0);
    draw_batch(engine, &batch, 0)
}

fn single(list: &FrameDrawList) -> &DrawTile {
    assert_eq!(1, list.tiles().len());
    &list.tiles()[0]
}

fn bottom(py: i32) -> f32 {
    (grid::SCENE_HEIGHT - py - 1) as f32
}

fn top(py: i32) -> f32 {
    (grid::SCENE_HEIGHT - py) as f32
}

fn assert_scene_box(tile: &DrawTile, min: (f32, f32, f32), max: (f32, f32, f32)) {
    let mesh: &TileMesh = tile.mesh.as_ref().unwrap();
    let mut low = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut high = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    for vertex in mesh.vertices() {
        let scene = tile.transform.transform_point(vertex.position);
        low = Vec3::new(low.x.min(scene.x), low.y.min(scene.y), low.z.min(scene.z));
        high = Vec3::new(high.x.max(scene.x), high.y.max(scene.y), high.z.max(scene.z));
    }

    assert_close(min.0, low.x, 3);
    assert_close(max.0, high.x, 3);
    assert_close(min.1, low.y, 3);
    assert_close(max.1, high.y, 3);
    assert_close(min.2, low.z, 3);
    assert_close(max.2, high.z, 3);
}

#[test]
fn req_e3d_410_the_scene_frame_places_the_nes_image_in_y_from_eight_to_two_hundred_forty_eight() {
    // Pixel (0, 0) occupies y in [247, 248], pixel (0, 239) y in [8, 9].
    assert_eq!(248, grid::SCENE_HEIGHT);
    assert_eq!(247.0, bottom(0));
    assert_eq!(248.0, top(0));
    assert_eq!(8.0, bottom(239));
    assert_eq!(9.0, top(239));

    // The geometric centre of the visible image is (128, 128); the reference frame is centred
    // on C = (128, 120, 0), eight units below it.
    assert_eq!(128.0, (bottom(239) + top(0)) / 2.0);
    assert_eq!(120.0, 240.0 / 2.0);

    // Inverse relation: a point (x, y) of the plane z = 0 shows the pixel
    // (floor x, floor (Hs − y)).
    assert_eq!(239.0, (grid::SCENE_HEIGHT as f32 - 8.5).floor());
    assert_eq!(0.0, (grid::SCENE_HEIGHT as f32 - 247.5).floor());
}

#[test]
fn tv_w1_a_background_tile_is_placed_in_front_of_its_layer() {
    // REQ-E3D-403, 404, 405, 406.
    let mut engine = engine(4);
    let list = draw(&mut engine, img::a(), 16, 32);

    let tile = single(&list);
    assert_eq!(Vec3::new(20.0, 212.0, 2.0), tile.anchor);
    assert_eq!(Vec3::new(20.0, 212.0, 2.0), tile.tile_center);
    assert_scene_box(tile, (16.0, 208.0, 0.0), (24.0, 216.0, 4.0));
    assert_eq!(6, rectangles(tile.mesh.as_ref().unwrap()));

    // In L2 the whole object moves twelve units away.
    let mut second = engine_with(4, ProfileLayer::L2, None);
    assert_scene_box(single(&draw(&mut second, img::a(), 16, 32)), (16.0, 208.0, 12.0), (24.0, 216.0, 16.0));

    // CENTER and BACK alignments, with a zero layer offset.
    engine.settings_mut().layer_mut(ZLayer::L1).alignment = ZAlignment::Center;
    assert_scene_box(single(&draw(&mut engine, img::a(), 16, 32)), (16.0, 208.0, -2.0), (24.0, 216.0, 2.0));

    engine.settings_mut().layer_mut(ZLayer::L1).alignment = ZAlignment::Back;
    assert_scene_box(single(&draw(&mut engine, img::a(), 16, 32)), (16.0, 208.0, -4.0), (24.0, 216.0, 0.0));
    assert_eq!(1, engine.geometry_builder().unwrap().object_count());
}

#[test]
fn tv_w2_a_sprite_tile_gets_the_same_transform_and_the_nudge_flags() {
    // REQ-E3D-405, DEC-R2-T4: the ±0.1 nudge is the renderer's job, not ours.
    let mut engine = engine(4);

    let mut background = Batch::new();
    background.bg(img::a(), 16, 32, 0);
    let background_list = draw_batch(&mut engine, &background, 0);
    let background_tile = single(&background_list);

    let mut sprite = Batch::new();
    sprite.sprite(img::a(), 16, 32);
    let sprite_list = draw_batch(&mut engine, &sprite, 2);
    let sprite_tile = single(&sprite_list);

    assert_eq!(background_tile.transform, sprite_tile.transform);
    assert!(sprite_tile.sprite);
    assert!(!sprite_tile.behind_background);
    assert!(!background_tile.sprite);
    assert_scene_box(sprite_tile, (16.0, 208.0, 0.0), (24.0, 216.0, 4.0));

    let mut behind = Batch::new();
    behind.sprite_full(img::a(), 16, 32, 0, true);
    let behind_list = draw_batch(&mut engine, &behind, 3);
    let hidden_tile = single(&behind_list);
    assert!(hidden_tile.sprite);
    assert!(hidden_tile.behind_background);
}

#[test]
fn tv_w3_scale_pivot_and_rotation_act_around_the_anchor() {
    // REQ-E3D-406, 407.
    let mut engine = engine(4);

    reference(&mut engine).scale = Vec3::new(2.0, 1.0, 1.0);
    assert_scene_box(single(&draw(&mut engine, img::a(), 16, 32)), (12.0, 208.0, 0.0), (28.0, 216.0, 4.0));

    reference(&mut engine).pivot = Vec3::ZERO;
    let list = draw(&mut engine, img::a(), 16, 32);
    let shifted = single(&list);
    assert_eq!(Vec3::new(16.0, 208.0, 0.0), shifted.anchor);
    assert_scene_box(shifted, (16.0, 208.0, 0.0), (32.0, 216.0, 4.0));

    // A quarter turn about z keeps the square where it is and moves its pixel (0, 0) from the
    // top left corner to the bottom left one.
    let instance = reference(&mut engine);
    instance.scale = Vec3::ONE;
    instance.pivot = Vec3::HALF;
    instance.rotation = Vec3::new(0.0, 0.0, 90.0);
    let list = draw(&mut engine, img::a(), 16, 32);
    let turned = single(&list);
    assert_scene_box(turned, (16.0, 208.0, 0.0), (24.0, 216.0, 4.0));

    // The local corner of the pixel (0, 0) is (−4, 4); a 90° turn sends it to (−4, −4).
    let corner = turned.transform.transform_point(Vec3::new(-4.0, 4.0, 0.0));
    assert_close(16.0, corner.x, 3);
    assert_close(208.0, corner.y, 3);

    // REQ-E3D-407: rot = (0, 0, 90) sends the relative vector (8, 0, 0) to (0, 8, 0).
    let turned_vector = Mat4::rotation_z(90.0).transform_point(Vec3::new(8.0, 0.0, 0.0));
    assert_close(0.0, turned_vector.x, 3);
    assert_close(8.0, turned_vector.y, 3);
}

#[test]
fn req_e3d_407_the_gaxis_tag_swaps_the_first_two_rotations() {
    // Without _gaxis the rotation is Rx·Ry·Rz, with it Ry·Rx·Rz: (0, 0, 1) goes to (1, 0, 0)
    // or to (0, −1, 0).
    let mut plain = engine(4);
    reference(&mut plain).rotation = Vec3::new(90.0, 90.0, 0.0);
    let without_tag = single(&draw(&mut plain, img::a(), 16, 32)).transform;

    let mut tagged = engine_with(4, ProfileLayer::L1, Some("_gaxis"));
    reference(&mut tagged).rotation = Vec3::new(90.0, 90.0, 0.0);
    let with_tag = single(&draw(&mut tagged, img::a(), 16, 32)).transform;

    let anchor = Vec3::new(20.0, 212.0, 2.0);
    let first = without_tag.transform_point(Vec3::new(0.0, 0.0, 1.0) + anchor - Vec3::new(20.0, 212.0, 2.0))
        - without_tag.transform_point(Vec3::ZERO);
    let second = with_tag.transform_point(Vec3::new(0.0, 0.0, 1.0)) - with_tag.transform_point(Vec3::ZERO);

    assert_close(1.0, first.x, 3);
    assert_close(0.0, first.y, 3);
    assert_close(0.0, first.z, 3);

    assert_close(0.0, second.x, 3);
    assert_close(-1.0, second.y, 3);
    assert_close(0.0, second.z, 3);
}

#[test]
fn tv_w4_marching_shrinks_the_scale_back_to_the_pixel_box() {
    // REQ-E3D-408: a one-pixel shape with D = 1 shows the octahedron of radius 1 at radius
    // 0.5, centred on the anchor.
    let mut engine = engine(1);
    engine.settings_mut().render_algorithm = RenderAlgorithm::Marching;
    let pixel = img::pixel(0, 0, false, false);
    engine.load_profile(&profile(pixel, 1, ProfileLayer::L1, None), true).unwrap();

    let list = draw(&mut engine, pixel, 16, 32);
    let tile = single(&list);

    assert_eq!(Vec3::new(16.5, 215.5, 0.5), tile.anchor);
    assert_scene_box(tile, (16.0, 215.0, 0.0), (17.0, 216.0, 1.0));
    let mesh = tile.mesh.as_ref().unwrap();
    assert_eq!(8, mesh.triangle_count());

    // The octahedron is centred on the anchor, with a radius of 0.5.
    let centre = Vec3::new(16.5, 215.5, 0.5);
    for vertex in mesh.vertices() {
        let scene = tile.transform.transform_point(vertex.position);
        assert_close(0.5, Vec3::distance(scene, centre), 3);
    }
}

fn identity_palette_batch() -> Batch {
    let mut batch = Batch::new();
    batch.palette(0, 0x0F);
    for entry in 1..32u8 {
        batch.palette(usize::from(entry), entry);
    }
    batch
}

#[test]
fn tv_k1_indexed_colours_come_from_the_frame_palette_table() {
    // REQ-E3D-560, 561: colour = NES_RGB[pal[4·P + (t mod 4)]].
    let mut engine = engine(4);

    let mut batch = identity_palette_batch();
    batch.bg(img::a(), 16, 32, 1).sprite_full(img::a(), 64, 64, 1, false);
    let list = draw_batch(&mut engine, &batch, 0);

    assert_eq!(2, list.tiles().len());
    let background = &list.tiles()[0];
    let sprite = &list.tiles()[1];

    assert_eq!(1, background.palette_table);
    assert_eq!(5, sprite.palette_table);
    let entry = |table: i32, column: i32| usize::try_from((4 * table) + column).unwrap();

    // Background tile of palette 1, texel 2 -> core[6]; the same sprite -> core[22].
    assert_eq!(nes_color(6), list.palette()[entry(background.palette_table, 2)]);
    assert_eq!(nes_color(22), list.palette()[entry(sprite.palette_table, 2)]);

    // Texel 0 is the universal background colour in every table.
    assert_eq!(nes_color(0x0F), list.palette()[entry(background.palette_table, 0)]);
    assert_eq!(nes_color(0x0F), list.palette()[entry(sprite.palette_table, 0)]);

    // A custom palette of 9 reaches the same colour as the table 1 (REQ-E3D-561). It is set
    // on the frame instance: a pattern instance's value is reset every frame (REQ-E3D-190).
    let first = batch.convert(&mut engine, 2).instances()[0];
    engine.converting_frame_mut().instance_mut(first).custom_palette = 9;
    let custom = engine.build_draw_list_of(FrameRole::Converting);
    assert_eq!(9, custom.tiles()[0].palette_table);
    assert_eq!(nes_color(6), custom.palette()[entry(9, 2)]);
}

#[test]
fn req_e3d_560_the_palette_table_is_the_one_of_each_tile_not_of_the_shape() {
    // REQ-E3D-560 and REQ-SCN-034: "la palette est celle de chaque tuile de la frame". A
    // background shape that spans two attribute quads therefore keeps the colours of both,
    // instead of taking the germ's palette for all of its tiles (REQ-E3D-047).
    let mut engine = engine(4);

    // Four neighbouring tiles of the same image: two of attribute palette 0, two of 1.
    let mut batch = identity_palette_batch();
    batch
        .bg(img::a(), 16, 32, 0)
        .bg(img::a(), 24, 32, 0)
        .bg(img::a(), 32, 32, 1)
        .bg(img::a(), 40, 32, 1);

    let list = draw_batch(&mut engine, &batch, 0);

    assert_eq!(4, list.tiles().len());
    let tables: Vec<i32> = list.tiles().iter().map(|tile| tile.palette_table).collect();
    assert_eq!(vec![0, 0, 1, 1], tables);

    // The tiles belong to one single shape, whose own palette is the germ's.
    let shapes: HashSet<_> = list.tiles().iter().map(|tile| tile.instance.shape).collect();
    assert_eq!(1, shapes.len());
    let first = list.tiles()[0].instance;
    assert_eq!(0, engine.converting_frame().instance(first).palette_table());
}

#[test]
fn req_e3d_560_a_custom_palette_overrides_every_tile_of_the_shape() {
    let mut engine = engine(4);

    let mut batch = Batch::new();
    batch.palette(0, 0x0F);
    batch.bg(img::a(), 16, 32, 0).bg(img::a(), 24, 32, 1);

    let first = batch.convert(&mut engine, 0).instances()[0];
    engine.converting_frame_mut().instance_mut(first).custom_palette = 9;
    let list = engine.build_draw_list_of(FrameRole::Converting);

    let tables: Vec<i32> = list.tiles().iter().map(|tile| tile.palette_table).collect();
    assert_eq!(vec![9, 9], tables);
}

#[test]
fn req_e3d_562_a_direct_texture_freezes_its_zero_texels_when_it_is_built() {
    let pattern = geo::one_tile(geo::solid(1));
    let texture = VariantTexture::new(
        2,
        1,
        Some(TexturePixels::new(TextureFormat::Direct, vec![0x00FF_0000, 0], Vec::new())),
    );

    let resolved = ObjectTexture::for_object(&pattern, Some(&texture), Rgb24::new(1, 2, 3));
    assert_eq!(TextureFormat::Direct, resolved.format());
    assert_eq!(Rgb24::new(0xFF, 0, 0), resolved.color(0, 0));
    assert_eq!(Rgb24::new(1, 2, 3), resolved.color(1, 0));

    // Changing the background colour afterwards does not move the texels already loaded. The
    // value used before the first frame is the builder's (Q-017).
    let later = ObjectTexture::for_object(&pattern, Some(&texture), Rgb24::new(9, 9, 9));
    assert_eq!(Rgb24::new(1, 2, 3), resolved.color(1, 0));
    assert_eq!(Rgb24::new(9, 9, 9), later.color(1, 0));

    // REQ-E3D-424: with a direct texture only the texel whose integer value is exactly e is
    // transparent.
    assert_eq!(0, resolved.texel(1, 0));
    assert_eq!(0x00FF_0000, resolved.texel(0, 0));
}

#[test]
fn req_e3d_563_the_texture_repeats_outside_its_bounds() {
    // DEC-04B-Q3: every read is modulo the texture size.
    let pattern = geo::one_tile(geo::tile(&["123....."]));
    let texture = ObjectTexture::from_pattern(&pattern);

    assert_eq!(texture.texel(0, 0), texture.texel(8, 0));
    assert_eq!(texture.texel(1, 0), texture.texel(-7, 0));
    assert_eq!(texture.texel(2, 3), texture.texel(2, 11));
}

#[test]
fn tv_d1_the_deformation_is_a_standing_wave_across_the_shape() {
    // REQ-E3D-568, 641.
    let mut profile = ProfileBuilder::new()
        .pattern_with(
            2,
            2,
            &cells(&[img::a(), img::b(), img::c(), img::d()]),
            PatternOptions {
                depth: 4,
                ..PatternOptions::default()
            },
        )
        .build();
    profile.version_text = Some("2".to_owned());
    assert!(profile.is_v2_format());

    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.load_profile(&profile, true).unwrap();

    let instance = reference(&mut engine);
    instance.deform_amplitude = Vec3::new(0.0, 0.1, 0.0);
    instance.deform_speed[(1, 0)] = 2.0 * std::f32::consts::PI;

    let mut batch = Batch::new();
    batch
        .bg(img::a(), 0, 0, 0)
        .bg(img::b(), 8, 0, 0)
        .bg(img::c(), 0, 8, 0)
        .bg(img::d(), 8, 8, 0);
    let list = draw_batch(&mut engine, &batch, 0);

    assert_eq!(4, list.tiles().len());
    assert_eq!(0.0, list.time());

    let right = list
        .tiles()
        .iter()
        .find(|tile| tile.tile.x() == 8 && tile.tile.y() == 0)
        .unwrap();
    let deform = right.deform.unwrap();

    // sz = (16, 16, 4) and Pd = (8, 241, 0), so the tile sits at (4, 3, 0) from the pivot.
    assert_eq!(Vec3::new(16.0, 16.0, 4.0), deform.size);
    assert_eq!(Vec3::new(4.0, 3.0, 0.0), deform.tile_relative_to_pivot);

    // nrm.x = 0.25 -> no displacement; 0 -> +1.6; 0.5 -> −1.6.
    assert_close(0.0, deform.displacement(Vec3::new(0.0, 0.0, 0.0), 0.0).y, 3);
    assert_close(1.6, deform.displacement(Vec3::new(-4.0, 0.0, 0.0), 0.0).y, 3);
    assert_close(-1.6, deform.displacement(Vec3::new(4.0, 0.0, 0.0), 0.0).y, 3);
    assert_close(0.0, deform.displacement(Vec3::new(0.0, 0.0, 0.0), 0.0).x, 3);
    assert_close(0.0, deform.displacement(Vec3::new(0.0, 0.0, 0.0), 0.0).z, 3);
}

#[test]
fn req_e3d_568_the_default_deformation_is_a_constant_translation() {
    // Zero speeds and pi/2 phases give delta = deltaAmp ⊙ sz.
    let mut engine = engine(4);
    reference(&mut engine).deform_amplitude = Vec3::new(0.25, 0.5, 1.0);

    let list = draw(&mut engine, img::a(), 16, 32);
    let deform = single(&list).deform.unwrap();
    let delta = deform.displacement(Vec3::ZERO, 1.25);

    assert_close(0.25 * 8.0, delta.x, 3);
    assert_close(0.5 * 8.0, delta.y, 3);
    assert_close(4.0, delta.z, 3);
}

#[test]
fn req_e3d_569_the_free_script_row_never_moves_a_vertex() {
    let mut engine = engine(4);
    reference(&mut engine).deform_amplitude = Vec3::new(0.0, 0.1, 0.0);

    let before = single(&draw(&mut engine, img::a(), 16, 32)).deform.unwrap();
    let plain = before.displacement(Vec3::new(1.0, 2.0, 0.0), 0.5);

    for index in 0..8u8 {
        reference(&mut engine).set_custom_data(usize::from(index), 3.5 + f32::from(index));
    }

    let after = single(&draw(&mut engine, img::a(), 16, 32)).deform.unwrap();
    let with_data = after.displacement(Vec3::new(1.0, 2.0, 0.0), 0.5);

    assert_close(plain.x, with_data.x, 5);
    assert_close(plain.y, with_data.y, 5);
    assert_close(plain.z, with_data.z, 5);

    for index in 0..8u8 {
        assert_eq!(3.5 + f32::from(index), reference(&mut engine).custom_data(usize::from(index)));
    }
}

#[test]
fn req_e3d_566_the_material_class_follows_alpha_and_the_render_style() {
    let mut engine = engine(4);

    assert_eq!(MaterialClass::Opaque, single(&draw(&mut engine, img::a(), 16, 32)).material);

    reference(&mut engine).alpha = 0.5;
    let list = draw(&mut engine, img::a(), 16, 32);
    let transparent = single(&list);
    assert_eq!(MaterialClass::Transparent, transparent.material);
    assert_eq!(0.5, transparent.alpha);
    assert!(!transparent.cast_shadow);

    reference(&mut engine).style = RenderStyle::ShadowOnly;
    let list = draw(&mut engine, img::a(), 16, 32);
    let shadow_only = single(&list);
    assert_eq!(MaterialClass::ShadowOnly, shadow_only.material);
    assert!(shadow_only.cast_shadow);

    reference(&mut engine).style = RenderStyle::Hidden;
    assert!(draw(&mut engine, img::a(), 16, 32).tiles().is_empty());
}

#[test]
fn req_e3d_640_the_output_is_platform_free_data_with_one_submesh() {
    let mut engine = engine(4);
    let list = draw(&mut engine, img::a(), 16, 32);
    let tile = single(&list);
    let mesh = tile.mesh.as_ref().unwrap();

    assert_eq!(1, TileMesh::SUBMESH_COUNT);
    assert_eq!(0, mesh.triangles().len() % 3);
    assert_eq!(mesh.triangle_count() * 3, mesh.triangles().len());
    assert!(close(Vec3::new(-4.0, -4.0, -2.0), mesh.bounds_min()));
    assert!(close(Vec3::new(4.0, 4.0, 2.0), mesh.bounds_max()));

    // The texture reference and its format reach the renderer (REQ-E3D-641).
    let texture = tile.texture.as_ref().unwrap();
    assert_eq!(TextureFormat::Indexed, texture.format());
    assert_eq!(8, texture.width());
    assert_eq!(8, texture.height());
    assert_eq!(SceneContainer::Game, tile.container);
    assert_eq!(Vec2::ZERO, tile.tex_offset);
    assert_eq!(Vec2::ONE, tile.tex_tilting);
}
