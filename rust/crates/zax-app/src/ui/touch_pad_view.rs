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
    circle_button(ui, &list, layout.menu, "", view.is_pressed(TouchControl::Menu));
    // The menu's three bars.
    let [x, y] = layout.menu.center;
    let width = layout.menu.radius * 0.5;
    for row in [-1.0, 0.0, 1.0] {
        let line_y = y + row * layout.menu.radius * 0.35;
        list.add_line([x - width, line_y], [x + width, line_y], LABEL)
            .thickness(2.0)
            .build();
    }
    pill(ui, &list, layout.select, "SELECT", view.is_pressed(TouchControl::Select));
    pill(ui, &list, layout.start, "START", view.is_pressed(TouchControl::Start));
}

fn draw_dpad(list: &DrawListMut<'_>, dpad: Circle, view: &TouchPadView) {
    let [cx, cy] = dpad.center;
    let (reach, half) = (dpad.radius, dpad.radius * 0.34);
    let arms = [
        (TouchControl::Up, [cx - half, cy - reach], [cx + half, cy - half]),
        (TouchControl::Down, [cx - half, cy + half], [cx + half, cy + reach]),
        (TouchControl::Left, [cx - reach, cy - half], [cx - half, cy + half]),
        (TouchControl::Right, [cx + half, cy - half], [cx + reach, cy + half]),
    ];
    list.add_rect([cx - half, cy - half], [cx + half, cy + half], FILL)
        .filled(true)
        .build();
    for (control, min, max) in arms {
        let color = if view.is_pressed(control) { PRESSED } else { FILL };
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
        (rect.min[0] + rect.max[0]) * 0.5,
        (rect.min[1] + rect.max[1]) * 0.5,
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
