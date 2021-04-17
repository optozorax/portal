use crate::gui::common::*;

use egui::*;
use glam::*;

pub trait ComboBoxChoosable {
    fn variants() -> &'static [&'static str];
    fn get_number(&self) -> usize;
    fn set_number(&mut self, number: usize);
}

pub fn egui_combo_label<T: ComboBoxChoosable>(
    ui: &mut Ui,
    label: &str,
    size: f64,
    t: &mut T,
) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    ui.horizontal(|ui| {
        egui_label(ui, label, size);
        for (pos, name) in T::variants().iter().enumerate() {
            ui.selectable_value(&mut current_type, pos, *name);
        }
    });

    if current_type != previous_type {
        t.set_number(current_type);
        is_changed = true;
    }

    is_changed
}

pub fn egui_combo_box<T: ComboBoxChoosable>(
    ui: &mut Ui,
    label: &str,
    size: f64,
    t: &mut T,
) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    ui.horizontal(|ui| {
        egui_label(ui, label, size);
        egui::ComboBox::from_label("")
            .selected_text(T::variants()[current_type])
            .show_ui(ui, |ui| {
                for (pos, name) in T::variants().iter().enumerate() {
                    ui.selectable_value(&mut current_type, pos, *name);
                }
            });
    });

    if current_type != previous_type {
        t.set_number(current_type);
        is_changed = true;
    }

    is_changed
}
