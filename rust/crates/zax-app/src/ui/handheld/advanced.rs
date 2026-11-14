//! The settings of the desktop's options window the console offers as stepped `Setting` rows
//! (Rust-only, owner request): the anti-aliasing, the camera, the light and the rest, each moved
//! with the pad. A strong Android device can afford them.
//!
//! Until RO-3 these **were** a page: one flat list of 56 rows behind "Advanced options". RO-3
//! spread them over the tree of `super::options::OptionsPage`, so what is left here is the
//! catalogue ([`ROW_SETTINGS`]) and the three functions a `Setting` row is drawn and stepped with
//! ([`label`], [`value_text`], [`step`]). The catalogue still earns its keep: it is where a row's
//! **widget bounds** come from, and a page that showed a setting outside it would have no step to
//! move the value by.

use std::sync::LazyLock;

use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::SettingKind;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::options::OptionRow;
use crate::ui::options::{OptionsModel, OptionsRow};
use crate::ui::text;

/// Every setting the console offers as a `Setting` row, in the order the flat page of RO-2 listed
/// them. Where each one is **shown** since RO-3 is the business of `super::options`'s page tables;
/// this is the list of what a console row may hold, and nothing else.
///
/// Until RO-1 it was computed: four tabs of the desktop window (Video, Camera, Light and
/// Emulation) walked in order, keeping every row a pad can step. RO-1 reorganises those tabs for
/// the desktop alone, so the list was written out here instead. The filter the old computation
/// applied (a switch, a choice or a bounded number; never the graphics API, which is the desktop's
/// alone, and never a folder) is a test of this list rather than a run-time rule.
///
/// Adding a setting to the desktop window therefore does not add a console row: a console row is
/// an owner decision, written down here and placed by a page table.
pub const ROW_SETTINGS: [SettingId; 75] = [
    SettingId::VideoVSync,
    SettingId::VideoFrameRate,
    SettingId::VideoSkipFrame,
    SettingId::VideoRenderMethod,
    SettingId::VideoMaxQueuedFrames,
    SettingId::VideoResScale,
    SettingId::VideoAntiAlias,
    SettingId::RenderMeshAlgo,
    SettingId::SceneSkybox,
    SettingId::RenderVisualStyle,
    SettingId::RenderStyleAo,
    SettingId::RenderStyleAoStrength,
    SettingId::RenderStyleAoRadius,
    SettingId::RenderStyleAoThickness,
    SettingId::RenderStyleAoQuality,
    SettingId::RenderStylePixelShadow,
    SettingId::RenderStyleVoxelGrid,
    SettingId::RenderStyleReflections,
    SettingId::RenderStyleSsrShine,
    SettingId::RenderStyleSsrClarity,
    SettingId::RenderStyleSsrLength,
    SettingId::RenderStyleSsrQuality,
    SettingId::SceneBendLr,
    SettingId::SceneBendTb,
    SettingId::SceneArcH,
    SettingId::SceneArcCenter,
    SettingId::SceneBend2dMode,
    SettingId::VideoDynamicResolution,
    SettingId::VideoShaderResolution,
    SettingId::VideoWidescreenStrictEdges,
    SettingId::VideoRemoveSpriteLimit,
    SettingId::VideoOverscanNtscTop,
    SettingId::VideoOverscanNtscBottom,
    SettingId::VideoOverscanNtscLeft,
    SettingId::VideoOverscanNtscRight,
    SettingId::VideoOverscanPalTop,
    SettingId::VideoOverscanPalBottom,
    SettingId::VideoOverscanPalLeft,
    SettingId::VideoOverscanPalRight,
    SettingId::VideoOverscanMode,
    SettingId::VideoNesPalette,
    SettingId::CameraFov,
    SettingId::RenderShadow,
    SettingId::RenderBackdropLighting,
    SettingId::LightIntensity,
    SettingId::EmuGameSpeed,
    SettingId::EmuTvMode,
    SettingId::EmuAutoTvMode,
    // The engine of a game (owner request, 2026-11-12: the choice on the console as in the
    // window). A game's option like the run-ahead, so offered globally too (RO-3).
    SettingId::EmuEngine,
    // Lot CARTDB: offered on the console as in the window.
    SettingId::EmuCartridgeDb,
    SettingId::EmuPowerOnRam,
    SettingId::EmuPowerOnPpuRam,
    SettingId::EmuOamCorruption,
    SettingId::EmuPpuBusConflicts,
    SettingId::EmuOamaddrBug,
    SettingId::EmuOamDecay,
    SettingId::EmuPowerOnTop,
    SettingId::EmuPpu2006Delay,
    SettingId::EmuPpu2006Glitch,
    SettingId::EmuPpu2000Glitch,
    SettingId::EmuPaletteCorruption,
    SettingId::EmuPpuResetLine,
    SettingId::EmuCpuPpuAlignment,
    SettingId::EmuRunAhead,
    // KIRBY-MERGE: the experimental smooth scrolling from the run-ahead. A console row because
    // RO-3's trees are one tree — `ro_3_every_setting_a_game_can_set_has_a_global_value_to_point_at`
    // holds every `Setting` row of a game's options to being offered globally too, and this is a
    // game's option (`game_options::GAME_EMULATION_ROWS`). Its debugging view stays desktop-only.
    SettingId::EmuSmoothScrollAhead,
    SettingId::EmuCatchUp,
    SettingId::EmuLuaSmoothGc,
    SettingId::EmuLuaOptimized,
    SettingId::EmuLuaExperimental,
    SettingId::EmuPipeline,
    SettingId::EmuPause3dIn2d,
    SettingId::EmuCatchUpLast3d,
    SettingId::InputEmulatedMenuButton,
    // AUDIO-2: the fallback to the output of `v0.4.2`. The work order asks for it on the console
    // screen as well as in the desktop window, and this is the device the new output is most
    // likely to misbehave on.
    SettingId::AudioClassicOutput,
    SettingId::AudioConsoleProfile,
];

