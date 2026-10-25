//! The options window as data: the tabs, the rows of each tab and the bounds of their widgets
//! (AMD-010 REQ-UI-202, AMD-035 §1). It draws nothing, so a test can check it without a window.

use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::SettingDescriptor;
use zax_settings::meta::{SettingKind, SettingScope, SettingTheme};
use zax_settings::model::SettingsModel;

use super::keys;
use super::strings;
use super::text;

/// A tab of the options window (REQ-UI-202).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionsTab {
    /// Video, render and scene.
    Video,
    /// The camera.
    Camera,
    /// Light and shadows.
    Light,
    /// Audio.
    Audio,
    /// Emulation, input, conversion, clipping, layers.
    Emulation,
    /// The interface.
    Interface,
}

/// One setting of a tab, with the bounds of its widget.
#[derive(Debug, Clone, Copy)]
pub struct OptionsRow {
    /// The setting.
    pub descriptor: &'static SettingDescriptor,
    /// The lower bound of a numeric widget (0 when there is none).
    pub minimum: f64,
    /// The upper bound of a numeric widget (0 when there is none).
    pub maximum: f64,
    /// The step of a stepped numeric widget (0 for a continuous one).
    pub step: f64,
}

impl OptionsRow {
    /// The label of the setting, in the language in force.
    #[must_use]
    pub fn label(&self) -> String {
        text::label_of(self.descriptor)
    }

    /// Whether a profile load overwrites the setting (REQ-PERS-005).
    #[must_use]
    pub fn is_profile_scoped(&self) -> bool {
        self.descriptor.scope() == SettingScope::Profile
    }
}

/// The settings the window leaves out although they are user settings: the last browsed folder
/// and the window rectangle are state, and the menu music no longer exists (AMD-024
/// REQ-PUB-005).
pub const EXCLUDED: [SettingId; 9] = [
    SettingId::UiLastBrowseDir,
    SettingId::UiWindowRect,
    SettingId::AudioBgMusic,
    // Rust-only (owner request): the hacks belong to each game, in its hacks screen.
    SettingId::EmuHackOptions,
    SettingId::EmuBuiltinPatches,
    SettingId::EmuForcedPatch,
    SettingId::VideoWidescreen,
    SettingId::VideoWidescreenExtra,
    SettingId::VideoWidescreenAdvanced,
];

/// The settings the window edits as a folder: a field and a button that picks it.
pub const FOLDERS: [SettingId; 1] = [SettingId::UiRomsFolder];

/// The profile settings the window shows all the same: the video region (REQ-UI-061).
pub const INCLUDED_PROFILE: [SettingId; 1] = [SettingId::EmuTvMode];

/// The settings whose changes play no validation sound: they change continuously while the
/// slider moves.
pub const SILENT: [SettingId; 2] = [SettingId::EmuGameSpeed, SettingId::CameraFov];

/// The tabs, in order.
pub const TABS: [OptionsTab; 6] = [
    OptionsTab::Video,
    OptionsTab::Camera,
    OptionsTab::Light,
    OptionsTab::Audio,
    OptionsTab::Emulation,
    OptionsTab::Interface,
];

/// The title of `tab`, in the language in force.
#[must_use]
pub fn title_of(tab: OptionsTab) -> String {
    strings::get(match tab {
        OptionsTab::Video => keys::TAB_VIDEO,
        OptionsTab::Camera => keys::TAB_CAMERA,
        OptionsTab::Light => keys::TAB_LIGHT,
        OptionsTab::Audio => keys::TAB_AUDIO,
        OptionsTab::Emulation => keys::TAB_EMULATION,
        OptionsTab::Interface => keys::TAB_INTERFACE,
    })
}

/// The rows of every tab.
#[derive(Debug, Clone)]
pub struct OptionsModel {
    rows: Vec<(OptionsTab, Vec<OptionsRow>)>,
}

impl Default for OptionsModel {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionsModel {
    /// Builds the rows from the catalogue: one per shown setting, in the catalogue's order.
    #[must_use]
    pub fn new() -> Self {
        let mut rows: Vec<(OptionsTab, Vec<OptionsRow>)> =
            TABS.iter().map(|tab| (*tab, Vec::new())).collect();
        for descriptor in catalogue::descriptors() {
            let Some(tab) = tab_of(descriptor).filter(|_| is_shown(descriptor)) else {
                continue;
            };
            if let Some((_, list)) = rows.iter_mut().find(|(candidate, _)| *candidate == tab) {
                list.push(build_row(descriptor));
            }
        }
        Self { rows }
    }

