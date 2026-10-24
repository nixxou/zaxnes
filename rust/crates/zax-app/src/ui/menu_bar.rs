//! The menu bar as data (SPEC-10 REQ-UI-200 to REQ-UI-205, AMD-010 §3): the menus, their
//! entries, which are enabled and which are checked. It draws nothing, so the tests check it
//! without a window.
//!
//! The entries do not hold closures over the host: a submenu names what it lists
//! ([`Submenu`]) and a choice names what makes it selected ([`Selection`]); the model reads the
//! host it is given at the moment it builds the children or answers.

use zax_nes::ControllerPortType;
use zax_render::camera::CameraMode;
use zax_settings::catalogue::{self, SettingId};

use super::strings;
use super::text;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs, Shortcut};
use crate::emulation::host::SlotPatchStatus;
use crate::emulation::speed_setting;
use crate::persistence::app_settings::{AppSettings, TvRegionSetting};
use crate::persistence::save_slots::{
    numbered_slot_id, SaveSlotRecord, EXPOSED_SLOT_COUNT, QUICK_SLOT_ID,
};
use crate::persistence::save_state_file;

/// What an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItemKind {
    /// Runs a command.
    Command,
    /// Runs a command and shows its state as a check mark.
    Toggle,
    /// One value of a set: a radio entry.
    Choice,
    /// Opens a submenu.
    Submenu,
    /// A horizontal rule.
    Separator,
}

/// The submenus whose entries depend on the state of the host, so are built when opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Submenu {
    /// The recent ROMs.
    RecentRoms,
    /// The slots to load.
    LoadState,
    /// The slots to save to.
    SaveState,
    /// The occupied slots to delete.
    DeleteState,
    /// The speed settings.
    Speed,
    /// The video regions.
    Region,
    /// The two controller ports.
    Ports,
    /// The device of one port.
    Port(usize),
    /// The camera mode.
    CameraMode,
    /// The skyboxes.
    Skybox,
    /// The volume steps.
    Volume,
}

/// What makes a [`MenuItemKind::Choice`] the selected one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// The game speed is this setting.
    Speed(i32),
    /// The video region is this one and not detected automatically.
    Region(TvRegionSetting),
    /// This port holds this device.
    Port(usize, ControllerPortType),
    /// The camera is in this mode.
    Camera(CameraMode),
    /// The skybox is this one.
    Skybox(i32),
    /// The volume is this percentage.
    Volume(i32),
}

/// One entry of a menu.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuItem {
    /// What it is.
    pub kind: MenuItemKind,
    /// The label, in the language in force.
    pub label: String,
    /// The command it runs; none for a note and a rule.
    pub command: Option<Command>,
    /// The argument of the command.
    pub args: CommandArgs,
    /// The shortcut shown beside the label.
    pub shortcut: String,
    /// For a choice, what selects it.
    pub selection: Option<Selection>,
    /// For a submenu, what it lists.
    pub submenu: Option<Submenu>,
    /// Shown in a warning colour: a save state made with the built-in hack set the other way
    /// (orange) or with another version of it (red).
    pub warning: MenuWarning,
}

/// The warning colour of a menu item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuWarning {
    /// None.
    #[default]
    None,
    /// Orange: it works once something is switched (the built-in hack).
    Switch,
    /// Red: it cannot work.
    Blocked,
}

impl MenuItem {
    fn plain(kind: MenuItemKind, label: String) -> Self {
        Self {
            kind,
            label,
            command: None,
            args: CommandArgs::default(),
            shortcut: String::new(),
            selection: None,
            submenu: None,
            warning: MenuWarning::None,
        }
    }

    /// A horizontal rule.
    #[must_use]
    pub fn rule() -> Self {
        Self::plain(MenuItemKind::Separator, String::new())
    }

