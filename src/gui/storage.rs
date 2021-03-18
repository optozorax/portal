use crate::gui::common::*;
use crate::gui::uniform::*;

use egui::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum GetEnum<T> {
    Ok(T),
    NotFound,
    Recursion,
}

#[macro_export]
macro_rules! get_try {
    ($x:expr) => {
        match $x {
            GetEnum::Ok(t) => t,
            other => return other,
        }
    };
}

pub trait StorageElem: Sized + Default {
    type GetType;
    type Input;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        f: F,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<Self::GetType>;

    fn defaults() -> (Vec<String>, Vec<Self>);

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut Self::Input,
        names: &[String],
    ) -> WhatChanged;

    fn errors_count(&self, pos: usize, input: &Self::Input, names: &[String]) -> usize;
}

// Checks if this name is used, sends name to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageWithNames<T> {
    pub names: Vec<String>,
    pub storage: Vec<T>,
}

impl<T: StorageElem> Default for StorageWithNames<T> {
    fn default() -> Self {
        let (names, storage) = T::defaults();
        StorageWithNames { names, storage }
    }
}

impl<T: StorageElem> StorageWithNames<T> {
    pub fn get(
        &self,
        name: &str,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<T::GetType> {
        let mut visited = vec![];
        self.get_inner(name, &mut visited, uniforms, formulas_cache)
    }

    pub fn add(&mut self, name: String, t: T) {
        self.names.push(name);
        self.storage.push(t);
    }

    pub fn remove(&mut self, pos: usize) {
        self.names.remove(pos);
        self.storage.remove(pos);
    }

    pub fn names_iter(&self) -> std::slice::Iter<String> {
        self.names.iter()
    }

    pub fn iter(&self) -> std::iter::Zip<std::slice::Iter<String>, std::slice::Iter<T>> {
        self.names.iter().zip(self.storage.iter())
    }

    fn get_inner<'a>(
        &'a self,
        name: &'a str,
        visited: &mut Vec<String>,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<T::GetType> {
        if visited.iter().any(|x| *x == name) {
            return GetEnum::Recursion;
        }

        visited.push(name.to_owned());
        let pos = if let Some(pos) = self.names.iter().position(|x| x == name) {
            pos
        } else {
            return GetEnum::NotFound;
        };
        let result = get_try!(self.storage[pos].get(
            |name| self.get_inner(name, visited, uniforms, formulas_cache),
            uniforms,
            formulas_cache
        ));
        visited.pop().unwrap();
        GetEnum::Ok(result)
    }

    pub fn rich_egui(&mut self, ui: &mut Ui, input: &mut T::Input, name: &str) -> WhatChanged {
        use std::borrow::Cow;

        let errors_count = self.errors_count(0, input);
        let header = if errors_count > 0 {
            Cow::Owned(format!("{} ({} err)", name, errors_count))
        } else {
            Cow::Borrowed(name)
        };
        let mut changed = WhatChanged::default();
        CollapsingHeader::new(header)
            .id_source(name)
            .default_open(false)
            .show(ui, |ui| {
                changed |= self.egui(ui, input);
            });
        changed
    }
}

impl<T: StorageElem> StorageWithNames<T> {
    pub fn egui(&mut self, ui: &mut Ui, input: &mut T::Input) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut to_delete = None;
        let mut to_move_up = None;
        let mut to_move_down = None;
        let storage = &mut self.storage;
        let names = &mut self.names;
        let len = storage.len();
        for (pos, elem) in storage.iter_mut().enumerate() {
            let errors_count =
                elem.errors_count(pos, input, names) + names[..pos].contains(&names[pos]) as usize;
            CollapsingHeader::new(if errors_count > 0 {
                format!("{} ({} err)", names[pos], errors_count)
            } else {
                names[pos].to_owned()
            })
            .id_source(pos)
            .show(ui, |ui| {
                let previous = names[pos].clone();
                ui.horizontal(|ui| {
                    egui_label(ui, "Name:", 45.);
                    ui.put(
                        Rect::from_min_size(
                            ui.min_rect().min + egui::vec2(49., 0.),
                            egui::vec2(ui.available_width() - 120., 0.),
                        ),
                        TextEdit::singleline(&mut names[pos]),
                    );
                    if ui
                        .add(
                            Button::new("⏶")
                                .text_color(ui.visuals().hyperlink_color)
                                .enabled(pos != 0),
                        )
                        .clicked()
                    {
                        to_move_up = Some(pos);
                    }
                    if ui
                        .add(
                            Button::new("⏷")
                                .text_color(ui.visuals().hyperlink_color)
                                .enabled(pos + 1 != len),
                        )
                        .clicked()
                    {
                        to_move_down = Some(pos);
                    }
                    if ui
                        .add(Button::new("Delete").text_color(Color32::RED))
                        .clicked()
                    {
                        to_delete = Some(pos);
                    }
                });
                if names[..pos].contains(&names[pos]) {
                    ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                        ui.add(Label::new("Error: ").text_color(Color32::RED));
                        ui.label(format!("name '{}' already used", names[pos]));
                    });
                }
                changed.shader |= previous != names[pos];

                changed |= elem.egui(ui, pos, input, names);
            });
        }
        if let Some(pos) = to_delete {
            changed.shader = true;
            self.remove(pos);
        } else if let Some(pos) = to_move_up {
            self.storage.swap(pos, pos - 1);
            self.names.swap(pos, pos - 1);
        } else if let Some(pos) = to_move_down {
            self.storage.swap(pos, pos + 1);
            self.names.swap(pos, pos + 1);
        }
        if ui
            .add(Button::new("Add").text_color(Color32::GREEN))
            .clicked()
        {
            self.add(format!("_{}", self.names.len()), Default::default());
            changed.shader = true;
        }
        changed
    }
}

impl<T: StorageElem> StorageWithNames<T> {
    pub fn errors_count(&self, _: usize, data: &T::Input) -> usize {
        self.storage
            .iter()
            .enumerate()
            .map(|(pos, x)| {
                x.errors_count(pos, data, &self.names)
                    + self.names[..pos].contains(&self.names[pos]) as usize
            })
            .sum()
    }
}
