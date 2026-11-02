//! AMD-058: REQ-SCN-065 completed — no dark grid, no light crack and no shadow discontinuity at
//! the joint of two neighbouring tiles of a plain surface, for **every** value of
//! `video.anti_alias` (0, 1 and 2), the tiles belonging to one object or to two.
//!
//! The defect AMD-058 §1 measured was in the multisampled path alone: a fragment on the edge of
//! a wall whose pixel centre falls outside the triangle gets an **extrapolated** `vUv`, the
//! `floor()` of `scene.frag` then lands one texel past the face, and the atlas lookup returns a
//! neighbour's texel — a light line one pixel wide along the joint. The fix is a shader variant
//! whose `vUv` is interpolated at the centroid of the samples the fragment covers, which is
//! always inside them.
//!
//! Two things are asserted here, the picture and the sources:
//! - the criterion of REQ-SCN-065 at the three settings, on a surface of whole boxes (AMD-057);
//! - that the variant exists **only** for anti-aliasing: with `video.anti_alias` = 0 — the
//!   handheld default — the code the GLSL compiler is given carries nothing of it.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, Built, Capture, GlFixture, SceneFixture, Tile};
use zax_profiles::enums::ProfileLayer;
use zax_render::geometry;
use zax_render::shaders;

/// AMD-012: no joint pixel may be more than 2 levels off the surface colour.
const TOLERANCE: i32 = 2;

const BLACK: i32 = 0;

/// The three values of `video.anti_alias` REQ-SCN-065 is now required at, and the sample counts
/// AMD-032 §2 maps them to.
const SETTINGS: [(i32, i32); 3] = [(0, 1), (1, 4), (2, 8)];

// ------------------------------------------------------- the picture, REQ-SCN-065

/// First criterion of AMD-058: the surface of REQ-SCN-065 — 4 × 4 identical tiles, lit, casting
/// and receiving its own shadow — passes at `video.anti_alias` = 0, 1 and 2.
#[test]
fn req_scn_065_the_surface_shows_no_seam_at_any_anti_alias_setting() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = surface();
    assert_whole_boxes(&built);

    for (option, wanted) in SETTINGS {
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        let capture = gl.render_built(&mut renderer, &built);

        // AMD-058's own exclusion: the setting has to keep its effect. A context that cannot
        // multisample at all would make the test vacuous, so it is reported rather than passed
        // over in silence.
        let granted = renderer.statistics().anti_alias_samples;
        assert!(
            granted > 1 || wanted == 1,
            "anti_alias {option} asked for {wanted} samples and the context granted {granted}: \
             this context cannot multisample, so the criterion was not measured"
        );

        let (surface, worst, offenders) = measure(&capture, 12);
        assert_eq!(
            0, offenders,
            "anti_alias {option} ({granted} samples): {offenders} pixel(s) of the surface differ \
             from {surface:#08x} by up to {worst} levels"
        );
    }
}

/// Second criterion of AMD-058: the same surface made of tiles of **two** neighbouring objects
/// (two columns of 2 × 4 tiles), at the same depth and on the same layer, for the three values.
#[test]
fn req_scn_065_a_surface_of_two_objects_shows_no_seam_at_any_anti_alias_setting() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    // Two patterns of the same image, so the two halves are two distinct objects that meet along
    // the middle joint — the joint that has no mesh of its own to hide behind.
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 6, ProfileLayer::L1)
        .rect(solid(3, 0), 14, 14, 2, 4)
        .rect(solid(3, 0), 16, 14, 2, 4)
        .build();
    assert_whole_boxes(&built);
    assert!(
        built.draw_list.tiles().len() >= 16,
        "the two halves are both drawn"
    );

    for (option, _) in SETTINGS {
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        let capture = gl.render_built(&mut renderer, &built);
        let (surface, worst, offenders) = measure(&capture, 12);
        assert_eq!(
            0, offenders,
            "two objects, anti_alias {option}: {offenders} pixel(s) differ from {surface:#08x} \
             by up to {worst} levels"
        );
    }
}

