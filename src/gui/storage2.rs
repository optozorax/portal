use crate::gui::common::*;
use crate::gui::storage::StorageWithNames;
use crate::gui::unique_id::*;
use egui::*;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum StorageInner<T> {
    Named(T, String),
    Inline(T),
}

impl<T: Default> Default for StorageInner<T> {
    fn default() -> Self {
        StorageInner::Inline(T::default())
    }
}

impl<T> AsRef<T> for StorageInner<T> {
    fn as_ref(&self) -> &T {
        use StorageInner::*;
        match self {
            Named(t, _) => t,
            Inline(t) => t,
        }
    }
}

impl<T> AsMut<T> for StorageInner<T> {
    fn as_mut(&mut self) -> &mut T {
        use StorageInner::*;
        match self {
            Named(t, _) => t,
            Inline(t) => t,
        }
    }
}

impl<T> StorageInner<T> {
    fn is_named_as(&self, name: &str) -> bool {
        use StorageInner::*;
        match self {
            Named(_, n) => n == name,
            Inline(_) => false,
        }
    }

    fn is_inline(&self) -> bool {
        use StorageInner::*;
        match self {
            Named(_, _) => false,
            Inline(_) => true,
        }
    }

    fn name(&self) -> Option<&str> {
        use StorageInner::*;
        match self {
            Named(_, n) => Some(n),
            Inline(_) => None,
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Storage2<T: StorageElem2> {
    ids: UniqueIds,
    storage: BTreeMap<UniqueId, StorageInner<T>>,
    storage_order: Vec<UniqueId>,
}

pub struct GetHelper<'a, T: StorageElem2>(&'a Storage2<T>, &'a T::GetInput);

impl<'a, T: StorageElem2> GetHelper<'a, T> {
    pub fn get(&self, id: T::IdWrapper) -> Option<T::GetType> {
        self.0.get(id, self.1)
    }

    pub fn find_id(&self, name: &str) -> Option<T::IdWrapper> {
        self.0.find_id(name)
    }
}

pub struct InlineHelper<'a, T: StorageElem2>(&'a mut Storage2<T>);

impl<'a, T: StorageElem2> InlineHelper<'a, T> {
    pub fn inline(
        &mut self,
        label: &str,
        label_size: f64,
        id: &mut Option<T::IdWrapper>,
        ui: &mut Ui,
        input: &mut T::Input,
        data_id: egui::Id,
    ) -> WhatChanged {
        self.0.inline(label, label_size, id, ui, input, data_id)
    }
}

impl<T: StorageElem2> Storage2<T> {
    pub fn get(&self, id: T::IdWrapper, input: &T::GetInput) -> Option<T::GetType> {
        let mut visited = vec![];
        self.get_inner(id, &mut visited, input)
    }

    pub fn get_original(&self, id: T::IdWrapper) -> Option<&T> {
        self.storage.get(&id.un_wrap()).map(|x| x.as_ref())
    }

    pub fn get_original_mut(&mut self, id: T::IdWrapper) -> Option<&mut T> {
        self.storage.get_mut(&id.un_wrap()).map(|x| x.as_mut())
    }

    /// First Option shows is element presented or not. Second Option is represente is element has a name
    pub fn get_name(&self, id: T::IdWrapper) -> Option<Option<&str>> {
        self.storage.get(&id.un_wrap()).map(|x| x.name())
    }

    pub fn find_id(&self, name: &str) -> Option<T::IdWrapper> {
        self.storage_order
            .iter()
            .find(|id| {
                self.storage
                    .get(id)
                    .map(|elem| elem.is_named_as(name))
                    .unwrap_or(false)
            })
            .map(|id| T::IdWrapper::wrap(*id))
    }

    pub fn all_ids(&self) -> impl Iterator<Item = T::IdWrapper> + '_ {
        self.storage.iter().map(|(key, _)| T::IdWrapper::wrap(*key))
    }

    fn get_inner(
        &self,
        id: T::IdWrapper,
        visited: &mut Vec<T::IdWrapper>,
        input: &T::GetInput,
    ) -> Option<T::GetType> {
        if visited.iter().any(|x| x.un_wrap() == id.un_wrap()) {
            return None;
        }

        visited.push(id);
        let result = self
            .storage
            .get(&id.un_wrap())?
            .as_ref()
            .get(&GetHelper(self, input), input);
        visited.pop().unwrap();
        result
    }

    fn remove(&mut self, id: T::IdWrapper, input: &mut T::Input) {
        let id = id.un_wrap();
        let element = self.storage.remove(&id).unwrap();
        self.ids.remove_existing(id);
        if let Some(pos) = self.storage_order.iter().position(|x| *x == id) {
            self.storage_order.remove(pos);
        }

        // Recursively delete inline elements
        element
            .as_ref()
            .remove(|id, input| self.remove_as_field(id, input), input);
    }

    pub fn set(&mut self, id: T::IdWrapper, value: T) {
        if let Some(t) = self.storage.get_mut(&id.un_wrap()) {
            *t.as_mut() = value;
        } else {
            crate::error!()
        }
    }

    pub fn set_id(&mut self, id: T::IdWrapper, from: T::IdWrapper) {
        let id = id.un_wrap();
        let from = from.un_wrap();

        if let Some(t) = self.storage.get(&from).map(|x| (*x).as_ref().clone()) {
            *self.storage.get_mut(&id).unwrap().as_mut() = t;
        } else {
            crate::error!();
        }
    }

    fn push_default(&mut self) {
        let id = self.ids.get_unique();
        self.storage_order.push(id);
        self.storage.insert(
            id,
            StorageInner::Named(Default::default(), format!("id{}", id)),
        );
    }

    /// Removes field like it is was field of someone, then recursively removes inside content if it's inline.
    pub fn remove_as_field(&mut self, id: T::IdWrapper, input: &mut T::Input) {
        let id = id.un_wrap();
        if self.storage.get(&id).unwrap().is_inline() {
            let element = self.storage.remove(&id).unwrap();
            self.ids.remove_existing(id);

            element
                .as_ref()
                .remove(|id, input| self.remove_as_field(id, input), input);
        }
    }

    fn remove_by_pos(&mut self, pos: usize, input: &mut T::Input) {
        let id = self.storage_order.remove(pos);
        self.remove(T::IdWrapper::wrap(id), input);
    }

    pub fn visible_elements(&self) -> impl Iterator<Item = (T::IdWrapper, &str)> + '_ {
        let storage = &self.storage;
        self.storage_order.iter().map(move |id| {
            (
                T::IdWrapper::wrap(*id),
                storage.get(id).unwrap().name().unwrap(),
            )
        })
    }