/// Whether the setting is a whole number the page has to step as a **list** rather than as a
/// number: its domain publishes named values but no bounds, so there is no step to move by.
///
/// `render.visual_style` is the one such setting (AMD-055 REQ-SCN-078: the two values 0 and 4,
/// with 1 accepted in a write — an `aliased_choice`, which has no minimum and no maximum). The
/// twelve style parameters are *not* in this case: their domain is stepped, so the page moves
/// them one rung at a time like any other number.
fn is_named_list(row: &OptionsRow) -> bool {
    row.descriptor.kind() == SettingKind::Int
        && row.maximum <= row.minimum
        && row
            .descriptor
            .domain()
            .options()
            .is_some_and(|options| options.len() > 1)
}

/// [`ROW_SETTINGS`] with the widget bounds of the options model, which is what [`step`] moves by.
static SETTINGS: LazyLock<Vec<OptionsRow>> = LazyLock::new(|| {
    let model = OptionsModel::new();
    ROW_SETTINGS
        .iter()
        .filter_map(|id| {
            model
                .all_rows()
                .find(|row| row.descriptor.id() == *id)
                .copied()
        })
        .collect()
});

static ROWS: LazyLock<Vec<OptionRow>> = LazyLock::new(|| {
    SETTINGS
        .iter()
        .map(|row| OptionRow::Setting(row.descriptor.id()))
        .collect()
});

/// Every setting of [`ROW_SETTINGS`] as a `Setting` row: the settings the console offers, whatever
/// page of the RO-3 tree each one is shown on.
#[must_use]
pub fn rows() -> &'static [OptionRow] {
    &ROWS
}

fn row_of(id: SettingId) -> Option<&'static OptionsRow> {
    SETTINGS.iter().find(|row| row.descriptor.id() == id)
}

/// The label of the setting `id`, in the language in force.
#[must_use]
pub fn label(id: SettingId) -> String {
    text::label_of(catalogue::descriptor(id))
}

/// The choices of an enumerated setting, as (value, label).
fn choices(id: SettingId) -> Vec<(i32, String)> {
    catalogue::descriptor(id)
        .domain()
        .options()
        .unwrap_or_default()
        .iter()
        .map(|option| {
            #[allow(clippy::cast_possible_truncation)] // A choice's index.
            let value = option.value as i32;
            (value, option.label.clone())
        })
        .collect()
}

