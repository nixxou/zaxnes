//! The dialogs: the file browser, the profile chooser, the confirmation box and the about window
//! (REQ-UI-110, REQ-UI-201, DEC-10-Q3).

use imgui::{Condition, MouseButton, Ui, WindowFlags};

use super::{centre, play, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::emulation::commands::{Command, CommandArgs};
use crate::ui::file_browser::{FileEntry, FileEntryKind};
use crate::ui::profile_chooser::ProfileChoice;
use crate::ui::{strings, text};

/// The colour of a destructive first button (AMD-056 §3 leaves it free, AMD-010 §4).
const DESTRUCTIVE_BUTTON: [f32; 4] = [0.60, 0.16, 0.16, 1.0];

/// What marks a favourite entry in the list (AMD-053 REQ-UI-210; the form is free).
const FAVORITE_MARKER: &str = "*";

impl ZaxUi {
    pub(super) fn draw_file_browser(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.file_browser;
        let window = ui
            .window(self.browser.title())
            .size([700.0, 480.0], Condition::FirstUseEver)
            .position(centre(ui, 700.0, 480.0), Condition::Appearing)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if window.is_some() {
            self.draw_browser_trail(ui);
            // AMD-030 §2: the 3dSen picker says what it expects, since a folder is an unusual
            // answer for this dialog.
            if !self.browser.hint().is_empty() {
                ui.text_wrapped(self.browser.hint());
            }
            let mut search = self.browser.search().to_owned();
            ui.set_next_item_width(220.0);
            if ui
                .input_text(format!("{}##search", text::search()), &mut search)
                .build()
            {
                self.browser.set_search(&search);
            }
            ui.same_line();
            if ui.button(text::parent_folder()) && self.browser.directory().is_some() {
                self.browser.go_up();
            }
            ui.separator();
            let height = -2.0 * ui.frame_height_with_spacing();
            let mut chosen = None;
            ui.child_window("##entries").size([0.0, height]).build(|| {
                chosen = self.draw_browser_entries(ui, host);
            });
            ui.separator();
            chosen = chosen.or(self.draw_browser_footer(ui, &mut open));
            if let Some(path) = chosen {
                open = false;
                play(host, UiSound::Confirm);
                self.on_file_chosen(host, &path);
            }
        }
        self.state.file_browser = open;
    }

    fn draw_browser_trail(&mut self, ui: &Ui) {
        let mut target = None;
        for (index, (label, path)) in self.browser.trail().iter().enumerate() {
            if index > 0 {
                ui.same_line();
            }
            let _id = ui.push_id_usize(index);
            if ui.small_button(label) {
                target = Some(path.clone());
            }
        }
        if let Some(path) = target {
            self.browser
                .go_to((!path.is_empty()).then_some(path.as_str()));
        }
    }

    /// Draws the list and returns the path chosen by a double click.
    fn draw_browser_entries(&mut self, ui: &Ui, host: &mut AppHost) -> Option<String> {
        let mut activated: Option<FileEntry> = None;
        let mut selected_now: Option<FileEntry> = None;
        for (index, entry) in self.browser.entries().iter().enumerate() {
            let selected = self.browser.selected.as_ref() == Some(entry);
            let label = if entry.kind == FileEntryKind::File {
                entry.name.clone()
            } else {
                format!("[{}]", entry.name)
            };
            let _id = ui.push_id_usize(index);
            if !ui.selectable_config(&label).selected(selected).build() {
                continue;
            }
            // REQ-UI-104: a click selects, a click on the selected row or a double click opens
            // it.
            if selected || ui.is_mouse_double_clicked(MouseButton::Left) {
                activated = Some(entry.clone());
            } else {
                selected_now = Some(entry.clone());
            }
        }
        if let Some(entry) = selected_now {
            self.browser.selected = Some(entry);
            play(host, UiSound::Move);
        }
        let entry = activated?;
        if self.browser.activate(&entry) {
            self.browser.result().map(str::to_owned)
        } else {
            None
        }
    }

    /// Draws the file name, the filter, Cancel and Open; returns the path the Open button chose.
    fn draw_browser_footer(&mut self, ui: &Ui, open: &mut bool) -> Option<String> {
        let name = self.browser.file_name();
        ui.text(if name.is_empty() { " " } else { name.as_str() });
        let labels: Vec<String> = self
            .browser
            .filters()
            .iter()
            .map(|filter| filter.label.clone())
            .collect();
        if !labels.is_empty() {
            let mut filter = self.browser.filter_index();
            ui.set_next_item_width(260.0);
            if ui.combo("##filter", &mut filter, &labels, |label| {
                label.as_str().into()
            }) {
                self.browser.choose_filter(filter);
            }
            ui.same_line();
        }
        if ui.button(text::cancel()) {
            *open = false;
        }
        ui.same_line();
        let disabled = ui.begin_disabled(!self.browser.can_confirm());
        let pressed = ui.button(text::open());
        drop(disabled);
        if pressed && self.browser.confirm() {
            return self.browser.result().map(str::to_owned);
        }
        None
    }

    pub(super) fn draw_profile_chooser(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.profile_chooser;
        let window = ui
            .window(text::profile_chooser_title())
            .size([520.0, 460.0], Condition::FirstUseEver)
            .position(centre(ui, 520.0, 460.0), Condition::Appearing)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if window.is_some() {
            if let Some(rom) = &self.chooser.rom_path {
                let name = std::path::Path::new(rom)
                    .file_name()
                    .map_or_else(|| rom.clone(), |name| name.to_string_lossy().into_owned());
                ui.text_wrapped(name);
            }
            // AMD-053 REQ-UI-211: the search of REQ-UI-201, now ignoring case and accents, and
            // the favourites filter of REQ-UI-210 beside it.
            let mut search = self.chooser.search().to_owned();
            ui.set_next_item_width(-160.0);
            if ui
                .input_text(format!("##{}", text::search()), &mut search)
                .build()
            {
                self.chooser.set_search(&search);
            }
            ui.same_line();
            let mut favorites_only = self.chooser.favorites_only();
            if ui.checkbox(
                strings::get(&Command::FilterFavorites.label_key()),
                &mut favorites_only,
            ) {
                self.chooser.set_favorites_only(favorites_only);
                host.set_favorites_only(favorites_only);
            }
            // REQ-LOOP-076: the Resume entry heads the enumeration of the games, and no filter
            // reaches it.
            Self::draw_resume_entry(ui, host, &mut open);
            ui.separator();
            let height = -ui.frame_height_with_spacing();
            let mut picked: Option<ProfileChoice> = None;
            let mut highlighted: Option<String> = None;
            ui.child_window("##profiles").size([0.0, height]).build(|| {
                for (index, choice) in self.chooser.filtered().enumerate() {
                    let selected = self.chooser.selected_id.as_deref() == Some(choice.id.as_str());
                    let _id = ui.push_id_usize(index);
                    // REQ-UI-210: a favourite entry is distinguished; the form of the marker is
                    // free, and this one is a star before the title.
                    let label = if choice.favorite {
                        format!("{FAVORITE_MARKER} {}", choice.title)
                    } else {
                        choice.title.clone()
                    };
                    if ui.selectable_config(&label).selected(selected).build() {
                        if selected {
                            picked = Some(choice.clone());
                        } else {
                            highlighted = Some(choice.id.clone());
                        }
                    }
                    ui.same_line();
                    ui.text_disabled(choice.source_label());
                }
            });
            if let Some(id) = highlighted {
                self.chooser.selected_id = Some(id);
                play(host, UiSound::Move);
            }
            let disabled = ui.begin_disabled(self.chooser.selected_id.is_none());
            let launch = ui.button(text::launch());
            ui.same_line();
            let toggle_favorite = ui.button(strings::get(&Command::ToggleFavorite.label_key()));
            drop(disabled);
            if toggle_favorite {
                self.toggle_selected_favorite(host);
            }
            if launch {
                picked = self
                    .chooser
                    .all()
                    .iter()
                    .find(|choice| self.chooser.selected_id.as_deref() == Some(choice.id.as_str()))
                    .cloned();
            }
            ui.same_line();
            if ui.button(text::cancel()) {
                open = false;
            }
            if let Some(choice) = picked {
                open = false;
                self.choose(host, &choice);
            }
        }
        self.state.profile_chooser = open;
    }

    /// `favorite.toggle` on the selected entry, with the list's marker following it (REQ-UI-210).
    fn toggle_selected_favorite(&mut self, host: &mut AppHost) {
        let Some(id) = self.chooser.selected_id.clone() else {
            return;
        };
        if let Some(favorite) = host.toggle_favorite(&id) {
            self.chooser.set_favorite(&id, favorite);
            play(host, UiSound::Confirm);
        }
    }

    /// The `Resume` entry of AMD-053 REQ-LOOP-076, with the title of its game: shown only while
    /// the resume point stands, and above the list, which is the head of the enumeration.
    fn draw_resume_entry(ui: &Ui, host: &mut AppHost, open: &mut bool) {
        let Some(game) = host.resume_game_title() else {
            return;
        };
        if ui.button(format!(
            "{} — {game}",
            strings::get(&Command::ResumeLast.label_key())
        )) {
            play(host, UiSound::Confirm);
            host.enqueue(Command::ResumeLast, CommandArgs::default());
            *open = false;
        }
    }

    fn choose(&self, host: &mut AppHost, choice: &ProfileChoice) {
        play(host, UiSound::Confirm);
        match self
            .chooser
            .rom_path
            .as_deref()
            .filter(|rom| !rom.is_empty())
        {
            // REQ-UI-201: the pair is remembered, so the next opening of this file is direct.
            Some(rom) => host.launch_rom_with_profile(rom, &choice.id),
            None => host.enqueue(Command::LaunchGame, CommandArgs::of_text(&choice.id)),
        }
    }

    /// The import dialogue of AMD-056 REQ-UI-214. Its form is free (AMD-010 §4); it is drawn like
    /// the confirmation box of REQ-UI-110, with the destructive first button in red.
    pub(super) fn draw_import_dialog(&mut self, ui: &Ui, host: &mut AppHost) {
        let Some(dialog) = self.import_dialog().cloned() else {
            return;
        };
        let flags = WindowFlags::NO_COLLAPSE
            | WindowFlags::NO_RESIZE
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::ALWAYS_AUTO_RESIZE;
        let window = ui
            .window(format!("{}##import", dialog.title))
            .position(centre(ui, 420.0, 0.0), Condition::Always)
            .size([420.0, 0.0], Condition::Always)
            .flags(flags)
            .begin();
        if window.is_none() {
            return;
        }
        ui.text_wrapped(&dialog.body);
        ui.separator();
        if ui.is_window_appearing() {
            ui.set_keyboard_focus_here();
        }
        let colour = dialog
            .destructive
            .then(|| ui.push_style_color(imgui::StyleColor::Button, DESTRUCTIVE_BUTTON));
        let pressed = ui.button_with_size(&dialog.primary_label, [120.0, 0.0]);
        drop(colour);
        if pressed {
            play(host, UiSound::Confirm);
            self.confirm_import_dialog(host);
            return;
        }
        if let Some(cancel) = &dialog.cancel_label {
            ui.same_line();
            if ui.button_with_size(cancel, [120.0, 0.0]) {
                self.cancel_import_dialog();
            }
        }
    }

    pub(super) fn draw_confirm_dialog(&mut self, ui: &Ui, host: &mut AppHost) {
        let Some(view) = Self::confirmation(host) else {
            return;
        };
        let (title, message) = (view.title().to_owned(), view.message());
        let (confirm, cancel) = (view.confirm_label(), view.cancel_label());
        let alternative = view.alternative_label();
        let flags = WindowFlags::NO_COLLAPSE
            | WindowFlags::NO_RESIZE
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::ALWAYS_AUTO_RESIZE;
        let window = ui
            .window(format!("{title}##confirm"))
            .position(centre(ui, 380.0, 0.0), Condition::Always)
            .size([380.0, 0.0], Condition::Always)
            .flags(flags)
            .begin();
        if window.is_none() {
            return;
        }
        ui.text_wrapped(&message);
        ui.separator();
        // REQ-UI-110: the first button has the focus by default.
        if ui.is_window_appearing() {
            ui.set_keyboard_focus_here();
        }
        if ui.button_with_size(&confirm, [110.0, 0.0]) {
            play(host, UiSound::Confirm);
            host.confirm_pending();
        }
        // The question's second answer, if it has one.
        if let Some(alternative) = alternative {
            ui.same_line();
            if ui.button_with_size(&alternative, [110.0, 0.0]) {
                play(host, UiSound::Confirm);
                host.alternative_pending();
            }
        }
        // A message to acknowledge has no cancelling button.
        if !cancel.is_empty() {
            ui.same_line();
            if ui.button_with_size(&cancel, [110.0, 0.0]) {
                host.cancel_pending();
            }
        }
        self.sync_confirmation(host);
    }

    /// The note for the frame on screen of the movie: written with the button or Ctrl+Enter.
    pub(super) fn draw_movie_note(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.movie_note;
        let window = ui
            .window(format!("{}##movie_note", text::movie_note_title()))
            .size([460.0, 0.0], Condition::Always)
            .position(centre(ui, 460.0, 160.0), Condition::Appearing)
            .flags(WindowFlags::NO_COLLAPSE | WindowFlags::NO_SAVED_SETTINGS)
            .opened(&mut open)
            .begin();
        let mut write = false;
        if window.is_some() {
            if ui.is_window_appearing() {
                ui.set_keyboard_focus_here();
            }
            ui.input_text_multiline("##note", &mut self.movie_note, [440.0, 90.0])
                .build();
            write |= ui.io().key_ctrl && ui.is_key_pressed(imgui::Key::Enter);
            write |= ui.button_with_size(text::movie_note_write(), [110.0, 0.0]);
            ui.same_line();
            if ui.button_with_size(text::cancel(), [110.0, 0.0]) {
                open = false;
            }
        }
        if write {
            open = false;
            host.enqueue(
                Command::AnnotateMovie,
                CommandArgs::of_text(&std::mem::take(&mut self.movie_note)),
            );
        }
        self.state.movie_note = open;
    }

    pub(super) fn draw_about(&mut self, ui: &Ui) {
        let mut open = self.state.about;
        let window = ui
            .window(text::about_title())
            .size([460.0, 220.0], Condition::FirstUseEver)
            .position(centre(ui, 460.0, 220.0), Condition::Appearing)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if window.is_some() {
            for (index, line) in text::about_lines().iter().enumerate() {
                let heading = (index == 0).then_some(self.heading_font).flatten();
                let _font = heading.map(|font| ui.push_font(font));
                centred(ui, line);
            }
            ui.separator();
            if ui.button(text::close()) {
                open = false;
            }
        }
        self.state.about = open;
    }
}

fn centred(ui: &Ui, line: &str) {
    let width = ui.calc_text_size(line)[0];
    let [cursor_x, cursor_y] = ui.cursor_pos();
    let x = cursor_x.max((ui.window_size()[0] - width) * 0.5);
    ui.set_cursor_pos([x, cursor_y]);
    ui.text(line);
}
