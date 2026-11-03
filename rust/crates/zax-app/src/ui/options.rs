//! The options window as data: the tabs, the rows of each tab and the bounds of their widgets
//! (AMD-010 REQ-UI-202, AMD-035 §1). It draws nothing, so a test can check it without a window.

use zax_render::style::{AmbientOcclusion, StyleSettings};
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
    /// RetroAchievements (Rust-only): the account and the switches.
    Achievements,
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
pub const EXCLUDED: [SettingId; 19] = [
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
    // Rust-only (owner request): a game's own overscan, in its options.
    SettingId::VideoOverscanTop,
    SettingId::VideoOverscanBottom,
    SettingId::VideoOverscanLeft,
    SettingId::VideoOverscanRight,
    // Rust-only: the account of RetroAchievements, written by its sign-in only.
    SettingId::AchievementsUsername,
    SettingId::AchievementsToken,
    // Rust-only (owner request): a handheld option, in its own options screen only.
    SettingId::VideoHalfRate3d,
    // Rust-only (owner request): the touch pad of Android, in the console's options only.
    SettingId::InputTouchPad,
    // AMD-059 REQ-LUA-116: no interface at all — they are read and written by a 2.0 script and
    // by nothing else. `vr.refresh_rate` needs no line, the Vr theme having no tab.
    SettingId::UiLibraryView,
    SettingId::VideoMsaa20,
    // The settings the Lua members of 3dSen 2.0 stand for whose effect belonged to a later lot
    // (RU-2: Lua surface only). Each one left this list in the lot that made it act, which is
    // also the lot that wrote its row of the options window: `video.max_queued_frames` in RU-3a,
    // which gave it the GPU frame queue of AMD-052 REQ-LOOP-077; `emu.run_ahead` in RU-3b, which
    // gave it the run-ahead of AMD-041 REQ-LOOP-064; the four curvature keys in RU-4a, which gave
    // them the curvature of AMD-043 REQ-SCN-067 — they are now rows of the "Geometry" group
    // (`GEOMETRY`); and `render.visual_style` with the twelve `render.style.*` in RU-5, which
    // gave them the Enhanced style of AMD-055 — they are now the "Style" group (`STYLE`).
];

/// The "Geometry" group of AMD-043 §6 and AMD-046 §3, in the order the two amendments give it:
/// the three angles, the arc centre, then the 2D-mode switch.
///
/// AMD-043 §6 asks for the group to be "marqué avancé" (SHOULD). The options window has no
/// advanced/basic split, so the group is marked by a heading of its own above its first row,
/// which is what [`geometry_heading`] gives the drawing site.
pub const GEOMETRY: [SettingId; 5] = [
    SettingId::SceneBendLr,
    SettingId::SceneBendTb,
    SettingId::SceneArcH,
    SettingId::SceneArcCenter,
    SettingId::SceneBend2dMode,
];

/// The heading to draw **before** the row of `id`, or `None` for a row that opens no group.
///
/// Only the first setting of [`GEOMETRY`] opens one, so a tab costs one comparison per row. A
/// pure function, so the grouping is tested without a window.
#[must_use]
pub fn geometry_heading(id: SettingId) -> Option<String> {
    (id == GEOMETRY[0]).then(|| strings::get(keys::GROUP_GEOMETRY))
}

/// The twelve parameters of the Enhanced style (AMD-055 REQ-SCN-079), in the order of its table
/// and of §4: the five of the occlusion, the pixel shadows, the voxel grid, then the four of the
/// reflections behind their switch.
///
/// They are a **collapsible group** titled "Style" (SPEC-10 REQ-UI-061 as amended by AMD-055 §3
/// point 6), opened by [`style_heading`] and placed right after the `Style` row itself, which is
/// placed right after the `Skybox` row ([`OptionsModel::new`]). Which of the twelve a frame shows
/// is [`style_row_visible`]'s answer, not this list's: REQ-SCN-079 shows the occlusion's four
/// settings only while `spl.ao` ≠ 0 and the reflections' four only while `spl.reflections` = 1.
pub const STYLE: [SettingId; 12] = [
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
];

/// The two style parameters AMD-055 §4 shows as a check box rather than as a list or a slider
/// (`spl.pixel_shadow` and `spl.reflections`, "case" in its UI column).
///
/// They stay whole numbers in the model — their domain is the stepped 0 to 1 that carries the
/// normalisation of REQ-SCN-079, and Lua reads and writes them as numbers — so what a check box
/// changes is only how the window draws them.
pub const SWITCHES: [SettingId; 2] = [
    SettingId::RenderStylePixelShadow,
    SettingId::RenderStyleReflections,
];

/// The heading of the collapsible "Style" group, to draw **before** the row of `id`, or `None`
/// for a row that opens no group (AMD-055 §3 point 6).
///
/// Only the first setting of [`STYLE`] opens it, so a tab costs one comparison per row — the
/// shape [`geometry_heading`] established.
#[must_use]
pub fn style_heading(id: SettingId) -> Option<String> {
    (id == STYLE[0]).then(|| strings::get(keys::GROUP_STYLE))
}

