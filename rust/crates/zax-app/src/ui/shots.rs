//! `--ui-shots <dir>`: a fixed plan of screens, each arranged, given a few frames to settle, then
//! photographed. The tool the author checks the look of the interface with (AMD-035 §1: the
//! capture set shows the Interface tab, where the language is chosen).

use std::path::PathBuf;

use super::options::OptionsTab;
use super::text;
use super::zax_ui::ZaxUi;

/// Frames a screen is drawn before it is photographed, so menus and tabs have opened.
pub const SETTLE_FRAMES: usize = 4;

/// One screen of the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    FileMenu,
    Options,
    OptionsInterface,
    Keys,
    ProfileChooser,
    About,
}

const PLAN: [(&str, Screen); 7] = [
    ("accueil", Screen::Welcome),
    ("menu-fichier", Screen::FileMenu),
    ("options", Screen::Options),
    ("options-interface", Screen::OptionsInterface),
    ("touches", Screen::Keys),
    ("profil", Screen::ProfileChooser),
    ("a-propos", Screen::About),
];

/// What a frame of the plan asks of the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotStep {
    /// Keep drawing.
    Wait,
    /// Write the window to this file.
    Capture(PathBuf),
}

/// The plan and where it stands.
#[derive(Debug)]
pub struct UiShots {
    directory: PathBuf,
    index: usize,
    frame: usize,
}

impl UiShots {
    /// A plan whose files go to `directory`, created if need be.
    ///
    /// # Errors
    /// The folder could not be created.
    pub fn new(directory: &str) -> std::io::Result<Self> {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory)?;
        Ok(Self {
            directory,
            index: 0,
            frame: 0,
        })
    }

    /// Whether every screen has been photographed.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.index >= PLAN.len()
    }

    /// The number of frames the whole plan takes.
    #[must_use]
    pub const fn total_frames() -> usize {
        PLAN.len() * (SETTLE_FRAMES + 1)
    }

    /// Called once per frame, after the interface was drawn: arranges the screen on its first
    /// frame, and asks for the capture once it has settled.
    pub fn advance(&mut self, ui: &mut ZaxUi) -> ShotStep {
        let Some((name, screen)) = PLAN.get(self.index).copied() else {
            return ShotStep::Wait;
        };
        if self.frame == 0 {
            arrange(ui, screen);
        }
        self.frame += 1;
        if self.frame <= SETTLE_FRAMES {
            return ShotStep::Wait;
        }
        self.index += 1;
        self.frame = 0;
        ShotStep::Capture(self.directory.join(format!("{name}.png")))
    }
}

fn arrange(ui: &mut ZaxUi, screen: Screen) {
    ui.state.close_all();
    ui.force_open_menu = None;
    match screen {
        Screen::Welcome => {}
        Screen::FileMenu => ui.force_open_menu = Some(text::menu_file()),
        Screen::Options => ui.state.options = true,
        Screen::OptionsInterface => {
            ui.state.options = true;
            ui.force_options_tab = Some(OptionsTab::Interface);
        }
        Screen::Keys => ui.state.key_config = true,
        Screen::ProfileChooser => ui.open_profile_chooser_for_capture(),
        Screen::About => ui.state.about = true,
    }
}
