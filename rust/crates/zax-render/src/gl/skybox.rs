//! Draws the Gradient and Fantasy skyboxes of SPEC-06 REQ-SCN-049 to REQ-SCN-051.
//!
//! The programs and the empty vertex array are built on first use, so a renderer that never
//! leaves the Default or Color skybox pays nothing. Without scene assets the sky fields are
//! computed in the shader (AMD-024 REQ-PUB-003), which is what the publishable build does: no
//! cubemap, no image file. With them, the cubemaps SPEC-06 names are sampled, which is both the
//! user-supplied asset folder of `--assets` and the uniform test cubemaps the acceptance
//! criteria of REQ-SCN-051 inject.

use glow::VertexArray;
use zax_engine3d::palette::Rgb24;

use super::api::{Gl, GlError};
use super::cubemap_cache::CubemapCache;
use super::program::ShaderProgram;
use super::shaders;
use crate::assets::SceneAssets;
use crate::camera::CameraView;
use crate::color;
use crate::numerics::Vector2;
use crate::scene::{SceneFrame, SceneState, SkyboxKind};
use crate::sky::{self, SkyBodySettings};

/// The skybox pass; released when dropped.
#[derive(Debug)]
pub struct SkyboxPass {
    gl: Gl,
    cubemaps: Option<CubemapCache>,
    gradient: Option<ShaderProgram>,
    fantasy: Option<ShaderProgram>,
    vertex_array: Option<VertexArray>,
}

impl SkyboxPass {
    /// Creates the pass; `assets` are where the cubemaps come from, or `None` to compute the sky
    /// fields in the shader.
    pub fn new(gl: &Gl, assets: Option<Box<dyn SceneAssets>>) -> Self {
        Self {
            gl: gl.clone(),
            cubemaps: assets.map(|assets| CubemapCache::new(gl, assets)),
            gradient: None,
            fantasy: None,
            vertex_array: None,
        }
    }

    /// Draws the skybox of `scene` over the whole target, with the depth test off so that
    /// everything drawn afterwards covers it (REQ-SCN-058). Returns whether one was drawn.
    pub fn draw(
        &mut self,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
    ) -> Result<bool, GlError> {
        let kind = scene.settings.skybox;
        if !matches!(kind, SkyboxKind::Gradient | SkyboxKind::Fantasy) {
            return Ok(false);
        }

        let procedural = self.cubemaps.is_none();
        let gl = &self.gl;
        let program = if kind == SkyboxKind::Gradient {
            program(gl, &mut self.gradient, &shaders::gradient(procedural))?
        } else {
            program(gl, &mut self.fantasy, &shaders::fantasy(procedural))?
        };
        program.use_program();

        // The direction is rebuilt in scene coordinates: f + x·tan(fov/2)·aspect·right +
        // y·tan(fov/2)·up (REQ-SCN-004).
        let tan = (view.fov * std::f32::consts::PI / 360.0).tan();
        program.set_vec3("uRight", view.right);
        program.set_vec3("uUp", view.up);
        program.set_vec3("uForward", view.forward);
        program.set_vec2("uTanHalf", Vector2::new(tan * view.aspect, tan));
        bind_bodies(program, &scene.settings.sky_bodies);

        if kind == SkyboxKind::Gradient {
            bind_gradient(gl, program, self.cubemaps.as_mut(), scene)?;
        } else {
            bind_fantasy(gl, program, self.cubemaps.as_mut(), scene, frame)?;
        }

        let vertex_array = match self.vertex_array {
            Some(array) => array,
            None => *self.vertex_array.insert(gl.create_vertex_array()?),
        };
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.bind_vertex_array(Some(vertex_array));
        gl.draw_triangles(3);
        gl.bind_vertex_array(None);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_mask(true);
        Ok(true)
    }
}

impl Drop for SkyboxPass {
    fn drop(&mut self) {
        if let Some(array) = self.vertex_array {
            self.gl.delete_vertex_array(array);
        }
    }
}