    /// An entry that runs `command`, with its own label unless `label` is given.
    #[must_use]
    pub fn of(command: Command, label: Option<String>, args: CommandArgs) -> Self {
        Self {
            command: Some(command),
            args,
            shortcut: shortcut_text(command),
            ..Self::plain(
                if command.is_toggle() {
                    MenuItemKind::Toggle
                } else {
                    MenuItemKind::Command
                },
                label.unwrap_or_else(|| strings::get(&command.label_key())),
            )
        }
    }

    fn choice(command: Command, label: String, args: CommandArgs, selection: Selection) -> Self {
        Self {
            command: Some(command),
            args,
            selection: Some(selection),
            ..Self::plain(MenuItemKind::Choice, label)
        }
    }

    fn sub(label: String, submenu: Submenu) -> Self {
        Self {
            submenu: Some(submenu),
            ..Self::plain(MenuItemKind::Submenu, label)
        }
    }

    /// A line of text that does nothing, such as "(empty)".
    #[must_use]
    pub fn note(label: String) -> Self {
        Self::plain(MenuItemKind::Command, label)
    }
}

/// The shortcut shown beside `command` (REQ-UI-204).
fn shortcut_text(command: Command) -> String {
    match command.shortcut() {
        Shortcut::None => String::new(),
        Shortcut::Fixed(text) => text.to_owned(),
        Shortcut::Key(key) => strings::get(key),
    }
}

/// A menu of the bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    /// The title, in the language in force.
    pub title: String,
    /// The entries.
    pub items: Vec<MenuItem>,
}

/// The menus of the bar, rebuilt when the language changes (AMD-035 §1).
#[derive(Debug, Clone)]
pub struct MenuBarModel {
    menus: Vec<Menu>,
    built_for: u32,
}

impl Default for MenuBarModel {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuBarModel {
    /// Builds the menus in the language in force.
    #[must_use]
    pub fn new() -> Self {
        Self {
            menus: build_menus(),
            built_for: strings::generation(),
        }
    }

    /// The menus, in the language in force.
    pub fn menus(&mut self) -> &[Menu] {
        if self.built_for != strings::generation() {
            self.built_for = strings::generation();
            self.menus = build_menus();
        }
        &self.menus
    }

    /// Every entry of every menu, the children of each submenu after it.
    pub fn all_items(&mut self, host: &AppHost) -> Vec<MenuItem> {
        let mut items = Vec::new();
        for menu in self.menus() {
            expand(&menu.items, host, &mut items);
        }
        items
    }

    /// Whether `item` can run now.
    #[must_use]
    pub fn is_enabled(item: &MenuItem, host: &AppHost) -> bool {
        item.command
            .is_some_and(|command| host.can_execute(command, &item.args))
    }

    /// Whether `item` shows a check mark: a toggle that is on, a choice that is selected.
    #[must_use]
    pub fn is_checked(item: &MenuItem, host: &AppHost) -> bool {
        match item.kind {
            MenuItemKind::Choice => item
                .selection
                .is_some_and(|selection| is_selected(selection, host)),
            MenuItemKind::Toggle => item.command.is_some_and(|command| host.is_checked(command)),
            _ => false,
        }
    }

