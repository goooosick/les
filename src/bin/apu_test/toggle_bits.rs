use bit_field::BitField;
use eframe::egui::{self, Ui};

pub fn toggle_bits<T: BitField>(
    ui: &mut Ui,
    b: &mut T,
    (name, text, mask): (&str, &[u8], T),
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.heading(name);
        ui.style_mut().spacing.item_spacing = (1.0, 0.0).into();
        for (i, c) in (0..T::BIT_LENGTH).rev().zip(text.iter().cycle()) {
            changed |= toggle_bit(ui, b, (i, *c, mask.get_bit(i)));
        }
    });

    changed
}

fn toggle_bit<T: BitField>(ui: &mut Ui, b: &mut T, (i, c, toggle): (usize, u8, bool)) -> bool {
    let mut changed = false;
    let mut val = b.get_bit(i);

    let desired_size = ui.spacing().interact_size.y * egui::vec2(1.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() && toggle {
        val = !val;
        b.set_bit(i, val);

        response.mark_changed();
        changed = true;
    }

    let visuals = ui.style().interact_selectable(&response, val);
    let (bg_color, fg_color) = if toggle && val {
        (
            ui.visuals().selection.bg_fill,
            ui.visuals().widgets.active.text_color(),
        )
    } else {
        (
            ui.visuals().widgets.inactive.bg_fill,
            ui.visuals().widgets.inactive.text_color(),
        )
    };
    ui.painter().rect(rect, 1.0, bg_color, visuals.bg_stroke);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        c as char,
        ui.style().body_text_style,
        fg_color,
    );

    changed
}