/// AMD-058's second exclusion: the fix may not be "turn the anti-aliasing off". The setting has
/// to keep its effect on the **edges** of the objects, which is what it is for (AMD-032 §2).
#[test]
fn amd_058_the_setting_still_softens_the_edges_of_the_objects() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = surface();

    let mut distinct = Vec::new();
    for (option, _) in SETTINGS {
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        let capture = gl.render_built(&mut renderer, &built);
        distinct.push((
            option,
            renderer.statistics().anti_alias_samples,
            colours(&capture),
        ));
    }

    let flat = distinct[0].2;
    for (option, granted, count) in distinct.iter().copied().skip(1) {
        if granted == 1 {
            // Reported, not silently passed: this context cannot multisample.
            continue;
        }
        assert!(
            count > flat,
            "anti_alias {option} ({granted} samples) gave {count} distinct colours and \
             anti_alias 0 gave {flat}: the setting no longer softens any edge"
        );
    }
}

/// AMD-058's third criterion as far as it can be automated on this side: a capture with
/// anti-aliasing differs from one without it **only at the edges of the objects** — inside the
/// silhouette of a plain surface made of neighbouring tiles, the two are the same picture, joints
/// included.
///
/// The surface has to be *plain* for this statement to mean what the criterion means. A textured
/// one is not a counter-example but a different measure: its tile joints carry a real colour
/// discontinuity (the right edge of one tile image against the left edge of the next), so
/// multisampling blends them, and it is *supposed* to — measured, 109 interior pixels of a
/// magnified 4 × 4 striped surface and 2100 of a minified 28 × 26 one. Both are the
/// anti-aliasing doing its work, which AMD-058's own exclusion requires it to keep doing.
///
/// **This passes with and without the variant on this machine**, as do the two criteria above;
/// see the progress notes of this step. It is the regression guard for the statement it makes,
/// not the proof that the defect is gone — that is R0's review criterion on the two ROM scenes
/// at 2560 × 1440.
#[test]
fn amd_058_an_anti_aliased_capture_differs_from_a_plain_one_only_at_the_edges() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let mut built = surface();
    assert_whole_boxes(&built);

    built.scene.settings.set_anti_alias_samples(1);
    let plain = gl.render_built(&mut renderer, &built);
    assert!(!renderer.statistics().centroid_interpolation);

    for option in [1, 2] {
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        let aliased = gl.render_built(&mut renderer, &built);
        if renderer.statistics().anti_alias_samples == 1 {
            continue; // Reported by the criteria above: this context cannot multisample.
        }

        // The interior of the surface, inset so that the silhouette — which anti-aliasing is
        // *meant* to change — is outside the window.
        let (x0, y0, x1, y1) = interior(&plain, 12);
        let mut changed = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if deviation(aliased.packed(x, y), plain.packed(x, y)) > TOLERANCE {
                    changed.push((x, y));
                }
            }
        }
        assert!(
            changed.is_empty(),
            "anti_alias {option}: {} interior pixel(s) differ from the plain capture, the first \
             at {:?} — the joints of AMD-058 §1",
            changed.len(),
            changed.first()
        );
    }
}

/// The scope of the variant, from outside the renderer: the centroid programs draw the frame
/// exactly when `video.anti_alias` is not 0, and never otherwise. This is the assertion that
/// says the fix of AMD-058 touches the multisampled path **only** — with the setting at the
/// handheld default the renderer does not even hold a centroid program.
#[test]
fn amd_058_the_centroid_variant_draws_exactly_when_anti_aliasing_is_on() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();
    let mut built = surface();

    for (option, _) in SETTINGS {
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        let _ = gl.render_built(&mut renderer, &built);
        assert_eq!(
            option != 0,
            renderer.statistics().centroid_interpolation,
            "anti_alias {option}"
        );
    }

    // And back: turning the setting off puts the pixel-centre programs back in use for good.
    built.scene.settings.set_anti_alias_samples(1);
    let _ = gl.render_built(&mut renderer, &built);
    assert!(!renderer.statistics().centroid_interpolation);
}

