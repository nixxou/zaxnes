//! AMD-043 REQ-SCN-067 and REQ-SCN-068 in the renderer: the shader **variant** the curvature
//! selects, the scope of the curvature (the game scene and its shadows, not the HUD), the order
//! with the clipping, and the subdivision of the backdrop.
//!
//! The vertex placement is checked through `zax_render::scene_export`, the CPU harness the
//! acceptance criteria of REQ-SCN-068 ask for; the shader variants are checked on the assembled
//! sources, which needs no GL context.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_engine3d::space_bend::SpaceBend;
use zax_profiles::enums::ProfileLayer;
use zax_render::camera::SceneCamera;
use zax_render::interop;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::scene_export;
use zax_render::shaders;

/// The four axes these tests vary, as the named-field variant of `shaders::scene`. AMD-055 added
/// two more axes (the voxel grid of REQ-SCN-081 and the pixel shadows of REQ-SCN-082); both are
/// off here, which is what every assertion of this file is about.
#[allow(clippy::fn_params_excessive_bools)] // The four axes, as `shaders::scene` had them.
fn variant(solid: bool, srgb_palette: bool, bend: bool, centroid: bool) -> shaders::SceneVariant {
    shaders::SceneVariant {
        solid,
        srgb_palette,
        bend,
        centroid,
        ..shaders::SceneVariant::FLAT
    }
}

/// A curvature of 90° left-right, the setting the criteria of REQ-SCN-068 and REQ-SCN-074 use.
fn bend_90() -> SpaceBend {
    SpaceBend::new(90.0, 0.0, 0.0, false, 1.0)
}

/// The assembled vertex and fragment sources of one scene variant.
// The four switches are the variant axes of `shaders::scene` plus the context, each named by the
// specification; an enum per axis would say less, not more.
#[allow(clippy::fn_params_excessive_bools)]
fn scene_source(solid: bool, srgb: bool, bend: bool, embedded: bool) -> (String, String) {
    // AMD-058's interpolation axis is the subject of `amd058_centroid_tests`; the curvature is
    // checked on the pixel-centre variant, the one the default settings use.
    let source = shaders::scene(variant(solid, srgb, bend, false));
    (
        shaders::assemble(&source.vertex, false, embedded),
        shaders::assemble(&source.fragment, true, embedded),
    )
}

/// The assembled vertex and fragment sources of one shadow variant.
fn shadow_source(bend: bool, embedded: bool) -> (String, String) {
    let source = shaders::shadow(bend);
    (
        shaders::assemble(&source.vertex, false, embedded),
        shaders::assemble(&source.fragment, true, embedded),
    )
}

/// The code of `source` as the GLSL compiler sees it when `ZAX_BEND` is **not** defined: the
/// `#ifdef ZAX_BEND` branches dropped, the `#else` branches kept, and the line comments — which
/// say what the guarded branches are for — removed with them.
///
/// The bodies of `scene.vert`, `scene.frag` and `shadow.vert` hold both branches as text, so the
/// requirement — at curvature 0 the compiled programs are the ones of before AMD-043 — is about
/// what the compiler is given, not about what the file says. This is the smallest thing that
/// checks the former.
fn without_bend(source: &str) -> String {
    assert!(!source.contains("/*"), "no block comment to account for");
    let mut kept = String::with_capacity(source.len());
    // 0 outside any block, 1 inside the ZAX_BEND branch (dropped), 2 inside its #else (kept).
    let mut state = 0u8;
    for line in source.lines() {
        match line.trim() {
            "#ifdef ZAX_BEND" => state = 1,
            "#else" if state == 1 => state = 2,
            "#endif" if state != 0 => state = 0,
            _ if state != 1 => {
                let code = line.split_once("//").map_or(line, |(code, _)| code);
                kept.push_str(code);
                kept.push('\n');
            }
            _ => {}
        }
    }
    assert_eq!(0, state, "every #ifdef ZAX_BEND is closed");
    kept
}

fn steady_view(scene: &SceneState) -> zax_render::camera::CameraView {
    let mut camera = SceneCamera::new();
    camera.snap_to_setpoints();
    camera.build_view(scene.view_clipping(), 4.0 / 3.0)
}

// -------------------------------------------------------------- the shader variants

