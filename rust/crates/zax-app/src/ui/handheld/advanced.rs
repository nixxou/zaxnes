//! The "Advanced options" page of the console (Rust-only, owner request): the settings of the
//! desktop's options window a handheld does not otherwise show (anti-aliasing, the camera, the
//! light and the rest), each stepped with the pad. A strong Android device can afford them.

use std::sync::LazyLock;

use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::SettingKind;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::options::OptionRow;
use crate::ui::options::{self as desktop, OptionsModel, OptionsRow, OptionsTab};
use crate::ui::text;

/// The tabs of the desktop window the page takes its settings from.
const TABS: [OptionsTab; 4] = [
    OptionsTab::Video,
    OptionsTab::Camera,
    OptionsTab::Light,
    OptionsTab::Emulation,
];

/// The settings of those tabs a pad can step: switches, choices and bounded numbers. The
/// renderer's backend is the desktop's alone.
static SETTINGS: LazyLock<Vec<OptionsRow>> = LazyLock::new(|| {
    let model = OptionsModel::new();
    TABS.iter()
        .flat_map(|tab| model.rows(*tab).iter().copied())
        .filter(|row| {
            row.descriptor.id() != SettingId::VideoBackend
                && !desktop::is_folder(row)
                && match row.descriptor.kind() {
                    SettingKind::Bool | SettingKind::Enum => true,
                    SettingKind::Int | SettingKind::Float => row.maximum > row.minimum,
                    _ => false,
                }
        })
        .collect()
});

static ROWS: LazyLock<Vec<OptionRow>> = LazyLock::new(|| {
    SETTINGS
        .iter()
        .map(|row| OptionRow::Setting(row.descriptor.id()))
        .collect()
});

/// The rows of the page.
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

/// [`step`] for a setting the page has no row of (a game's own run-ahead): its domain's bounds
/// and step, round from the last value to the first.
fn step_by_domain(id: SettingId, settings: &mut SettingsModel, step: i32) {
    let domain = catalogue::descriptor(id).domain();
    let (Some(minimum), Some(maximum)) = (domain.minimum(), domain.maximum()) else {
        return;
    };
    let increment = domain.step().filter(|step| *step > 0.0).unwrap_or(1.0);
    let mut value = settings.get(id).as_double() + f64::from(step) * increment;
    if value > maximum {
        value = minimum;
    } else if value < minimum {
        value = maximum;
    }
    #[allow(clippy::cast_possible_truncation)] // within the setting's bounds
    let value = SettingValue::Int(value.round() as i32);
    settings.set(id, value);
}

/// The value of the setting `id` as the page shows it.
#[must_use]
pub fn value_text(id: SettingId, settings: &SettingsModel) -> String {
    let value = settings.get(id);
    match catalogue::descriptor(id).kind() {
        SettingKind::Bool => text::label_of_option(if value.as_bool() {
            "enabled"
        } else {
            "disabled"
        }),
        SettingKind::Enum => {
            let current = value.as_int();
            choices(id)
                .into_iter()
                .find(|(value, _)| *value == current)
                .map_or_else(
                    || current.to_string(),
                    |(_, label)| text::label_of_option(&label),
                )
        }
        SettingKind::Int => {
            // A stepped number with labels (the run-ahead's "2 frames") shows its label.
            let current = value.as_int();
            choices(id)
                .into_iter()
                .nth(usize::try_from(current).unwrap_or(usize::MAX))
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

/// Steps the setting `id` by `step`: a switch flips, a choice cycles, a number moves by its
/// step (a twentieth of its range when it has none) and stops at its bounds.
pub fn step(id: SettingId, settings: &mut SettingsModel, step: i32) {
    let Some(row) = row_of(id) else {
        step_by_domain(id, settings, step);
        return;
    };
    match row.descriptor.kind() {
        SettingKind::Bool => {
            let on = settings.get(id).as_bool();
            settings.set(id, SettingValue::Bool(!on));
        }
        SettingKind::Enum => {
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
            settings.set(id, SettingValue::Enum(choices[next].0));
        }
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
