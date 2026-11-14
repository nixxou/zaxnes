//! The options window as data: the tabs, the groups, the rows of each tab and the bounds of their
//! widgets (AMD-010 REQ-UI-202, AMD-035 §1). It draws nothing, so a test can check it without a
//! window.
//!
//! # The layout is a table, not a derivation (RO-1)
//!
//! Until RO-1 the window was derived from the catalogue: the theme of a setting chose its tab and
//! the catalogue's order chose its place, with one hand-written exception for the Style group. That
//! put 41 of the 70 rows on a single "Video" tab holding three unrelated families — what the NES
//! produces, how the picture reaches the screen, and what the 3D scene looks like — which is the
//! owner's complaint RO-1 answers.
//!
//! The layout is therefore [`LAYOUT`]: one line per tab, one entry per row or group heading, in the
//! order the window draws them. It is a **divergence from the tab list of REQ-UI-202**, recorded in
//! `docs/OPTIONS-LAYOUT.md` and raised with R1 as Q-107.
//!
//! No setting may leave the window on the way: `goldens/desktop-options-shown.txt` holds the set
//! the window showed at `0b0aa8c` and `tests/desktop_options_golden_tests.rs` compares it with
//! [`LAYOUT`] on every run.

use zax_render::style::{AmbientOcclusion, StyleSettings};
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::SettingDescriptor;
use zax_settings::meta::{SettingKind, SettingScope, SettingTheme};
use zax_settings::model::SettingsModel;

use super::keys;
use super::strings;
use super::text;
use crate::emulation::fceux_validated;
use crate::persistence::app_settings::AppSettings as _;
use crate::render_scaling;

/// A tab of the options window (REQ-UI-202 as diverged by RO-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionsTab {
    /// How the picture reaches the screen, and what it costs: the window, the refresh, the
    /// resolution, the graphics back end and the performance switches.
    Display,
    /// What the NES produces: the colour palette, the sprite limit, the overscan, the widescreen.
    NesPicture,
    /// What the 3D scene looks like: the style, the mesh algorithm, the background, the curvature.
    Render3d,
    /// Light and shadows, and the Enhanced effects that decide where light does not reach.
    Light,
    /// The camera.
    Camera,
    /// Audio.
    Audio,
    /// Emulation: speed, run-ahead, region, and the two settings nothing else wanted.
    Emulation,
    /// The interface.
    Interface,
    /// RetroAchievements (Rust-only): the account and the switches.
    Achievements,
}

/// A group of rows inside a tab.
///
/// A group with three rows or more is drawn as a **collapsible** header (the shape AMD-055 §3
/// point 6 asks of the Style group); a shorter one is a plain heading, which is what AMD-043 §6
/// asked of the Geometry group before it grew. [`OptionsGroup::is_collapsible`] is the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionsGroup {
    /// How large the scene is drawn and how its edges are smoothed.
    Resolution,
    /// What the picture costs: the switches of `docs/PERFORMANCE-HISTORY.md`.
    Performance,
    /// The picture's edges a television did not show.
    Overscan,
    /// The widescreen extension.
    Widescreen,
    /// The twelve parameters of the Enhanced style (AMD-055 §3 point 6). The group appears on two
    /// tabs: the six effects that change a surface are with the style, the six that decide where
    /// light does not reach are with the light.
    Style,
    /// The curvature of AMD-043 §6 and AMD-046 §3.
    Geometry,
    /// The video region.
    Region,
    /// What a player should not need: used on more than one tab.
    Advanced,
}

impl OptionsGroup {
    /// The title of the group, in the language in force.
    #[must_use]
    pub fn title(self) -> String {
        strings::get(self.key())
    }

    /// Its text key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Resolution => keys::GROUP_RESOLUTION,
            Self::Performance => keys::GROUP_PERFORMANCE,
            Self::Overscan => keys::GROUP_OVERSCAN,
            Self::Widescreen => keys::GROUP_WIDESCREEN,
            Self::Style => keys::GROUP_STYLE,
            Self::Geometry => keys::GROUP_GEOMETRY,
            Self::Region => keys::GROUP_REGION,
            Self::Advanced => keys::GROUP_ADVANCED,
        }
    }

    /// Whether the window draws it as a collapsible header rather than as a plain heading: a
    /// family of three rows or more is worth folding away, a shorter one is not.
    #[must_use]
    pub fn is_collapsible(self) -> bool {
        matches!(
            self,
            Self::Resolution | Self::Performance | Self::Overscan | Self::Style | Self::Geometry
        )
    }
}

/// A row that edits **several** settings of one quantity side by side (RO-1 point 6).
///
/// The eight overscan margins took eight rows of their own, one quantity each, which is four
/// fifths of a tab for a thing a player sets once. They are now two rows — one per region — with
/// the four sides beside each other under a short caption. Every one of the eight is still a
/// setting of its own, still edited by its own widget, and still carries its own tooltip: what
/// changes is where the widgets sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompactRow {
    /// The four margins of an NTSC game.
    OverscanNtsc,
    /// The four of a PAL game.
    OverscanPal,
}

