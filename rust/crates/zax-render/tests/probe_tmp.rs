//! Scratch probe, not committed. Runs only with `ZAX_PROBE` set.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use common::{solid, Capture, GlFixture, SceneFixture};
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::numerics::Vector3;
use zax_render::style::StyleSettings;

fn block_and_cube(scale: Vec3, beside: f32, lift: f32, distance: f32) -> common::Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x00)
        .palette(2, 0x16)
        .pattern(solid(1, 0), 8, ProfileLayer::L2)
        .tile(solid(1, 0), 15 * 8, 14 * 8, 0)
        .pattern(solid(2, 0), 8, ProfileLayer::L2)
        .tile(solid(2, 0), 17 * 8, 14 * 8, 0)
        .build();

    let cube = |tile: &zax_engine3d::draw_list::DrawTile| tile.tile_center.x > 132.0;
    for id in built.instances(cube) {
        built.instance_mut(id).offset = Vec3::new(4.0 + beside - 16.0, 4.0 + lift, 4.0);
    }
    for id in built.instances(|tile| !cube(tile)) {
        let instance = built.instance_mut(id);
        instance.scale = scale;
        instance.offset = Vec3::new(4.0, 4.0, 4.0);
    }
    built.rebuild_draw_list();

    built.scene.settings.shadows = false;
    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.camera.set_fov(70.0);
    let setpoint = built.scene.camera.current_mut();
    setpoint.angles = Vector3::new(30.0, 35.0, 0.0);
    setpoint.target = Vector3::new(128.0, 128.0, 4.0);
    setpoint.set_distance(distance);
    built.scene.camera.snap_to_setpoints();
    built
}

fn changed(off: &Capture, on: &Capture) -> usize {
    let mut count = 0;
    for y in 0..off.height {
        for x in 0..off.width {
            if off.packed(x, y) != on.packed(x, y) {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn probe() {
    if std::env::var_os("ZAX_PROBE").is_none() {
        return;
    }
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    for (label, scale) in [
        ("8 units", Vec3::new(1.0, 1.0, 1.0)),
        ("4 units", Vec3::new(1.0, 1.0, 0.5)),
    ] {
        for (beside, lift) in [
            (10.0, 8.0),
            (10.0, 4.0),
            (14.0, 8.0),
            (10.0, 0.0),
            (18.0, 8.0),
        ] {
            let mut built = block_and_cube(scale, beside, lift, 30.0);
            let view = built
                .scene
                .camera
                .build_view(built.scene.view_clipping(), 4.0 / 3.0);
            let exported = zax_render::scene_export::objects(&built.scene, &built.frame(), &view);
            let boxes: Vec<_> = exported
                .iter()
                .map(|object| {
                    let mut lo = [f32::MAX; 3];
                    let mut hi = [f32::MIN; 3];
                    for p in &object.unbent {
                        for (axis, v) in [p.x, p.y, p.z].into_iter().enumerate() {
                            lo[axis] = lo[axis].min(v);
                            hi[axis] = hi[axis].max(v);
                        }
                    }
                    (object.floor, lo, hi)
                })
                .collect();
            let off = gl.render_built(&mut renderer, &built);
            built.scene.settings.style = StyleSettings {
                enhanced: true,
                reflections: true,
                ..StyleSettings::default()
            };
            let on = gl.render_built(&mut renderer, &built);
            println!(
                "{label}, beside {beside}, lift {lift}: {} pixels; boxes {boxes:?}",
                changed(&off, &on)
            );
        }
    }
}