    pub fn egui(&mut self, ui: &mut Ui, input: &mut T::Input, name: &str) -> WhatChanged {
        use std::borrow::Cow;

        let data_id = ui.make_persistent_id(name).with("inner");

        let errors_count = self.errors_count_all(input);
        let header = if errors_count > 0 {
            Cow::Owned(format!("{} ({} err)", name, errors_count))
        } else {
            Cow::Borrowed(name)
        };
        let mut changed = WhatChanged::default();
        egui::CollapsingHeader::new(header)
            .id_source(name)
            .default_open(false)
            .show(ui, |ui| {
                changed |= self.egui_inner(ui, input, data_id);
            });
        changed
    }

    fn egui_inner(&mut self, ui: &mut Ui, input: &mut T::Input, data_id: egui::Id) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut to_delete = None;
        let mut to_move_up = None;
        let mut to_move_down = None;

        let mut storage_order = Vec::new();
        std::mem::swap(&mut storage_order, &mut self.storage_order);

        let len = storage_order.len();
        for (pos, id) in storage_order.iter().enumerate() {
            let errors_count = self.errors_count_id(T::IdWrapper::wrap(*id), input);

            let mut elem = StorageInner::default();
            std::mem::swap(&mut elem, self.storage.get_mut(id).unwrap());

            if let StorageInner::Named(elem, name) = &mut elem {
                let name_error = self.storage.iter().any(|x| x.1.is_named_as(&name));

                let errors_count = errors_count + name_error as usize;

                let header_name = if errors_count > 0 {
                    format!("{} ({} err)", name, errors_count)
                } else {
                    name.clone()
                };

                egui::CollapsingHeader::new(header_name)
                    .id_source(id)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            egui_label(ui, "Name:", 45.);
                            ui.with_layout(Layout::right_to_left(), |ui| {
                                if ui
                                    .add(Button::new("Delete").text_color(Color32::RED))
                                    .clicked()
                                {
                                    to_delete = Some(pos);
                                }
                                if ui
                                .add(
                                    Button::new("⏷") // down
                                        .text_color(ui.visuals().hyperlink_color)
                                        .enabled(pos + 1 != len),
                                )
                                .clicked()
                            {
                                to_move_down = Some(pos);
                            }
                                if ui
                                .add(
                                    Button::new("⏶") // up
                                        .text_color(ui.visuals().hyperlink_color)
                                        .enabled(pos != 0),
                                )
                                .clicked()
                            {
                                to_move_up = Some(pos);
                            }
                                let mut name_response = egui_with_red_field(ui, name_error, |ui| {
                                    ui.text_edit_singleline(name)
                                });
                                changed.shader |= name_response.changed();
                                if !T::SAFE_TO_RENAME {
                                    name_response = name_response.on_hover_text(
                                        "This name is not safe to rename, you will\n\
                                    need to rename it in other places by yourself",
                                    );
                                }
                                if name_error {
                                    name_response.on_hover_text(format!(
                                        "Error: name '{}' already used",
                                        name
                                    ));
                                }
                            });
                        });