/// The caption of each cell of a compact row, in the order [`CompactRow::settings`] gives them.
pub const COMPACT_CAPTIONS: [&str; 4] = [
    keys::CAPTION_TOP,
    keys::CAPTION_BOTTOM,
    keys::CAPTION_LEFT,
    keys::CAPTION_RIGHT,
];

impl CompactRow {
    /// The settings of the row, in the order they are drawn: top, bottom, left, right — the order
    /// `table.rs` lists them in, so nothing is reordered on the way.
    #[must_use]
    pub const fn settings(self) -> [SettingId; 4] {
        match self {
            Self::OverscanNtsc => [
                SettingId::VideoOverscanNtscTop,
                SettingId::VideoOverscanNtscBottom,
                SettingId::VideoOverscanNtscLeft,
                SettingId::VideoOverscanNtscRight,
            ],
            Self::OverscanPal => [
                SettingId::VideoOverscanPalTop,
                SettingId::VideoOverscanPalBottom,
                SettingId::VideoOverscanPalLeft,
                SettingId::VideoOverscanPalRight,
            ],
        }
    }

    /// The title of the row, in the language in force.
    #[must_use]
    pub fn title(self) -> String {
        strings::get(match self {
            Self::OverscanNtsc => keys::COMPACT_OVERSCAN_NTSC,
            Self::OverscanPal => keys::COMPACT_OVERSCAN_PAL,
        })
    }
}

/// One entry of a tab, as [`LAYOUT`] writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutEntry {
    /// One setting, one widget.
    Row(SettingId),
    /// Several settings of one quantity, side by side (RO-1 point 6).
    Compact(CompactRow),
    /// The heading of a group. Every row until the next heading belongs to it.
    Group(OptionsGroup),
}

impl LayoutEntry {
    /// The settings the entry places: one for a row, four for a compact one, none for a heading.
    #[must_use]
    pub fn settings(self) -> Vec<SettingId> {
        match self {
            Self::Row(id) => vec![id],
            Self::Compact(compact) => compact.settings().to_vec(),
            Self::Group(_) => Vec::new(),
        }
    }
}

/// The rows of the **Display** tab: what the picture costs to put on the screen.
const DISPLAY: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::UiFullscreen),
    LayoutEntry::Row(SettingId::VideoVSync),
    LayoutEntry::Row(SettingId::VideoFrameRate),
    LayoutEntry::Group(OptionsGroup::Resolution),
    LayoutEntry::Row(SettingId::VideoRenderMethod),
    LayoutEntry::Row(SettingId::VideoResScale),
    LayoutEntry::Row(SettingId::VideoDynamicResolution),
    LayoutEntry::Row(SettingId::VideoShaderResolution),
    LayoutEntry::Row(SettingId::VideoAntiAlias),
    LayoutEntry::Group(OptionsGroup::Performance),
    LayoutEntry::Row(SettingId::VideoSkipFrame),
    LayoutEntry::Row(SettingId::EmuCatchUp),
    LayoutEntry::Row(SettingId::EmuCatchUpLast3d),
    LayoutEntry::Row(SettingId::EmuPause3dIn2d),
    LayoutEntry::Row(SettingId::EmuLuaSmoothGc),
    LayoutEntry::Row(SettingId::EmuLuaOptimized),
    LayoutEntry::Row(SettingId::EmuPipeline),
    LayoutEntry::Row(SettingId::EmuLuaExperimental),
    LayoutEntry::Group(OptionsGroup::Advanced),
    LayoutEntry::Row(SettingId::VideoBackend),
    LayoutEntry::Row(SettingId::VideoMaxQueuedFrames),
];

/// The rows of the **NES picture** tab: what the console puts out, before any 3D.
const NES_PICTURE: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::VideoNesPalette),
    LayoutEntry::Row(SettingId::VideoRemoveSpriteLimit),
    LayoutEntry::Group(OptionsGroup::Overscan),
    LayoutEntry::Row(SettingId::VideoOverscanMode),
    LayoutEntry::Compact(CompactRow::OverscanNtsc),
    LayoutEntry::Compact(CompactRow::OverscanPal),
    LayoutEntry::Group(OptionsGroup::Widescreen),
    LayoutEntry::Row(SettingId::VideoWidescreenStrictEdges),
];

/// The rows of the **3D rendering** tab: the look of the scene.
///
/// The background comes first, then the style **right after the `Skybox` row and the colour button
/// that belongs to it** — which is where SPEC-10 REQ-UI-061 as amended by AMD-055 puts it, and the
/// reason the mesh algorithm is in an Advanced group at the end rather than at the top where its
/// family would otherwise suggest.
///
/// Of the twelve Enhanced parameters, the six here are the ones that change a **surface** — the
/// voxel grooves and the reflections of the floor. The other six decide where light does not
/// reach and are on [`OptionsTab::Light`] (see [`LIGHT`]).
const RENDER_3D: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::SceneSkybox),
    LayoutEntry::Row(SettingId::SceneCustomBgColor),
    LayoutEntry::Row(SettingId::RenderVisualStyle),
    LayoutEntry::Group(OptionsGroup::Style),
    LayoutEntry::Row(SettingId::RenderStyleVoxelGrid),
    LayoutEntry::Row(SettingId::RenderStyleReflections),
    LayoutEntry::Row(SettingId::RenderStyleSsrShine),
    LayoutEntry::Row(SettingId::RenderStyleSsrClarity),
    LayoutEntry::Row(SettingId::RenderStyleSsrLength),
    LayoutEntry::Row(SettingId::RenderStyleSsrQuality),
    LayoutEntry::Group(OptionsGroup::Geometry),
    LayoutEntry::Row(SettingId::SceneBendLr),
    LayoutEntry::Row(SettingId::SceneBendTb),
    LayoutEntry::Row(SettingId::SceneArcH),
    LayoutEntry::Row(SettingId::SceneArcCenter),
    LayoutEntry::Row(SettingId::SceneBend2dMode),
    LayoutEntry::Group(OptionsGroup::Advanced),
    LayoutEntry::Row(SettingId::RenderMeshAlgo),
];