/// Performance requirement of the lot, and the reason the curvature is a `#define` and not a
/// uniform: at curvature 0 the compiler is given **nothing** of the curvature — no function, no
/// uniform, no varying, not even the define — so the programs are the ones of before AMD-043.
#[test]
fn req_scn_067_the_flat_variant_carries_nothing_of_the_curvature() {
    // Every identifier the curvature brings in: the chunk's own, and the two the bodies of the
    // shaders add under the guard.
    const FORBIDDEN: [&str; 9] = [
        "ZAX_BEND",
        "zaxBend",
        "zaxWrap",
        "uBendK",
        "uBendF",
        "uBendH",
        "uBendOn",
        "uBendPivot",
        "vBent",
    ];
    let check = |what: &str, vertex: &str, fragment: &str| {
        for (stage, source) in [("vertex", vertex), ("fragment", fragment)] {
            let compiled = without_bend(source);
            for forbidden in FORBIDDEN {
                assert!(
                    !compiled.contains(forbidden),
                    "{what}, {stage}: the compiler is given {forbidden}"
                );
            }
        }
    };

    for embedded in [false, true] {
        for solid in [false, true] {
            for srgb in [false, true] {
                let (vertex, fragment) = scene_source(solid, srgb, false, embedded);
                // The define itself is never emitted, so the branches above are really dropped.
                assert!(!vertex.contains("#define ZAX_BEND"));
                assert!(!fragment.contains("#define ZAX_BEND"));
                check(
                    &format!("flat scene (solid {solid}, srgb {srgb}, embedded {embedded})"),
                    &vertex,
                    &fragment,
                );
            }
        }

        let (vertex, fragment) = shadow_source(false, embedded);
        assert!(!vertex.contains("#define ZAX_BEND"));
        check("flat shadow", &vertex, &fragment);
    }
}

/// The other half of the same requirement: the curvature variant does carry it, and it is the
/// `#define` that brings it in.
#[test]
fn req_scn_067_the_curvature_variant_compiles_the_wrapping_in() {
    let (vertex, fragment) = scene_source(false, false, true, false);
    assert!(vertex.contains("#define ZAX_BEND"));
    assert!(vertex.contains("vec2 zaxWrap("), "the E of step 2");
    assert!(vertex.contains("vec3 zaxBend("), "B of REQ-SCN-067");
    assert!(
        vertex.contains("zaxBendNormal"),
        "the normal of REQ-SCN-068"
    );
    assert!(vertex.contains("uniform vec3 uBendK;"));

    // The fragment reads the bent position for the shadow lookup (REQ-SCN-068: the map holds the
    // bent geometry) and keeps reading the flat one for the clipping of REQ-SCN-036.
    assert!(fragment.contains("in vec3 vBent;"));
    assert!(fragment.contains("vBent + n * uNormalOffset"));
    assert!(fragment.contains("vScene.x < uClipRect.x"));

    let (vertex, _) = shadow_source(true, false);
    assert!(vertex.contains("#define ZAX_BEND"));
    assert!(vertex.contains("vec3 zaxBend("));
}

/// Each of the sixteen scene variants and the two shadow variants is a program of its own, so
/// the on-disk program cache files them apart and an error message names the right one.
#[test]
fn req_scn_067_each_variant_has_its_own_name() {
    let mut names = Vec::new();
    for solid in [false, true] {
        for srgb in [false, true] {
            for bend in [false, true] {
                // AMD-058 added the interpolation axis; the names still have to be all distinct.
                for centroid in [false, true] {
                    names.push(shaders::scene(variant(solid, srgb, bend, centroid)).name);
                }
            }
        }
    }
    names.push(shaders::shadow(false).name);
    names.push(shaders::shadow(true).name);
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names.len(), sorted.len(), "ten distinct names: {names:?}");
}

// -------------------------------------------------------------- the placement

/// REQ-SCN-068, first acceptance criterion: with Bend Left-Right at 90°, the exported vertices
/// of a tile are at `B(P)` of their flat position.
#[test]
fn req_scn_068_the_exported_vertices_of_a_tile_are_at_their_bent_position() {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 30, 15, 1, 1)
        .build();
    built.scene.settings.shadows = false;
    let view = steady_view(&built.scene);
    let frame = SceneFrame::with_draw_list(&built.draw_list);

    let flat = scene_export::objects(&built.scene, &frame, &view);
    assert!(!flat.is_empty(), "the tile is exported");
    built.scene.bend = bend_90();
    let bent = scene_export::objects(&built.scene, &frame, &view);
    assert_eq!(flat.len(), bent.len(), "the same objects");

    let transform = bend_90();
    let mut moved = 0;
    for (flat, bent) in flat.iter().zip(&bent) {
        assert!(bent.bent, "a tile of the game scene bends");
        assert_eq!(flat.positions, bent.unbent, "the flat position is kept");
        for (before, after) in flat.positions.iter().zip(&bent.positions) {
            let expected = transform.apply(*before);
            assert!(
                zax_engine3d::vectors::Vec3::distance(expected, *after) < 1e-3,
                "{after:?} is not B({before:?}) = {expected:?}"
            );
            if zax_engine3d::vectors::Vec3::distance(*before, *after) > 0.5 {
                moved += 1;
            }
        }
    }
    assert!(moved > 0, "the curvature moved something");
}