                        changed |= elem.egui(
                            ui,
                            input,
                            &mut InlineHelper(self),
                            data_id.with(pos),
                            T::IdWrapper::wrap(*id),
                        );
                    });
            } else {
                ui.label("Internal error, this is inline element, it shouldn't be here.");
            }

            std::mem::swap(&mut elem, self.storage.get_mut(id).unwrap());
        }
        std::mem::swap(&mut storage_order, &mut self.storage_order);

        if let Some(pos) = to_delete {
            changed.shader = true;
            self.remove_by_pos(pos, input);
        } else if let Some(pos) = to_move_up {
            self.storage_order.swap(pos, pos - 1);
        } else if let Some(pos) = to_move_down {
            self.storage_order.swap(pos, pos + 1);
        }

        if ui
            .add(Button::new("Add").text_color(Color32::GREEN))
            .clicked()
        {
            self.push_default();
            changed.shader = true;
        }

        changed
    }

    pub fn inline(
        &mut self,
        label: &str,
        label_size: f64,
        id: &mut Option<T::IdWrapper>,
        ui: &mut Ui,
        input: &mut T::Input,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();

        ui.vertical(|ui| {
            if let Some(id_inner) = id {
                if self.storage.get(&id_inner.un_wrap()).is_none() {
                    crate::error!(format, "id {:?} transformed to `None`", id_inner);
                    *id = None;
                    changed.uniform = true;
                }
            }

            let mut inline = if let Some(id_inner) = id {
                self.storage
                    .get(&id_inner.un_wrap())
                    .map(|x| x.is_inline())
                    .unwrap() // Because we earlier checked this
            } else {
                false
            };

            ui.horizontal(|ui| {
                egui_label(ui, label, label_size);

                ui.with_layout(Layout::right_to_left(), |ui| {
                    if ui
                        .add(egui::SelectableLabel::new(inline, "📌"))
                        .on_hover_text(
                            "Toggle inline anonymous element instead\nof referencing to name of the other.",
                        )
                        .clicked()
                    {
                        if inline {
                            if let Some(id) = id {
                                self.remove(*id, input);
                                ui.memory().id_data.remove(&data_id);
                            }
                        }

                        inline = !inline;

                        if inline {
                            let new_id = self.ids.get_unique();
                            self.storage
                                .insert(new_id, StorageInner::Inline(Default::default()));
                            *id = Some(T::IdWrapper::wrap(new_id));
                        } else {
                            *id = None;
                        }
                    }

                    if inline {
                        ui.label("inline");
                    } else {
                        // Named
                        changed.uniform |= self.find_named_id(id, ui, data_id);
                    }
                });
            });

            if inline {
                // id now must be correct
                with_swapped!(elem => (*self.storage.get_mut(&id.unwrap().un_wrap()).unwrap()); {
                    ui.group(|ui| {
                        changed |= elem.0.as_mut().egui(ui, input, &mut InlineHelper(self), data_id.with("inline"), id.unwrap());
                    });
                });
            }
        });

        changed
    }

    fn find_named_id(&self, id: &mut Option<T::IdWrapper>, ui: &mut Ui, data_id: egui::Id) -> bool {
        let mut current_name = if let Some(id_inner) = id {
            self.storage
                .get(&id_inner.un_wrap())
                .unwrap()
                .name()
                .unwrap()
                .to_owned()
        } else {
            ui.memory()
                .id_data
                .get_or_default::<String>(data_id)
                .clone()
        };

        let changed = ui
            .horizontal(|ui| {
                let mut response = egui_with_red_field(ui, id.is_none(), |ui| {
                    ui.text_edit_singleline(&mut current_name)
                });
                if id.is_none() {
                    response = response.on_hover_text("This name is not found");
                }

                response.changed()
            })
            .inner;
        if changed {
            if let Some((new_id, _)) = self
                .storage
                .iter()
                .find(|(_, elem)| elem.is_named_as(&current_name))
            {
                *id = Some(T::IdWrapper::wrap(*new_id));
                ui.memory().id_data.remove(&data_id);
            } else {
                *id = None;
                ui.memory().id_data.insert(data_id, current_name);
            }
        }

        changed
    }

    pub fn inline_only_name(
        &mut self,
        label: &str,
        label_size: f64,
        id: &mut Option<T::IdWrapper>,
        ui: &mut Ui,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        ui.vertical(|ui| {
            if let Some(id_inner) = id {
                if self.storage.get(&id_inner.un_wrap()).is_none() {
                    crate::error!(format, "id {:?} transformed to `None`", id_inner);
                    *id = None;
                    changed.uniform = true;
                }
            }

            ui.horizontal(|ui| {
                egui_label(ui, label, label_size);
                changed.uniform |= self.find_named_id(id, ui, data_id);
            });
        });

        changed
    }

    // This element is inline, so errors counted only for inline elements and inner inline elements
    pub fn errors_inline(&self, id: T::IdWrapper, input: &T::Input) -> usize {
        let mut visited = vec![];
        self.errors_count_inner(id, &mut visited, input, false)
    }

    pub fn errors_count_all(&self, input: &T::Input) -> usize {
        self.storage_order
            .iter()
            .map(|id| self.errors_count_id(T::IdWrapper::wrap(*id), input))
            .sum()
    }

    // Errors count for current id, it must be not inline element.
    pub fn errors_count_id(&self, id: T::IdWrapper, input: &T::Input) -> usize {
        let mut visited = vec![];
        self.errors_count_inner(id, &mut visited, input, true)
    }

    fn errors_count_inner(
        &self,
        id: T::IdWrapper,
        visited: &mut Vec<T::IdWrapper>,
        input: &T::Input,
        allow_not_inline: bool,
    ) -> usize {
        if let Some(elem) = self.storage.get(&id.un_wrap()) {
            if !allow_not_inline && !elem.is_inline() {
                return 0;
            }

            if visited.iter().any(|x| x.un_wrap() == id.un_wrap()) {
                return 0;
            }

            visited.push(id);
            let result = elem.as_ref().errors_count(
                |id| self.errors_count_inner(id, visited, input, false),
                input,
                id,
            );
            visited.pop().unwrap();
            result
        } else {
            1
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.storage_order.len()
    }
}

