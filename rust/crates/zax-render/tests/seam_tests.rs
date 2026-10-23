//! The two artefacts AMD-012 is about: the faces that close each tile mesh on itself (SPEC-04B
//! REQ-E3D-505) must not show between the tiles of a translucent object (REQ-SCN-040, Q-020
//! revised), nor at the joints of a plain surface (REQ-SCN-065) (port of `SeamTests.cs`).
//!
//! Both are measured by pixel statistics on an offscreen capture, as SPEC-06 §5 asks: the
//! surface of a flat, uniformly lit plane is one single colour, so any joint that shows at all
//! turns up as a column or a row that departs from it.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::collections::{HashMap, HashSet};

use common::{solid, Built, Capture, GlFixture, PatternOptions, SceneFixture};
use zax_engine3d::draw_list::MaterialClass;
use zax_engine3d::frame::FrameInstanceId;
use zax_engine3d::settings::ZLayer;
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::{ProfileLayer, VariantRenderStyle};
use zax_render::numerics::Vector3;
use zax_render::SceneRenderer;

const BLACK: i32 = 0;

/// AMD-012: no joint pixel may be more than 2 levels off the surface colour.
const TOLERANCE: i32 = 2;

#[test]
fn req_scn_065_a_four_by_four_surface_shows_no_seam_at_its_joints() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let mut built = surface();

    // The criterion is stated for the default view. The seam the users reported comes from the
    // shadow map, so the map resolution is what decides whether it shows: one texel of the map
    // has to stay smaller than the bias for the inner walls of the tiles not to shadow their
    // neighbours. The default of 4096 texels (AMD-034 §2) is checked first, then the two
    // fallback sizes and coarser maps still, which is where the artefact used to appear.
    for map_size in [4096, 2048, 1024, 512, 256, 128] {
        built.scene.settings.set_shadow_map_size(map_size);
        let capture = gl.render_built(&mut renderer, &built);
        assert!(renderer.statistics().shadow_pass);

        let (surface, worst, offenders) = measure(&capture, 12);
        assert_eq!(
            0,
            offenders,
            "with a {map_size}-texel shadow map, {offenders} pixel(s) of the surface differ \
             from {} by up to {worst} levels",
            rgb(surface)
        );
    }
}

#[test]
fn req_scn_065_a_deep_surface_shows_no_seam_and_no_fin_from_any_angle() {
    // FIX-2 (Q-067): the ground of Zelda II and the track of Excitebike are tiles of depth 24
    // to 56 whose walls used to show as a grid at every joint and as dark fins sticking out of
    // the silhouette. The surface is flat and of one colour, so the drawn box has to stay a
    // plain rectangle of that colour whatever the angle.
    let gl = GlFixture::create(800, 600);
    let mut renderer = gl.renderer();

    for depth in [8, 24, 56] {
        let mut built = SceneFixture::new()
            .background(0x0F)
            .palette(3, 0x30)
            .pattern_with(
                solid(3, 0),
                depth,
                ProfileLayer::L1,
                PatternOptions {
                    full_warp: true,
                    ..PatternOptions::default()
                },
            )
            .rect(solid(3, 0), 12, 12, 6, 6)
            .build();

        // Head-on views, pitched up and down: the ones the users reported, and the ones where
        // "a line across the surface" and "the lower edge" mean one face each.
        for (rx, ry) in [(0.81, 0.81), (14.0, 0.81), (20.0, 0.81), (-14.0, 0.81)] {
            built.scene.camera.current_mut().angles = Vector3::new(rx, ry, 0.0);
            built.scene.camera.snap_to_setpoints();
            let capture = gl.render_built(&mut renderer, &built);

            // The grid of walls crossed the front face: a line across it used to change colour
            // at every joint. On a surface of one colour it has to stay single.
            let line = front_face_line(&capture);
            assert_eq!(
                1,
                line.len(),
                "depth {depth} seen from ({rx}, {ry}): a line across the surface shows {} \
                 colours — the joints show",
                line.len()
            );

            // The fins of the walls used to hang a tile below the silhouette, one per tile
            // column: the lower edge of a flat surface became a sawtooth.
            let jumps = sawtooth_jumps(&capture);
            assert_eq!(
                0, jumps,
                "depth {depth} seen from ({rx}, {ry}): the lower edge jumps at {jumps} \
                 column(s) — walls stick out of the silhouette"
            );
        }
    }
}

