//! The same shaders on OpenGL ES 3.0 (REQ-ARCH-030): every program compiles under the
//! `#version 300 es` header and a scene renders as it does on desktop GL 3.3 core. No C#
//! counterpart: the C# renderer only ran on desktop GL.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, Capture, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::offscreen::ContextApi;
use zax_render::scene::SkyboxKind;

/// Renders the shadow scene of REQ-SCN-030, then the same with each procedural skybox.
fn captures(api: ContextApi) -> Vec<Capture> {
    let gl = GlFixture::with_api(320, 240, api);
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

    [
        SkyboxKind::Default,
        SkyboxKind::Gradient,
        SkyboxKind::Fantasy,
    ]
    .into_iter()
    .map(|sky| {
        built.scene.settings.skybox = sky;
        gl.render_built(&mut renderer, &built)
    })
    .collect()
}

#[allow(clippy::cast_precision_loss)] // Pixel counts are far below 2^52.
/// Share of pixels whose three channels are within 2 levels of the other capture's.
fn matching(left: &Capture, right: &Capture) -> f64 {
    let close = left
        .pixels
        .chunks_exact(4)
        .zip(right.pixels.chunks_exact(4))
        .filter(|(a, b)| (0..3).all(|c| a[c].abs_diff(b[c]) <= 2))
        .count();
    close as f64 / (left.pixels.len() / 4) as f64
}

#[test] // REQ-ARCH-030, REQ-SCN-056
fn the_scene_renders_on_gles_3_0_as_on_desktop_gl_3_3() {
    let core = captures(ContextApi::DesktopCore);
    let gles = captures(ContextApi::Gles3);

    for (index, (desktop, embedded)) in core.iter().zip(&gles).enumerate() {
        let share = matching(desktop, embedded);
        assert!(
            share >= 0.995,
            "capture {index}: only {:.2} % of the pixels match between GL 3.3 and GLES 3.0",
            share * 100.0
        );
    }
}