/// The rows of the **Light and shadows** tab.
///
/// The owner's second complaint: the shadow switch and the backdrop lighting were here, the pixel
/// shadows and the ambient occlusion were on the Video tab under the Enhanced style, and nothing
/// said how they relate. They are now one tab, in the order the relationship reads in: the light,
/// the shadows, then the Enhanced effects — the pixel shadows, which only change **how** those
/// same shadows are drawn, and the ambient occlusion, which is a different effect altogether.
const LIGHT: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::LightIntensity),
    LayoutEntry::Row(SettingId::RenderShadow),
    LayoutEntry::Row(SettingId::RenderBackdropLighting),
    LayoutEntry::Group(OptionsGroup::Style),
    LayoutEntry::Row(SettingId::RenderStylePixelShadow),
    LayoutEntry::Row(SettingId::RenderStyleAo),
    LayoutEntry::Row(SettingId::RenderStyleAoStrength),
    LayoutEntry::Row(SettingId::RenderStyleAoRadius),
    LayoutEntry::Row(SettingId::RenderStyleAoThickness),
    LayoutEntry::Row(SettingId::RenderStyleAoQuality),
];

/// The rows of the **Emulation** tab.
///
/// AMD-010 REQ-UI-202 gave this tab the conversion, the clipping and the layers as well; every one
/// of those is profile-scoped, so none has ever been a row (REQ-UI-061 keeps the video region
/// alone). What is left is the speed, the run-ahead, the region and the two settings no other tab
/// wanted — which is why it is not split further.
const EMULATION: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::EmuGameSpeed),
    LayoutEntry::Row(SettingId::EmuRunAhead),
    // KIRBY-MERGE: the experimental smooth scrolling and its view, under the run-ahead, because
    // one of its frames is what the smoothing spends and neither does anything without it.
    LayoutEntry::Row(SettingId::EmuSmoothScrollAhead),
    LayoutEntry::Row(SettingId::EmuSmoothScrollView),
    LayoutEntry::Group(OptionsGroup::Region),
    LayoutEntry::Row(SettingId::EmuAutoTvMode),
    LayoutEntry::Row(SettingId::EmuTvMode),
    LayoutEntry::Group(OptionsGroup::Advanced),
    LayoutEntry::Row(SettingId::InputEmulatedMenuButton),
    LayoutEntry::Row(SettingId::EmuCore),
    // Lot FCEUX-23 step 5: the per-game engine. `is_shown` drops it off a build without the
    // `fceux-core` feature, as it drops the graphics API off a build without `wgpu`.
    LayoutEntry::Row(SettingId::EmuEngine),
    // Lot CARTDB: the cartridge database and the power-on RAM.
    LayoutEntry::Row(SettingId::EmuCartridgeDb),
    LayoutEntry::Row(SettingId::EmuPowerOnRam),
    LayoutEntry::Row(SettingId::EmuOamCorruption),
    LayoutEntry::Row(SettingId::EmuPpuBusConflicts),
    LayoutEntry::Row(SettingId::EmuOamaddrBug),
    LayoutEntry::Row(SettingId::EmuOamDecay),
    LayoutEntry::Row(SettingId::EmuPowerOnTop),
    LayoutEntry::Row(SettingId::EmuPpu2006Delay),
    LayoutEntry::Row(SettingId::EmuPpu2006Glitch),
    LayoutEntry::Row(SettingId::EmuPpu2000Glitch),
    LayoutEntry::Row(SettingId::EmuPaletteCorruption),
    LayoutEntry::Row(SettingId::EmuPpuResetLine),
    LayoutEntry::Row(SettingId::EmuCpuPpuAlignment),
];

/// The rows of the **Interface** tab, from the most common to the most advanced.
///
/// The editor mode comes last, after the profiler: like it, it is a tool for whoever makes a
/// profile rather than for whoever plays (EDITOR-1).
const INTERFACE: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::UiLanguage),
    LayoutEntry::Row(SettingId::UiRomsFolder),
    LayoutEntry::Row(SettingId::UiSortOption),
    LayoutEntry::Row(SettingId::UiShowFps),
    LayoutEntry::Row(SettingId::UiProfiler),
    LayoutEntry::Row(SettingId::UiEditorMode),
];

