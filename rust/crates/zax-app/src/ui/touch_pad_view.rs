//! The touch pad drawn over everything (Rust-only, owner request): transparent outlines of the
//! cross and the buttons, filled while pressed. Its fingers are handled by
//! [`zax_platform::input::touch::TouchPad`].

use imgui::{DrawListMut, Ui};
use zax_platform::input::touch::{Circle, Rect, TouchControl, TouchPadView};

const LINE: [f32; 4] = [1.0, 1.0, 1.0, 0.35];
const FILL: [f32; 4] = [1.0, 1.0, 1.0, 0.08];
const PRESSED: [f32; 4] = [1.0, 1.0, 1.0, 0.35];
const LABEL: [f32; 4] = [1.0, 1.0, 1.0, 0.55];

/// Draws `view` on the foreground of `ui`.
pub fn draw(ui: &Ui, view: &TouchPadView) {
    let list = ui.get_foreground_draw_list();
    let layout = &view.layout;
    draw_dpad(&list, layout.dpad, view);
    circle_button(ui, &list, layout.a, "A", view.is_pressed(TouchControl::A));
    circle_button(ui, &list, layout.b, "B", view.is_pressed(TouchControl::B));
    circle_button(
        ui,
        &list,
        layout.menu,
        "",
        view.is_pressed(TouchControl::Menu),
    );
    // The menu's three bars.
    let [x, y] = layout.menu.center;
    let width = layout.menu.radius * 0.5;
    for row in [-1.0, 0.0, 1.0] {
        let line_y = y + row * layout.menu.radius * 0.35;
        list.add_line([x - width, line_y], [x + width, line_y], LABEL)
            .thickness(2.0)
            .build();
    }
    circle_button(
        ui,
        &list,
        layout.flat,
        "3D",
        view.is_pressed(TouchControl::Flat),
    );
    circle_button(
        ui,
        &list,
        layout.lock,
        "",
        view.is_pressed(TouchControl::Lock),
    );
    draw_padlock(&list, layout.lock, view.camera_locked);
    circle_button(
        ui,
        &list,
        layout.recenter,
        "",
        view.is_pressed(TouchControl::Recenter),
    );
    draw_target(&list, layout.recenter);
    if let Some(rewind) = layout.rewind {
        circle_button(ui, &list, rewind, "", view.is_pressed(TouchControl::Rewind));
        draw_arrows(&list, rewind, -1.0);
    }
    if let Some(forward) = layout.forward {
        circle_button(
            ui,
            &list,
            forward,
            "",
            view.is_pressed(TouchControl::FastForward),
        );
        draw_arrows(&list, forward, 1.0);
    }
    pill(
        ui,
        &list,
        layout.select,
        "SELECT",
        view.is_pressed(TouchControl::Select),
    );
    pill(
        ui,
        &list,
        layout.start,
        "START",
        view.is_pressed(TouchControl::Start),
    );
}

fn draw_dpad(list: &DrawListMut<'_>, dpad: Circle, view: &TouchPadView) {
    let [cx, cy] = dpad.center;
    let (reach, half) = (dpad.radius, dpad.radius * 0.34);
    let arms = [
        (
            TouchControl::Up,
            [cx - half, cy - reach],
            [cx + half, cy - half],
        ),
        (
            TouchControl::Down,
            [cx - half, cy + half],
            [cx + half, cy + reach],
        ),
        (
            TouchControl::Left,
            [cx - reach, cy - half],
            [cx - half, cy + half],
        ),
        (
            TouchControl::Right,
            [cx + half, cy - half],
            [cx + reach, cy + half],
        ),
    ];
    list.add_rect([cx - half, cy - half], [cx + half, cy + half], FILL)
        .filled(true)
        .build();
    for (control, min, max) in arms {
        let color = if view.is_pressed(control) {
            PRESSED
        } else {
            FILL
        };
        list.add_rect(min, max, color)
            .filled(true)
            .rounding(half * 0.3)
            .build();
        list.add_rect(min, max, LINE)
            .rounding(half * 0.3)
            .thickness(2.0)
            .build();
    }
}

