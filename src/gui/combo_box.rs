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
    size: f32,
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
    size: f32,
    t: &mut T,
    id: usize,
) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    let id = ui.make_persistent_id(id);
    ui.horizontal(|ui| {
        egui_label(ui, label, size);
        egui::combo_box(ui, id, T::variants()[current_type], |ui| {
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
