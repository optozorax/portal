use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;
use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

use crate::megatuple;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimationUniform {
    ProvidedToUser,
    Remains,
    Changed(AnyUniform),
    ChangedAndToUser(AnyUniform),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationStage {
    pub uniforms: Vec<AnimationUniform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalUserUniforms {
    pub uniforms: Vec<bool>,
}

impl Default for GlobalUserUniforms {
    fn default() -> Self {
        Self { uniforms: vec![] }
    }
}

impl Default for AnimationStage {
    fn default() -> Self {
        Self { uniforms: vec![] }
    }
}

impl ComboBoxChoosable for AnimationUniform {
    fn variants() -> &'static [&'static str] {
        &["To user", "Remains", "Changed", "Changed + To user"]
    }

    fn get_number(&self) -> usize {
        use AnimationUniform::*;
        match self {
            ProvidedToUser => 0,
            Remains => 1,
            Changed { .. } => 2,
            ChangedAndToUser { .. } => 3,
        }
    }

    fn set_number(&mut self, number: usize) {
        use AnimationUniform::*;
        *self = match number {
            0 => ProvidedToUser,
            1 => Remains,
            2 => Changed(AnyUniform::Bool(false)),
            3 => ChangedAndToUser(AnyUniform::Bool(false)),
            _ => unreachable!(),
        };
    }
}

impl GlobalUserUniforms {
    pub fn egui(&mut self, ui: &mut Ui, formulas_names: &mut Vec<String>) -> WhatChanged {
        let mut changed = false;
        self.uniforms.resize(formulas_names.len(), false);
        for (enabled, name) in self.uniforms.iter_mut().zip(formulas_names.iter()) {
            changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
        }
        WhatChanged::from_uniform(changed)
    }
}

impl StorageElem for AnimationStage {
    type GetType = Self;
    type Input = megatuple!(Vec<String>, GlobalUserUniforms, Vec<AnyUniformComboBox>);

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
        glob_pos: usize,
        megapattern!(formulas_names, global_uniforms, uniforms_value): &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        self.uniforms
            .resize(formulas_names.len(), AnimationUniform::Remains);
        for (pos, (((anim, global), name), uniform)) in self
            .uniforms
            .iter_mut()
            .zip(global_uniforms.uniforms.iter())
            .zip(formulas_names.iter())
            .zip(uniforms_value.iter())
            .enumerate()
        {
            if *global {
                ui.horizontal(|ui| {
                    egui_label(ui, name, 60.);
                    ui.label("Global uniform");
                });
            } else {
                ui.horizontal(|ui| {
                    changed.uniform |= egui_combo_box(ui, name, 60., anim, glob_pos + pos);
                    match anim {
                        AnimationUniform::Changed(x) | AnimationUniform::ChangedAndToUser(x) => {
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
        changed
    }

    fn errors_count(&self, _: usize, _: &Self::Input, _: &[String]) -> usize {
        0
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["First stage".to_owned()], vec![Default::default()])
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
                            ui.add(Slider::i32(value, *min..=*max).clamp_to_range(true));
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
                            ui.add(Slider::f64(value, *min..=*max).clamp_to_range(true));
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
                                    (*value).into()
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