    /// The entries of `submenu`, built from the state of `host` when it opens.
    #[must_use]
    pub fn children(submenu: Submenu, host: &AppHost) -> Vec<MenuItem> {
        match submenu {
            Submenu::RecentRoms => recent_rom_items(&recent_roms(host)),
            Submenu::LoadState => slot_items(Command::LoadSlot, Command::QuickLoad, host),
            Submenu::SaveState => slot_items(Command::SaveSlot, Command::QuickSave, host),
            Submenu::DeleteState => delete_slot_items(host),
            Submenu::Speed => speed_items(),
            Submenu::Region => region_items(),
            Submenu::Ports => port_items(),
            Submenu::Port(port) => port_device_items(port),
            Submenu::CameraMode => camera_items(),
            Submenu::Skybox => skybox_items(),
            Submenu::Volume => volume_items(),
        }
    }
}

fn expand(items: &[MenuItem], host: &AppHost, into: &mut Vec<MenuItem>) {
    for item in items {
        into.push(item.clone());
        if let Some(submenu) = item.submenu {
            expand(&MenuBarModel::children(submenu, host), host, into);
        }
    }
}

fn recent_roms(host: &AppHost) -> Vec<String> {
    host.settings_store()
        .map_or_else(Vec::new, |store| store.games().recent_roms().to_vec())
}

fn is_selected(selection: Selection, host: &AppHost) -> bool {
    let settings = host.settings();
    match selection {
        Selection::Speed(value) => settings.game_speed() == value,
        Selection::Region(region) => !settings.auto_tv_mode() && settings.tv_mode() == region,
        Selection::Port(port, device) => host.port_type(port) == device,
        Selection::Camera(mode) => host.scene().camera.mode() == mode,
        Selection::Skybox(value) => settings.skybox() == value,
        Selection::Volume(percent) => {
            // The C# `(int)MathF.Round(Volume * 100f)`: ties to even.
            #[allow(clippy::cast_possible_truncation)]
            let shown = (settings.volume() * 100.0).round_ties_even() as i32;
            shown == percent
        }
    }
}

fn build_menus() -> Vec<Menu> {
    let item = |command| MenuItem::of(command, None, CommandArgs::default());
    let file = Menu {
        title: text::menu_file(),
        items: vec![
            item(Command::OpenRom),
            MenuItem::sub(text::recent_roms(), Submenu::RecentRoms),
            MenuItem::rule(),
            item(Command::ChooseProfile),
            item(Command::ImportProfile),
            // AMD-030 §2: "Fichier → Importer depuis 3dSen…", right after the single-file import
            // it extends.
            item(Command::ImportThreeDSen),
            // SPEC-10 REQ-UI-037 with AMD-010 §3: the deletion of a user profile is one of the
            // functions the amendment keeps mandatory, and the tile button it describes becomes
            // a menu entry here: the presentation is informative, the function is not.
            item(Command::DeleteGame),
            MenuItem::rule(),
            // REQ-PERS-036 is P3 after AMD-020 (Q-050); the two sources a menu can offer.
            item(Command::CreateGameArt),
            item(Command::ImportCover),
            MenuItem::rule(),
            MenuItem::sub(text::load_state(), Submenu::LoadState),
            MenuItem::sub(text::save_state(), Submenu::SaveState),
            // REQ-UI-052: the slot deletion of SPEC-09 REQ-PERS-054 needs a route of its own,
            // since a classic menu has no selected tile to hang a small button on.
            MenuItem::sub(text::delete_state(), Submenu::DeleteState),
            MenuItem::rule(),
            item(Command::Quit),
        ],
    };
    let emulation = Menu {
        title: text::menu_emulation(),
        items: vec![
            item(Command::PauseResume),
            item(Command::Reset),
            item(Command::Stop),
            MenuItem::rule(),
            item(Command::FastForward),
            item(Command::Rollback),
            MenuItem::rule(),
            MenuItem::sub(text::speed(), Submenu::Speed),
            MenuItem::sub(text::region(), Submenu::Region),
            MenuItem::sub(text::ports(), Submenu::Ports),
        ],
    };
    let video = Menu {
        title: text::menu_video(),
        items: vec![
            item(Command::ToggleTwoDimensional),
            MenuItem::sub(text::camera_mode(), Submenu::CameraMode),
            item(Command::ResetView),
            MenuItem::rule(),
            MenuItem::sub(
                text::label_of(catalogue::descriptor(SettingId::SceneSkybox)),
                Submenu::Skybox,
            ),
            item(Command::ToggleShadows),
            MenuItem::rule(),
            item(Command::ToggleFullscreen),
            item(Command::ToggleFpsCounter),
        ],
    };
    // AMD-024 REQ-PUB-005 (revised 2026-10-21): there is no menu music any more, so the "Musique
    // de menu" entry is gone. The setting itself stays in the model for the compatibility of
    // settings.json, without effect.
    let audio = Menu {
        title: text::menu_audio(),
        items: vec![
            MenuItem::sub(text::volume(), Submenu::Volume),
            item(Command::ToggleMute),
        ],
    };
    let tools = Menu {
        title: text::menu_tools(),
        items: vec![
            item(Command::ShowOptions),
            item(Command::ShowKeyConfig),
            MenuItem::rule(),
            item(Command::ShowDataFolder),
            item(Command::ShowAbout),
        ],
    };
    vec![file, emulation, video, audio, tools]
}

fn recent_rom_items(roms: &[String]) -> Vec<MenuItem> {
    if roms.is_empty() {
        return vec![MenuItem::note(text::nothing_here())];
    }
    roms.iter()
        .enumerate()
        .map(|(index, rom)| {
            let name = std::path::Path::new(rom)
                .file_name()
                .map_or_else(|| rom.clone(), |name| name.to_string_lossy().into_owned());
            MenuItem::of(
                Command::OpenRecentRom,
                Some(name),
                CommandArgs::of_number(i32::try_from(index).unwrap_or(i32::MAX)),
            )
        })
        .collect()
}

fn slot_items(numbered: Command, quick: Command, host: &AppHost) -> Vec<MenuItem> {
    let mut items = Vec::with_capacity(EXPOSED_SLOT_COUNT + 2);
    for slot in 1..=slot_count() {
        let id = numbered_slot_id(slot);
        let status = host.slot_patch_status(&id);
        let mut label = slot_label(slot, host.slots.find(&id));
        let warning = match status {
            SlotPatchStatus::Fits => MenuWarning::None,
            SlotPatchStatus::OtherMode { .. } => {
                label = format!("{label} ({})", text::slot_other_patch());
                MenuWarning::Switch
            }
            SlotPatchStatus::OtherVersion => {
                label = format!("{label} ({})", text::slot_other_hack_version());
                MenuWarning::Blocked
            }
        };
        let mut item = MenuItem::of(numbered, Some(label), CommandArgs::of_number(slot));
        item.warning = warning;
        items.push(item);
    }
    items.push(MenuItem::rule());
    items.push(MenuItem::of(
        quick,
        Some(format!(
            "{} — {}",
            text::quick_slot(),
            state_stamp(host.slots.find(QUICK_SLOT_ID))
        )),
        CommandArgs::default(),
    ));
    items
}

fn delete_slot_items(host: &AppHost) -> Vec<MenuItem> {
    let mut items = Vec::new();
    for slot in 1..=slot_count() {
        let id = numbered_slot_id(slot);
        if let Some(record) = host.slots.find(&id).filter(|record| record.is_occupied()) {
            items.push(MenuItem::of(
                Command::DeleteSlot,
                Some(slot_label(slot, Some(record))),
                CommandArgs::of_text(&id),
            ));
        }
    }
    if let Some(quick) = host
        .slots
        .find(QUICK_SLOT_ID)
        .filter(|record| record.is_occupied())
    {
        items.push(MenuItem::of(
            Command::DeleteSlot,
            Some(format!(
                "{} — {}",
                text::quick_slot(),
                state_stamp(Some(quick))
            )),
            CommandArgs::of_text(QUICK_SLOT_ID),
        ));
    }
    if items.is_empty() {
        vec![MenuItem::note(text::nothing_here())]
    } else {
        items
    }
}

fn slot_count() -> i32 {
    i32::try_from(EXPOSED_SLOT_COUNT).unwrap_or(i32::MAX)
}

/// The label of numbered slot `slot` in the state submenus: "Slot 3 — date" or "Slot 3 — empty".
#[must_use]
pub fn slot_label(slot: i32, record: Option<&SaveSlotRecord>) -> String {
    format!("{} {slot} — {}", text::slot_prefix(), state_stamp(record))
}

fn state_stamp(record: Option<&SaveSlotRecord>) -> String {
    match record {
        Some(record) if record.is_occupied() => save_state_file::format_timestamp(record.saved_at),
        _ => text::empty_slot(),
    }
}

fn speed_items() -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::of(Command::SpeedUp, None, CommandArgs::default()),
        MenuItem::of(Command::SpeedDown, None, CommandArgs::default()),
        MenuItem::rule(),
    ];
    for value in (speed_setting::MINIMUM..=speed_setting::MAXIMUM).rev() {
        items.push(MenuItem::choice(
            Command::SetSpeed,
            text::speed_label(value),
            CommandArgs::of_number(value),
            Selection::Speed(value),
        ));
    }
    items
}

