//! Builds a scene from a JSON frame description and a profile, without the ROM (port of
//! `RenderExport.FromJson`).

use std::fs;
use std::path::Path;

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::settings::RenderAlgorithm;
use zax_nes::FrameBatch;
use zax_profiles::profile::GameProfile;
use zax_render::scene::SceneState;

use super::frame_json;
use super::scenes::SyntheticScene;
use crate::HarnessError;

/// Converts the frame `path` describes with `profile` and builds its scene.
///
/// # Errors
///
/// [`HarnessError::Io`] when the file cannot be read, [`HarnessError::Format`] when it does not
/// describe a frame and [`HarnessError::Engine`] when the engine refuses the profile.
pub fn from_json(
    path: &Path,
    profile: &GameProfile,
    mesh: RenderAlgorithm,
) -> Result<SyntheticScene, HarnessError> {
    let frame = frame_json::read(&fs::read_to_string(path)?)?;

    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.settings_mut().render_algorithm = mesh;
    engine.two_dimensional_mode =
        profile.permanent_patterns.is_empty() && profile.temporary_patterns.is_empty();
    engine.load_profile(profile, true)?;
    engine.convert(
        &FrameBatch {
            records: &frame.records,
            palette: frame.palette,
            mask_x: 0,
            mask_y: 0,
            margin: 0,
        },
        0,
    );
    engine.publish();

    let mut scene = SceneState::default();
    scene.camera.snap_to_setpoints();
    scene.game_running = true;
    scene.clipping = engine.settings().clipping;
    scene.nes_background_color = engine.frame_color(0);
    let draw_list = engine.build_draw_list().clone();

    Ok(SyntheticScene {
        name: path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()),
        description: format!(
            "{} tile record(s) from {}",
            frame.records.len(),
            path.display()
        ),
        scene,
        engine,
        draw_list,
        nes_picture: Vec::new(),
    })
}