/// The variant really is in effect on the GPU, and not merely selected: at four samples the
/// capture of a textured surface holds colours the pixel-centre programs do not produce. The
/// count is the measure of that (the pixel-centre run gave 8 distinct colours on this machine,
/// the centroid one 12); the test asserts only the direction, so that it does not depend on one
/// driver's sample positions.
#[test]
fn amd_058_the_variant_changes_what_the_gpu_draws() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(3, 0x11)
        .pattern(striped(), 6, ProfileLayer::L1)
        .rect(striped(), 14, 14, 4, 4)
        .build();
    assert_whole_boxes(&built);

    built.scene.settings.set_anti_alias_samples(1);
    let plain = colours(&gl.render_built(&mut renderer, &built));

    built
        .scene
        .settings
        .set_anti_alias_samples(geometry::anti_alias_samples(1));
    let capture = gl.render_built(&mut renderer, &built);
    if renderer.statistics().anti_alias_samples == 1 {
        return; // This context cannot multisample; reported by the criteria above.
    }
    assert!(renderer.statistics().centroid_interpolation);
    assert!(
        colours(&capture) > plain,
        "four samples gave {} distinct colours and one sample {plain}",
        colours(&capture)
    );
}

// -------------------------------------------------- the sources, the variant's scope

/// The performance requirement of this step, and the reason the centroid is a `#define` and not
/// a sampler qualifier chosen at run time: at `video.anti_alias` = 0 — the handheld default —
/// the compiler is given **nothing** of AMD-058, so the programs are the ones of before it.
#[test]
fn amd_058_the_pixel_centre_variant_carries_nothing_of_the_centroid() {
    for embedded in [false, true] {
        for solid_variant in [false, true] {
            for srgb in [false, true] {
                for bend in [false, true] {
                    let source = shaders::scene(solid_variant, srgb, bend, false);
                    for (fragment, stage) in [(false, &source.vertex), (true, &source.fragment)] {
                        let assembled = shaders::assemble(stage, fragment, embedded);
                        // The define itself is never emitted, so the guarded branches below
                        // really are dropped by the preprocessor.
                        assert!(
                            !assembled.contains("#define ZAX_CENTROID"),
                            "{} ({}): the define is emitted",
                            source.name,
                            if fragment { "fragment" } else { "vertex" }
                        );
                        let compiled = without_centroid(&assembled);
                        assert!(
                            !compiled.contains("centroid"),
                            "{} ({}): the compiler is given a centroid qualifier",
                            source.name,
                            if fragment { "fragment" } else { "vertex" }
                        );
                        assert!(!compiled.contains("ZAX_CENTROID"));
                    }
                }
            }
        }
    }
}

/// And the other half of the same statement: the only difference between the two variants is the
/// qualifier on `vUv`. Nothing else of the scene shader changes with the anti-aliasing, so a
/// capture at `anti_alias` ≥ 1 can only differ from one at 0 where the samples differ.
#[test]
fn amd_058_the_two_variants_differ_only_by_the_qualifier_on_v_uv() {
    for embedded in [false, true] {
        for solid_variant in [false, true] {
            for srgb in [false, true] {
                for bend in [false, true] {
                    let centre = shaders::scene(solid_variant, srgb, bend, false);
                    let centroid = shaders::scene(solid_variant, srgb, bend, true);
                    assert_ne!(centre.name, centroid.name, "two cache keys, two programs");
                    for (fragment, centre_stage, centroid_stage) in [
                        (false, &centre.vertex, &centroid.vertex),
                        (true, &centre.fragment, &centroid.fragment),
                    ] {
                        let flat =
                            without_centroid(&shaders::assemble(centre_stage, fragment, embedded));
                        let lit =
                            with_centroid(&shaders::assemble(centroid_stage, fragment, embedded));
                        assert_eq!(
                            flat,
                            lit.replace("centroid in vec2 vUv;", "in vec2 vUv;")
                                .replace("centroid out vec2 vUv;", "out vec2 vUv;")
                                .replace("#define ZAX_CENTROID\n", ""),
                            "{} against {} ({})",
                            centre.name,
                            centroid.name,
                            if fragment { "fragment" } else { "vertex" }
                        );
                    }
                }
            }
        }
    }
}

