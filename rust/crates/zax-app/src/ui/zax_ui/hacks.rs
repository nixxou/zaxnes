//! The hacks window (Rust-only, owner request): the hacks of the game in play, from the game
//! options menu. Each change is in force at once and kept for the game.

use std::path::Path;

use imgui::{Condition, StyleColor, Ui, WindowFlags};

use super::{centre, ZaxUi};
use crate::application::AppHost;
use crate::ui::handheld::compat::Tone;
use crate::ui::handheld::hacks::HackRow;
use crate::ui::{keys, strings};

impl ZaxUi {
    pub(super) fn draw_hacks(&mut self, ui: &Ui, host: &mut AppHost) {
        // The window belongs to the game in play: it goes with it.
        let same_game = match (&self.hacks, host.current_rom_path()) {
            (Some(hacks), Some(rom)) => hacks.rom() == Path::new(rom),
            _ => false,
        };
        if !same_game {
            self.state.hacks = false;
            self.hacks = None;
            return;
        }
        let Some(hacks) = self.hacks.as_mut() else {
            return;
        };
        let mut open = true;
        let title = strings::format(&strings::get(keys::HACKS_TITLE), &[hacks.name()]);
        let window = ui
            .window(format!("{title}###hacks"))
            .size([460.0, 0.0], Condition::Always)
            .position(centre(ui, 460.0, 260.0), Condition::Appearing)
            .flags(WindowFlags::NO_COLLAPSE | WindowFlags::NO_SAVED_SETTINGS)
            .opened(&mut open)
            .begin();
        let mut changed = false;
        if window.is_some() {
            if !hacks.is_capable() {
                match hacks.other_rom_message() {
                    Some(message) => {
                        let _red = ui.push_style_color(StyleColor::Text, [1.0, 0.45, 0.35, 1.0]);
                        ui.text_wrapped(message);
                    }
                    None => ui.text_wrapped(strings::get(keys::HACKS_NONE)),
                }
            }
            for (index, row) in hacks.rows().into_iter().enumerate() {
                let _id = ui.push_id_usize(index);
                let label = hacks.label(row);
                // The options of the patch come under those of the widescreen.
                if matches!(row, HackRow::PatchOption(0)) {
                    ui.separator();
                }
                match row {
                    HackRow::Force
                    | HackRow::Widescreen
                    | HackRow::Advanced
                    | HackRow::PatchOption(_) => {
                        let mut on = hacks.is_on(row);
                        if ui.checkbox(&label, &mut on) {
                            hacks.step(row, 1);
                            changed = true;
                        }
                    }
                    HackRow::Compatibility => {
                        // How the hack fits this ROM: the summary, the detail under it.
                        let header = format!("{label}: {}###compat", hacks.value_text(row));
                        if ui.collapsing_header(&header, imgui::TreeNodeFlags::empty()) {
                            let _wrap = ui.push_text_wrap_pos_with_pos(440.0);
                            for (text, tone) in hacks.compat_lines() {
                                let colour = match tone {
                                    Tone::Good => [0.45, 0.92, 0.55, 1.0],
                                    Tone::Warning => [1.0, 0.62, 0.2, 1.0],
                                    Tone::Bad => [1.0, 0.45, 0.35, 1.0],
                                    Tone::Plain => ui.style_color(StyleColor::Text),
                                };
                                let _colour = ui.push_style_color(StyleColor::Text, colour);
                                ui.text_wrapped(text);
                            }
                        }
                    }
                    HackRow::Ratio | HackRow::Margin => {
                        let choices = hacks.choices(row);
                        let mut index = hacks.choice_index(row);
                        let disabled = ui.begin_disabled(!hacks.widescreen_on());
                        if ui.combo(&label, &mut index, &choices, |choice| {
                            choice.as_str().into()
                        }) {
                            hacks.choose(row, index);
                            changed = true;
                        }
                        drop(disabled);
                    }
                }
            }
            if hacks.patch().is_some() {
                ui.separator();
                let _wrap = ui.push_text_wrap_pos_with_pos(440.0);
                ui.text_disabled(strings::get(keys::HACKS_PATCH_NOTE));
            }
        }
        drop(window);
        if changed {
            hacks.apply(host);
        }
        self.state.hacks = open;
        if !open {
            self.hacks = None;
        }
    }
}