/// The value of the setting `id` as the page shows it.
#[must_use]
pub fn value_text(id: SettingId, settings: &SettingsModel) -> String {
    let value = settings.get(id);
    // RO-5 part 2: `scene.arc_center` is a real whose five rungs carry names, so it reads
    // "Screen Center", "50 %", "Screen Edge" and not "0", "0.5", "1". The list is the window's
    // own (`crate::ui::options::LABELLED_FRACTIONS`), so the two interfaces cannot disagree.
    if crate::ui::options::is_labelled_fraction(id) {
        let current = value.as_double();
        if let Some(label) = catalogue::descriptor(id).domain().label_of(current) {
            return text::label_of_option(label);
        }
    }
    match catalogue::descriptor(id).kind() {
        SettingKind::Bool => text::label_of_option(if value.as_bool() {
            "enabled"
        } else {
            "disabled"
        }),
        // A whole number whose domain publishes labelled values is a **list**, exactly as the
        // desktop window reads it (`draw_row`'s `enumerated`): `scene.arc_center` shows
        // "Screen Center" / "Screen Edge" (AMD-043 §6) and `emu.run_ahead` the four names of
        // AMD-041 §3, instead of the bare rung. A rung the domain labels with its own number —
        // the three curvature angles — falls back on that number, which is what it should show.
        SettingKind::Enum | SettingKind::Int => {
            let current = value.as_int();
            choices(id)
                .into_iter()
                .find(|(value, _)| *value == current)
                .map_or_else(
                    || current.to_string(),
                    |(_, label)| text::label_of_option(&label),
                )
        }
        _ => {
            let shown = format!("{:.2}", value.as_double());
            shown.trim_end_matches('0').trim_end_matches('.').to_owned()
        }
    }
}

/// Moves the selection of an enumerated setting by `step`, wrapping at both ends. `whole` writes
/// the value as a number rather than as a choice, which is what a whole-number list wants
/// (`render.visual_style`, and what Lua reads).
fn cycle(id: SettingId, settings: &mut SettingsModel, step: i32, whole: bool) {
    let choices = choices(id);
    if choices.is_empty() {
        return;
    }
    let current = settings.get(id).as_int();
    let at = choices
        .iter()
        .position(|(value, _)| *value == current)
        .unwrap_or(0);
    let count = i32::try_from(choices.len()).unwrap_or(1);
    let at = i32::try_from(at).unwrap_or(0);
    let next = usize::try_from((at + step).rem_euclid(count)).unwrap_or(0);
    let picked = choices[next].0;
    settings.set(
        id,
        if whole {
            SettingValue::Int(picked)
        } else {
            SettingValue::Enum(picked)
        },
    );
}

/// The rungs a whole-number row moves between, or `None` when its domain publishes none (so the
/// row is a plain number with a step).
fn rungs_of(row: &OptionsRow) -> Option<Vec<i32>> {
    let options = row.descriptor.domain().options()?;
    (options.len() > 1).then(|| {
        options
            .iter()
            .map(|option| {
                #[allow(clippy::cast_possible_truncation)] // a rung of a whole-number domain
                let value = option.value.round() as i32;
                value
            })
            .collect()
    })
}

/// Moves `id` by `step` rungs of its domain and stops at the ends — never wraps, unlike
/// [`cycle`], because this is the branch that replaced an increment that clamped.
fn step_by_rung(id: SettingId, row: &OptionsRow, settings: &mut SettingsModel, step: i32) {
    let Some(rungs) = rungs_of(row) else {
        return;
    };
    let current = settings.get(id).as_int();
    // The rung the value holds, or the nearest one below it for a value off the grid.
    let at = rungs
        .iter()
        .position(|rung| *rung == current)
        .or_else(|| rungs.iter().rposition(|rung| *rung <= current))
        .unwrap_or(0);
    let last = rungs.len() - 1;
    let next = i32::try_from(at).unwrap_or(0).saturating_add(step);
    let next = usize::try_from(next).unwrap_or(0).min(last);
    settings.set(id, SettingValue::Int(rungs[next]));
}