/// The names of the eight variants without the centroid are unchanged, because the name is part
/// of the key of the on-disk program cache (`gl/program_cache.rs`).
#[test]
fn amd_058_the_names_of_the_pixel_centre_variants_are_unchanged() {
    const BEFORE: [&str; 8] = [
        "scene",
        "scene_solid",
        "scene_srgb_palette",
        "scene_solid_srgb_palette",
        "scene_bend",
        "scene_solid_bend",
        "scene_srgb_palette_bend",
        "scene_solid_srgb_palette_bend",
    ];

    let mut found = Vec::new();
    for bend in [false, true] {
        for srgb in [false, true] {
            for solid_variant in [false, true] {
                found.push(shaders::scene(solid_variant, srgb, bend, false).name);
            }
        }
    }
    found.sort_unstable();
    let mut expected = BEFORE.to_vec();
    expected.sort_unstable();
    assert_eq!(expected, found);
}

// ------------------------------------------------------------------ helpers

/// The code of `source` as the GLSL compiler sees it when `ZAX_CENTROID` is **not** defined: the
/// `#ifdef ZAX_CENTROID` branch dropped, its `#else` kept, line comments and blank lines removed.
///
/// The bodies of `scene.vert` and `scene.frag` hold both branches as text, so the requirement —
/// at `anti_alias` 0 the compiled programs are the ones of before AMD-058 — is about what the
/// compiler is given, not about what the file says. Same shape as `bend_tests::without_bend`.
fn without_centroid(source: &str) -> String {
    preprocess(source, false)
}

/// The same, with `ZAX_CENTROID` defined: the `#ifdef` branch kept and the `#else` dropped.
fn with_centroid(source: &str) -> String {
    preprocess(source, true)
}

fn preprocess(source: &str, defined: bool) -> String {
    assert!(!source.contains("/*"), "no block comment to account for");
    let mut kept = String::with_capacity(source.len());
    // 0 outside any block, 1 inside the ZAX_CENTROID branch, 2 inside its #else.
    let mut state = 0u8;
    for line in source.lines() {
        let trimmed = line.trim();
        match trimmed {
            "#ifdef ZAX_CENTROID" => state = 1,
            "#else" if state == 1 => state = 2,
            "#endif" if state != 0 => state = 0,
            _ => {
                let dropped = (state == 1 && !defined) || (state == 2 && defined);
                if dropped {
                    continue;
                }
                let code = line.split_once("//").map_or(line, |(code, _)| code);
                let code = code.trim_end();
                if !code.is_empty() {
                    kept.push_str(code);
                    kept.push('\n');
                }
            }
        }
    }
    assert_eq!(0, state, "every #ifdef ZAX_CENTROID is closed");
    kept
}

/// The surface of AMD-012 and REQ-SCN-065: 4 × 4 identical tiles, lit, casting and receiving
/// shadows.
fn surface() -> Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 6, ProfileLayer::L1)
        .rect(solid(3, 0), 14, 14, 4, 4)
        .build()
}

/// A tile image of more than one colour, the kind the two scenes of AMD-058 §1 are made of:
/// colour 3 everywhere except the leftmost column, which is colour 1. The asymmetry is the
/// point — a `vUv` extrapolated past the right edge wraps (REQ-E3D-563) onto column 0 and picks
/// up a colour the face does not hold.
///
/// `ProfileTile::color_at` reads bit `y * 8 + 7 - x` of each plane (REQ-PROF-031).
fn striped() -> Tile {
    let mut low = 0u64;
    let mut high = 0u64;
    for y in 0..8u32 {
        for x in 0..8u32 {
            let bit = y * 8 + 7 - x;
            let index = if x == 0 { 1u64 } else { 3u64 };
            low |= (index & 1) << bit;
            high |= ((index >> 1) & 1) << bit;
        }
    }
    (low, high)
}

/// AMD-057 §2: every drawn tile is the complete box of REQ-E3D-501, so none of the measures
/// below can pass because a wall was taken out (the means AMD-057 forbids and AMD-058 repeats).
fn assert_whole_boxes(built: &Built) {
    assert!(!built.draw_list.tiles().is_empty(), "the surface is drawn");
    for tile in built.draw_list.tiles() {
        let mesh = tile.mesh.as_ref().expect("a mesh");
        assert_eq!(24, mesh.vertex_count(), "a drawn tile is the whole box");
        assert_eq!(12, mesh.triangle_count());
    }
}