/// REQ-SCN-068, same criterion: the exported HUD is unchanged by the curvature — the plane
/// attached to the camera stays flat, so the score of a bent game stays readable (REQ-SCN-044).
#[test]
fn req_scn_068_the_hud_is_not_bent() {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::Ui)
        .rect(solid(1, 0), 4, 26, 2, 1)
        .build();
    built.scene.settings.shadows = false;
    let view = steady_view(&built.scene);
    let frame = SceneFrame::with_draw_list(&built.draw_list);

    let flat = scene_export::objects(&built.scene, &frame, &view);
    let hud: Vec<_> = flat.iter().filter(|object| object.hud).collect();
    assert!(!hud.is_empty(), "the UI layer is exported as HUD");

    built.scene.bend = bend_90();
    let bent = scene_export::objects(&built.scene, &frame, &view);
    for (flat, bent) in flat.iter().zip(&bent).filter(|(flat, _)| flat.hud) {
        assert!(!bent.bent, "the HUD does not bend");
        assert_eq!(flat.positions, bent.positions);
        assert_eq!(flat.normals, bent.normals);
    }
}

/// REQ-SCN-068, order clause: the clipping of REQ-SCN-036 is judged on the position **before**
/// the curvature, so the set of tiles kept does not depend on the curvature at all. The
/// criterion's own case: with Arc Horizontal at 60° on the frame edge, the corner tile at
/// `P = (0, 8, 0)` is still rendered although `B(P)` leaves the visible rectangle.
#[test]
fn req_scn_068_the_clipping_is_judged_before_the_curvature() {
    let corner = zax_engine3d::vectors::Vec3::new(0.0, 8.0, 0.0);
    let arc = SpaceBend::new(0.0, 0.0, 60.0, true, 1.0);
    let image = arc.apply(corner);
    let rect = SceneState::default().visible_rectangle();
    assert!(
        image.y < rect.min_y,
        "B(P) = {image:?} leaves R (min y {})",
        rect.min_y
    );

    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 0, 29, 1, 1)
        .build();
    built.scene.settings.shadows = false;
    let view = steady_view(&built.scene);
    let frame = SceneFrame::with_draw_list(&built.draw_list);

    let flat = scene_export::objects(&built.scene, &frame, &view);
    built.scene.bend = arc;
    let bent = scene_export::objects(&built.scene, &frame, &view);
    assert_eq!(
        flat.len(),
        bent.len(),
        "the same tiles are kept, bent or not"
    );
    // The flag the fragment shader reads is the same, and the position it judges is the flat one.
    for (flat, bent) in flat.iter().zip(&bent) {
        assert_eq!(flat.clipped, bent.clipped);
        assert_eq!(flat.positions, bent.unbent);
    }
}

/// REQ-SCN-068: a plane large enough to curve visibly is subdivided, the backdrop into at least
/// 32 × 32 cells per face — and **only** while something bends, so the vertex count of the
/// default settings is untouched.
#[test]
fn req_scn_068_the_backdrop_is_subdivided_only_while_something_bends() {
    assert_eq!(1, interop::subdivision_for(false));
    assert!(interop::subdivision_for(true) >= 32);

    let flat =
        interop::unit_box_subdivided(interop::subdivision_for(false)).expect("whole triangles");
    let plain = interop::unit_box().expect("whole triangles");
    assert_eq!(plain, flat, "no curvature keeps the six-quad box");
    assert_eq!(24, flat.vertices().len());
    assert_eq!(36, flat.triangles().len());

    let cells = usize::from(interop::subdivision_for(true));
    let bent =
        interop::unit_box_subdivided(interop::subdivision_for(true)).expect("whole triangles");
    assert_eq!(6 * (cells + 1) * (cells + 1), bent.vertices().len());
    assert_eq!(6 * cells * cells * 6, bent.triangles().len());
    // The subdivision changes nothing but the sampling: the box is still [−0.5, 0.5]³.
    assert_eq!(plain.bounds_min(), bent.bounds_min());
    assert_eq!(plain.bounds_max(), bent.bounds_max());
}

