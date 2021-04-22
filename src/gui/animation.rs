use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::eng_rus::EngRusText;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValueToUser {
    help_description: Option<EngRusText>,
    overrided_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Animation<T: StorageElem2> {
    ProvidedToUser(ValueToUser),
    FromDev,
    Changed(Option<T::IdWrapper>),
    ChangedAndToUser(Option<T::IdWrapper>, ValueToUser),
}

impl<T: StorageElem2> Animation<T> {
    pub fn get_t(&self) -> Option<&T::IdWrapper> {
        if let Animation::Changed(Some(t)) | Animation::ChangedAndToUser(Some(t), _) = self {
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

impl ValueToUser {
    pub fn egui(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                egui_label(ui, "Name:", 45.);
                ui.text_edit_singleline(&mut self.overrided_name);
            });
            egui_option(
                ui,
                &mut self.help_description,
                "Has description",
                Default::default,
                |ui, help| {
                    ui.horizontal(|ui| {
                        egui_label(ui, "Desc:", 45.);
                        help.egui_multiline(ui);
                    });
                    false
                },
            );
        });
    }

    pub fn user_egui(&self, ui: &mut Ui) {
        let previous = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = 0.;
        ui.label(format!("{}", self.overrided_name));
        if let Some(help) = &self.help_description {
            let response = ui.add(egui::Label::new("(?)").small_raised());
            response.on_hover_text(help.text(ui));
        }
        ui.label(": ");
        ui.spacing_mut().item_spacing.x = previous;
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
            if global.0 {
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
            changed |= element.user_egui(ui, storage, &user_egui, *id);
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
pub struct GlobalStage<T: StorageElem2>(HashMap<T::IdWrapper, (bool, ValueToUser)>);

impl<T: StorageElem2> Default for GlobalStage<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: StorageElem2> GlobalStage<T> {
    pub fn egui(&mut self, ui: &mut Ui, storage: &mut Storage2<T>) -> bool {
        let mut changed = false;
        for (id, name) in storage.visible_elements() {
            let (enabled, to_user) = self.0.entry(id).or_default();
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
            to_user.egui(ui);
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
        if self.0.iter().any(|x| x.1 .0) {
            for (id, name) in self
                .0
                .iter()
                .filter_map(|(id, (has, name))| {
                    if *has {
                        Some((*id, name.clone()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
            {
                if let Some(element) = storage.get_original_mut(id) {
                    ui.horizontal(|ui| {
                        name.user_egui(ui);
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
    pub name: EngRusText,
    pub description: Option<EngRusText>,
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
            ProvidedToUser { .. } => 0,
            FromDev => 1,
            Changed { .. } => 2,
            ChangedAndToUser { .. } => 3,
        }
    }

    fn set_number(&mut self, number: usize) {
        use Animation::*;
        *self = match number {
            0 => ProvidedToUser(Default::default()),
            1 => FromDev,
            2 => Changed(None),
            3 => ChangedAndToUser(None, Default::default()),
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
            if let Animation::Changed(x) | Animation::ChangedAndToUser(x, _) = self {
                changed |= storage.inline(&name, 60.0, x, ui, input, data_id);
            }
            if let Animation::ProvidedToUser(x) | Animation::ChangedAndToUser(_, x) = self {
                x.egui(ui);
            }
        });
        changed
    }

    pub fn user_egui(
        &self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        user_egui: impl Fn(&mut T, &mut Ui) -> WhatChanged,
        id: T::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        use Animation::*;
        match self {
            ProvidedToUser(name) | ChangedAndToUser(_, name) => drop(ui.horizontal(|ui| {
                let element = storage.get_original_mut(id).unwrap();
                name.user_egui(ui);
                changed |= user_egui(element, ui);
            })),
            FromDev => {}
            Changed(_) => {}
        }
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
        self.name.egui_singleline(ui);
        ui.separator();
        egui_option(
            ui,
            &mut self.description,
            "Has description",
            Default::default,
            |ui, desc| {
                ui.horizontal(|ui| {
                    egui_label(ui, "Desc.:", 45.);
                    desc.egui_multiline(ui);
                });
                false
            },
        );
        ui.separator();
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
            Progress(a) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_0_1(ui, a))),
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
