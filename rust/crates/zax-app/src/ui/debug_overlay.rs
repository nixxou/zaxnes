//! The developer panel (F12, SPEC-10 REQ-UI-095, DEC-10-Q7): off by default, it lists what the
//! loop, the renderer and the mixer are doing.

use imgui::{Condition, Ui};
use zax_render::statistics::RenderStatistics;

use crate::application::AppHost;
use crate::emulation::rollback_history;
use crate::persistence::app_settings::AppSettings as _;
use crate::persistence::save_slots::QUICK_SLOT_ID;
use crate::persistence::save_state_file;

/// What the loop knows that the host does not.
#[derive(Debug, Clone, Copy)]
pub struct DebugInfo<'a> {
    /// Displayed frames per second.
    pub frames_per_second: f64,
    /// Milliseconds per displayed frame.
    pub frame_milliseconds: f64,
    /// Frames displayed since the start.
    pub rendered_frames: u64,
    /// Frames the scene was drawn for.
    pub scene_frames: u64,
    /// Frames the scene was not drawn for.
    pub skipped_frames: u64,
    /// The renderer's counters for the last scene frame.
    pub statistics: &'a RenderStatistics,
    /// The name of the GL renderer.
    pub gl_renderer: &'a str,
}

/// Draws the panel.
pub fn draw(ui: &Ui, host: &AppHost, chrome_height: f32, info: &DebugInfo<'_>) {
    let window = ui
        .window("Statistiques (F12)")
        .size([420.0, 340.0], Condition::FirstUseEver)
        .position([16.0, chrome_height + 16.0], Condition::FirstUseEver)
        .begin();
    if window.is_none() {
        return;
    }
    let settings = host.settings();
    let stats = &info.statistics;
    ui.text(format!(
        "{:.1} FPS  ({:.2} ms/frame)",
        info.frames_per_second, info.frame_milliseconds
    ));
    ui.text(format!(
        "render {}   emulated {}",
        info.rendered_frames,
        host.emulated_frames()
    ));
    ui.text(format!(
        "scene {}   skipped {}   target {} fps",
        info.scene_frames,
        info.skipped_frames,
        settings.frame_rate()
    ));
    ui.separator();
    let loading = if host.is_loading() {
        format!("  loading {:.0} pct", host.load_progress() * 100.0)
    } else {
        String::new()
    };
    ui.text(format!("state {:?}{loading}", host.state()));
    let fast_forward = if host.fast_forward.is_active() {
        format!("  fast forward {}", host.fast_forward.remaining())
    } else {
        String::new()
    };
    ui.text(format!(
        "speed s={} (k={:.2}){fast_forward}",
        settings.game_speed(),
        settings.speed_factor()
    ));
    let quick = host
        .slots
        .find(QUICK_SLOT_ID)
        .filter(|record| record.is_occupied())
        .map_or_else(
            || "empty".to_owned(),
            |record| save_state_file::format_timestamp(record.saved_at),
        );
    ui.text(format!(
        "rollback {}/{}   quick slot {quick}",
        host.rollback.count(),
        rollback_history::CAPACITY
    ));
    if let Some(title) = host.current_game_title() {
        ui.separator();
        ui.text_wrapped(title);
        if let Some(session) = host.session() {
            ui.text(format!(
                "{:?}  {} fps nominal  frame {}",
                session.core().region(),
                session.nominal_frame_rate(),
                session.core().frame_count()
            ));
        }
    }
    ui.separator();
    ui.text(format!(
        "tiles  opaque {}  alpha {}  HUD {}",
        stats.opaque_tiles, stats.transparent_tiles, stats.hud_tiles
    ));
    ui.text(format!(
        "casters {}   draw calls {}   tris {}",
        stats.shadow_casters, stats.draw_calls, stats.triangles
    ));
    if let Some(mixer) = host.mixer() {
        ui.separator();
        let track = mixer
            .current_track()
            .map_or_else(|| "none".to_owned(), |track| track.to_string());
        ui.text(format!(
            "audio {} Hz  emu gain {:.3}  music {:.4}  track {track}  underruns {}",
            mixer.sample_rate(),
            mixer.emulation_gain(),
            mixer.music_gain(),
            mixer.underrun_blocks
        ));
    }
    ui.separator();
    ui.text_wrapped(info.gl_renderer);
}
