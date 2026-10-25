//! `--ui-shots <dir>`: a fixed plan of screens, each arranged, given a few frames to settle, then
//! photographed. The tool the author checks the look of the interface with (AMD-035 §1: the
//! capture set shows the Interface tab, where the language is chosen).

use std::path::PathBuf;

use super::handheld::{HandheldUi, Screen as HandheldScreen};
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
    /// A screen of the handheld interface.
    Handheld(HandheldScreen),
}

const DESKTOP_PLAN: [(&str, Screen); 7] = [
    ("accueil", Screen::Welcome),
    ("menu-fichier", Screen::FileMenu),
    ("options", Screen::Options),
    ("options-interface", Screen::OptionsInterface),
    ("touches", Screen::Keys),
    ("profil", Screen::ProfileChooser),
    ("a-propos", Screen::About),
];

/// The handheld screens, photographed at the size of the window (640x480 for the devices).
const HANDHELD_PLAN: [(&str, Screen); 8] = [
    ("handheld-accueil", Screen::Handheld(HandheldScreen::Home)),
    (
        "handheld-profils",
        Screen::Handheld(HandheldScreen::Profiles),
    ),
    (
        "handheld-options",
        Screen::Handheld(HandheldScreen::Options),
    ),
    (
        "handheld-options-jeu",
        Screen::Handheld(HandheldScreen::GameOptions),
    ),
    (
        "handheld-commandes",
        Screen::Handheld(HandheldScreen::Controls),
    ),
    ("handheld-menu", Screen::Handheld(HandheldScreen::Overlay)),
    (
        "handheld-etats-charger",
        Screen::Handheld(HandheldScreen::States),
    ),
    (
        "handheld-etats-sauver",
        Screen::Handheld(HandheldScreen::States),
    ),
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
    plan: &'static [(&'static str, Screen)],
    directory: PathBuf,
    index: usize,
    frame: usize,
}

impl UiShots {
    /// A plan whose files go to `directory`, created if need be: the screens of the handheld
    /// interface when `handheld`, those of the desktop one otherwise.
    ///
    /// # Errors
    /// The folder could not be created.
    pub fn new(directory: &str, handheld: bool) -> std::io::Result<Self> {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory)?;
        Ok(Self {
            plan: if handheld {
                &HANDHELD_PLAN
            } else {
                &DESKTOP_PLAN
            },
            directory,
            index: 0,
            frame: 0,
        })
    }

    /// Whether every screen has been photographed.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.index >= self.plan.len()
    }

    /// The number of frames the whole plan takes.
    #[must_use]
    pub const fn total_frames(&self) -> usize {
        self.plan.len() * (SETTLE_FRAMES + 1)
    }

    /// Called once per frame, after the interface was drawn: arranges the screen on its first
    /// frame, and asks for the capture once it has settled.
    pub fn advance(&mut self, ui: &mut ZaxUi, handheld: Option<&mut HandheldUi>) -> ShotStep {
        let Some((name, screen)) = self.plan.get(self.index).copied() else {
            return ShotStep::Wait;
        };
        if self.frame == 0 {
            match (screen, handheld) {
                (Screen::Handheld(screen), Some(handheld)) => handheld.arrange_for_capture(screen),
                (screen, _) => arrange(ui, screen),
            }
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
        // The welcome screen is what shows with nothing open, and only the handheld interface
        // can show its own screens.
        Screen::Welcome | Screen::Handheld(_) => {}
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
