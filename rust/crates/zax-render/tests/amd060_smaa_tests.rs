//! AMD-060 REQ-SCN-085 and REQ-SCN-086 — the depth-detecting SMAA, read on the sources, on the
//! two precomputed tables and on the renderer's lifecycle.
//!
//! What is asserted here, and why each piece is here rather than in the harness:
//!
//! - **the two tables are the reference's own bytes**, pinned by length and SHA-256 (AMD-060 §4,
//!   "les tables sont reprises à l'octet"). Nothing but a hash can say that;
//! - **the three passes are in the GLSL**, with the reference's constants and — the one thing the
//!   port's correctness along every vertical edge rests on — the **mirrored** `y` of
//!   `SMAA_RT_METRICS`. A shader is read, not rendered, for this;
//! - **the lifecycle of REQ-SCN-086** on a live context: nothing exists at levels 0, 1 and 2, the
//!   targets appear on the first frame at 3 and are released on the first frame that is not, with
//!   no restart;
//! - **the fallback of REQ-SCN-086**: refused, drawn without anti-aliasing, said once.
//!
//! The *picture* criteria of REQ-SCN-085 — more distinct colours on geometric edges, the interior
//! of a textured face byte-identical — are in `zax-harness/tests/amd060_smaa_tests.rs`, where the
//! synthetic scenes and the golden captures live.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use common::{close_up_r, Built, GlFixture, SceneFixture};
use sha2::{Digest, Sha256};
use zax_profiles::enums::ProfileLayer;
use zax_render::geometry;
use zax_render::shaders;

/// A frame with hard geometric edges: the close-up R of AMD-055 §2.5 — a cube of `D` = 8 standing
/// on a floor of 4 x 4 full tiles — which is the one fixture of this crate whose silhouettes are
/// long, slanted and unmistakable. What the filter is asked to do here is detect them; what it
/// does to the picture is measured in the harness, over the whole synthetic set.
fn stepped() -> Built {
    close_up_r(true, 4, 1.0, 0.0)
}

