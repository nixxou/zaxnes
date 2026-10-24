//! A renderer whose buffers keep more room than the target (`SceneRenderer::reserve_target`,
//! the automatic resolution): the scene drawn into part of them is the very picture of a
//! renderer sized exactly, with and without anti-aliasing.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, Built, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::geometry;

#[test]
fn a_picture_drawn_into_reserved_room_is_the_exact_one() {
    let gl = GlFixture::create(320, 240);
    for option in [0, 1] {
        let mut built = scene();
        built
            .scene
            .settings
            .set_anti_alias_samples(geometry::anti_alias_samples(option));

        let mut exact = gl.renderer();
        let expected = gl.render_built(&mut exact, &built);

        let mut roomy = gl.renderer();
        roomy.reserve_target(640, 480);
        let reserved = gl.render_built(&mut roomy, &built);
        assert!(expected == reserved, "anti-aliasing option {option}");

        // The room given back, the buffers are the target's again: same picture.
        roomy.reserve_target(0, 0);
        let given_back = gl.render_built(&mut roomy, &built);
        assert!(
            expected == given_back,
            "anti-aliasing option {option}, room given back"
        );
    }
}

/// A white block over the background, slanted in screen space, with a darker one beside it.
fn scene() -> Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .palette(2, 0x16)
        .pattern(solid(3, 0), 24, ProfileLayer::L1)
        .pattern(solid(2, 1), 16, ProfileLayer::L1)
        .rect(solid(3, 0), 14, 12, 4, 4)
        .rect(solid(2, 1), 6, 6, 3, 5)
        .build()
}