#[test]
fn req_scn_065_the_joints_stay_invisible_when_the_surface_is_in_shadow() {
    // The same surface, with an invisible caster above it (REQ-SCN-041 style 2) so that part of
    // it is in shadow while nothing else is drawn: the shadowed part must be as flat as the lit
    // one (REQ-SCN-031, REQ-SCN-032).
    let fixture = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .palette(1, 0x16)
        .pattern(solid(3, 0), 6, ProfileLayer::L2)
        .pattern_with(
            solid(1, 0),
            8,
            ProfileLayer::L1,
            PatternOptions {
                style: VariantRenderStyle::ShadowOnly,
                ..PatternOptions::default()
            },
        )
        .rect(solid(3, 0), 14, 14, 4, 4)
        .rect(solid(1, 0), 12, 12, 2, 2);

    // The surface has to cast as well as receive, so that the walls closing its own tiles go
    // into the shadow map: that is what used to darken the joints. L2 only receives by default
    // (REQ-SCN-033), and Lua may turn its casting on.
    let mut built =
        fixture.build_with(|settings| settings.layer_mut(ZLayer::L2).cast_shadow = true);
    built.scene.settings.set_shadow_map_size(256);

    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let capture = gl.render_built(&mut renderer, &built);

    // Both shades of the surface are flat; only the two of them plus the block are allowed.
    let (lit, shadowed) = two_shades(&capture);
    assert_ne!(lit, shadowed);

    let mut strays = 0;
    let (x0, y0, x1, y1) = interior(&capture, 12);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let packed = capture.packed(x, y);
            if deviation(packed, lit) > TOLERANCE && deviation(packed, shadowed) > TOLERANCE {
                strays += 1;
            }
        }
    }

    assert_eq!(
        0,
        strays,
        "{strays} pixel(s) are neither the lit surface {} nor its shadow {}",
        rgb(lit),
        rgb(shadowed)
    );
}

#[test]
fn req_scn_040_a_translucent_multi_tile_object_shows_no_internal_face() {
    // AMD-012 (Q-020 revised): every tile of one object writes its depth before the object's
    // colour pass, so the walls that close each tile against its neighbours never blend.
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern_block(solid(3, 0), 4, 4, 24, ProfileLayer::L1, 0.5)
        .rect(solid(3, 0), 14, 14, 4, 4)
        .build();

    // Seen from an angle, so that the walls between the tiles have a real width on screen —
    // head-on they are edge-on and the artefact hides itself — and from the four corners,
    // because a wall only shows on the side of the neighbour that is drawn after it.
    for (rx, ry) in [(8.0, 8.0), (8.0, -8.0), (-8.0, 8.0), (-8.0, -8.0)] {
        built.scene.camera.current_mut().angles = Vector3::new(rx, ry, 0.0);
        built.scene.camera.snap_to_setpoints();
        let capture = gl.render_built(&mut renderer, &built);

        // One object of sixteen tiles: the grouping of AMD-012 has to see them as one.
        assert_eq!(16, renderer.statistics().transparent_tiles);
        assert_eq!(1, renderer.statistics().transparent_objects);

        // Up to two pixels are tolerated: where a wall meets the face it closes against, the
        // two are coincident to the last bit of the depth buffer, so the colour pass keeps them
        // both and blends that pixel twice (Q-032). The artefact AMD-012 is about covers
        // hundreds of pixels here.
        let (surface, worst, offenders) = measure(&capture, 6);
        assert!(
            offenders <= 2,
            "seen from ({rx}, {ry}), {offenders} pixel(s) inside the translucent object differ \
             from {} by up to {worst} levels: internal faces show",
            rgb(surface)
        );
    }
}

