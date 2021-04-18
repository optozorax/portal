use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::matrix::Matrix;
use crate::gui::storage::*;
use crate::gui::uniform::*;
use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

use crate::hlist;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Animation<T> {
    ProvidedToUser,
    Remains,
    Changed(T),
    ChangedAndToUser(T),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationStage {
    pub uniforms: Vec<Animation<AnyUniform>>,
    pub matrices: Vec<Animation<Matrix>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalUserUniforms {
    pub uniforms: Vec<bool>,
    pub matrices: Vec<bool>,
}

impl<T: Default> ComboBoxChoosable for Animation<T> {
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
            2 => Changed(Default::default()),
            3 => ChangedAndToUser(Default::default()),
            _ => unreachable!(),
        };
    }
}

impl GlobalUserUniforms {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        matrices_names: &mut Vec<String>,
        formulas_names: &mut Vec<String>,
    ) -> WhatChanged {
        let mut changed = false;
        self.uniforms.resize(formulas_names.len(), false);
        for (enabled, name) in self.uniforms.iter_mut().zip(formulas_names.iter()) {
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
        }
        ui.separator();
        self.matrices.resize(matrices_names.len(), false);
        for (enabled, name) in self.matrices.iter_mut().zip(matrices_names.iter()) {
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
        }
        WhatChanged::from_uniform(changed)
    }
}

impl StorageElem for AnimationStage {
    type GetType = Self;
    type Input = hlist!(
        StorageWithNames<Matrix>,
        StorageWithNames<AnyUniformComboBox>,
        GlobalUserUniforms
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
        _: usize,
        hpat!(matrices, uniforms, global_uniforms): &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        self.uniforms
            .resize(uniforms.names.len(), Animation::Remains);
        for (((anim, global), name), uniform) in self
            .uniforms
            .iter_mut()
            .zip(global_uniforms.uniforms.iter())
            .zip(uniforms.names.iter())
            .zip(uniforms.storage.iter())
        {
            if *global {
                ui.horizontal(|ui| {
                    egui_label(ui, name, 60.);
                    ui.label("Global uniform");
                });
            } else {
                ui.horizontal(|ui| {
                    changed.uniform |= egui_combo_box(ui, name, 60., anim);
                    match anim {
                        Animation::Changed(x) | Animation::ChangedAndToUser(x) => {
                            if x.get_number() != uniform.0.get_number() {
                                *x = uniform.0.clone();
                                changed.uniform = true;
                            }
                            changed |= x.simple_egui(ui);
                        }
                        _ => {}
                    }
                });
            }
        }

        ui.separator();
        self.matrices
            .resize(matrices.names.len(), Animation::Remains);
        for (((anim, global), name), matrix) in self
            .matrices
            .iter_mut()
            .zip(global_uniforms.matrices.iter())
            .zip(matrices.names.iter())
            .zip(matrices.storage.iter())
        {
            if *global {
                ui.horizontal(|ui| {
                    egui_label(ui, name, 60.);
                    ui.label("Global uniform");
                });
            } else {
                ui.horizontal(|ui| {
                    changed.uniform |= egui_combo_box(ui, name, 60., anim);
                    match anim {
                        Animation::Changed(x) | Animation::ChangedAndToUser(x) => {
                            if x.get_number() != matrix.get_number() {
                                *x = matrix.clone();
                                changed.uniform = true;
                            }
                            changed |= x.simple_egui(ui);
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
    pub fn simple_egui(&mut self, ui: &mut Ui) -> WhatChanged {
        use AnyUniform::*;
        let mut result = WhatChanged::default();
        match self {
            Bool(x) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_bool(ui, x))),
            Int { min, max, value } => {
                ui.centered_and_justified(|ui| {
                    if let Some((min, max)) = min.as_ref().zip(max.as_ref()) {
                        result.uniform |= check_changed(value, |value| {
                            ui.add(Slider::new(value, *min..=*max).clamp_to_range(true));
                        })
                    } else {
                        result.uniform |= check_changed(value, |value| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        *value = v as i32;
                                        if let Some(min) = min {
                                            if value < min {
                                                *value = *min;
                                            }
                                        }
                                        if let Some(max) = max {
                                            if value > max {
                                                *value = *max;
                                            }
                                        }
                                    }
                                    (*value).into()
                                })
                                .speed(1),
                            );
                        });
                    }
                });
            }
            Angle(a) => {
                drop(ui.centered_and_justified(|ui| result.uniform |= egui_angle_f64(ui, a)))
            }
            Float { min, max, value } => {
                ui.centered_and_justified(|ui| {
                    if let Some((min, max)) = min.as_ref().zip(max.as_ref()) {
                        result.uniform |= check_changed(value, |value| {
                            ui.add(Slider::new(value, *min..=*max).clamp_to_range(true));
                        });
                    } else {
                        result.uniform |= check_changed(value, |value| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        *value = v;
                                        if let Some(min) = min {
                                            if value < min {
                                                *value = *min;
                                            }
                                        }
                                        if let Some(max) = max {
                                            if value > max {
                                                *value = *max;
                                            }
                                        }
                                    }
                                    *value
                                })
                                .speed(0.01)
                                .min_decimals(0)
                                .max_decimals(2),
                            );
                        });
                    }
                });
            }
            Formula(_) => {
                drop(ui.label("Internal error, formulas are not allowed to be accessed by user."))
            }
        }
        result
    }
}
