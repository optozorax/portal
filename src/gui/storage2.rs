use crate::gui::combo_box::*;
use crate::gui::common::*;
use egui::*;

use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Id(usize);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct Ids {
    available: VecDeque<Id>,
    max: usize,
}

impl Ids {
    pub fn get_unique(&mut self) -> Id {
        if let Some(result) = self.available.pop_front() {
            result
        } else {
            let result = Id(self.max);
            self.max += 1;
            result
        }
    }

    pub fn remove_existing(&mut self, id: Id) {
        self.available.push_back(id);
        self.available.make_contiguous().sort();
        while self
            .available
            .back()
            .map(|x| x.0 == self.max - 1)
            .unwrap_or(false)
        {
            self.max -= 1;
            self.available.pop_back().unwrap();
        }
    }
}

#[cfg(test)]
mod id_test {
    use super::*;

    #[test]
    fn test() {
        let mut ids = Ids::default();
        assert_eq!(ids.get_unique().0, 0);
        assert_eq!(ids.get_unique().0, 1);
        assert_eq!(ids.get_unique().0, 2);
        assert_eq!(ids.get_unique().0, 3);
        ids.remove_existing(Id(2));
        assert_eq!(
            ids,
            Ids {
                available: vec![Id(2)].into_iter().collect(),
                max: 4,
            }
        );
        ids.remove_existing(Id(3));
        assert_eq!(
            ids,
            Ids {
                available: vec![].into_iter().collect(),
                max: 2,
            }
        );
        ids.remove_existing(Id(1));
        assert_eq!(
            ids,
            Ids {
                available: vec![].into_iter().collect(),
                max: 1,
            }
        );
        assert_eq!(ids.get_unique().0, 1);
        ids.remove_existing(Id(0));
        assert_eq!(ids.get_unique().0, 0);
    }
}

#[derive(Clone, Debug)]
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

#[derive(Default, Clone, Debug)]
pub struct Storage2<T> {
    ids: Ids,
    storage: BTreeMap<Id, StorageInner<T>>,
    storage_order: Vec<Id>,
}

impl<T: StorageElem2> Storage2<T> {
    pub fn get(&self, id: T::IdWrapper, input: &T::GetInput) -> Option<T::GetType> {
        let mut visited = vec![];
        self.get_inner(id, &mut visited, input)
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
            .get(|id| self.get_inner(id, visited, input), input);
        visited.pop().unwrap();
        result
    }

    pub fn remove(&mut self, id: T::IdWrapper) {
        let id = id.un_wrap();
        self.storage.remove(&id);
        self.ids.remove_existing(id);
        if let Some(pos) = self.storage_order.iter().find(|x| **x == id) {
            self.storage.remove(pos);
        }
        // TODO: recursively delete other elements, require recursive travel by trait
    }

