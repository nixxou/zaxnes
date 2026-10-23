//! The key configuration window (REQ-UI-203): the bindings of both players, the camera, the
//! hotkeys and the commands, a cell per device.

use imgui::{
    Condition, TableColumnFlags, TableColumnSetup, TableFlags, TreeNodeFlags, Ui, WindowFlags,
};
use zax_platform::input::labels;

use super::{centre, play, UiFrame, ZaxUi};
use crate::audio::UiSound;
use crate::input::input_action::PLAYER_COUNT;
use crate::input::BindingDeviceKind;
use crate::ui::key_config::{self, KeyConfigRow, KeyConfigSection};
use crate::ui::text;

impl ZaxUi {
    pub(super) fn draw_key_config(&mut self, ui: &Ui, frame: &mut UiFrame<'_>) {
        let mut open = self.state.key_config;
        let window = ui
            .window(text::key_config_title())
            .size([640.0, 560.0], Condition::FirstUseEver)
            .position(centre(ui, 640.0, 560.0), Condition::FirstUseEver)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if let Some(_window) = window {
            draw_header(ui, frame);
            ui.separator();
            let height = -ui.frame_height_with_spacing();
            ui.child_window("##bindings").size([0.0, height]).build(|| {
                for section in self.keys.sections() {
                    if ui.collapsing_header(&section.title, TreeNodeFlags::DEFAULT_OPEN) {
                        draw_table(ui, frame, section);
                    }
                }
            });
            if ui.button(text::close()) {
                open = false;
            }
        }
        if !open {
            // REQ-UI-067: closing the screen stops a capture in progress.
            Self::close_capture(frame.input);
        }
        self.state.key_config = open;
    }
}

fn draw_header(ui: &Ui, frame: &mut UiFrame<'_>) {
    if frame.input.rebinding().is_listening() {
        ui.text(text::press_a_control());
        ui.same_line();
        if ui.button(text::stop_binding()) {
            frame.input.edit(|_, _, session| session.stop());
        }
    } else {
        ui.text_disabled(text::click_a_cell());
    }
    draw_gamepad_choosers(ui, frame);
    if ui.button(text::reset_bindings()) {
        // REQ-INP-049: back to the defaults of §4.2 and §4.3, both players, both devices.
        frame.input.edit(|bindings, _, session| {
            session.stop();
            bindings.reset_all();
        });
        play(frame.host, UiSound::Confirm);
    }
}

fn draw_gamepad_choosers(ui: &Ui, frame: &mut UiFrame<'_>) {
    let available = frame.hub.devices();
    for player in 0..PLAYER_COUNT {
        let mut labels = vec![text::no_gamepad()];
        labels.extend(
            available
                .iter()
                .map(|device| device.display_name().to_owned()),
        );
        let assigned = frame.input.pads().device(frame.hub, player);
        let mut current = assigned.map_or(0, |assigned| {
            available
                .iter()
                .position(|device| device.identity() == assigned.identity())
                .map_or(0, |index| index + 1)
        });
        let _id = ui.push_id_usize(player);
        let label = format!("{} J{}", text::gamepad_column(), player + 1);
        if ui.combo(label, &mut current, &labels, |label| label.as_str().into()) {
            let device = current
                .checked_sub(1)
                .and_then(|index| available.get(index));
            frame.input.edit(|bindings, pads, session| {
                pads.assign(bindings, player, device.map(|device| &**device));
                session.notify_gamepad_changed();
            });
            play(frame.host, UiSound::Confirm);
        }
    }
}

fn draw_table(ui: &Ui, frame: &mut UiFrame<'_>, section: &KeyConfigSection) {
    let flags = TableFlags::SIZING_STRETCH_PROP | TableFlags::ROW_BG | TableFlags::BORDERS_INNER_V;
    let Some(_table) = ui.begin_table_with_flags(format!("##table-{}", section.title), 3, flags)
    else {
        return;
    };
    for (name, weight) in [
        (text::action_column(), 2.0),
        (text::keyboard_column(), 1.5),
        (text::gamepad_column(), 1.5),
    ] {
        ui.table_setup_column_with(TableColumnSetup {
            flags: TableColumnFlags::WIDTH_STRETCH,
            init_width_or_weight: weight,
            ..TableColumnSetup::new(name)
        });
    }
    ui.table_headers_row();
    for (index, row) in section.rows.iter().enumerate() {
        ui.table_next_row();
        ui.table_next_column();
        ui.text(&row.label);
        let _id = ui.push_id_usize(index);
        ui.table_next_column();
        draw_cell(ui, frame, row, BindingDeviceKind::Keyboard);
        ui.table_next_column();
        draw_cell(ui, frame, row, BindingDeviceKind::Gamepad);
    }
}

fn draw_cell(ui: &Ui, frame: &mut UiFrame<'_>, row: &KeyConfigRow, kind: BindingDeviceKind) {
    let session = frame.input.rebinding();
    let set = key_config::set_of(frame.input.bindings(), frame.input.pads(), row.player, kind);
    let listening = key_config::is_listening_for(session, row, kind);
    let label = if listening {
        "…".to_owned()
    } else {
        set.map_or_else(text::no_gamepad, |set| labels::of(set.get(&row.target)))
    };
    // REQ-UI-071: while a capture is listening the other cells are inactive.
    let inactive = (session.is_listening() && !listening) || set.is_none();
    let _id = ui.push_id_usize(kind as usize);
    let disabled = ui.begin_disabled(inactive);
    let pressed = ui.button_with_size(format!("{label}##cell"), [-1.0, 0.0]);
    drop(disabled);
    if pressed {
        let hub = frame.hub;
        frame.input.edit(|bindings, pads, session| {
            key_config::begin(session, bindings, pads, hub, row, kind)
        });
    }
}