/// Whether `id` is a row of the "Style" group: the style itself or one of the twelve.
#[must_use]
pub fn is_style_row(id: SettingId) -> bool {
    id == SettingId::RenderVisualStyle || STYLE.contains(&id)
}

/// Whether the collapsible group is drawn at all: only while `Style` = `Enhanced`, which is what
/// REQ-UI-061 as amended says ("quand `Style` = `Enhanced`, les douze paramètres apparaissent à
/// la suite, dans un groupe repliable « Style »"). The `Style` row itself is always a row.
#[must_use]
pub fn style_group_shown(model: &SettingsModel) -> bool {
    StyleSettings::style_of(model.get(SettingId::RenderVisualStyle).as_int())
}

/// Whether the window shows the style row `id`, given the values in force (REQ-SCN-079's
/// visibility rules, and the amended acceptance criterion of REQ-UI-061).
///
/// `true` for every row that is not one of the twelve, so the drawing site can test it on each
/// row of a tab without asking first. With the default values and `Style` = `Enhanced` exactly
/// four of the twelve are shown — the occlusion, the pixel shadows, the grid and the reflections —
/// and the twelve with `spl.ao` ≠ 0 and `spl.reflections` = 1, which is what the criterion counts.
#[must_use]
pub fn style_row_visible(id: SettingId, model: &SettingsModel) -> bool {
    if !STYLE.contains(&id) {
        return true;
    }
    if !StyleSettings::style_of(model.get(SettingId::RenderVisualStyle).as_int()) {
        return false;
    }
    match id {
        SettingId::RenderStyleAoStrength
        | SettingId::RenderStyleAoRadius
        | SettingId::RenderStyleAoThickness
        | SettingId::RenderStyleAoQuality => model.get(SettingId::RenderStyleAo).as_int() != 0,
        SettingId::RenderStyleSsrShine
        | SettingId::RenderStyleSsrClarity
        | SettingId::RenderStyleSsrLength
        | SettingId::RenderStyleSsrQuality => {
            model.get(SettingId::RenderStyleReflections).as_int() != 0
        }
        _ => true,
    }
}

/// Why the style row `id` is shown greyed out, or `None` while it is live.
///
/// One row answers: `ao.thickness`, which REQ-SCN-079 presents "grisé **(D)**" for as long as the
/// *Mode MSVO* is rendered as Depth — REQ-SCN-083's declared fallback, and
/// [`zax_render::style::MSVO_PORTED`] is the one constant that says so. The setting itself is kept
/// and persisted (a write of 4 still reads back 4): what is greyed is the widget, and the note
/// beside it says why, as the resolution-scale row does under the direct render method.
#[must_use]
pub fn style_inert_note(id: SettingId, model: &SettingsModel) -> Option<String> {
    let thickness = id == SettingId::RenderStyleAoThickness
        && !zax_render::style::thickness_acts(AmbientOcclusion::of(
            model.get(SettingId::RenderStyleAo).as_int(),
        ));
    thickness.then(|| strings::get(keys::HINT_THICKNESS_NO_EFFECT))
}

/// The help sentence AMD-043 §6, AMD-046 §3 and AMD-055 §4 give for `id`, shown as a tooltip, or
/// `None` for a setting they give none for.
#[must_use]
pub fn help_of(id: SettingId) -> Option<String> {
    if let Some(key) = style_help_of(id) {
        return Some(strings::get(key));
    }
    let key = match id {
        SettingId::SceneBendLr => keys::HELP_BEND_LR,
        SettingId::SceneBendTb => keys::HELP_BEND_TB,
        SettingId::SceneArcH => keys::HELP_ARC_H,
        SettingId::SceneArcCenter => keys::HELP_ARC_CENTER,
        SettingId::SceneBend2dMode => keys::HELP_BEND_2D_MODE,
        // AMD-049 gives no UI wording, the original having no such setting; the sentence is
        // Zaxnes's own, and worth having because the switch shows nothing on a profile without a
        // backdrop (REQ-SCN-048: it is Lua-driven and off by default).
        SettingId::RenderBackdropLighting => keys::HELP_BACKDROP_LIGHTING,
        _ => return None,
    };
    Some(strings::get(key))
}

/// The help key AMD-055 §4 gives for the style and each of its twelve parameters — the thirteen
/// sentences of its label table, which are the amendment's own words and not Zaxnes's.
fn style_help_of(id: SettingId) -> Option<&'static str> {
    Some(match id {
        SettingId::RenderVisualStyle => keys::HELP_VISUAL_STYLE,
        SettingId::RenderStyleAo => keys::HELP_STYLE_AO,
        SettingId::RenderStyleAoStrength => keys::HELP_STYLE_AO_STRENGTH,
        SettingId::RenderStyleAoRadius => keys::HELP_STYLE_AO_RADIUS,
        SettingId::RenderStyleAoThickness => keys::HELP_STYLE_AO_THICKNESS,
        SettingId::RenderStyleAoQuality => keys::HELP_STYLE_AO_QUALITY,
        SettingId::RenderStylePixelShadow => keys::HELP_STYLE_PIXEL_SHADOW,
        SettingId::RenderStyleVoxelGrid => keys::HELP_STYLE_VOXEL_GRID,
        SettingId::RenderStyleReflections => keys::HELP_STYLE_REFLECTIONS,
        SettingId::RenderStyleSsrShine => keys::HELP_STYLE_SSR_SHINE,
        SettingId::RenderStyleSsrClarity => keys::HELP_STYLE_SSR_CLARITY,
        SettingId::RenderStyleSsrLength => keys::HELP_STYLE_SSR_LENGTH,
        SettingId::RenderStyleSsrQuality => keys::HELP_STYLE_SSR_QUALITY,
        _ => return None,
    })
}