    pub fn visible_elements<'a>(&'a self) -> impl Iterator<Item = T::IdWrapper> + 'a {
        self.storage_order.iter().map(|id| T::IdWrapper::wrap(*id))
    }

    fn remove_by_pos(&mut self, pos: usize) {
        let id = self.storage_order.remove(pos);
        self.storage.remove(&id);
        self.ids.remove_existing(id);
    }

    pub fn egui(&mut self, ui: &mut Ui, input: &mut T::EguiInput, name: &str) -> WhatChanged {
        use std::borrow::Cow;

        let data_id = ui.make_persistent_id(name);

        let errors_count = self.errors_count_all((*input).as_t());
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

    fn egui_inner(&mut self, ui: &mut Ui, input: &mut T::EguiInput, data_id: egui::Id) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut to_delete = None;
        let mut to_move_up = None;
        let mut to_move_down = None;

        let mut storage_order = Vec::new();
        std::mem::swap(&mut storage_order, &mut self.storage_order);

        let len = storage_order.len();
        for (pos, id) in storage_order.iter().enumerate() {
            let errors_count = self.errors_count_id(T::IdWrapper::wrap(*id), (*input).as_t());

            let mut elem = StorageInner::default();
            std::mem::swap(&mut elem, self.storage.get_mut(id).unwrap());

            if let StorageInner::Named(elem, name) = &mut elem {
                let name_error = self
                    .storage
                    .iter()
                    .any(|x| x.1.is_named_as(&name));

                let errors_count = errors_count + name_error as usize;

                let header_name = if errors_count > 0 {
                    format!("{} ({} err)", name, errors_count)
                } else {
                    name.clone()
                };

                egui::CollapsingHeader::new(header_name)
                .id_source(id)
                .show(ui, |ui| {
                    let mut name_response = None;
                    ui.horizontal(|ui| {
                        egui_label(ui, "Name:", 45.);
                        name_response = Some(ui.put(
                            Rect::from_min_size(
                                ui.min_rect().min + egui::vec2(49., 0.),
                                egui::vec2(ui.available_width() - 120., 0.),
                            ),
                            TextEdit::singleline(name),
                        ));
                        changed.shader |= name_response.as_ref().unwrap().changed();
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
                            .add(Button::new("Delete").text_color(Color32::RED))
                            .clicked()
                        {
                            to_delete = Some(pos);
                        }
                    });
                    if name_error {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            ui.add(Label::new("Error: ").text_color(Color32::RED));
                            ui.label(format!("name '{}' already used", name));
                        });
                    }
                    if !T::SAFE_TO_RENAME && name_response.unwrap().has_focus() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            ui.add(Label::new("Note: ").text_color(ui.visuals().hyperlink_color));
                            ui.label("this name is not safe to rename, you will need to rename it in other places by yourself");
                        });
                    }

                    changed |= elem.egui(ui, input, self, data_id.with(pos));
                });
            } else {
                ui.label("Internal error, this is inline element, it shouldn't be here.");
            }

            std::mem::swap(&mut elem, self.storage.get_mut(id).unwrap());
        }
        std::mem::swap(&mut storage_order, &mut self.storage_order);

        if let Some(pos) = to_delete {
            changed.shader = true;
            self.remove_by_pos(pos);
        } else if let Some(pos) = to_move_up {
            self.storage_order.swap(pos, pos - 1);
        } else if let Some(pos) = to_move_down {
            self.storage_order.swap(pos, pos + 1);
        }

        if ui
            .add(Button::new("Add").text_color(Color32::GREEN))
            .clicked()
        {
            let id = self.ids.get_unique();
            self.storage_order.push(id);
            self.storage.insert(
                id,
                StorageInner::Named(Default::default(), format!("_{}", id.0)),
            );
            changed.shader = true;
        }

        changed
    }

    pub fn inline(
        &mut self,
        id: &mut Option<T::IdWrapper>,
        ui: &mut Ui,
        input: &mut T::EguiInput,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();

        if let Some(id_inner) = id {
            if self.storage.get(&id_inner.un_wrap()).is_none() {
                eprintln!("id {:?} transformed to `None`", id_inner.un_wrap());
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

        // 📌 — inline
        // or 📎 — inline

        if ui
            .add(egui::SelectableLabel::new(inline, "📌"))
            .on_hover_text("Toggle inline anonymous element instead\nof referencing to name of the other.")
            .clicked() 
        {
            if inline {
                if let Some(id) = id {
                    self.remove(*id);
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
            // id now must be correct
            with_swapped!(elem => (*self.storage.get_mut(&id.unwrap().un_wrap()).unwrap()); {
                ui.group(|ui| {
                    changed |= elem.0.as_mut().egui(ui, input, self, data_id.with("inline"));    
                });
            });
        } else {
            // Named
            let mut current_name = if let Some(id_inner) = id {
                self.storage.get(&id_inner.un_wrap()).unwrap().name().unwrap().to_owned()
            } else {
                ui.memory().id_data.get_or_default::<String>(data_id).clone()
            };

            let changed = ui.horizontal(|ui| {
                egui_label(ui, "Name:", 45.);
                egui_with_red_field(ui, id.is_none(), |ui| ui.text_edit_singleline(&mut current_name)).changed()
            }).inner;
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
        }

        changed
    }

    pub fn errors_count_all(&self, input: &T::ErrInput) -> usize {
        self.storage_order
            .iter()
            .map(|id| self.errors_count_id(T::IdWrapper::wrap(*id), input))
            .sum()
    }

    pub fn errors_count_id(&self, id: T::IdWrapper, input: &T::ErrInput) -> usize {
        let mut visited = vec![];
        self.errors_count_inner(id, &mut visited, input)
    }

    fn errors_count_inner(
        &self,
        id: T::IdWrapper,
        visited: &mut Vec<T::IdWrapper>,
        input: &T::ErrInput,
    ) -> usize {
        if visited.iter().any(|x| x.un_wrap() == id.un_wrap()) {
            return 0;
        }

        visited.push(id);
        let result = self
            .storage
            .get(&id.un_wrap())
            .map(|elem| {
                elem.as_ref()
                    .errors_count(|id| self.errors_count_inner(id, visited, input), input)
            })
            .unwrap_or(1);
        visited.pop().unwrap();
        result
    }
}

pub trait Wrapper<T> {
    fn wrap(t: T) -> Self;
    fn un_wrap(self) -> T;
}

pub trait As<T> {
    fn as_t(&self) -> &T;
}

impl<T> As<T> for T {
    fn as_t(&self) -> &T {
        self
    }
}

pub trait StorageElem2: Sized + Default {
    type IdWrapper: Wrapper<Id> + Copy;
    type GetType;

    const SAFE_TO_RENAME: bool;

    type EguiInput: As<Self::ErrInput>;
    type GetInput;
    type ErrInput;

    fn egui(
        &mut self,
        ui: &mut Ui,
        input: &mut Self::EguiInput,
        self_storage: &mut Storage2<Self>,
        data_id: egui::Id,
    ) -> WhatChanged;

    fn get<F: FnMut(Self::IdWrapper) -> Option<Self::GetType>>(
        &self,
        f: F,
        input: &Self::GetInput,
    ) -> Option<Self::GetType>;

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        f: F,
        input: &Self::ErrInput,
    ) -> usize;
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arithmetic {
    Float(f32),
    Sum(Option<ArithmeticId>, Option<ArithmeticId>),
    Mul(Option<ArithmeticId>, Option<ArithmeticId>),
}

impl ComboBoxChoosable for Arithmetic {
    fn variants() -> &'static [&'static str] {
        &["Float", "Sum", "Mul"]
    }
    fn get_number(&self) -> usize {
        use Arithmetic::*;
        match self {
            Float { .. } => 0,
            Sum { .. } => 1,
            Mul { .. } => 2,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Arithmetic::*;
        *self = match number {
            0 => Float(0.0),
            1 => Sum(None, None),
            2 => Mul(None, None),
            _ => unreachable!(),
        };
    }
}

impl Default for Arithmetic {
    fn default() -> Self {
        Arithmetic::Float(0.0)
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ArithmeticId(Id);

impl Wrapper<Id> for ArithmeticId {
    fn wrap(id: Id) -> Self {
        ArithmeticId(id)
    }
    fn un_wrap(self) -> Id {
        self.0
    }
}

impl StorageElem2 for Arithmetic {
    type IdWrapper = ArithmeticId;
    type GetType = f32;

    const SAFE_TO_RENAME: bool = false;

    type EguiInput = ();
    type GetInput = ();
    type ErrInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        (): &mut Self::EguiInput,
        self_storage: &mut Storage2<Self>,
        data_id: egui::Id,
    ) -> WhatChanged {
        use Arithmetic::*;

        egui_combo_label(ui, "Type:", 45., self);

        match self {
            Float(f) => WhatChanged::from_uniform(egui_f32(ui, f)),
            Sum(a, b) => {
                let mut result = WhatChanged::default();

                ui.label("Sum first argument:");
                result |= self_storage.inline(&mut *a, ui, &mut (), data_id.with(0));

                ui.label("Sum second argument:");
                result |= self_storage.inline(&mut *b, ui, &mut (), data_id.with(1));

                result
            }
            Mul(a, b) => {
                let mut result = WhatChanged::default();

                ui.label("Mul first argument:");
                result |= self_storage.inline(&mut *a, ui, &mut (), data_id.with(0));

                ui.label("Mul second argument:");
                result |= self_storage.inline(&mut *b, ui, &mut (), data_id.with(1));

                result
            }
        }
    }

    fn get<F: FnMut(Self::IdWrapper) -> Option<Self::GetType>>(
        &self,
        mut f: F,
        (): &Self::GetInput,
    ) -> Option<Self::GetType> {
        use Arithmetic::*;
        match self {
            Float(f) => Some(*f),
            Sum(a, b) => Some(f((*a)?)? + f((*b)?)?),
            Mul(a, b) => Some(f((*a)?)? * f((*b)?)?),
        }
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        mut f: F,
        (): &Self::ErrInput,
    ) -> usize {
        use Arithmetic::*;
        match self {
            Float(_) => 0,
            Sum(a, b) => a.map(|a| f(a)).unwrap_or(1) + b.map(|b| f(b)).unwrap_or(1),
            Mul(a, b) => a.map(|a| f(a)).unwrap_or(1) + b.map(|b| f(b)).unwrap_or(1),
        }
    }
}

/*

Фичи:
    * не имена а айдишники
    * имена ограничены только английскими буквами, цифрами, _, не с цифры начинается
    * есть инлайн элементы, который скрыты в основном хранилище, но используются кем-то
    * есть метод для рисования юая написания имени, где можно задать инлайн элементы, и где автоматически происходит ассоциирование с айдишником
    * можно получить элемент используя рекурсию
    * айдишники лежат под специальным типом-обёрткой, чтобы не путаться в типах когда работаю с разными хранилищами
    * ассоциированная константа «Is it safe to rename», которая если задана в true, то человеку говорится при ренейминге, что тут придётся вручную переименовать во всех остальных местах

    * сделать что-то для сборки мусора

Очень сложные фичи:
    * можно было включать режим переноса вещей, чтобы их можно было двигать драг&дропом
    * группировка элементов? (чтобы имя этого прибавлялось внутрь?)
    * при изменении имени вызывается метод, который позволяет изменять это имя в местах, где используются не айдишники
    * есть подсчёт использований, и благодаря этому можно любой элемент превратить в инлайн

 */
