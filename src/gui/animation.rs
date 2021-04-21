use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::matrix::Matrix;
use crate::gui::matrix::MatrixId;
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
    Remains,
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
        Animation::Remains
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageChanging<T: StorageElem2>(HashMap<T::IdWrapper, Animation<T>>);

impl<T: StorageElem2> Default for StageChanging<T> {
    fn default() -> Self {
        StageChanging(HashMap::new())
    }
}

impl<T: StorageElem2> StageChanging<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        global: &mut HashMap<T::IdWrapper, bool>,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let visible_elements = storage
            .visible_elements()
            .map(|(id, name)| (id, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name) in visible_elements {
            let global = global.entry(id).or_default();
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

    pub fn iter(&self) -> impl Iterator<Item = (&T::IdWrapper, &Animation<T>)> + '_ {
        self.0.iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationStage {
    pub uniforms: StageChanging<AnyUniform>,
    pub matrices: StageChanging<Matrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalUserUniforms {
    pub uniforms: HashMap<UniformId, bool>,
    pub matrices: HashMap<MatrixId, bool>,
}

impl<T: StorageElem2> ComboBoxChoosable for Animation<T> {
    fn variants() -> &'static [&'static str] {
        &["To user", "Remains", "Changed", "Changed + To user"]
    }

    fn get_number(&self) -> usize {
        use Animation::*;
        match self {
            ProvidedToUser => 0,
            Remains => 1,
            Changed { .. } => 2,
            ChangedAndToUser { .. } => 3,
        }
    }

    fn set_number(&mut self, number: usize) {
        use Animation::*;
        *self = match number {
            0 => ProvidedToUser,
            1 => Remains,
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
        for (id, name) in uniforms.visible_elements() {
            let enabled = self.uniforms.entry(id).or_default();
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
        }
        ui.separator();
        for (id, name) in matrices.visible_elements() {
            let enabled = self.matrices.entry(id).or_default();
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
        }
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