    /// The rows of `tab`.
    #[must_use]
    pub fn rows(&self, tab: OptionsTab) -> &[OptionsRow] {
        self.rows
            .iter()
            .find(|(candidate, _)| *candidate == tab)
            .map_or(&[], |(_, list)| list)
    }

    /// Every row of every tab.
    pub fn all_rows(&self) -> impl Iterator<Item = &OptionsRow> {
        self.rows.iter().flat_map(|(_, list)| list)
    }

    /// Takes every setting of `tab` back to its default (REQ-UI-202).
    pub fn reset_tab(&self, model: &mut SettingsModel, tab: OptionsTab) {
        for row in self.rows(tab) {
            model.reset(row.descriptor.id());
        }
    }
}

/// Whether the window shows `descriptor`: a user setting a widget can edit, and the few profile
/// settings REQ-UI-061 keeps.
#[must_use]
pub fn is_shown(descriptor: &SettingDescriptor) -> bool {
    let id = descriptor.id();
    if EXCLUDED.contains(&id) {
        return false;
    }
    // The graphics API is a choice only where Vulkan and DirectX 12 are built in.
    if id == SettingId::VideoBackend && !cfg!(feature = "wgpu") {
        return false;
    }
    if INCLUDED_PROFILE.contains(&id) {
        return true;
    }
    descriptor.scope() == SettingScope::User && is_editable(descriptor)
}

/// The tab that holds `descriptor`, or none for a setting without a tab.
#[must_use]
pub fn tab_of(descriptor: &SettingDescriptor) -> Option<OptionsTab> {
    // The global shadow switch belongs with the light, which is what the tab "Lumière et ombres"
    // of REQ-UI-202 is for; the rest of the Render theme is video.
    if descriptor.id() == SettingId::RenderShadow {
        return Some(OptionsTab::Light);
    }
    match descriptor.theme() {
        SettingTheme::Video | SettingTheme::Render | SettingTheme::Scene => Some(OptionsTab::Video),
        SettingTheme::Camera => Some(OptionsTab::Camera),
        SettingTheme::Light => Some(OptionsTab::Light),
        SettingTheme::Audio => Some(OptionsTab::Audio),
        SettingTheme::Emulation
        | SettingTheme::Input
        | SettingTheme::Conversion
        | SettingTheme::Clipping
        | SettingTheme::Layers => Some(OptionsTab::Emulation),
        SettingTheme::Interface => Some(OptionsTab::Interface),
        _ => None,
    }
}

/// Whether a change of the setting `id` plays the validation sound (REQ-UI-202).
#[must_use]
pub fn plays_validation_sound(id: SettingId) -> bool {
    !SILENT.contains(&id)
}

/// Whether the window edits `row` as a folder.
#[must_use]
pub fn is_folder(row: &OptionsRow) -> bool {
    FOLDERS.contains(&row.descriptor.id())
}

fn is_editable(descriptor: &SettingDescriptor) -> bool {
    if FOLDERS.contains(&descriptor.id()) {
        return true;
    }
    match descriptor.kind() {
        SettingKind::Bool
        | SettingKind::Int
        | SettingKind::Float
        | SettingKind::Enum
        | SettingKind::Color => true,
        SettingKind::Text => descriptor
            .text_options()
            .is_some_and(|options| !options.is_empty()),
        _ => false,
    }
}

fn build_row(descriptor: &'static SettingDescriptor) -> OptionsRow {
    let (minimum, maximum, step) = bounds(descriptor);
    OptionsRow {
        descriptor,
        minimum,
        maximum,
        step,
    }
}

fn bounds(descriptor: &SettingDescriptor) -> (f64, f64, f64) {
    if !matches!(descriptor.kind(), SettingKind::Int | SettingKind::Float) {
        return (0.0, 0.0, 0.0);
    }
    if let Some(values) = descriptor
        .libretro_values()
        .filter(|values| values.len() > 1)
    {
        let first = values[0].value;
        let last = values[values.len() - 1].value;
        let step = (values[1].value - first).abs();
        return (first.min(last), first.max(last), step);
    }
    let domain = descriptor.domain();
    if let (Some(low), Some(high)) = (domain.minimum(), domain.maximum()) {
        return (low, high, 0.0);
    }
    // No bounds at all: the widget becomes a free numeric field.
    (0.0, 0.0, 0.0)
}