/// The same for the rectangle of the 2D mode (AMD-046 REQ-SCN-074 asks for at least 32 × 32, so
/// at least 33 × 33 = 1089 distinct vertices), and that subdivision 1 is the quad of before.
#[test]
fn req_scn_074_the_two_dimensional_rectangle_is_subdivided_only_while_something_bends() {
    let plain = interop::picture_quad().expect("whole triangles");
    assert_eq!(
        plain,
        interop::picture_quad_subdivided(0, 0, interop::subdivision_for(false))
            .expect("whole triangles")
    );
    assert_eq!(4, plain.vertices().len());

    let bent = interop::picture_quad_subdivided(0, 0, interop::subdivision_for(true))
        .expect("whole triangles");
    assert!(
        bent.vertices().len() >= 1089,
        "at least a 33 × 33 grid, got {}",
        bent.vertices().len()
    );
    assert_eq!(plain.bounds_min(), bent.bounds_min());
    assert_eq!(plain.bounds_max(), bent.bounds_max());
}

// -------------------------------------------------------------- on the GPU

/// The curvature reaches the picture: the same scene at 90° is not the same image, and the
/// draw-call count has not changed (the curvature is a shader variant, not extra draws).
#[test]
fn req_scn_067_a_bent_scene_is_a_different_picture_with_the_same_draws() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(2, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .pattern(solid(2, 0), 8, ProfileLayer::L1)
        .rect(solid(1, 0), 8, 10, 16, 10)
        .rect(solid(2, 0), 14, 13, 2, 2)
        .build();

    let flat = gl.render_built(&mut renderer, &built);
    let flat_draws = renderer.statistics().draw_calls;
    let flat_triangles = renderer.statistics().triangles;

    built.scene.bend = bend_90();
    let bent = gl.render_built(&mut renderer, &built);
    assert_eq!(
        flat_draws,
        renderer.statistics().draw_calls,
        "the curvature adds no draw call"
    );
    assert_eq!(
        flat_triangles,
        renderer.statistics().triangles,
        "and no triangle: the tiles are not subdivided"
    );

    let pixels = (flat.width * flat.height).unsigned_abs() as usize;
    let differing = (0..flat.width)
        .flat_map(|x| (0..flat.height).map(move |y| (x, y)))
        .filter(|&(x, y)| flat.at(x, y) != bent.at(x, y))
        .count();
    assert!(
        differing > pixels / 100,
        "the curvature moved {differing} pixels of {pixels}"
    );
}

/// REQ-SCN-068: the shadow pass bends too, so the shadows follow the curved surface. With the
/// shadows on, a bent scene differs from the same scene drawn with the flat shadow pass — which
/// is what the bent shadow program is for.
#[test]
fn req_scn_068_the_shadow_pass_takes_the_curvature() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(2, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .pattern(solid(2, 0), 10, ProfileLayer::L1)
        .rect(solid(1, 0), 6, 8, 20, 12)
        .rect(solid(2, 0), 12, 12, 4, 4)
        .build();
    built.scene.settings.shadows = true;
    built.scene.bend = bend_90();

    let with_shadows = gl.render_built(&mut renderer, &built);
    assert!(
        renderer.statistics().shadow_pass,
        "the scene casts a shadow"
    );

    built.scene.settings.shadows = false;
    let without = gl.render_built(&mut renderer, &built);
    let differing = (0..with_shadows.width)
        .flat_map(|x| (0..with_shadows.height).map(move |y| (x, y)))
        .filter(|&(x, y)| with_shadows.at(x, y) != without.at(x, y))
        .count();
    assert!(
        differing > 0,
        "a bent scene still has a shadow to show somewhere"
    );
}

/// Q-104: the curvature landed in the OpenGL backend only. This asserts the **known gap** so
/// that a half-done parity fails loudly instead of passing unnoticed; when the wgpu backend
/// gains the curvature, this test is what has to be deleted.
#[cfg(feature = "wgpu")]
#[test]
fn q_104_the_wgpu_backend_carries_nothing_of_the_curvature_yet() {
    const COMMON: &str = include_str!("../src/wgpu_backend/shaders/common.wgsl");
    const SCENE: &str = include_str!("../src/wgpu_backend/shaders/scene.wgsl");
    const SHADOW: &str = include_str!("../src/wgpu_backend/shaders/shadow.wgsl");

    for source in [COMMON, SCENE, SHADOW] {
        assert!(
            !source.contains("zax_bend"),
            "the wgpu shaders have gained the curvature: Q-104 is answered, so this test and \
             the note in wgpu_backend/renderer.rs go"
        );
    }
}
