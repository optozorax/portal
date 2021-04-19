use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::matrix::Matrix;
use crate::gui::matrix::MatrixId;
use crate::gui::storage::*;
use crate::gui::storage2::Storage2;
use crate::gui::uniform::*;
use egui::*;
use glam::*;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::hlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Animation<T> {
    ProvidedToUser,
    Remains,
    Changed(Option<T>),
    ChangedAndToUser(Option<T>),
}

impl<T> Default for Animation<T> {
    fn default() -> Self {
        Animation::Remains
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationStage {
    pub uniforms: HashMap<UniformId, Animation<UniformId>>,
    pub matrices: HashMap<MatrixId, Animation<MatrixId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalUserUniforms {
    pub uniforms: HashMap<UniformId, bool>,
    pub matrices: HashMap<MatrixId, bool>, // TODO add more data: description?
}

impl<T> ComboBoxChoosable for Animation<T> {
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
        // TODO clear this struct if there is any non-existing Id
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

impl StorageElem for AnimationStage {
    type GetType = Self;
    type Input = hlist!(
        Storage2<Matrix>,
        Storage2<AnyUniform>,
        GlobalUserUniforms,
        FormulasCache
    );

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.clone())
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        hpat!(matrices, uniforms, global, formulas_cache): &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let visible_elements = uniforms
            .visible_elements()
            .map(|(id, name)| (id, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name) in visible_elements {
            let global = global.uniforms.entry(id).or_default();
            let anim = self.uniforms.entry(id).or_default();
            if *global {
                ui.horizontal(|ui| {
                    egui_label(ui, &name, 60.);
                    ui.label("Global uniform");
                });
            } else {
                // TODO засунуть это в egui структуры Animation
                ui.horizontal(|ui| {
                    changed.uniform |= egui_combo_box(ui, &name, 60., anim);
                    match anim {
                        Animation::Changed(x) | Animation::ChangedAndToUser(x) => {
                            changed |= uniforms.inline(
                                &name,
                                60.0,
                                x,
                                ui,
                                formulas_cache,
                                egui::Id::new("anim").with(pos).with(id),
                            );
                        }
                        _ => {}
                    }
                });
            }
        }

        ui.separator();

        // TODO избавиться от копипасты
        let visible_elements = matrices
            .visible_elements()
            .map(|(id, name)| (id, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name) in visible_elements {
            let global = global.matrices.entry(id).or_default();
            let anim = self.matrices.entry(id).or_default();
            if *global {
                ui.horizontal(|ui| {
                    egui_label(ui, &name, 60.);
                    ui.label("Global uniform");
                });
            } else {
                ui.horizontal(|ui| {
                    changed.uniform |= egui_combo_box(ui, &name, 60., anim);
                    match anim {
                        Animation::Changed(x) | Animation::ChangedAndToUser(x) => {
                            with_swapped!(y => (*uniforms, *formulas_cache);
                                changed |= matrices.inline(&name, 60.0, x, ui, &mut y, egui::Id::new("anim").with(pos).with(id)));
                        }
                        _ => {}
                    }
                });
            }
        }

        changed
    }

    fn errors_count(&self, _: usize, _: &Self::Input, _: &[String]) -> usize {
        0
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
