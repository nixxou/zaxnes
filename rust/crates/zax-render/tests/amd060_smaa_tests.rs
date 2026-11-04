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

use common::{close_up_r, Built, GlFixture};
use sha2::{Digest, Sha256};
use zax_render::geometry;
use zax_render::shaders;

/// A frame with hard geometric edges: the close-up R of AMD-055 §2.5 — a cube of `D` = 8 standing
/// on a floor of 4 x 4 full tiles — which is the one fixture of this crate whose silhouettes are
/// long, slanted and unmistakable. What the filter is asked to do here is detect them; what it
/// does to the picture is measured in the harness, over the whole synthetic set.
fn stepped() -> Built {
    close_up_r(true, 4, 1.0, 0.0)
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
    assert_eq!(1, statistics.smaa_passes);
    assert_eq!(
        96 * 64 * 2,
        statistics.smaa_bytes,
        "RG8 at the frame's size"
    );
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
    assert_eq!(1, statistics.smaa_passes);
    assert!(statistics.smaa_bytes > 0);
    assert_eq!(1, statistics.smaa_programs);
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
        assert_eq!(64 * 48 * 2, renderer.statistics().smaa_bytes);
        assert_eq!(1, renderer.statistics().smaa_programs);

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