/// The settings the window edits as a folder: a field and a button that picks it.
pub const FOLDERS: [SettingId; 1] = [SettingId::UiRomsFolder];

/// The profile settings the window shows all the same: the video region (REQ-UI-061).
pub const INCLUDED_PROFILE: [SettingId; 1] = [SettingId::EmuTvMode];

/// The settings whose changes play no validation sound: they change continuously while the
/// slider moves.
pub const SILENT: [SettingId; 2] = [SettingId::EmuGameSpeed, SettingId::CameraFov];

/// The tabs, in order.
pub const TABS: [OptionsTab; 7] = [
    OptionsTab::Video,
    OptionsTab::Camera,
    OptionsTab::Light,
    OptionsTab::Audio,
    OptionsTab::Emulation,
    OptionsTab::Interface,
    OptionsTab::Achievements,
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
        OptionsTab::Achievements => keys::TAB_ACHIEVEMENTS,
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
    /// Builds the rows from the catalogue: one per shown setting, in the catalogue's order — and
    /// then the one place where the panel's order and the catalogue's differ (see
    /// [`move_style_after_skybox`]).
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
        for (tab, list) in &mut rows {
            if *tab == OptionsTab::Video {
                move_style_after_skybox(list);
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
    // of REQ-UI-202 is for; the rest of the Render theme is video. AMD-049 REQ-SCN-075 goes with
    // it: it chooses the lighting model of one surface, so it belongs beside the light it uses
    // and not among the video settings.
    if matches!(
        descriptor.id(),
        SettingId::RenderShadow | SettingId::RenderBackdropLighting
    ) {
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
        SettingTheme::Achievements => Some(OptionsTab::Achievements),
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

/// Q-103: the note the window shows beside the Run-ahead row while the game being played forces
/// its **effective** value to 0 — `None` for every other row, and for that row when nothing
/// forces it. `forced` is `GameSession::forces_run_ahead_to_zero` of the running game.
///
/// The row itself stays live: the setting is kept and applies again to the next game, so there is
/// nothing to grey out, only something to say.
// Q-103
#[must_use]
pub fn forced_run_ahead_note(id: SettingId, forced: bool) -> Option<String> {
    (forced && id == SettingId::EmuRunAhead).then(text::run_ahead_forced_to_zero)
}

/// Moves the thirteen rows of the style to just after the `Skybox` row, keeping their own order
/// (SPEC-10 REQ-UI-061 as amended: "la grille de paramètres desktop reçoit, **après la ligne
/// `Skybox`**, une ligne `Style` … les douze paramètres apparaissent à la suite").
///
/// The catalogue's order is the order of the settings **file**, where the render keys come long
/// before the scene's; the panel's order is this requirement's, and the two have no reason to be
/// the same. This is the only place they differ, which is why it is a function of five lines over
/// the built list rather than a reshuffle of the catalogue — moving `render.visual_style` there
/// would change the order of every written settings file for a presentation reason.
///
/// The `Skybox` row carries the custom background colour with it: REQ-UI-061 makes the colour
/// button part of that same row (REQ-UI-063, "le bouton de couleur placé à droite de la liste
/// `Skybox`"), so the style goes after the pair.
fn move_style_after_skybox(list: &mut Vec<OptionsRow>) {
    let style: Vec<OptionsRow> = list
        .iter()
        .filter(|row| is_style_row(row.descriptor.id()))
        .copied()
        .collect();
    if style.is_empty() {
        return;
    }
    list.retain(|row| !is_style_row(row.descriptor.id()));
    // After the pair, or at the end of the tab when a build shows neither of them.
    let after = list
        .iter()
        .rposition(|row| {
            matches!(
                row.descriptor.id(),
                SettingId::SceneSkybox | SettingId::SceneCustomBgColor
            )
        })
        .map_or(list.len(), |last| last + 1);
    list.splice(after..after, style);
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
        // A stepped domain publishes its own step (AMD-040 §3), which is the one the widget has
        // to move by: without it a setting with no libretro list fell back on a twentieth of its
        // range, and a two-rung one (`scene.arc_center`) could not be moved at all. It also
        // un-sticks `emu.run_ahead` on the handheld's advanced page, which had the same gap.
        return (low, high, domain.step().unwrap_or(0.0));
    }
    // No bounds at all: the widget becomes a free numeric field.
    (0.0, 0.0, 0.0)
}
