use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::matrix::Matrix;
use crate::gui::storage2::GetHelper;
use crate::gui::storage2::*;
use crate::gui::uniform::*;
use crate::gui::unique_id::UniqueId;
use egui::*;
use glam::*;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::hlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Animation<T: StorageElem2> {
    ProvidedToUser,
    FromDev,
    Changed(Option<T::IdWrapper>),
    ChangedAndToUser(Option<T::IdWrapper>),
}

impl<T: StorageElem2> Animation<T> {
    pub fn get_t(&self) -> Option<&T::IdWrapper> {
        if let Animation::Changed(Some(t)) | Animation::ChangedAndToUser(Some(t)) = self {
            Some(t)
        } else {
            None
        }
    }
}

impl<T: StorageElem2> Default for Animation<T> {
    fn default() -> Self {
        Animation::FromDev
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageChanging<T: StorageElem2>(HashMap<T::IdWrapper, Animation<T>>);

impl<T: StorageElem2> Default for StageChanging<T> {
    fn default() -> Self {
        StageChanging(HashMap::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevStageChanging<T: StorageElem2>(HashMap<T::IdWrapper, T>);

impl<T: StorageElem2> Default for DevStageChanging<T> {
    fn default() -> Self {
        DevStageChanging(HashMap::new())
    }
}

impl<T: StorageElem2> StageChanging<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        global: &mut GlobalStage<T>,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let visible_elements = storage
            .visible_elements()
            .map(|(id, name)| (id, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name) in visible_elements {
            let global = global.0.entry(id).or_default();
            let anim = self.0.entry(id).or_default();
            if *global {
                ui.horizontal(|ui| {
                    egui_label(ui, &name, 60.);
                    ui.label("Global parameter");
                });
            } else {
                changed |= anim.egui(ui, storage, input, &name, data_id.with(id));
            }
        }
        changed
    }

    pub fn remove(&self, storage: &mut Storage2<T>, input: &mut T::Input) {
        for id in self.0.values().filter_map(|x| x.get_t()) {
            storage.remove_as_field(*id, input);
        }
    }

    pub fn errors_count(&self, storage: &Storage2<T>, input: &T::Input) -> usize {
        self.0
            .values()
            .map(|x| {
                x.get_t()
                    .map(|id| storage.errors_inline(*id, input))
                    .unwrap_or(0)
            })
            .sum::<usize>()
    }

    pub fn init_stage(&self, storage: &mut Storage2<T>, dev_stage: &DevStageChanging<T>) {
        for (id, uniform) in self.0.iter() {
            if let Some(new_id) = uniform.get_t() {
                storage.set_id(*id, *new_id);
            } else if let Animation::FromDev = uniform {
                storage.set(*id, dev_stage.0.get(id).unwrap().clone());
            }
        }
    }

    pub fn user_egui(
        &self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        user_egui: impl Fn(&mut T, &mut Ui) -> WhatChanged,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        for (id, element) in self.0.iter() {
            use Animation::*;
            match element {
                ProvidedToUser | ChangedAndToUser(_) => drop(ui.horizontal(|ui| {
                    let name = storage.get_name(*id).unwrap().unwrap().to_owned();
                    let element = storage.get_original_mut(*id).unwrap();
                    ui.label(name);
                    changed |= user_egui(element, ui);
                })),
                FromDev => {}
                Changed(_) => {}
            }
        }
        changed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevStage {
    pub uniforms: DevStageChanging<AnyUniform>,
    pub matrices: DevStageChanging<Matrix>,
}

impl<T: StorageElem2> DevStageChanging<T> {
    pub fn init_stage(&self, storage: &mut Storage2<T>) {
        for (id, value) in self.0.iter() {
            storage.set(*id, value.clone());
        }
    }

    pub fn copy(&mut self, storage: &Storage2<T>) {
        self.0.clear();
        for (id, _) in storage.visible_elements() {
            let value = storage.get_original(id).unwrap().clone();
            self.0.insert(id, value);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStage<T: StorageElem2>(HashMap<T::IdWrapper, bool>);

impl<T: StorageElem2> Default for GlobalStage<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: StorageElem2> GlobalStage<T> {
    pub fn egui(&mut self, ui: &mut Ui, storage: &mut Storage2<T>) -> bool {
        let mut changed = false;
        for (id, name) in storage.visible_elements() {
            let enabled = self.0.entry(id).or_default();
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
        }
        changed
    }

    pub fn user_egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        user_egui: impl Fn(&mut T, &mut Ui) -> WhatChanged,
    ) -> WhatChanged {
        let mut result = WhatChanged::default();
        if self.0.iter().any(|x| *x.1) {
            for id in self
                .0
                .iter()
                .filter(|(_, has)| **has)
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
            {
                if let Some((name, element)) = storage
                    .get_name(id)
                    .flatten()
                    .map(|x| x.to_owned())
                    .zip(storage.get_original_mut(id))
                {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        result |= user_egui(element, ui);
                    });
                } else {
                    self.0.remove(&id);
                }
            }
            ui.separator();
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationStage {
    pub uniforms: StageChanging<AnyUniform>,
    pub matrices: StageChanging<Matrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalUserUniforms {
    pub uniforms: GlobalStage<AnyUniform>,
    pub matrices: GlobalStage<Matrix>,
}

impl<T: StorageElem2> ComboBoxChoosable for Animation<T> {
    fn variants() -> &'static [&'static str] {
        &["To user", "From dev", "Changed", "Changed + To user"]
    }

    fn get_number(&self) -> usize {
        use Animation::*;
        match self {
            ProvidedToUser => 0,
            FromDev => 1,
            Changed { .. } => 2,
            ChangedAndToUser { .. } => 3,
        }
    }

    fn set_number(&mut self, number: usize) {
        use Animation::*;
        *self = match number {
            0 => ProvidedToUser,
            1 => FromDev,
            2 => Changed(None),
            3 => ChangedAndToUser(None),
            _ => unreachable!(),
        };
    }
}

impl GlobalUserUniforms {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        uniforms: &mut Storage2<AnyUniform>,
        matrices: &mut Storage2<Matrix>,
    ) -> WhatChanged {
        let mut changed = false;
        changed |= self.uniforms.egui(ui, uniforms);
        ui.separator();
        changed |= self.matrices.egui(ui, matrices);
        WhatChanged::from_uniform(changed)
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AnimationId(UniqueId);

impl Wrapper for AnimationId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl<T: StorageElem2> Animation<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        name: &str,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        ui.horizontal(|ui| {
            changed.uniform |= egui_combo_box(ui, &name, 60., self, data_id.with("combo"));
            if let Animation::Changed(x) | Animation::ChangedAndToUser(x) = self {
                changed |= storage.inline(&name, 60.0, x, ui, input, data_id);
            }
        });
        changed
    }
}

impl StorageElem2 for AnimationStage {
    type IdWrapper = AnimationId;
    type GetType = AnimationStage;

    const SAFE_TO_RENAME: bool = true;

    type Input = hlist![
        GlobalUserUniforms,
        Storage2<Matrix>,
        Storage2<AnyUniform>,
        FormulasCache
    ];
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        (global, (matrices, input)): &mut Self::Input,
        _: &mut InlineHelper<Self>,
        mut data_id: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        changed |= self.matrices.egui(
            ui,
            matrices,
            input,
            &mut global.matrices,
            data_id.with("uniforms"),
        );
        data_id = data_id.with("matrices");
        ui.separator();
        let hpat![uniforms, formulas_cache] = input;
        changed |= self.uniforms.egui(
            ui,
            uniforms,
            formulas_cache,
            &mut global.uniforms,
            data_id.with("uniforms"),
        );
        changed
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(self.clone())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(
        &self,
        _: F,
        (_, (matrices, input)): &mut Self::Input,
    ) {
        self.matrices.remove(matrices, input);
        let hpat![uniforms, formulas_cache] = input;
        self.uniforms.remove(uniforms, formulas_cache);
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        (_, (matrices, input)): &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        self.matrices.errors_count(matrices, input) + {
            let hpat![uniforms, formulas_cache] = input;
            self.uniforms.errors_count(uniforms, formulas_cache)
        }
    }
}

impl AnyUniform {
    pub fn user_egui(&mut self, ui: &mut Ui) -> WhatChanged {
        use AnyUniform::*;
        let mut result = WhatChanged::default();
        match self {
            Bool(x) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_bool(ui, x))),
            Int(value) => {
                ui.centered_and_justified(|ui| {
                    result |= value.user_egui(ui, 1.0, 0..=0);
                });
            }
            Angle(a) => {
                drop(ui.centered_and_justified(|ui| result.uniform |= egui_angle_f64(ui, a)))
            }
            Float(value) => {
                ui.centered_and_justified(|ui| {
                    result |= value.user_egui(ui, 0.01, 0..=2);
                });
            }
            Formula(_) => {
                drop(ui.label("Internal error, formulas are not allowed to be accessed by user."))
            }
        }
        result
    }
}