fn region_items() -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::of(
            Command::ToggleAutoRegion,
            Some(text::region_auto()),
            CommandArgs::default(),
        ),
        MenuItem::rule(),
    ];
    for (name, region) in [
        ("NTSC", TvRegionSetting::Ntsc),
        ("PAL", TvRegionSetting::Pal),
        ("Dendy", TvRegionSetting::Dendy),
    ] {
        items.push(MenuItem::choice(
            Command::SetRegion,
            name.to_owned(),
            CommandArgs::of_number(region as i32),
            Selection::Region(region),
        ));
    }
    items
}

fn port_items() -> Vec<MenuItem> {
    (0..2)
        .map(|port| {
            MenuItem::sub(
                format!("{} {}", text::port(), port + 1),
                Submenu::Port(port),
            )
        })
        .collect()
}

fn port_device_items(port: usize) -> Vec<MenuItem> {
    let number = i32::try_from(port).unwrap_or(0);
    [
        (text::port_controller(), ControllerPortType::Controller),
        (text::port_zapper(), ControllerPortType::Zapper),
    ]
    .into_iter()
    .map(|(label, device)| {
        MenuItem::choice(
            Command::SetPortType,
            label,
            CommandArgs {
                number,
                text: None,
                number2: device as i32,
            },
            Selection::Port(port, device),
        )
    })
    .collect()
}

