//! The commands of the program (SPEC-10 REQ-UI-205): every action a menu entry, a hotkey or a
//! binding can trigger, by its stable name.

/// The prefix of the interface key of a command's label.
pub const LABEL_KEY_PREFIX: &str = "cmd.";

/// The argument of a command: a number, a text, or both.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandArgs {
    /// A number: a slot, a speed, a region, a percentage.
    pub number: i32,
    /// A text: a path, an identifier.
    pub text: Option<String>,
    /// A second number.
    pub number2: i32,
}

impl CommandArgs {
    /// An argument made of a number.
    #[must_use]
    pub const fn of_number(number: i32) -> Self {
        Self {
            number,
            text: None,
            number2: 0,
        }
    }

    /// An argument made of a text.
    #[must_use]
    pub fn of_text(text: &str) -> Self {
        Self {
            number: 0,
            text: Some(text.to_owned()),
            number2: 0,
        }
    }
}

/// How the shortcut of a command is spelled in the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    /// No shortcut.
    None,
    /// Spelled the same in every language.
    Fixed(&'static str),
    /// Spelled by the interface table under this key.
    Key(&'static str),
}

macro_rules! commands {
    ($($variant:ident => $name:literal, $shortcut:expr, $toggle:literal;)*) => {
        /// A command of the program.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Command {
            $(
                #[doc = concat!("`", $name, "`.")]
                $variant,
            )*
        }

        impl Command {
            /// Every command, in the order of the menus.
            pub const ALL: &'static [Command] = &[$(Command::$variant),*];

            /// The stable name of the command, `area.action`.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Command::$variant => $name,)*
                }
            }

            /// The command with the stable name `name`.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Command> {
                match name {
                    $($name => Some(Command::$variant),)*
                    _ => None,
                }
            }

            /// The default shortcut shown beside the command (REQ-UI-204).
            #[must_use]
            pub const fn shortcut(self) -> Shortcut {
                match self {
                    $(Command::$variant => $shortcut,)*
                }
            }

            /// Whether the command is a toggle, whose state the menu shows.
            #[must_use]
            pub const fn is_toggle(self) -> bool {
                match self {
                    $(Command::$variant => $toggle,)*
                }
            }
        }
    };
}

commands! {
    OpenRom => "file.open_rom", Shortcut::Fixed("Ctrl+O"), false;
    LaunchGame => "file.launch_game", Shortcut::None, false;
    OpenRecentRom => "file.open_recent_rom", Shortcut::None, false;
    ChooseProfile => "file.choose_profile", Shortcut::None, false;
    ImportProfile => "file.import_profile", Shortcut::None, false;
    ImportThreeDSen => "file.import_3dsen", Shortcut::None, false;
    DeleteGame => "file.delete_game", Shortcut::None, false;
    CreateGameArt => "file.create_game_art", Shortcut::None, false;
    ImportCover => "file.import_cover", Shortcut::None, false;
    Quit => "app.quit", Shortcut::Fixed("Ctrl+Q"), false;
    SaveSlot => "state.save_slot", Shortcut::Key("shortcut.save_slots"), false;
    LoadSlot => "state.load_slot", Shortcut::Key("shortcut.load_slots"), false;
    QuickSave => "state.quick_save", Shortcut::Key("shortcut.quick_save"), false;
    QuickLoad => "state.quick_load", Shortcut::Key("shortcut.quick_load"), false;
    DeleteSlot => "state.delete_slot", Shortcut::None, false;
    PauseResume => "emulation.pause_resume", Shortcut::Fixed("P"), true;
    Pause => "emulation.pause", Shortcut::None, false;
    Resume => "emulation.resume", Shortcut::None, false;
    Reset => "emulation.reset", Shortcut::Fixed("Ctrl+R"), false;
    Stop => "emulation.stop", Shortcut::None, false;
    FastForward => "emulation.fast_forward", Shortcut::Key("shortcut.fast_forward"), true;
    Rollback => "emulation.rollback", Shortcut::Key("shortcut.rollback"), false;
    SetSpeed => "emulation.set_speed", Shortcut::None, false;
    SpeedUp => "emulation.speed_up", Shortcut::None, false;
    SpeedDown => "emulation.speed_down", Shortcut::None, false;
    SetRegion => "emulation.set_region", Shortcut::None, false;
    ToggleAutoRegion => "emulation.auto_region", Shortcut::None, true;
    SetPortType => "emulation.set_port_type", Shortcut::None, false;
    ToggleTwoDimensional => "video.toggle_2d", Shortcut::Fixed("F11"), true;
    ResetView => "video.reset_view", Shortcut::Key("shortcut.long_press_menu"), false;
    SetSkybox => "video.set_skybox", Shortcut::None, false;
    SetCameraMode => "video.set_camera_mode", Shortcut::None, false;
    ToggleLocalCamera => "video.toggle_local_camera", Shortcut::None, true;
    ToggleShadows => "video.toggle_shadows", Shortcut::None, true;
    ToggleFullscreen => "video.toggle_fullscreen", Shortcut::Key("shortcut.alt_enter"), true;
    ToggleFpsCounter => "video.toggle_fps", Shortcut::None, true;
    SetVolume => "audio.set_volume", Shortcut::None, false;
    VolumeUp => "audio.volume_up", Shortcut::None, false;
    VolumeDown => "audio.volume_down", Shortcut::None, false;
    ToggleMute => "audio.toggle_mute", Shortcut::None, true;
    ToggleMenuMusic => "audio.toggle_menu_music", Shortcut::None, true;
    ShowOptions => "tools.options", Shortcut::Fixed("Ctrl+,"), false;
    ShowKeyConfig => "tools.key_config", Shortcut::None, false;
    ShowDataFolder => "tools.data_folder", Shortcut::None, false;
    ShowAbout => "tools.about", Shortcut::None, false;
    CancelModal => "ui.cancel_modal", Shortcut::None, false;
}

impl Command {
    /// The interface key of the label of the command.
    #[must_use]
    pub fn label_key(self) -> String {
        format!("{LABEL_KEY_PREFIX}{}", self.name())
    }
}