fn program<'a>(
    gl: &Gl,
    slot: &'a mut Option<ShaderProgram>,
    source: &shaders::ProgramSource,
) -> Result<&'a mut ShaderProgram, GlError> {
    match slot {
        Some(program) => Ok(program),
        None => Ok(slot.insert(ShaderProgram::create(gl, source)?)),
    }
}

/// The sun, the moon and the clouds of REQ-PUB-003. Their gains are 0 unless the host has set
/// them, which is the « disabled » of REQ-SCN-050 (Q-064).
fn bind_bodies(program: &mut ShaderProgram, bodies: &SkyBodySettings) {
    program.set_vec3("uSunDirection", bodies.sun_direction());
    program.set_vec3("uSunLinear", color::rgb_to_linear(bodies.sun_color));
    program.set_f32("uSunGain", bodies.sun_gain());
    program.set_vec2(
        "uSunCos",
        disc_cosines(bodies.sun_radius_degrees(), bodies.disc_softness),
    );

    program.set_vec3("uMoonDirection", bodies.moon_direction());
    program.set_vec3("uMoonLinear", color::rgb_to_linear(bodies.moon_color));
    program.set_f32("uMoonGain", bodies.moon_gain());
    program.set_vec2(
        "uMoonCos",
        disc_cosines(bodies.moon_radius_degrees(), bodies.disc_softness),
    );

    program.set_vec3("uCloudLinear", color::rgb_to_linear(bodies.cloud_color));
    program.set_f32("uCloudGain", bodies.cloud_gain());
    program.set_f32("uCloudCoverage", bodies.cloud_coverage());
    program.set_f32("uCloudScale", bodies.cloud_scale);
}

/// The cosines of the inner and the outer cone of a disc of angular radius `degrees`,
/// feathered by `softness`.
fn disc_cosines(degrees: f32, softness: f32) -> Vector2 {
    let inner = degrees * std::f32::consts::PI / 180.0;
    let outer = inner * (1.0 + softness.clamp(0.0, 4.0));
    Vector2::new(inner.cos(), outer.min(std::f32::consts::PI).cos())
}

fn bind_gradient(
    gl: &Gl,
    program: &mut ShaderProgram,
    cubemaps: Option<&mut CubemapCache>,
    scene: &SceneState,
) -> Result<(), GlError> {
    // REQ-SCN-052: the sky follows the colour of the **displayed** frame.
    let m = scene.nes_background_color;
    program.set_vec3("uBackground", color::to_vector(m));
    program.set_f32("uStars", if sky::stars_visible(m) { 1.0 } else { 0.0 });

    // REQ-SCN-050, REQ-SCN-059: the twinkle follows real time, 3° per second.
    let angle = scene.real_time * sky::STAR_DEGREES_PER_SECOND * std::f32::consts::PI / 180.0;
    program.set_f32("uStarAngle", angle % (2.0 * std::f32::consts::PI));

    if let Some(cubemaps) = cubemaps {
        program.set_i32("uStarMap", 0);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(cubemaps.get("Stars_LP_01")?));
    }

    Ok(())
}

fn bind_fantasy(
    gl: &Gl,
    program: &mut ShaderProgram,
    cubemaps: Option<&mut CubemapCache>,
    scene: &SceneState,
    frame: &SceneFrame<'_>,
) -> Result<(), GlError> {
    let f0 = scene.nes_background_color;
    let entry = |index: usize| {
        frame
            .draw_list
            .map_or(Rgb24::default(), |list| list.palette()[index])
    };
    program.set_vec3("uF0", color::to_vector(f0));
    program.set_vec3("uF1", color::to_vector(entry(1)));
    program.set_vec3("uF2", color::to_vector(entry(2)));
    program.set_vec3("uF3", color::to_vector(entry(3)));
    program.set_f32("uGamma", sky::grey(f0));

    if let Some(cubemaps) = cubemaps {
        program.set_i32("uStarfield", 0);
        program.set_i32("uDensity", 1);
        program.set_i32("uDiffusion", 2);
        for (unit, name) in [
            (0, "Starfield"),
            (1, "NebulaDensity"),
            (2, "NebulaDiffusion"),
        ] {
            gl.active_texture(unit);
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(cubemaps.get(name)?));
        }
        gl.active_texture(0);
    }

    Ok(())
}