/// The rows of the **Audio** tab: the volume, then the output itself (AUDIO-1, AUDIO-2).
///
/// The latency comes before the fallback and is greyed while the fallback is on: the classic
/// output has the fixed four blocks it always had, and neither `audio.latency_ms` nor the
/// counters of Q-153 §6 reach it.
const AUDIO: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::AudioVolume),
    LayoutEntry::Group(OptionsGroup::Advanced),
    LayoutEntry::Row(SettingId::AudioLatencyMs),
    LayoutEntry::Row(SettingId::AudioClassicOutput),
    LayoutEntry::Row(SettingId::AudioConsoleProfile),
];

/// The rows of the **RetroAchievements** tab, under the account block the tab draws itself.
const ACHIEVEMENTS: &[LayoutEntry] = &[
    LayoutEntry::Row(SettingId::AchievementsEnabled),
    LayoutEntry::Row(SettingId::AchievementsSound),
    LayoutEntry::Row(SettingId::AchievementsChallenges),
    LayoutEntry::Row(SettingId::AchievementsUnofficial),
    LayoutEntry::Row(SettingId::AchievementsEncore),
];

/// The window, tab by tab: every row and every group heading, in the order they are drawn.
///
/// The nine tabs are a divergence from the six of SPEC-10 REQ-UI-202, decided by the owner and
/// recorded in `docs/OPTIONS-LAYOUT.md`.
// Q-107
pub const LAYOUT: [(OptionsTab, &[LayoutEntry]); 9] = [
    (OptionsTab::Display, DISPLAY),
    (OptionsTab::NesPicture, NES_PICTURE),
    (OptionsTab::Render3d, RENDER_3D),
    (OptionsTab::Light, LIGHT),
    (
        OptionsTab::Camera,
        &[LayoutEntry::Row(SettingId::CameraFov)],
    ),
    (OptionsTab::Audio, AUDIO),
    (OptionsTab::Emulation, EMULATION),
    (OptionsTab::Interface, INTERFACE),
    (OptionsTab::Achievements, ACHIEVEMENTS),
];

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

/// One entry of a built tab: a group heading, a row with its bounds, or a compact row with the
/// bounds of each of its cells.
#[derive(Debug, Clone, Copy)]
pub enum TabEntry {
    /// The heading of a group.
    Group(OptionsGroup),
    /// One row.
    Row(OptionsRow),
    /// Several settings of one quantity, side by side (RO-1 point 6).
    Compact(CompactRow, [OptionsRow; 4]),
}

/// The settings the window leaves out although they are user settings: the last browsed folder
/// and the window rectangle are state, and the menu music no longer exists (AMD-024
/// REQ-PUB-005).
pub const EXCLUDED: [SettingId; 24] = [
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
    SettingId::InputTouchScale,
    SettingId::InputTouchHapticDpad,
    SettingId::InputTouchHapticButtons,
    SettingId::InputTouchExtra,
    SettingId::UiOrientation,
    // AMD-059 REQ-LUA-116: no interface at all — they are read and written by a 2.0 script and
    // by nothing else. `vr.refresh_rate` needs no line, the Vr theme having no tab.
    SettingId::UiLibraryView,
    SettingId::VideoMsaa20,
];

/// The five curvature settings, in the order AMD-043 §6 and AMD-046 §3 give them: the three
/// angles, the arc centre, then the 2D-mode switch. They are the rows of the
/// [`OptionsGroup::Geometry`] group of [`RENDER_3D`].
pub const GEOMETRY: [SettingId; 5] = [
    SettingId::SceneBendLr,
    SettingId::SceneBendTb,
    SettingId::SceneArcH,
    SettingId::SceneArcCenter,
    SettingId::SceneBend2dMode,
];

/// The twelve parameters of the Enhanced style (AMD-055 REQ-SCN-079), in the order of its table
/// and of §4: the five of the occlusion, the pixel shadows, the voxel grid, then the four of the
/// reflections behind their switch.
///
/// Which of the twelve a frame shows is [`style_row_visible`]'s answer, not this list's:
/// REQ-SCN-079 shows the occlusion's four settings only while `spl.ao` ≠ 0 and the reflections'
/// four only while `spl.reflections` = 1, and all twelve only while the style is Enhanced. RO-1
/// splits **where** they are drawn over two tabs ([`RENDER_3D`] and [`LIGHT`]) and leaves those
/// three rules untouched.
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

/// The settings whose value is a **fraction** and whose rungs carry names, so that both interfaces
/// show the name rather than the number (RO-5 part 2: `scene.arc_center`'s five positions, from
/// "Screen Center" to "Screen Edge" by quarters).
///
/// An explicit list rather than "a real whose domain publishes rungs": `audio.volume`,
/// `camera.fov` and `light.intensity` are stepped reals too, and their rungs are labelled with
/// their own numbers, so that rule would turn each of them into a drop-down of twenty entries.
// Q-107 §8
pub const LABELLED_FRACTIONS: [SettingId; 1] = [SettingId::SceneArcCenter];

/// Whether `id` is drawn as a named list although its value is a real
/// ([`LABELLED_FRACTIONS`]).
#[must_use]
pub fn is_labelled_fraction(id: SettingId) -> bool {
    LABELLED_FRACTIONS.contains(&id)
}

/// Whether `id` is one of the twelve style parameters or the style itself.
#[must_use]
pub fn is_style_row(id: SettingId) -> bool {
    id == SettingId::RenderVisualStyle || STYLE.contains(&id)
}