#[test]
fn req_scn_040_transparent_objects_are_sorted_by_the_centre_of_the_whole_object() {
    // Two translucent objects, a red one and a blue one, moved on top of each other. The nearer
    // one is blended last, so its colour weighs twice as much as the farther one's; swapping
    // their depths has to swap which colour dominates (AMD-012: the sort key is the centre of
    // the transformed bounding box of the whole object).
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16)
        .palette(2, 0x12)
        .pattern_block(solid(1, 0), 3, 3, 2, ProfileLayer::L1, 0.5)
        .pattern_block(solid(2, 0), 3, 3, 2, ProfileLayer::L1, 0.5)
        .rect(solid(1, 0), 12, 13, 3, 3)
        .rect(solid(2, 0), 18, 13, 3, 3)
        .build();

    let objects = built.instances(|tile| tile.material == MaterialClass::Transparent);
    assert_eq!(2, objects.len());

    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    // Slide the red object the six tiles that separate them, once in front of the blue one and
    // once behind it. +z goes away from the eye (REQ-SCN-001).
    let middle = built.instance(objects[0]).center.z;
    let in_front = moved(
        &gl,
        &mut renderer,
        &mut built,
        objects[0],
        48.0,
        middle - 40.0,
    );
    assert_eq!(2, renderer.statistics().transparent_objects);
    let behind = moved(
        &gl,
        &mut renderer,
        &mut built,
        objects[0],
        48.0,
        middle + 40.0,
    );

    // The two objects end up exactly on top of each other, so the drawn box is their overlap.
    let (x0, y0, x1, y1) = interior(&in_front, 12);
    let (front_r, _, front_b) = in_front.at((x0 + x1) / 2, (y0 + y1) / 2);
    let (back_r, _, back_b) = behind.at((x0 + x1) / 2, (y0 + y1) / 2);

    assert!(
        front_r > back_r && front_b < back_b,
        "the sort did not follow the depth: in front ({front_r}, _, {front_b}), behind \
         ({back_r}, _, {back_b})"
    );
}

/// Shifts one instance by `offset_x` units across and puts its depth centre at `centre_z`,
/// rebuilds the draw list and captures the result. The tile centres of REQ-E3D-405 take the
/// instance's offset and depth centre, while its anchor cancels out of REQ-E3D-406 at scale 1
/// without rotation.
fn moved(
    gl: &GlFixture,
    renderer: &mut SceneRenderer,
    built: &mut Built,
    instance: FrameInstanceId,
    offset_x: f32,
    centre_z: f32,
) -> Capture {
    let target = built.instance_mut(instance);
    target.offset = Vec3::new(offset_x, 0.0, 0.0);
    target.center.z = centre_z;
    built.rebuild_draw_list();
    gl.render_built(renderer, built)
}

/// The surface of AMD-012: 4 × 4 identical tiles, lit, casting and receiving shadows.
fn surface() -> Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 6, ProfileLayer::L1)
        .rect(solid(3, 0), 14, 14, 4, 4)
        .build()
}

// ------------------------------------------------------------------ measures

/// The drawn box of the capture, inset by a fraction of its side so that only the front face
/// of the surface is sampled, never its silhouette or its side faces.
fn interior(capture: &Capture, inset: i32) -> (i32, i32, i32, i32) {
    let (first_x, last_x) = capture.column_span(BLACK);
    let (first_y, last_y) = capture.row_span(BLACK);
    assert!(
        last_x - first_x > 24 && last_y - first_y > 24,
        "the surface is too small to measure"
    );

    let inset_x = (last_x - first_x + 1) / inset;
    let inset_y = (last_y - first_y + 1) / inset;
    (
        first_x + inset_x,
        first_y + inset_y,
        last_x - inset_x,
        last_y - inset_y,
    )
}