/// Steps the setting `id` by `step`: a switch flips, a choice cycles, a whole number whose domain
/// publishes rungs moves by one rung, and any other number moves by its step (a twentieth of its
/// range when it has none). All of them stop at their bounds except a choice, which cycles.
pub fn step(id: SettingId, settings: &mut SettingsModel, step: i32) {
    let Some(row) = row_of(id) else {
        return;
    };
    if is_named_list(row) {
        cycle(id, settings, step, true);
        return;
    }
    match row.descriptor.kind() {
        SettingKind::Bool => {
            let on = settings.get(id).as_bool();
            settings.set(id, SettingValue::Bool(!on));
        }
        SettingKind::Enum => cycle(id, settings, step, false),
        // A whole number whose domain publishes its rungs moves from one rung to the next and
        // stops at the ends, which is what the three curvature angles of RO-1 point 7 need: their
        // rungs are 5° apart near 0 and 30° apart beyond, so no one increment describes them. For
        // every other such row — the twelve style parameters, the run-ahead, the arc centre —
        // moving by a rung and moving by the step are the same thing, which is what the test
        // `the_rung_walk_matches_the_old_step_for_every_evenly_spaced_row` holds it to.
        SettingKind::Int if rungs_of(row).is_some() => step_by_rung(id, row, settings, step),
        kind => {
            let increment = if row.step > 0.0 {
                row.step
            } else if let Some(step) = row.descriptor.domain().step().filter(|step| *step > 0.0) {
                // A stepped number the window shows as a list (the run-ahead): its own step.
                step
            } else if kind == SettingKind::Int {
                // A whole number moves by one at least: half a step rounds back where it was.
                ((row.maximum - row.minimum) / 20.0).max(1.0)
            } else {
                (row.maximum - row.minimum) / 20.0
            };
            let value = (settings.get(id).as_double() + f64::from(step) * increment)
                .clamp(row.minimum, row.maximum);
            #[allow(clippy::cast_possible_truncation)] // within the setting's bounds
            let value = if kind == SettingKind::Int {
                SettingValue::Int(value.round() as i32)
            } else {
                SettingValue::Float(value as f32)
            };
            settings.set(id, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_has_anti_aliasing_and_steps_it() {
        assert!(rows().contains(&OptionRow::Setting(SettingId::VideoAntiAlias)));
        assert!(!rows().contains(&OptionRow::Setting(SettingId::VideoBackend)));
        let mut settings = SettingsModel::new();
        let before = settings.get(SettingId::VideoAntiAlias).as_int();
        step(SettingId::VideoAntiAlias, &mut settings, 1);
        assert_ne!(before, settings.get(SettingId::VideoAntiAlias).as_int());
        step(SettingId::VideoAntiAlias, &mut settings, -1);
        assert_eq!(before, settings.get(SettingId::VideoAntiAlias).as_int());
    }

    /// RO-1 point 7 replaced the increment of a whole-number row with a walk along its rungs, so
    /// that the curvature angles can have an uneven grid. For every **evenly** spaced row of the
    /// page — the twelve style parameters, the run-ahead, the arc centre, the GPU frame queue —
    /// the two give the same answer, which is what makes the change invisible on the console.
    #[test]
    fn the_rung_walk_matches_the_old_step_for_every_evenly_spaced_row() {
        for row in SETTINGS.iter() {
            let id = row.descriptor.id();
            let Some(rungs) = rungs_of(row) else { continue };
            if row.descriptor.kind() != SettingKind::Int {
                continue;
            }
            // Only the evenly spaced ones: the three curvature angles are the exception this lot
            // creates, and they have a test of their own in `bend_options_tests.rs`.
            let even = row.step > 0.0
                && rungs
                    .windows(2)
                    .all(|pair| (f64::from(pair[1] - pair[0]) - row.step).abs() < 1e-9);
            if !even {
                continue;
            }
            let mut walked = SettingsModel::new();
            let mut stepped = SettingsModel::new();
            for direction in [1, -1, 1, 1, 1, 1, 1, 1, -1] {
                step(id, &mut walked, direction);
                // What the branch before RO-1 did: move by the step and clamp.
                let value = (stepped.get(id).as_double() + f64::from(direction) * row.step)
                    .clamp(row.minimum, row.maximum);
                #[allow(clippy::cast_possible_truncation)] // within the setting's bounds
                stepped.set(id, SettingValue::Int(value.round() as i32));
                assert_eq!(
                    stepped.get(id).as_int(),
                    walked.get(id).as_int(),
                    "{}",
                    row.descriptor.key()
                );
            }
        }
    }

    #[test]
    fn a_number_stops_at_its_bounds() {
        let Some(row) = SETTINGS
            .iter()
            .find(|row| row.descriptor.kind() == SettingKind::Float)
        else {
            return;
        };
        let id = row.descriptor.id();
        let mut settings = SettingsModel::new();
        for _ in 0..100 {
            step(id, &mut settings, 1);
        }
        assert!(settings.get(id).as_double() <= row.maximum + 1e-6);
    }
}