/// Whether the rows of a [`OptionsGroup::Style`] group are drawn at all: only while
/// `Style` = `Enhanced`, which is what REQ-UI-061 as amended says ("quand `Style` = `Enhanced`,
/// les douze paramètres apparaissent à la suite, dans un groupe repliable « Style »"). The `Style`
/// row itself is always a row, on [`OptionsTab::Render3d`].
#[must_use]
pub fn style_group_shown(model: &SettingsModel) -> bool {
    StyleSettings::style_of(model.get(SettingId::RenderVisualStyle).as_int())
}

/// Why a [`OptionsGroup::Style`] group shows no row, or `None` while it shows them.
///
/// RO-1 keeps REQ-SCN-079's rule — in Classic the twelve are hidden, not greyed — but replaces the
/// silence there was with one sentence beside the heading, which is what the owner's second
/// complaint asked for. The heading itself is now drawn in both cases, so a player on the Light and
/// shadows tab can see that pixel shadows and ambient occlusion exist and what turns them on.
#[must_use]
pub fn style_group_note(model: &SettingsModel) -> Option<String> {
    (!style_group_shown(model)).then(|| strings::get(keys::HINT_STYLE_CLASSIC))
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
    if !style_group_shown(model) {
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

/// Why the row of `id` is drawn greyed out, or `None` while it is live.
///
/// RO-1 point 1: where one setting only acts while another is set a certain way, the row stays
/// where it is and keeps its value — the setting is persisted and applies again as soon as the
/// other one changes — and what is greyed is the widget, with the reason beside it. Each case below
/// was verified against the code path that reads the setting, never guessed; the comment names it.
///
/// Called once per drawn row, and the window is only drawn while it is open.
#[must_use]
pub fn inert_note(id: SettingId, model: &SettingsModel) -> Option<String> {
    inert_reason(id, model).map(strings::get)
}

/// [`inert_note`]'s key, so a test can name the reason it expects.
#[must_use]
pub fn inert_reason(id: SettingId, model: &SettingsModel) -> Option<&'static str> {
    if let Some(reason) = engine_reason(id, model) {
        return Some(reason);
    }
    let scale_applies = render_scaling::scale_applies(model.render_method());
    let optimized_lua = model.get(SettingId::EmuLuaOptimized).as_bool();
    Some(match id {
        // AMD-032 §3: under the direct method the scene is always drawn at the native resolution,
        // and `application.rs` arms the automatic resolution's governor only for the other method.
        SettingId::VideoResScale | SettingId::VideoDynamicResolution if !scale_applies => {
            keys::HINT_RES_SCALE_DIRECT
        }
        // `application.rs` then reads the governor's factor **instead of** `video.res_scale`, so
        // the scale row is inert a second way: while the automatic resolution is on.
        SettingId::VideoResScale if model.get(SettingId::VideoDynamicResolution).as_bool() => {
            keys::HINT_RES_SCALE_AUTOMATIC
        }
        // `renderer.rs`: with `render.shadow` off the shadow pass is skipped and `uReceiveShadow`
        // is 0 for every item, so the pixel-shadow displacement inside that branch never runs.
        SettingId::RenderStylePixelShadow if !model.get(SettingId::RenderShadow).as_bool() => {
            keys::HINT_NEEDS_SHADOWS
        }
        // AMD-055 REQ-SCN-079: `ao.thickness` is greyed while the MSVO mode is rendered as Depth
        // (REQ-SCN-083's declared fallback **(D)**, `zax_render::style::MSVO_PORTED`).
        SettingId::RenderStyleAoThickness
            if !zax_render::style::thickness_acts(AmbientOcclusion::of(
                model.get(SettingId::RenderStyleAo).as_int(),
            )) =>
        {
            keys::HINT_THICKNESS_NO_EFFECT
        }
        // `scripting::runtime`: the experimental paths are read as `optimized && experimental`.
        // `PipelineMode::of` returns `Off` outright without the optimised Lua, a pipelined frame
        // handing the Lua runtime to another thread.
        SettingId::EmuLuaExperimental | SettingId::EmuPipeline if !optimized_lua => {
            keys::HINT_NEEDS_LUA_OPTIMIZED
        }
        // `scene.rs`: the gradient and the fantasy sky are drawn from the game's own background
        // colour and palette, and cover the clear in full; the custom colour reaches neither.
        SettingId::SceneCustomBgColor
            if matches!(model.get(SettingId::SceneSkybox).as_int(), 1 | 2) =>
        {
            keys::HINT_BG_COLOR_UNUSED
        }
        // `space_bend.rs`: the pivot is read only inside `if self.arc.active`, and the arc is
        // inactive at an angle of 0.
        SettingId::SceneArcCenter if model.get(SettingId::SceneArcH).as_int() == 0 => {
            keys::HINT_NEEDS_ARC
        }
        // `scene.rs`: the switch chooses between the curvature and the identity, which are the
        // same thing while all three angles are 0.
        SettingId::SceneBend2dMode if !any_curvature(model) => keys::HINT_NEEDS_CURVATURE,
        // AUDIO-2: the classic output of `v0.4.2` has the fixed four blocks of 512 frames it
        // always had — `application.rs` never gives it the latency, and `stream.rs`, which is
        // where the prefill, the ramps and the counters live, is not open at all.
        SettingId::AudioLatencyMs if model.classic_audio_output() => keys::HINT_CLASSIC_AUDIO,
        // `jobs.rs`: the detection *writes* `emu.tv_mode` at the next ROM load, so whatever is
        // chosen here is overwritten rather than merely ignored.
        SettingId::EmuTvMode if model.get(SettingId::EmuAutoTvMode).as_bool() => {
            keys::HINT_REGION_AUTOMATIC
        }
        // Rule 2 of REQ-PERS-003: the value is taken modulo the number of cores, which is 1, so
        // every write reads back as 0 and nothing consumes it. The row is kept all the same — the
        // key is part of the settings file and of the Lua surface (`Setting.Core`).
        SettingId::EmuCore => keys::HINT_SINGLE_CORE,
        // DEC-08-6 and DEC-R2-D7: stored and handed to Lua with no effect of any kind. Kept for
        // the same reason as the core index, and greyed for the same reason — a live widget over
        // a setting nothing reads is the confusion this lot is about.
        SettingId::InputEmulatedMenuButton => keys::HINT_MENU_BUTTON_UNUSED,
        // `achievements/mod.rs`: without the switch there is no client at all, so the options are
        // never forwarded, no unlock can fire the sound, and the challenge view returns at once.
        SettingId::AchievementsEncore
        | SettingId::AchievementsUnofficial
        | SettingId::AchievementsSound
        | SettingId::AchievementsChallenges
            if !model.get(SettingId::AchievementsEnabled).as_bool() =>
        {
            keys::HINT_NEEDS_ACHIEVEMENTS
        }
        _ => return None,
    })
}

/// Why `id` does nothing on the engine chosen for the game in play, or `None` when the engine is
/// Zaxnes or `id` is not one of the controls only Zaxnes has.
///
/// The one list of those controls, shared by the desktop window ([`inert_reason`]) and the
/// console pages (`handheld::options::engine_lock`), so the two cannot disagree.
#[must_use]
pub fn engine_reason(id: SettingId, model: &SettingsModel) -> Option<&'static str> {
    if !engine_is_not_zaxnes(model) {
        return None;
    }
    Some(match id {
        // Lot FCEUX-23 step 5, §4 of `docs/review/FCEUX-ENGINE.md`: with FCEUX chosen for this
        // game the widescreen extension, the hack packs, the sprite limit and the run-ahead are
        // **shown disabled with the reason, not hidden** — "a hidden control reads as a bug".
        // Each is Zaxnes's own bus or PPU rather than a feature of an emulator, so
        // `EngineHandle` ignores it on the other engine (`Capabilities::FCEUX`).
        SettingId::VideoWidescreen
        | SettingId::VideoWidescreenExtra
        | SettingId::VideoWidescreenAdvanced
        | SettingId::VideoWidescreenStrictEdges => keys::HINT_FCEUX_NO_WIDESCREEN,
        SettingId::EmuBuiltinPatches | SettingId::EmuForcedPatch | SettingId::EmuHackOptions => {
            keys::HINT_FCEUX_NO_HACKS
        }
        SettingId::VideoRemoveSpriteLimit => keys::HINT_FCEUX_NO_SPRITE_LIMIT,
        SettingId::EmuRunAhead
        | SettingId::EmuSmoothScrollAhead
        | SettingId::EmuSmoothScrollView => keys::HINT_FCEUX_NO_RUN_AHEAD,
        _ => return None,
    })
}