// Q-038
fn camera_items() -> Vec<MenuItem> {
    vec![
        MenuItem::choice(
            Command::SetCameraMode,
            text::camera_orbital(),
            CommandArgs::of_number(0),
            Selection::Camera(CameraMode::Tpv),
        ),
        MenuItem::choice(
            Command::SetCameraMode,
            text::camera_first_person(),
            CommandArgs::of_number(1),
            Selection::Camera(CameraMode::Fpv),
        ),
        MenuItem::rule(),
        MenuItem::of(Command::ToggleLocalCamera, None, CommandArgs::default()),
    ]
}

fn skybox_items() -> Vec<MenuItem> {
    let options = catalogue::descriptor(SettingId::SceneSkybox)
        .domain()
        .options()
        .unwrap_or_default();
    options
        .iter()
        .map(|option| {
            #[allow(clippy::cast_possible_truncation)] // the C# `(int)option.Value`
            let value = option.value as i32;
            MenuItem::choice(
                Command::SetSkybox,
                text::label_of_option(&option.label),
                CommandArgs::of_number(value),
                Selection::Skybox(value),
            )
        })
        .collect()
}

fn volume_items() -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem::of(Command::VolumeUp, None, CommandArgs::default()),
        MenuItem::of(Command::VolumeDown, None, CommandArgs::default()),
        MenuItem::rule(),
    ];
    for percent in (0..=100).rev().step_by(10) {
        items.push(MenuItem::choice(
            Command::SetVolume,
            format!("{percent} %"),
            CommandArgs::of_number(percent),
            Selection::Volume(percent),
        ));
    }
    items
}