/// The colour of the surface — the commonest one of the interior — then the worst deviation
/// from it and how many pixels pass the 2 levels of AMD-012.
fn measure(capture: &Capture, inset: i32) -> (i32, i32, usize) {
    let (x0, y0, x1, y1) = interior(capture, inset);
    let surface = commonest(capture, x0, y0, x1, y1);

    let mut worst = 0;
    let mut offenders = 0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let deviation = deviation(capture.packed(x, y), surface);
            worst = worst.max(deviation);
            if deviation > TOLERANCE {
                offenders += 1;
            }
        }
    }

    (surface, worst, offenders)
}

/// The colours a horizontal line across the lower part of the drawn shape meets, away from its
/// edges. A flat surface of one colour gives exactly one.
fn front_face_line(capture: &Capture) -> HashSet<i32> {
    let (first_x, last_x) = capture.column_span(BLACK);
    let (first_y, last_y) = capture.row_span(BLACK);
    let margin_x = 2.max((last_x - first_x + 1) / 5);
    let y = last_y - 2.max((last_y - first_y + 1) / 5);

    (first_x + margin_x..=last_x - margin_x)
        .map(|x| capture.packed(x, y))
        .collect()
}

/// How many times the lower edge of the drawn shape jumps by more than two pixels from one
/// column to the next, over the middle of the shape. A jump means something hangs below it —
/// the fin of a tile wall (Q-067).
fn sawtooth_jumps(capture: &Capture) -> usize {
    let (span_start, span_end) = capture.column_span(BLACK);
    let margin = 2.max((span_end - span_start + 1) / 10);
    let bottom: Vec<i32> = (span_start + margin..=span_end - margin)
        .map(|x| {
            (0..capture.height)
                .rev()
                .find(|&y| capture.packed(x, y) != BLACK)
                .unwrap_or(-1)
        })
        .collect();

    bottom
        .windows(2)
        .filter(|pair| pair[0] >= 0 && pair[1] >= 0 && (pair[1] - pair[0]).abs() > 2)
        .count()
}

/// The two commonest colours of the interior: the lit surface and its shadow, lighter first.
fn two_shades(capture: &Capture) -> (i32, i32) {
    let (x0, y0, x1, y1) = interior(capture, 12);
    let mut ranked: Vec<(i32, usize)> = histogram(capture, x0, y0, x1, y1).into_iter().collect();
    ranked.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    assert!(ranked.len() >= 2, "the surface is not partly in shadow");

    let (first, second) = (ranked[0].0, ranked[1].0);
    if luma(first) >= luma(second) {
        (first, second)
    } else {
        (second, first)
    }
}

fn histogram(capture: &Capture, x0: i32, y0: i32, x1: i32, y1: i32) -> HashMap<i32, usize> {
    let mut counts = HashMap::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            *counts.entry(capture.packed(x, y)).or_insert(0) += 1;
        }
    }
    counts
}

fn commonest(capture: &Capture, x0: i32, y0: i32, x1: i32, y1: i32) -> i32 {
    histogram(capture, x0, y0, x1, y1)
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(color, _)| color)
        .expect("a non-empty interior")
}

fn deviation(left: i32, right: i32) -> i32 {
    let channel = |shift: i32| (((left >> shift) & 0xFF) - ((right >> shift) & 0xFF)).abs();
    channel(16).max(channel(8)).max(channel(0))
}

fn luma(packed: i32) -> f64 {
    (0.299 * f64::from((packed >> 16) & 0xFF))
        + (0.587 * f64::from((packed >> 8) & 0xFF))
        + (0.114 * f64::from(packed & 0xFF))
}

fn rgb(packed: i32) -> String {
    format!(
        "({}, {}, {})",
        (packed >> 16) & 0xFF,
        (packed >> 8) & 0xFF,
        packed & 0xFF
    )
}