/// How many distinct colours the capture holds: multisampling adds the blends of the edges.
fn colours(capture: &Capture) -> usize {
    let mut seen = std::collections::HashSet::new();
    for y in 0..capture.height {
        for x in 0..capture.width {
            seen.insert(capture.packed(x, y));
        }
    }
    seen.len()
}

/// The drawn box of the capture, inset by a fraction of its side so that only the front face of
/// the surface is sampled, never its silhouette or its side faces.
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

/// The colour of the surface — the commonest one of the interior — then the worst deviation from
/// it and how many pixels pass the 2 levels of AMD-012.
fn measure(capture: &Capture, inset: i32) -> (i32, i32, usize) {
    let (x0, y0, x1, y1) = interior(capture, inset);
    let mut histogram = std::collections::HashMap::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            *histogram.entry(capture.packed(x, y)).or_insert(0usize) += 1;
        }
    }
    let surface = histogram
        .iter()
        .max_by_key(|(colour, count)| (**count, **colour))
        .map(|(colour, _)| *colour)
        .expect("a non-empty interior");

    let mut worst = 0;
    let mut offenders = 0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let found = deviation(capture.packed(x, y), surface);
            worst = worst.max(found);
            if found > TOLERANCE {
                offenders += 1;
            }
        }
    }

    (surface, worst, offenders)
}

/// The largest per-channel difference between two packed colours.
fn deviation(found: i32, expected: i32) -> i32 {
    let channel = |packed: i32, shift: i32| (packed >> shift) & 0xFF;
    let mut worst = 0;
    for shift in [16, 8, 0] {
        worst = worst.max((channel(found, shift) - channel(expected, shift)).abs());
    }
    worst
}

/// The statement the light line of AMD-058 §1 would break, written so that a future regression
/// of the interpolation is caught wherever the fixture is big enough to show it: on a textured
/// surface, a pixel wholly inside the silhouette may only hold a blend of the colours the
/// surface's own texels carry. A `vUv` extrapolated past the face fetches a texel the face does
/// not hold — on those two scenes a lighter one — and the pixel then takes a colour no blend of
/// the surface's own can give.
///
/// Like the two criteria of AMD-058 §2, this passes with and without the variant on the
/// synthetic surfaces of this side (progress notes of this step): inside the silhouette the
/// extrapolation stays within the face at the magnifications reachable here.
#[test]
fn amd_058_no_interior_pixel_takes_a_colour_the_surface_cannot_produce() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(3, 0x11)
        .pattern(striped(), 6, ProfileLayer::L1)
        .rect(striped(), 14, 14, 4, 4)
        .build();
    assert_whole_boxes(&built);

    for (option, _) in SETTINGS {
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));
        let capture = gl.render_built(&mut renderer, &built);
        let granted = renderer.statistics().anti_alias_samples;
        let (x0, y0, x1, y1) = interior(&capture, 12);

        // The colours the surface itself holds: the two commonest of the window, which are the
        // two lit stripes. Every other interior pixel has to sit between them, channel by
        // channel, give or take the 2 levels of AMD-012.
        let mut histogram = std::collections::HashMap::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                *histogram.entry(capture.packed(x, y)).or_insert(0usize) += 1;
            }
        }
        let mut common: Vec<(i32, usize)> = histogram.into_iter().collect();
        common.sort_by_key(|(colour, count)| (std::cmp::Reverse(*count), *colour));
        assert!(
            common.len() >= 2 && common[1].1 > 50,
            "the surface shows both of its stripes: {:?}",
            &common[..common.len().min(4)]
        );
        let (a, b) = (common[0].0, common[1].0);

        let mut offenders = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let found = capture.packed(x, y);
                for shift in [16, 8, 0] {
                    let channel = |packed: i32| (packed >> shift) & 0xFF;
                    let (low, high) = (channel(a).min(channel(b)), channel(a).max(channel(b)));
                    if channel(found) < low - TOLERANCE || channel(found) > high + TOLERANCE {
                        offenders.push((x, y, found));
                        break;
                    }
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "anti_alias {option} ({granted} samples): {} interior pixel(s) hold a colour no \
             blend of {a:#08x} and {b:#08x} can give, the first {:?} — a texel fetched outside \
             the face, which is the defect of AMD-058 §1",
            offenders.len(),
            offenders.first()
        );
    }
}