fn circle_button(ui: &Ui, list: &DrawListMut<'_>, circle: Circle, label: &str, pressed: bool) {
    let color = if pressed { PRESSED } else { FILL };
    list.add_circle(circle.center, circle.radius, color)
        .filled(true)
        .build();
    list.add_circle(circle.center, circle.radius, LINE)
        .thickness(2.0)
        .build();
    centered_text(ui, list, circle.center, label);
}

/// A padlock: its shackle closed on the body when `locked`, raised and open otherwise.
fn draw_padlock(list: &DrawListMut<'_>, circle: Circle, locked: bool) {
    let [x, y] = circle.center;
    let r = circle.radius;
    let body = ([x - r * 0.4, y - r * 0.05], [x + r * 0.4, y + r * 0.5]);
    list.add_rect(body.0, body.1, LABEL)
        .filled(true)
        .rounding(r * 0.08)
        .build();
    // The shackle: two posts and a bar, raised and opened on its right post when free.
    let raised = if locked { 0.0 } else { r * 0.2 };
    let (left, right) = (x - r * 0.25, x + r * 0.25);
    let (top, bottom) = (y - r * 0.5 - raised, y - r * 0.05);
    list.add_line([left, bottom], [left, top], LABEL)
        .thickness(r * 0.12)
        .build();
    list.add_line([left, top], [right, top], LABEL)
        .thickness(r * 0.12)
        .build();
    let right_end = if locked { bottom } else { top + r * 0.2 };
    list.add_line([right, top], [right, right_end], LABEL)
        .thickness(r * 0.12)
        .build();
}

/// Two arrow heads side by side, pointing right when `direction` is 1 (fast forward) and left
/// when it is -1 (rewind).
fn draw_arrows(list: &DrawListMut<'_>, circle: Circle, direction: f32) {
    let [x, y] = circle.center;
    let r = circle.radius;
    let (width, half) = (r * 0.35, r * 0.32);
    for offset in [-width * 0.5, width * 0.5] {
        let back = x + offset - direction * width * 0.5;
        let tip = x + offset + direction * width * 0.5;
        list.add_triangle([back, y - half], [back, y + half], [tip, y], LABEL)
            .filled(true)
            .build();
    }
}

/// A target: the camera back to its default view.
fn draw_target(list: &DrawListMut<'_>, circle: Circle) {
    let [x, y] = circle.center;
    let r = circle.radius;
    list.add_circle([x, y], r * 0.4, LABEL)
        .thickness(2.0)
        .build();
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        list.add_line(
            [x + dx * r * 0.25, y + dy * r * 0.25],
            [x + dx * r * 0.65, y + dy * r * 0.65],
            LABEL,
        )
        .thickness(2.0)
        .build();
    }
}

fn pill(ui: &Ui, list: &DrawListMut<'_>, rect: Rect, label: &str, pressed: bool) {
    let rounding = (rect.max[1] - rect.min[1]) * 0.5;
    let color = if pressed { PRESSED } else { FILL };
    list.add_rect(rect.min, rect.max, color)
        .filled(true)
        .rounding(rounding)
        .build();
    list.add_rect(rect.min, rect.max, LINE)
        .rounding(rounding)
        .thickness(2.0)
        .build();
    let center = [
        f32::midpoint(rect.min[0], rect.max[0]),
        f32::midpoint(rect.min[1], rect.max[1]),
    ];
    centered_text(ui, list, center, label);
}

fn centered_text(ui: &Ui, list: &DrawListMut<'_>, center: [f32; 2], label: &str) {
    let size = ui.calc_text_size(label);
    list.add_text(
        [center[0] - size[0] * 0.5, center[1] - size[1] * 0.5],
        LABEL,
        label,
    );
}