/// The engine chosen for the game in play, from the setting.
///
/// Reads the setting rather than the live engine, deliberately: the options window is drawn from
/// the model, the setting is what the *next* load will use, and a user who has just chosen another
/// engine should see the consequence at once rather than after reloading. FCEUX chosen in a build
/// without it is Zaxnes, which is what that build plays the game on.
fn chosen_engine(model: &SettingsModel) -> zax_core_api::EngineKind {
    let chosen = zax_core_api::EngineKind::from_u8(
        u8::try_from(model.get(SettingId::EmuEngine).as_int()).unwrap_or(0),
    )
    .unwrap_or_default();
    if chosen == zax_core_api::EngineKind::Fceux && !cfg!(feature = "fceux-core") {
        return zax_core_api::EngineKind::Zaxnes;
    }
    chosen
}

/// Whether the engine in force for the game in play is **not** Zaxnes's own core, which is what
/// greys the four groups of controls above.
///
/// The widescreen extension, the hack packs, the sprite limit and the run-ahead are all patches to
/// *Zaxnes's* bus or PPU rather than features of an emulator, so neither FCEUX nor NesHawk has
/// them (`Capabilities::FCEUX` and `Capabilities::NESHAWK` agree on all four). The test is
/// therefore "is the engine Zaxnes" and not "is the engine FCEUX": a lot that adds a third engine
/// without those features needs no change here.
#[must_use]
pub fn engine_is_not_zaxnes(model: &SettingsModel) -> bool {
    chosen_engine(model) != zax_core_api::EngineKind::Zaxnes
}

/// Whether any of the three curvature angles of AMD-043 is set.
fn any_curvature(model: &SettingsModel) -> bool {
    [
        SettingId::SceneBendLr,
        SettingId::SceneBendTb,
        SettingId::SceneArcH,
    ]
    .iter()
    .any(|id| model.get(*id).as_int() != 0)
}