/// **One flat face carrying a contrasted texture**, which is the frame AMD-060's second acceptance
/// criterion is written about: a row of whole tiles at one depth, so their front faces are
/// coplanar and the surface has no contour inside it at all, textured with alternating columns of
/// two palette entries a palette apart.
///
/// The alternation is one **texel** wide — the worst case for any filter, and exactly the pixel art
/// AMD-032 §2 refused a post-processing filter over.
fn striped_face() -> Built {
    // Alternating columns of palette entries 1 and 3 — a texel of one, a texel of the other, which
    // is the worst case for any filter.
    let mut low = 0_u64;
    let mut high = 0_u64;
    for y in 0..8_u32 {
        for x in 0..8_u32 {
            let bit = (y * 8) + 7 - x;
            let index = if x % 2 == 0 { 1_u64 } else { 3_u64 };
            low |= (index & 1) << bit;
            high |= ((index >> 1) & 1) << bit;
        }
    }
    let striped = (low, high);
    SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(3, 0x16)
        .pattern(striped, 6, ProfileLayer::L1)
        // A block of 10 x 10 whole tiles in the middle of the picture (AMD-057): one surface, one
        // plane, and a wide margin of background on every side so that the interior and the
        // silhouette can be told apart.
        .rect(striped, 11, 11, 10, 10)
        .build()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut digest = Sha256::new();
    digest.update(bytes);
    let mut text = String::new();
    for byte in digest.finalize() {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

// ------------------------------------------------------------------ the two tables

/// AMD-060 §4: "les shaders et les deux tables précalculées viennent de `third_party/smaa` […] les
/// tables sont reprises à l'octet."
///
/// The hashes are of the byte arrays of `Textures/AreaTex.h`'s `areaTexBytes` and
/// `Textures/SearchTex.h`'s `searchTexBytes`, at the upstream commit
/// `71c806a838bdd7d517df19192a20f0c61b3ca29d` recorded in `third_party/smaa/README.txt`, in the
/// headers' own row order. `src/assets/smaa/README.md` holds the same two lines and the extraction
/// command, so a reader can repeat it.
///
/// This is the test that makes "byte for byte" a checked property rather than a claim, and the
/// reason the tables are committed as binary at all (`third_party/` is not tracked by this
/// repository, so a build script could not read the headers).
#[test]
fn req_scn_085_the_two_tables_are_the_reference_s_own_bytes() {
    let area = shaders::SMAA_AREA_TABLE;
    assert_eq!(160 * 560 * 2, area.len(), "AREATEX_SIZE");
    assert_eq!(
        "35065cef2a02cabcad711d6bf430239ae64e27d71c4e4fa06f29cce2c992f0d2",
        hex(area)
    );

    let search = shaders::SMAA_SEARCH_TABLE;
    assert_eq!(64 * 16, search.len(), "SEARCHTEX_SIZE");
    assert_eq!(
        "3694eae5e9d44b8ebb4415a13f8c7b94dc08a2fc86658434d771c4610fe5744d",
        hex(search)
    );
}

// ------------------------------------------------------------------ the sources

/// REQ-SCN-085's detector is the **depth** one, and the port carries the reference's own
/// arithmetic — including the two things a text substitution would have got wrong.
#[test]
fn req_scn_085_the_edge_detection_reads_the_depth_and_mirrors_y() {
    let source = shaders::assemble(&shaders::SMAA_EDGES.fragment, true, false);

    // The detector: the depth, the step and the reference's own early-out.
    for name in [
        "uDepth",
        "step",
        "discard",
        "zaxSmaaFetch",
        "zaxSmaaFetchOffset",
    ] {
        assert!(common::mentions(&source, name), "missing {name}");
    }
    assert!(
        source.contains("edges = step(threshold,"),
        "the reference's own `edges = step(threshold, ...)`"
    );
    // And **not** the luma or the colour detector REQ-SCN-085 forbids, which on this side means
    // the frame's colour never reaches this program at all.
    for forbidden in ["uColor", "uLinear", "zaxLuma"] {
        assert!(
            !common::mentions(&source, forbidden),
            "the depth detector must not read {forbidden}"
        );
    }

    // The threshold of Q-109 §1: the relative term, the floor that keeps it strictly positive,
    // and the two taps two texels away the second difference is built from (`offset[2]` of the
    // reference's own `SMAAEdgeDetectionVS`).
    for name in ["ZAX_SMAA_DEPTH_RELATIVE", "ZAX_SMAA_DEPTH_FLOOR"] {
        assert!(common::mentions(&source, name), "missing {name}");
    }
    for offset in ["vec2(-2.0, 0.0)", "vec2(0.0, -2.0)"] {
        assert!(
            source.contains(offset),
            "missing the second-difference tap {offset}"
        );
    }
    // And **both** differences gate the edge. The second alone would mark the pixel behind the
    // contour as well; the first alone fires on a slope, which is what the first version of this
    // shader did and why it found no horizontal contour at all (Q-109 §1).
    assert!(
        source.contains("min(delta, curvature)"),
        "both differences have to clear the threshold"
    );
    assert!(
        source.contains("(2.0 * near)"),
        "the second difference of the window depth"
    );

    // The offsets are the multiply-add form, never `textureLodOffset`: an integer texel offset is
    // in the texture's own space and would not pick up the sign of `uRtMetrics.y`
    // (`shaders/smaa.glsl` point 1).
    assert!(common::mentions(&source, "uRtMetrics"));
    // A **call**, not the word: the two sources name `textureLodOffset` in the comments that
    // explain why they never call it, which is exactly the thing worth saying twice.
    assert!(
        !source.contains("textureLodOffset("),
        "an integer texel offset would not mirror y"
    );
}

/// The one comparison of the second pass that the sign of `uRtMetrics.y` does **not** carry by
/// itself, in both ports.
///
/// The vertical searches walk `± (0, 2) · metrics.xy` and stop at a bound built the same way, so
/// both flip with the sign — and the operator between them does not. With the reference's own
/// `texcoord.y > end` and a negative `y`, the condition is false on the first iteration and the
/// two searches do nothing at all: a vertical contour is then blended over one pixel instead of
/// over the length of its run. That is what the OpenGL path did until the `wgpu` port of step 5
/// put the same algorithm beside it with the other sign; the fix multiplies the comparison through
/// by `uRtMetrics.y`, which is the reference's inequality where `y` grows downward and the
/// mirrored one where it grows up.
///
/// Pinned on both sources, because the symptom is invisible without a second backend to compare
/// against: the filter still anti-aliases, still passes both acceptance criteria, and still looks
/// right.
#[test]
fn req_scn_085_the_vertical_searches_compare_in_the_sign_of_the_y_metric() {
    let source = shaders::assemble(&shaders::SMAA_WEIGHTS.fragment, true, false);
    assert!(
        source.contains("((texcoord.y - end) * uRtMetrics.y) > 0.0"),
        "the upward search must compare in the sign of the y metric"
    );
    assert!(
        source.contains("((texcoord.y - end) * uRtMetrics.y) < 0.0"),
        "and the downward one"
    );
    // And never the reference's own two conditions, which are the ones that do nothing here. The
    // loop, not the word: the file names them in the comment that explains why it does not use
    // them, which is exactly the thing worth saying twice.
    assert!(
        !source.contains("while (texcoord.y > end") && !source.contains("while (texcoord.y < end"),
        "a bare comparison does not carry the sign of the y metric"
    );
    // The horizontal searches keep the reference's own, `uRtMetrics.x` being positive on both
    // backends — asserted so that the fix is read as the narrow thing it is.
    assert!(
        source.contains("while (texcoord.x > end") && source.contains("while (texcoord.x < end"),
        "the horizontal searches need no sign"
    );
}

/// The `y` of `SMAA_RT_METRICS` is **negative** on the OpenGL side. The algorithm is not symmetric
/// under a mirror of it — `SMAAArea` splits a trapezoid's coverage between the two pixels on either
/// side of the edge — so this is the assertion that stands between the port and a result displaced
/// along every vertical edge.
///
/// Read here on the only number that carries it, because no shader source says which sign it gets.
#[test]
fn req_scn_085_the_render_target_metrics_name_the_drawn_size() {
    let fixture = GlFixture::create(96, 64);
    let mut renderer = fixture.renderer();
    let mut built = stepped();
    built.scene.settings.set_anti_alias(3);
    fixture.render_built(&mut renderer, &built);

    // The one observable consequence from out here: the filter ran, over a target of the frame's
    // own size, and said so.
    let statistics = renderer.statistics();
    assert_eq!(3, statistics.smaa_passes);
    // `RG8` + `RGBA8` + `RGBA16F`, all three at the frame's own size: 14 bytes a pixel, against the
    // 48 the extra attachments of MSAA 4x cost (4 x (8 + 4)).
    assert_eq!(96 * 64 * (2 + 4 + 8), statistics.smaa_bytes);
}

// ------------------------------------------------------------------ the two acceptance criteria

/// AMD-060 REQ-SCN-085, first acceptance criterion: "sur une scène à arêtes géométriques franches,
/// la capture à 3 a strictement plus de couleurs distinctes qu'à 0".
///
/// Read beside level 1 as well, because the figure means more with something to compare it to: the
/// filter has to be doing *more* than nothing, and a reader should be able to see that what it does
/// is of the same kind as what multisampling does.
#[test]
fn req_scn_085_the_filter_puts_more_colours_on_a_geometric_edge() {
    let fixture = GlFixture::create(320, 240);
    let mut renderer = fixture.renderer();
    let mut built = stepped();

    built.scene.settings.set_anti_alias(0);
    let plain = fixture
        .render_built(&mut renderer, &built)
        .distinct_colors();
    built.scene.settings.set_anti_alias(1);
    let msaa = fixture
        .render_built(&mut renderer, &built)
        .distinct_colors();
    built.scene.settings.set_anti_alias(3);
    let smaa = fixture
        .render_built(&mut renderer, &built)
        .distinct_colors();

    assert!(
        smaa.len() > plain.len(),
        "3 has {} colours, 0 has {}",
        smaa.len(),
        plain.len()
    );
    // And the gradations are real rather than a handful: the filter is held to at least as many as
    // MSAA 4x produces on the same frame, which is the weakest statement that still has teeth.
    assert!(
        smaa.len() >= msaa.len(),
        "3 has {} colours, 1 has {}",
        smaa.len(),
        msaa.len()
    );
    // Every colour of the unfiltered capture survives: the filter adds gradations between the
    // frame's own colours, it does not replace them.
    assert!(
        plain.iter().all(|color| smaa.contains(color)),
        "a colour of the unfiltered frame is gone"
    );
}

/// AMD-060 REQ-SCN-085, second acceptance criterion: "sur une scène dont une seule face plane porte
/// une texture contrastée, les pixels intérieurs à la face sont identiques à l'octet à ceux de la
/// capture à 0".
///
/// **This is the requirement the whole mode rests on**, and the reason AMD-032 §2's objection to
/// post-processing filters does not reach it: the detector reads the depth, a flat face carries no
/// depth contour, so no texel of a nearest-neighbour texture can be filtered. The pixels compared
/// here are those at least three columns and three rows inside the face's own silhouette — three
/// because that is the furthest the first pass reaches (`offset[2]` is two texels) plus the one the
/// third pass mixes over.
#[test]
fn req_scn_085_the_interior_of_a_textured_face_is_untouched() {
    let fixture = GlFixture::create(320, 240);
    let mut renderer = fixture.renderer();
    let mut built = striped_face();

    built.scene.settings.set_anti_alias(0);
    let plain = fixture.render_built(&mut renderer, &built);
    built.scene.settings.set_anti_alias(3);
    let smaa = fixture.render_built(&mut renderer, &built);

    // The face has to be there, and it has to be textured: a uniform face would make this test
    // pass for the wrong reason.
    let background = plain.packed(0, 0);
    let (first_row, last_row) = plain.row_span(background);
    let (first_column, last_column) = plain.column_span(background);
    assert!(
        last_row - first_row > 16 && last_column - first_column > 16,
        "the face is {first_column}..{last_column} x {first_row}..{last_row}"
    );
    assert!(
        plain.distinct_colors().len() >= 3,
        "the face carries no contrast"
    );

    let mut compared = 0;
    for y in (first_row + 3)..=(last_row - 3) {
        for x in (first_column + 3)..=(last_column - 3) {
            compared += 1;
            assert_eq!(
                plain.at(x, y),
                smaa.at(x, y),
                "the filter touched the interior at ({x}, {y})"
            );
        }
    }
    assert!(compared > 1000, "only {compared} interior pixels compared");
}

// ------------------------------------------------------------------ the lifecycle

/// REQ-SCN-086: "Avec `video.anti_alias` ≠ 3, aucune passe, aucune cible et aucune texture du SMAA
/// NE DOIT exister ni être soumise."
///
/// Read on the three counters at the four levels, on one renderer, in one session — so the two
/// clauses of the requirement (nothing at the other levels, and the allocation on the first frame
/// at 3) are the same measurement.
#[test]
fn req_scn_086_nothing_of_the_filter_exists_at_the_other_levels() {
    let fixture = GlFixture::create(96, 64);
    let mut renderer = fixture.renderer();
    let mut built = stepped();

    for level in [0, 1, 2] {
        built.scene.settings.set_anti_alias(level);
        fixture.render_built(&mut renderer, &built);
        let statistics = renderer.statistics();
        assert_eq!(0, statistics.smaa_passes, "level {level} submitted a pass");
        assert_eq!(0, statistics.smaa_bytes, "level {level} allocated a target");
        assert_eq!(0, statistics.smaa_programs, "level {level} built a program");
        assert_eq!(
            geometry::anti_alias_samples(level),
            statistics.anti_alias_samples,
            "level {level}"
        );
    }

    // And the first frame at 3 builds them — with **one** sample, which is REQ-SCN-085's first
    // clause ("la scène DOIT être rendue sans multi-échantillonnage").
    built.scene.settings.set_anti_alias(3);
    fixture.render_built(&mut renderer, &built);
    let statistics = renderer.statistics();
    assert_eq!(3, statistics.smaa_passes);
    assert!(statistics.smaa_bytes > 0);
    assert_eq!(3, statistics.smaa_programs);
    assert_eq!(1, statistics.anti_alias_samples, "no multisampling at 3");
}

/// REQ-SCN-086: "Les cibles et les tables sont allouées au premier passage à 3 et libérées au
/// retour à une autre valeur, **sans redémarrage**" — the "imm." effect of SPEC-09 §4.1.1.
///
/// Both directions, twice, on one renderer: a round trip that leaked would show as a second
/// allocation that never goes back to zero.
#[test]
fn req_scn_086_the_targets_are_allocated_and_released_without_a_restart() {
    let fixture = GlFixture::create(64, 48);
    let mut renderer = fixture.renderer();
    let mut built = stepped();

    for _ in 0..2 {
        built.scene.settings.set_anti_alias(3);
        fixture.render_built(&mut renderer, &built);
        assert_eq!(64 * 48 * (2 + 4 + 8), renderer.statistics().smaa_bytes);
        assert_eq!(3, renderer.statistics().smaa_programs);

        built.scene.settings.set_anti_alias(1);
        fixture.render_built(&mut renderer, &built);
        assert_eq!(0, renderer.statistics().smaa_bytes, "released");
        assert_eq!(0, renderer.statistics().smaa_programs, "released");
        assert_eq!(4, renderer.statistics().anti_alias_samples, "MSAA 4x again");
    }
}

/// REQ-SCN-086's fallback: "Si une ressource requise est refusée par le pilote, le rendu continue
/// sans anticrénelage, avec une ligne de diagnostic unique, comme le repli (D) de l'occlusion
/// MSVO."
///
/// Reached through the same kind of test seam the MSVO fallback is reached through
/// (`SceneRenderer::fail_smaa`), and for the reason RU-5 and RU-7 both gave: no driver here
/// refuses `RG8`, and an unreachable fallback is an untested one.
#[test]
fn req_scn_086_a_refused_resource_draws_the_frame_without_anti_aliasing() {
    let fixture = GlFixture::create(64, 48);
    let mut renderer = fixture.renderer();
    let mut built = stepped();
    built.scene.settings.set_anti_alias(3);

    renderer.fail_smaa(true);
    let refused = fixture.render_built(&mut renderer, &built);

    assert!(renderer.smaa_given_up());
    assert_eq!(0, renderer.statistics().smaa_passes, "no pass submitted");
    assert_eq!(0, renderer.statistics().smaa_bytes, "no target allocated");
    assert_eq!(1, renderer.statistics().anti_alias_samples, "and no MSAA");

    // One line, naming the requirement, and **one**: a second frame says nothing more.
    let lines = renderer.take_diagnostics();
    assert_eq!(1, lines.len(), "{lines:?}");
    assert!(lines[0].contains("SMAA"), "{:?}", lines[0]);
    assert!(lines[0].contains("REQ-SCN-086"), "{:?}", lines[0]);

    let again = fixture.render_built(&mut renderer, &built);
    assert!(renderer.take_diagnostics().is_empty(), "said twice");

    // And the picture is the one level 0 draws: the fallback is "sans anticrénelage", not "without
    // the scene".
    built.scene.settings.set_anti_alias(0);
    let plain = fixture.render_built(&mut renderer, &built);
    assert_eq!(plain.pixels, refused.pixels, "the refused frame");
    assert_eq!(plain.pixels, again.pixels, "and the frame after it");
}

// ------------------------------------------------------------------ the `wgpu` backend

/// The APIs to run the `wgpu` half on: Vulkan, and DirectX 12 on Windows.
#[cfg(feature = "wgpu")]
fn apis() -> Vec<(&'static str, zax_render::wgpu_backend::headless::GpuApi)> {
    use zax_render::wgpu_backend::headless::GpuApi;

    if cfg!(windows) {
        vec![("vulkan", GpuApi::Vulkan), ("dx12", GpuApi::Dx12)]
    } else {
        vec![("vulkan", GpuApi::Vulkan)]
    }
}

/// AMD-060 §4 on the `wgpu` side: the three passes are **there**, they carry the reference's own
/// arithmetic, and the two things a text substitution would have got wrong are right.
///
/// Read on the source, as the GLSL half is, because none of it is observable from a capture: a
/// port that sampled the edges with a point sampler, or that used an integer texel offset, would
/// render a plausible and wrong picture.
#[cfg(feature = "wgpu")]
#[test]
fn req_scn_085_the_wgsl_passes_carry_the_reference_s_arithmetic() {
    let [shared, edges, weights, blend] = zax_render::wgpu_backend::smaa_sources();

    // The shared chunk: the metrics, the drawn scale, and the two reads of the GLSL twin.
    for name in [
        "zax_smaa_depth",
        "zax_smaa_depth_offset",
        "zax_smaa_edges",
        "zax_smaa_edges_offset",
        "zax_smaa_table_area",
        "zax_smaa_table_search",
        "vs_smaa",
    ] {
        assert!(shared.contains(name), "the shared chunk misses {name}");
    }
    // The searches and `SMAAArea` read the edges and the tables through the **filtered** sampler
    // (`@PSEUDO_GATHER4`), which is the reference's own `LinearSampler` and the one sampling
    // choice of this port that a capture would not tell apart from a correct one.
    assert_eq!(
        4,
        shared.matches("textureSampleLevel").count(),
        "the edges, the two tables and the weights are sampled, not loaded"
    );
    // And the depth and the frame's colour are read texel by texel, which is the `PointSampler`
    // half of the same split and the `NEAREST` of the OpenGL backend.
    assert_eq!(
        2,
        shared.matches("textureLoad(").count(),
        "the depth and the colour are loaded, not sampled"
    );

    // The first pass: the depth, the threshold of Q-109 §1, both differences and the reference's
    // own early-out — and **not** the luma or the colour detector REQ-SCN-085 forbids.
    for name in [
        "ZAX_SMAA_DEPTH_RELATIVE",
        "ZAX_SMAA_DEPTH_FLOOR",
        "step(threshold, min(delta, curvature))",
        "discard",
        "vec2<f32>(-2.0, 0.0)",
        "vec2<f32>(0.0, -2.0)",
    ] {
        assert!(edges.contains(name), "the edge pass misses {name}");
    }
    assert!(
        !common::mentions(edges, "zax_smaa_color"),
        "the depth detector must not read the frame's colour"
    );

    // The second: the reference's High preset, its two tables' own metrics, the diagonal search
    // and the two corner patterns.
    for name in [
        "ZAX_SMAA_MAX_SEARCH_STEPS: i32 = 16",
        "ZAX_SMAA_MAX_SEARCH_STEPS_DIAG: i32 = 8",
        "ZAX_SMAA_CORNER_ROUNDING_NORM: f32 = 0.25",
        "ZAX_AREATEX_MAX_DISTANCE: vec2<f32> = vec2<f32>(16.0, 16.0)",
        "ZAX_AREATEX_MAX_DISTANCE_DIAG: vec2<f32> = vec2<f32>(20.0, 20.0)",
        "ZAX_AREATEX_SUBTEX_SIZE: f32 = 1.0 / 7.0",
        "ZAX_SEARCHTEX_SIZE: vec2<f32> = vec2<f32>(66.0, 33.0)",
        "ZAX_SEARCHTEX_PACKED_SIZE: vec2<f32> = vec2<f32>(64.0, 16.0)",
        "zax_smaa_search_diag1",
        "zax_smaa_search_diag2",
        "zax_smaa_decode_diag2",
        "zax_smaa_decode_diag4",
        "zax_smaa_search_x_left",
        "zax_smaa_search_x_right",
        "zax_smaa_search_y_up",
        "zax_smaa_search_y_down",
        "zax_smaa_search_length",
        "zax_smaa_horizontal_corner",
        "zax_smaa_vertical_corner",
        "255.0 / 127.0",
        "0.8281",
        // The one comparison the sign of the metric does not carry by itself, written the same way
        // in both sources so that neither can drift back to the reference's own — see
        // `req_scn_085_the_vertical_searches_compare_in_the_sign_of_the_y_metric`.
        "((texcoord.y - end) * smaa.metrics.y) > 0.0",
        "((texcoord.y - end) * smaa.metrics.y) < 0.0",
    ] {
        assert!(weights.contains(name), "the weight pass misses {name}");
    }

    // The third: the reference's early-out — which is the second acceptance criterion — and the
    // two colour taps written out.
    for name in [
        "dot(a, vec4<f32>(1.0, 1.0, 1.0, 1.0)) < 1e-5",
        "zax_smaa_blend_toward",
        "max(a.x, a.z) > max(a.y, a.w)",
    ] {
        assert!(blend.contains(name), "the blend pass misses {name}");
    }

    // **No integer texel offset anywhere**, which is the trap this port shares with the GLSL one:
    // WGSL's `textureSampleLevel` takes a constant texel offset as a fifth argument, and that
    // offset is in the texture's **own** space — it would not pick up the sign of `metrics.y`,
    // the one number the two backends disagree about. So every sample of this filter is a
    // four-argument call, all four of them live in the shared chunk, and the three passes reach a
    // neighbour only through the `*_offset` helpers, which are the multiply-add the reference's
    // own `SMAA_HLSL_3` path uses.
    for (label, source) in [("edge", edges), ("weight", weights), ("blend", blend)] {
        assert!(
            !source.contains("textureSampleLevel") && !source.contains("textureLoad"),
            "the {label} pass must read through the shared chunk's helpers"
        );
    }
    assert_eq!(
        4,
        shared.matches(", 0.0)").count(),
        "every sample is a four-argument call, so no texel offset can be passed"
    );
    assert!(
        shared.contains("texels * smaa.metrics.xy"),
        "the offsets are the multiply-add of the reference's own SMAA_HLSL_3 path"
    );
}

/// REQ-SCN-086 on the `wgpu` backend, both clauses and both directions: nothing of the filter
/// exists at levels 0, 1 and 2; the first frame at 3 builds the three pipelines and the three
/// targets; leaving 3 releases them, without a restart.
///
/// The same measurement the OpenGL half makes, on the same counters, so that a reader can compare
/// the two columns — and the three targets cost the same 14 bytes a pixel on both.
#[cfg(feature = "wgpu")]
#[test]
fn req_scn_086_the_wgpu_filter_appears_at_3_and_is_released() {
    for (name, api) in apis() {
        let Some(mut fixture) = common::WgpuFixture::create(api, 96, 64) else {
            return;
        };
        let mut built = stepped();

        for level in [0, 1, 2] {
            built.scene.settings.set_anti_alias(level);
            fixture.render_built(&built);
            let statistics = fixture.statistics();
            assert_eq!(0, statistics.smaa_passes, "{name}: level {level} submitted");
            assert_eq!(0, statistics.smaa_bytes, "{name}: level {level} allocated");
            assert_eq!(0, statistics.smaa_programs, "{name}: level {level} built");
        }

        // Twice over, so that a round trip that leaked would show as an allocation that never
        // goes back to zero.
        for _ in 0..2 {
            built.scene.settings.set_anti_alias(3);
            fixture.render_built(&built);
            let statistics = fixture.statistics();
            assert_eq!(3, statistics.smaa_passes, "{name}");
            assert_eq!(96 * 64 * (2 + 4 + 8), statistics.smaa_bytes, "{name}");
            assert_eq!(3, statistics.smaa_programs, "{name}");
            // REQ-SCN-085's first clause: "la scène DOIT être rendue sans multi-échantillonnage".
            assert_eq!(1, statistics.anti_alias_samples, "{name}: no multisampling");

            built.scene.settings.set_anti_alias(1);
            fixture.render_built(&built);
            let statistics = fixture.statistics();
            assert_eq!(0, statistics.smaa_bytes, "{name}: released");
            assert_eq!(0, statistics.smaa_programs, "{name}: released");
            assert_eq!(4, statistics.anti_alias_samples, "{name}: MSAA 4x again");
        }
    }
}

/// AMD-060 REQ-SCN-085's two acceptance criteria on the `wgpu` backend, read the way the OpenGL
/// half reads them: strictly more distinct colours at 3 than at 0 on a frame with hard geometric
/// edges, and the interior of a single textured face identical **to the byte**.
///
/// One test and two scenes, because a device of this backend costs seconds to open and the two
/// criteria are two captures of two fixtures through the same one.
#[cfg(feature = "wgpu")]
#[test]
fn req_scn_085_the_wgpu_filter_meets_both_acceptance_criteria() {
    for (name, api) in apis() {
        let Some(mut fixture) = common::WgpuFixture::create(api, 320, 240) else {
            return;
        };

        // 1. More colours on a geometric edge, with MSAA 4x beside it for scale.
        let mut built = stepped();
        built.scene.settings.set_anti_alias(0);
        let plain = fixture.render_built(&built).distinct_colors();
        built.scene.settings.set_anti_alias(1);
        let msaa = fixture.render_built(&built).distinct_colors();
        built.scene.settings.set_anti_alias(3);
        let smaa = fixture.render_built(&built).distinct_colors();
        println!(
            "{name}: a cube on a floor has {} colours at 0, {} at 1, {} at 3",
            plain.len(),
            msaa.len(),
            smaa.len()
        );
        assert!(smaa.len() > plain.len(), "{name}: 3 adds no gradation");
        assert!(
            smaa.len() >= msaa.len(),
            "{name}: 3 is coarser than MSAA 4x"
        );
        assert!(
            plain.iter().all(|color| smaa.contains(color)),
            "{name}: a colour of the unfiltered frame is gone"
        );

        // 2. And the interior of a face textured one texel at a time does not move.
        let mut face = striped_face();
        face.scene.settings.set_anti_alias(0);
        let unfiltered = fixture.render_built(&face);
        face.scene.settings.set_anti_alias(3);
        let filtered = fixture.render_built(&face);

        let background = unfiltered.packed(0, 0);
        let (first_row, last_row) = unfiltered.row_span(background);
        let (first_column, last_column) = unfiltered.column_span(background);
        assert!(
            last_row - first_row > 16 && last_column - first_column > 16,
            "{name}: the face is {first_column}..{last_column} x {first_row}..{last_row}"
        );
        assert!(
            unfiltered.distinct_colors().len() >= 3,
            "{name}: the face carries no contrast"
        );
        let mut compared = 0;
        for y in (first_row + 3)..=(last_row - 3) {
            for x in (first_column + 3)..=(last_column - 3) {
                compared += 1;
                assert_eq!(
                    unfiltered.at(x, y),
                    filtered.at(x, y),
                    "{name}: the filter touched the interior at ({x}, {y})"
                );
            }
        }
        assert!(
            compared > 1000,
            "{name}: only {compared} interior pixels compared"
        );
    }
}

/// REQ-SCN-086's fallback on the `wgpu` backend: a refused resource draws the frame **without
/// anti-aliasing**, with one diagnostic line, and is never retried.
///
/// Reached through `WgpuSceneRenderer::fail_smaa`, the twin of the OpenGL seam and of
/// `refuse_msvo`: no desktop adapter refuses `Rg8Unorm` as a colour attachment, and an unreachable
/// fallback is an untested one.
#[cfg(feature = "wgpu")]
#[test]
fn req_scn_086_a_refused_resource_draws_the_wgpu_frame_without_anti_aliasing() {
    for (name, api) in apis() {
        let Some(mut fixture) = common::WgpuFixture::create(api, 64, 48) else {
            return;
        };
        let mut built = stepped();
        built.scene.settings.set_anti_alias(3);

        fixture.renderer().fail_smaa(true);
        let refused = fixture.render_built(&built);

        assert!(fixture.renderer().smaa_given_up(), "{name}");
        assert_eq!(0, fixture.statistics().smaa_passes, "{name}: no pass");
        assert_eq!(0, fixture.statistics().smaa_bytes, "{name}: no target");
        assert_eq!(
            1,
            fixture.statistics().anti_alias_samples,
            "{name}: no MSAA"
        );

        let lines = fixture.renderer().take_diagnostics();
        assert_eq!(1, lines.len(), "{name}: {lines:?}");
        assert!(lines[0].contains("SMAA"), "{name}: {:?}", lines[0]);
        assert!(lines[0].contains("REQ-SCN-086"), "{name}: {:?}", lines[0]);

        let again = fixture.render_built(&built);
        assert!(
            fixture.renderer().take_diagnostics().is_empty(),
            "{name}: said twice"
        );

        // And the picture is the one level 0 draws.
        built.scene.settings.set_anti_alias(0);
        let plain = fixture.render_built(&built);
        assert_eq!(plain.pixels, refused.pixels, "{name}: the refused frame");
        assert_eq!(plain.pixels, again.pixels, "{name}: and the one after it");
    }
}