pub trait Wrapper:
    Clone
    + std::fmt::Debug
    + Copy
    + Eq
    + PartialEq
    + Ord
    + PartialOrd
    + std::hash::Hash
    + serde::Serialize
    + for<'a> serde::Deserialize<'a>
{
    fn wrap(t: UniqueId) -> Self;
    fn un_wrap(self) -> UniqueId;
}

pub trait StorageElem2: Sized + Default + Clone + Serialize {
    type IdWrapper: Wrapper;
    type GetType;

    const SAFE_TO_RENAME: bool;

    type Input;
    type GetInput;

    fn egui(
        &mut self,
        ui: &mut Ui,
        input: &mut Self::Input,
        inline_helper: &mut InlineHelper<Self>,
        data_id: egui::Id,
        self_id: Self::IdWrapper,
    ) -> WhatChanged;

    fn get(&self, get_helper: &GetHelper<Self>, input: &Self::GetInput) -> Option<Self::GetType>;

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, f: F, input: &mut Self::Input);

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        f: F,
        input: &Self::Input,
        self_id: Self::IdWrapper,
    ) -> usize;
}

impl<T: StorageElem2> From<StorageWithNames<T>> for Storage2<T> {
    fn from(_: StorageWithNames<T>) -> Storage2<T> {
        todo!()
    }
}