/// The detailed tooltip of `id`, in the language in force: RO-1 point 3.
///
/// Where an amendment gives a short sentence for the setting ([`help_of`]: the thirteen of
/// AMD-055 §4, the five of AMD-043 §6 and AMD-046 §3, and the backdrop lighting of AMD-049) that
/// sentence comes **first, unchanged** — those are the amendment's own words, which the tests of
/// AMD-055 and AMD-043 check — and the paragraph RO-1 writes follows it after a blank line. A
/// setting with no amendment sentence has the paragraph alone.
///
/// Desktop only: the console pages and Android are driven by a pad and a touch screen, which have
/// no hover. Built only while the row is hovered, so an open window costs nothing per frame for
/// the sixty-nine rows that are not.
#[must_use]
pub fn tooltip_of(id: SettingId) -> String {
    let detail = strings::lookup_or_none(&keys::detail(&id.key()));
    match (help_of(id), detail) {
        (Some(help), Some(detail)) => format!("{help}\n\n{detail}"),
        (Some(help), None) => help,
        (None, Some(detail)) => detail.to_owned(),
        (None, None) => String::new(),
    }
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
        // AUDIO-2: no amendment knows of either audio output, so the one line that says when to
        // tick this box is Zaxnes's own, as the setting is.
        SettingId::AudioClassicOutput => keys::HELP_CLASSIC_AUDIO_OUTPUT,
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
pub const TABS: [OptionsTab; 9] = [
    OptionsTab::Display,
    OptionsTab::NesPicture,
    OptionsTab::Render3d,
    OptionsTab::Light,
    OptionsTab::Camera,
    OptionsTab::Audio,
    OptionsTab::Emulation,
    OptionsTab::Interface,
    OptionsTab::Achievements,
];

/// The title of `tab`, in the language in force.
#[must_use]
pub fn title_of(tab: OptionsTab) -> String {
    strings::get(match tab {
        OptionsTab::Display => keys::TAB_DISPLAY,
        OptionsTab::NesPicture => keys::TAB_NES_PICTURE,
        OptionsTab::Render3d => keys::TAB_RENDER_3D,
        OptionsTab::Light => keys::TAB_LIGHT,
        OptionsTab::Camera => keys::TAB_CAMERA,
        OptionsTab::Audio => keys::TAB_AUDIO,
        OptionsTab::Emulation => keys::TAB_EMULATION,
        OptionsTab::Interface => keys::TAB_INTERFACE,
        OptionsTab::Achievements => keys::TAB_ACHIEVEMENTS,
    })
}

/// The rows and the group headings of every tab, built from [`LAYOUT`].
#[derive(Debug, Clone)]
pub struct OptionsModel {
    tabs: Vec<BuiltTab>,
}

#[derive(Debug, Clone)]
struct BuiltTab {
    tab: OptionsTab,
    entries: Vec<TabEntry>,
    rows: Vec<OptionsRow>,
}

impl Default for OptionsModel {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionsModel {
    /// Builds the window from [`LAYOUT`], dropping the entries this build does not show
    /// ([`is_shown`]: the graphics API off a `wgpu` build) and the group headings left with no row.
    #[must_use]
    pub fn new() -> Self {
        let tabs = LAYOUT
            .iter()
            .map(|(tab, layout)| {
                let mut entries: Vec<TabEntry> = Vec::with_capacity(layout.len());
                for entry in *layout {
                    match *entry {
                        LayoutEntry::Group(group) => {
                            // A heading whose whole family is out of this build says nothing.
                            if matches!(entries.last(), Some(TabEntry::Group(_))) {
                                entries.pop();
                            }
                            entries.push(TabEntry::Group(group));
                        }
                        LayoutEntry::Row(id) => {
                            let descriptor = catalogue::descriptor(id);
                            if is_shown(descriptor) {
                                entries.push(TabEntry::Row(build_row(descriptor)));
                            }
                        }
                        LayoutEntry::Compact(compact) => {
                            let cells = compact.settings().map(catalogue::descriptor);
                            if cells.iter().all(|descriptor| is_shown(descriptor)) {
                                entries.push(TabEntry::Compact(compact, cells.map(build_row)));
                            } else {
                                // A build that hides one of the four cannot compact the rest:
                                // the ones it does show stay rows of their own rather than
                                // vanish. No build does this today — the four margins of a
                                // region are plain user settings — and the layout test is what
                                // would catch one that did.
                                for descriptor in cells {
                                    if is_shown(descriptor) {
                                        entries.push(TabEntry::Row(build_row(descriptor)));
                                    }
                                }
                            }
                        }
                    }
                }
                if matches!(entries.last(), Some(TabEntry::Group(_))) {
                    entries.pop();
                }
                let rows = entries
                    .iter()
                    .flat_map(|entry| match entry {
                        TabEntry::Row(row) => vec![*row],
                        TabEntry::Compact(_, cells) => cells.to_vec(),
                        TabEntry::Group(_) => Vec::new(),
                    })
                    .collect();
                BuiltTab {
                    tab: *tab,
                    entries,
                    rows,
                }
            })
            .collect();
        Self { tabs }
    }

    /// The entries of `tab`, rows and group headings alike, in the order they are drawn.
    #[must_use]
    pub fn entries(&self, tab: OptionsTab) -> &[TabEntry] {
        self.tabs
            .iter()
            .find(|built| built.tab == tab)
            .map_or(&[], |built| &built.entries)
    }

    /// The rows of `tab`, without the group headings.
    #[must_use]
    pub fn rows(&self, tab: OptionsTab) -> &[OptionsRow] {
        self.tabs
            .iter()
            .find(|built| built.tab == tab)
            .map_or(&[], |built| &built.rows)
    }

    /// Every row of every tab.
    pub fn all_rows(&self) -> impl Iterator<Item = &OptionsRow> {
        self.tabs.iter().flat_map(|built| built.rows.iter())
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
    // The engine row was gated on `fceux-core` by lot FCEUX-23 step 5, because a build without it
    // had one engine and the row would have been a choice of one. Lot NESHAWK-REWRITE added
    // `zax-neshawk`, which is safe Rust behind no feature at all, so **every** build now has at
    // least two engines and the row is always a real choice. In a build without `fceux-core` the
    // FCEUX value is still offered and `EngineHandle::of_kind` falls back to Zaxnes with a line in
    // the log saying why, which is the mechanism that lot already built.
    if INCLUDED_PROFILE.contains(&id) {
        return true;
    }
    // The slang shader slots have a section of their own (the preset file, its parameters),
    // not one row per key.
    if descriptor.key().starts_with("shader.") {
        return false;
    }
    descriptor.scope() == SettingScope::User
        && has_a_tab(descriptor.theme())
        && is_editable(descriptor)
}

/// Whether a setting of `theme` belongs to the window at all.
///
/// Before RO-1 this was the catch-all of the theme-to-tab mapping: a setting whose theme had no
/// tab had no row. The window has no virtual-reality tab (REQ-PERS-011 freezes those settings),
/// no editor and no movie recorder, and the Internal theme is the derived state — which is why
/// `vr.refresh_rate` needs no line in [`EXCLUDED`] (AMD-059 REQ-LUA-116 says so).
fn has_a_tab(theme: SettingTheme) -> bool {
    matches!(
        theme,
        SettingTheme::Video
            | SettingTheme::Render
            | SettingTheme::Scene
            | SettingTheme::Camera
            | SettingTheme::Light
            | SettingTheme::Audio
            | SettingTheme::Emulation
            | SettingTheme::Input
            | SettingTheme::Conversion
            | SettingTheme::Clipping
            | SettingTheme::Layers
            | SettingTheme::Interface
            | SettingTheme::Achievements
    )
}

/// Every setting [`LAYOUT`] places, in the order the window draws it — a compact row contributing
/// its four cells.
#[must_use]
pub fn placed_settings() -> Vec<SettingId> {
    LAYOUT
        .iter()
        .flat_map(|(_, layout)| layout.iter().flat_map(|entry| entry.settings()))
        .collect()
}

/// The tab [`LAYOUT`] puts `id` in, or `None` for a setting it has no row for.
#[must_use]
pub fn tab_of(id: SettingId) -> Option<OptionsTab> {
    LAYOUT
        .iter()
        .find(|(_, layout)| layout.iter().any(|entry| entry.settings().contains(&id)))
        .map(|(tab, _)| *tab)
}

/// The group [`LAYOUT`] puts `id` in, or `None` for a row that belongs to no group.
#[must_use]
pub fn group_of(id: SettingId) -> Option<OptionsGroup> {
    let (_, layout) = LAYOUT
        .iter()
        .find(|(_, layout)| layout.iter().any(|entry| entry.settings().contains(&id)))?;
    let mut group = None;
    for entry in *layout {
        if let LayoutEntry::Group(candidate) = *entry {
            group = Some(candidate);
        } else if entry.settings().contains(&id) {
            return group;
        }
    }
    None
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

/// Lot FCEUX-48 step 2: the note the window shows beside the **engine** row, once FCEUX is the
/// engine chosen for the game, saying whether lot FCEUX-6's sweep validated this ROM for 3D on it
/// — `None` for every other row, for Zaxnes, and for a ROM whose CRC the application has not read
/// (`Status::Unknown`: "nothing is known" is not "not validated").
///
/// The row itself stays **live**, like `forced_run_ahead_note` and unlike `inert_note`: the engine
/// option is available for every game, which is the owner's choice (§7 of
/// `docs/review/FCEUX-ENGINE.md`), so there is nothing to grey out, only something to say.
// FCEUX-48
#[must_use]
pub fn fceux_validation_note(
    id: SettingId,
    model: &SettingsModel,
    status: fceux_validated::Status,
) -> Option<String> {
    // FCEUX in particular: the validated-games list of lot FCEUX-48 is FCEUX's own, and says
    // nothing about any other engine.
    if id != SettingId::EmuEngine || chosen_engine(model) != zax_core_api::EngineKind::Fceux {
        return None;
    }
    match status {
        fceux_validated::Status::Validated(_) => Some(text::fceux_validated()),
        fceux_validated::Status::NotValidated => Some(text::fceux_not_validated()),
        fceux_validated::Status::Unknown => None,
    }
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
