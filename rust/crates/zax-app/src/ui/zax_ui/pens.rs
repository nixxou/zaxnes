//! The editor's pens (EDITOR-1, owner request): a red, blue, black and white pen to draw over
//! the view — circle a zone, write a note, draw what is to be modelled and where. The strokes
//! are the host's (`emulation::annotations`), so that the MCP server shows them in its
//! `screenshot`.
//!
//! The tool box lies at the left of the window while a game runs with the editor (`--mcp`). With
//! a pen in hand a transparent window covers the view and takes the pointer, so a drag draws
//! instead of turning the camera; the hand gives the pointer back to the camera.

use imgui::{Condition, MouseButton, Ui, WindowFlags};

use crate::application::AppHost;
use crate::emulation::annotations::Pen;
use crate::ui::{keys, strings};

/// The side of a pen's button.
const BUTTON: f32 = 26.0;

/// Draws the strokes over the view, the tool box, and with a pen in hand the canvas that takes
/// the drags.
pub(super) fn draw_pens(ui: &Ui, host: &mut AppHost) {
    if !host.annotations().enabled {
        return;
    }
    let [width, height] = ui.io().display_size;
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    if host.annotations().pen.is_some() {
        canvas(ui, host, [width, height]);
    }
    draw_strokes(ui, host, [width, height]);
    tool_box(ui, host, height);
}

/// The transparent window over the view that turns drags into strokes.
fn canvas(ui: &Ui, host: &mut AppHost, size: [f32; 2]) {
    let flags = WindowFlags::NO_DECORATION
        | WindowFlags::NO_BACKGROUND
        | WindowFlags::NO_MOVE
        | WindowFlags::NO_SAVED_SETTINGS
        | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
        | WindowFlags::NO_NAV
        | WindowFlags::NO_SCROLLBAR;
    let Some(_window) = ui
        .window("##pen_canvas")
        .position([0.0, 0.0], Condition::Always)
        .size(size, Condition::Always)
        .flags(flags)
        .begin()
    else {
        return;
    };
    ui.set_cursor_pos([0.0, 0.0]);
    ui.invisible_button("##pen_surface", size);
    let [x, y] = ui.io().mouse_pos;
    let point = [(x / size[0]).clamp(0.0, 1.0), (y / size[1]).clamp(0.0, 1.0)];
    let annotations = host.annotations_mut();
    if ui.is_item_activated() {
        annotations.begin(point);
    } else if ui.is_item_active() && ui.is_mouse_down(MouseButton::Left) {
        annotations.extend(point);
    }
    if ui.is_item_deactivated() {
        annotations.end();
    }
}

/// The strokes, under every window of the interface and over the view.
fn draw_strokes(ui: &Ui, host: &AppHost, [width, height]: [f32; 2]) {
    let list = ui.get_background_draw_list();
    let thickness = (width.max(height) / 400.0).max(1.5) * 2.0;
    for stroke in host.annotations().strokes() {
        let [r, g, b] = stroke.pen.rgb();
        let color = [
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ];
        let points: Vec<[f32; 2]> = stroke
            .points
            .iter()
            .map(|&[x, y]| [x * width, y * height])
            .collect();
        if let [only] = points[..] {
            list.add_circle(only, thickness / 2.0, color)
                .filled(true)
                .build();
        } else {
            list.add_polyline(points, color)
                .thickness(thickness)
                .build();
        }
    }
}

/// The column of buttons at the left of the window: the hand, the pens, undo and clear.
fn tool_box(ui: &Ui, host: &mut AppHost, height: f32) {
    let flags = WindowFlags::NO_DECORATION
        | WindowFlags::ALWAYS_AUTO_RESIZE
        | WindowFlags::NO_MOVE
        | WindowFlags::NO_SAVED_SETTINGS
        | WindowFlags::NO_FOCUS_ON_APPEARING
        | WindowFlags::NO_NAV;
    let Some(_window) = ui
        .window("##pens")
        .position([4.0, height * 0.3], Condition::Always)
        .bg_alpha(0.7)
        .flags(flags)
        .begin()
    else {
        return;
    };
    let in_hand = host.annotations().pen;
    if selectable_button(ui, "##hand", None, in_hand.is_none(), keys::PENS_HAND) {
        host.annotations_mut().pen = None;
    }
    for pen in Pen::ALL {
        if selectable_button(
            ui,
            pen.name(),
            Some(pen),
            in_hand == Some(pen),
            pen_key(pen),
        ) {
            host.annotations_mut().pen = Some(pen);
        }
    }
    ui.separator();
    if ui.button_with_size("<##undo", [BUTTON, BUTTON]) {
        host.annotations_mut().undo();
    }
    tooltip(ui, keys::PENS_UNDO);
    if ui.button_with_size("X##clear", [BUTTON, BUTTON]) {
        host.annotations_mut().clear();
    }
    tooltip(ui, keys::PENS_CLEAR);
}

/// A button of the tool box, framed while it is the one in hand; `pen` none for the hand.
fn selectable_button(ui: &Ui, id: &str, pen: Option<Pen>, chosen: bool, tip: &str) -> bool {
    let clicked = match pen {
        Some(pen) => {
            let [r, g, b] = pen.rgb();
            let color = [
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
                1.0,
            ];
            ui.color_button_config(format!("##{id}"), color)
                .size([BUTTON, BUTTON])
                .tooltip(false)
                .build()
        }
        None => ui.button_with_size(format!("M{id}"), [BUTTON, BUTTON]),
    };
    if chosen {
        let min = ui.item_rect_min();
        let max = ui.item_rect_max();
        ui.get_window_draw_list()
            .add_rect(
                [min[0] - 2.0, min[1] - 2.0],
                [max[0] + 2.0, max[1] + 2.0],
                [1.0, 0.85, 0.0, 1.0],
            )
            .thickness(2.0)
            .build();
    }
    tooltip(ui, tip);
    clicked
}

fn tooltip(ui: &Ui, key: &str) {
    if ui.is_item_hovered() {
        ui.tooltip_text(strings::get(key));
    }
}

const fn pen_key(pen: Pen) -> &'static str {
    match pen {
        Pen::Red => keys::PENS_RED,
        Pen::Blue => keys::PENS_BLUE,
        Pen::Black => keys::PENS_BLACK,
        Pen::White => keys::PENS_WHITE,
    }
}
