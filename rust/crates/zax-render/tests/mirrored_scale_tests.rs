//! FIX-3 (Q-069): an instance whose scale has a negative component is **mirrored**. The profile
//! of "Zelda II" scales its ground slab by −1 on z, and the renderer used to draw that solid
//! inside-out (port of `MirroredScaleTests.cs`).
//!
//! A box whose pivot is centred maps exactly onto itself when it is mirrored, so the capture of
//! the mirrored object has to be the capture of the ordinary one, shading and shadow included
//! (REQ-SCN-028, REQ-SCN-038, SPEC-04B REQ-E3D-406).

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, solid, Capture, GlFixture, SceneFixture};
use zax_engine3d::instance::RenderStyle;
use zax_engine3d::settings::{ZAlignment, ZLayer};
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::numerics::Vector3;

const BLACK: i32 = 0;

#[test] // REQ-SCN-038, REQ-SCN-028, SPEC-04B REQ-E3D-406 (Q-069)
fn a_mirrored_object_renders_the_same_solid_as_an_unmirrored_one() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    for depth in [8, 56] {
        for scale in [
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(-1.0, -1.0, -1.0),
        ] {
            let mut built = SceneFixture::new()
                .background(0x0F)
                .palette(3, 0x30)
                .pattern_block(solid(3, 0), 4, 4, depth, ProfileLayer::L1, 1.0)
                .rect(solid(3, 0), 14, 14, 4, 4)
                .build();
            built.scene.camera.current_mut().angles = Vector3::new(14.0, 10.0, 0.0);
            built.scene.camera.snap_to_setpoints();

            let plain = gl.render_built(&mut renderer, &built);
            assert!(plain.count_other(BLACK) > 1000, "the object is not drawn");

            let instance = built.instances(|_| true)[0];
            built.instance_mut(instance).scale = scale;
            built.rebuild_draw_list();
            let mirrored = gl.render_built(&mut renderer, &built);

            let differing = differing(&plain, &mirrored);
            assert_eq!(
                0, differing,
                "depth {depth} scaled by ({}, {}, {}): {differing} pixel(s) differ from the \
                 unmirrored solid — the object renders inside-out",
                scale.x, scale.y, scale.z
            );
        }
    }
}

#[test] // REQ-E3D-205, REQ-SCN-038 (Q-069)
fn a_mirrored_slab_stays_in_front_of_what_its_depth_interval_puts_behind_it() {
    // The arrangement of "Zelda II" in its side view: a deep ground slab mirrored on z, and a
    // shallow row of grass whose z offset puts it 32 units further away. The front face of the
    // slab is the nearest surface of the scene, so the pixels where the two overlap must be the
    // slab's colour and never the grass's.
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    // The grass is a sprite shape so that both objects can hold the same cells: a background
    // cell carries one tile only.
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .palette(17, 0x16)
        .pattern_block(solid(3, 0), 4, 4, 56, ProfileLayer::L1, 1.0)
        .pattern_block(solid(1, 1), 4, 4, 8, ProfileLayer::L1, 1.0)
        .rect(solid(3, 0), 14, 16, 4, 4);
    for row in 0..4 {
        for column in 0..4 {
            fixture.push_sprite(solid(1, 1), (14 + column) * 8, (16 + row) * 8, 0, false);
        }
    }

    let mut built = fixture.build();
    let objects = built.instances(|_| true);
    assert_eq!(2, objects.len());
    let (slab, grass) = (objects[0], objects[1]);

    built.instance_mut(slab).offset = Vec3::new(0.0, 0.0, -16.0);
    built.instance_mut(slab).scale = Vec3::new(1.0, 1.0, -1.0);
    built.instance_mut(grass).offset = Vec3::new(0.0, 0.0, 16.0);

    built.engine.settings_mut().layer_mut(ZLayer::L1).alignment = ZAlignment::Front;
    for id in [slab, grass] {
        let (instance, settings) = built.instance_and_settings_mut(id);
        instance.calculate_pos(settings);
    }

    // REQ-E3D-205 with the FRONT coefficient: the slab's front plane is 32 units nearer than
    // the grass's, whatever the mirroring does to the mesh inside that interval.
    let front = |id| {
        let instance = built.instance(id);
        f64::from(instance.center.z) - (f64::from(instance.depth) / 2.0)
    };
    assert_close(-16.0, front(slab), 5e-4);
    assert_close(16.0, front(grass), 5e-4);

    // Shadows off: the only question here is which surface wins the depth test, and the grass
    // would otherwise cast on the slab whether or not it is itself visible.
    built.scene.settings.shadows = false;
    built.engine.settings_mut().shadow = false;

    built.instance_mut(grass).style = RenderStyle::Hidden;
    built.rebuild_draw_list();
    let alone = gl.render_built(&mut renderer, &built);
    assert!(alone.count_other(BLACK) > 1000, "the slab is not drawn");

    built.instance_mut(grass).style = RenderStyle::Normal;
    built.rebuild_draw_list();
    let both = gl.render_built(&mut renderer, &built);

    let differing = differing(&alone, &both);
    assert_eq!(
        0, differing,
        "{differing} pixel(s) of the object 32 units behind the slab show through it — the \
         mirrored slab does not present its front face"
    );
}

/// Pixels more than two levels apart, the tolerance the other capture tests use.
fn differing(left: &Capture, right: &Capture) -> usize {
    let mut differing = 0;
    for y in 0..left.height {
        for x in 0..left.width {
            let (lr, lg, lb) = left.at(x, y);
            let (rr, rg, rb) = right.at(x, y);
            if (lr - rr).abs() > 2 || (lg - rg).abs() > 2 || (lb - rb).abs() > 2 {
                differing += 1;
            }
        }
    }
    differing
}
