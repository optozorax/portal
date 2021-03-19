use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::storage::*;
use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Formula(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormulaName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnyUniform {
    Bool(bool),
    Int {
        min: Option<i32>,
        max: Option<i32>,
        value: i32,
    },
    Float {
        min: Option<f64>,
        max: Option<f64>,
        value: f64,
    },
    Angle(f64),
    Formula(Formula),
}

impl AnyUniform {
    pub fn int(int: i32) -> AnyUniform {
        AnyUniform::Int {
            min: None,
            max: None,
            value: int,
        }
    }

    pub fn float(float: f64) -> AnyUniform {
        AnyUniform::Float {
            min: None,
            max: None,
            value: float,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnyUniformResult {
    Bool(bool),
    Int(i32),
    Float(f64),
}

impl From<AnyUniformResult> for f64 {
    fn from(u: AnyUniformResult) -> f64 {
        match u {
            AnyUniformResult::Bool(b) => {
                if b {
                    1.0
                } else {
                    0.0
                }
            }
            AnyUniformResult::Int(i) => i as f64,
            AnyUniformResult::Float(f) => f.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TVec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParametrizeOrNot {
    Yes(FormulaName),
    No(f32),
}

impl ParametrizeOrNot {
    pub fn errors_count(&self, formulas_names: &[String]) -> usize {
        match self {
            ParametrizeOrNot::Yes(name) => !formulas_names.contains(&name.0) as usize,
            ParametrizeOrNot::No { .. } => 0,
        }
    }
}

impl ParametrizeOrNot {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        formulas_names: &[String],
        label: &str,
        default: f32,
        f: impl FnOnce(&mut Ui, &mut f32) -> bool,
    ) -> bool {
        use ParametrizeOrNot::*;
        let mut not_found = false;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(label);
            let mut current = matches!(self, Yes { .. });
            changed |= egui_bool(ui, &mut current);
            if changed {
                *self = if current {
                    Yes(FormulaName("a".to_owned()))
                } else {
                    No(default)
                };
            }
            changed |= match self {
                Yes(current) => {
                    not_found = !formulas_names.contains(&current.0);
                    check_changed(&mut current.0, |text| drop(ui.text_edit_singleline(text)))
                }
                No(float) => f(ui, float),
            };
        });
        if not_found {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error:").text_color(Color32::RED));
                ui.label("uniform with this name is not found");
            });
        }
        changed
    }

    pub fn get(
        &self,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> f64 {
        use ParametrizeOrNot::*;
        match self {
            Yes(f) => match uniforms.get(&f.0, uniforms, formulas_cache) {
                GetEnum::Ok(x) => x.into(),
                _ => {
                    eprintln!("can't find uniform {}", f.0);
                    0.0
                }
            },
            No(f) => (*f).into(),
        }
    }

    pub fn freeget(&self) -> Option<f32> {
        use ParametrizeOrNot::*;
        match self {
            Yes(_) => None,
            No(f) => Some(*f),
        }
    }
}

impl Default for Formula {
    fn default() -> Self {
        Formula("sin(pi())".to_owned())
    }
}

impl Formula {
    pub fn egui(&mut self, ui: &mut Ui, formulas_cache: &mut FormulasCache) -> WhatChanged {
        let edit = formulas_cache.with_edit(&mut self.0, |text| {
            ui.add(TextEdit::multiline(text).text_style(TextStyle::Monospace).desired_rows(1));
        });
        if edit.has_errors {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error with this formula").text_color(Color32::RED));
            });
        }
        WhatChanged::from_uniform(edit.changed)
    }
}

pub struct FormulasCache {
    parser: fasteval::Parser,
    slab: fasteval::Slab,
    cache: BTreeMap<String, Option<fasteval::Instruction>>,
}

impl Default for FormulasCache {
    fn default() -> Self {
        let mut result = FormulasCache {
            parser: fasteval::Parser::new(),
            slab: fasteval::Slab::new(),
            cache: Default::default(),
        };
        result.compile("0");
        result
    }
}

use std::fmt::{self, Debug, Formatter};

impl Debug for FormulasCache {
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[derive(Default)]
pub struct EditResult {
    changed: bool,
    has_errors: bool,
}

impl FormulasCache {
    pub fn has_errors(&self, text: &str) -> bool {
        self.cache.get(text).map(|x| x.is_none()).unwrap_or(false)
    }

    pub fn compile(&mut self, text: &str) -> bool {
        use fasteval::*;

        let compiled = || -> Option<_> {
            Some(
                self.parser
                    .parse(text, &mut self.slab.ps)
                    .ok()?
                    .from(&self.slab.ps)
                    .compile(&self.slab.ps, &mut self.slab.cs),
            )
        }();

        let is_compiled = compiled.is_some();

        self.cache.insert(text.to_owned(), compiled);

        is_compiled
    }

    pub fn with_edit(&mut self, text: &mut String, f: impl FnOnce(&mut String)) -> EditResult {
        let previous = text.clone();
        f(text);
        if previous == *text && self.cache.get(text).is_some() {
            EditResult {
                changed: false,
                has_errors: self.cache.get(text).unwrap().is_none(),
            }
        } else {
            self.cache.remove(&previous);
            EditResult {
                changed: true,
                has_errors: !self.compile(text),
            }
        }
    }

    pub fn get<'a, 'b>(&'a self, text: &'b str) -> Option<&'a fasteval::Instruction> {
        self.cache.get(text).and_then(|x| x.as_ref())
    }
}

impl ComboBoxChoosable for AnyUniform {
    fn variants() -> &'static [&'static str] {
        &["bool", "int", "float", "angle", "formula"]
    }
    fn get_number(&self) -> usize {
        use AnyUniform::*;
        match self {
            Bool { .. } => 0,
            Int { .. } => 1,
            Float { .. } => 2,
            Angle { .. } => 3,
            Formula { .. } => 4,
        }
    }
    fn set_number(&mut self, number: usize) {
        use crate::gui::uniform::Formula as F;
        use AnyUniform::*;
        *self = match number {
            0 => Bool(match self {
                Bool(b) => *b,
                Int { value, .. } => value >= &mut 1,
                Float { value, .. } => value >= &mut 1.0,
                Angle(a) => a >= &mut 1.0,
                Formula { .. } => false,
            }),
            1 => match self {
                Bool(b) => AnyUniform::int(*b as i32),
                Int { .. } => self.clone(),
                Float { value, .. } => AnyUniform::int(*value as i32),
                Angle(a) => AnyUniform::int(rad2deg(*a as f32) as i32),
                Formula { .. } => AnyUniform::int(0),
            },
            2 => match self {
                Bool(b) => AnyUniform::float(*b as i32 as f64),
                Int { value, .. } => AnyUniform::float(*value as f64),
                Angle(a) => AnyUniform::float(*a),
                Float { .. } => self.clone(),
                Formula { .. } => AnyUniform::float(0.0),
            },
            3 => Angle(match self {
                Bool(b) => (*b as i32 as f64) * std::f64::consts::PI,
                Int { value, .. } => {
                    macroquad::math::clamp(deg2rad(*value as f32), 0., std::f32::consts::TAU) as f64
                }
                Angle(a) => *a,
                Float { value, .. } => macroquad::math::clamp(*value, 0., std::f64::consts::TAU),
                Formula { .. } => 0.0,
            }),
            4 => Formula(match self {
                Bool(b) => F((*b as i32).to_string()),
                Int { value, .. } => F(value.to_string()),
                Angle(a) => F(a.to_string()),
                Float { value, .. } => F(value.to_string()),
                Formula(f) => f.clone(),
            }),
            _ => unreachable!(),
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnyUniformComboBox(pub AnyUniform);

impl Default for AnyUniform {
    fn default() -> Self {
        AnyUniform::Formula(Default::default())
    }
}

impl AnyUniform {
    pub fn egui(&mut self, ui: &mut Ui, formulas_cache: &mut FormulasCache) -> WhatChanged {
        use AnyUniform::*;
        let mut result = WhatChanged::default();
        match self {
            Bool(x) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_bool(ui, x))),
            Int { min, max, value } => {
                ui.horizontal(|ui| {
                    let mut current = min.is_some();
                    if let Some(min) = min {
                        ui.checkbox(&mut current, "min");
                        result.uniform |= check_changed(min, |min| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if v as i32 > *value {
                                            *min = *value;
                                        } else {
                                            *min = v as i32;
                                        }
                                    }
                                    *min as f64
                                })
                                .speed(1),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "min");
                        ui.label("-inf");
                    }
                    if current && min.is_none() {
                        result.uniform = true;
                        let min_i = -100;
                        *min = Some(if *value < min_i { *value } else { min_i });
                    }
                    if !current && min.is_some() {
                        result.uniform = true;
                        *min = None;
                    }
                    ui.separator();
                    let mut current = max.is_some();
                    if let Some(max) = max {
                        ui.checkbox(&mut current, "max");
                        result.uniform |= check_changed(max, |max| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if (v as i32) < *value {
                                            *max = *value;
                                        } else {
                                            *max = v as i32;
                                        }
                                    }
                                    *max as f64
                                })
                                .speed(1),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "max");
                        ui.label("+inf");
                    }
                    if current && max.is_none() {
                        result.uniform = true;
                        let max_i = 100;
                        *max = Some(if *value > max_i { *value } else { max_i });
                    }
                    if !current && max.is_some() {
                        result.uniform = true;
                        *max = None;
                    }
                    ui.separator();
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
                });
            }
            Angle(a) => {
                drop(ui.centered_and_justified(|ui| result.uniform |= egui_angle_f64(ui, a)))
            }
            Float { min, max, value } => {
                ui.horizontal(|ui| {
                    let mut current = min.is_some();
                    if let Some(min) = min {
                        ui.checkbox(&mut current, "min");
                        result.uniform |= check_changed(min, |min| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if v > *value {
                                            *min = *value;
                                        } else {
                                            *min = v;
                                        }
                                    }
                                    *min as f64
                                })
                                .speed(0.01)
                                .min_decimals(0)
                                .max_decimals(2),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "min");
                        ui.label("-inf");
                    }
                    if current && min.is_none() {
                        result.uniform = true;
                        let min_i = -10.0;
                        *min = Some(if *value < min_i { *value } else { min_i });
                    }
                    if !current && min.is_some() {
                        result.uniform = true;
                        *min = None;
                    }
                    ui.separator();
                    let mut current = max.is_some();
                    if let Some(max) = max {
                        ui.checkbox(&mut current, "max");
                        result.uniform |= check_changed(max, |max| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if v < *value {
                                            *max = *value;
                                        } else {
                                            *max = v;
                                        }
                                    }
                                    *max
                                })
                                .speed(0.01)
                                .min_decimals(0)
                                .max_decimals(2),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "max");
                        ui.label("+inf");
                    }
                    if current && max.is_none() {
                        result.uniform = true;
                        let max_i = 10.0;
                        *max = Some(if *value > max_i { *value } else { max_i });
                    }
                    if !current && max.is_some() {
                        result.uniform = true;
                        *max = None;
                    }
                    ui.separator();
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
                });
            }
            Formula(x) => {
                drop(ui.centered_and_justified(|ui| result |= x.egui(ui, formulas_cache)))
            }
        }
        result
    }
}

impl StorageElem for AnyUniformComboBox {
    type GetType = AnyUniformResult;
    type Input = FormulasCache;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        mut f: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        let mut cb = |name: &str, args: Vec<f64>| -> Option<f64> {
            Some(match name {
                // Custom functions
                "if" => {
                    if *args.get(0)? == 1.0 {
                        *args.get(1)?
                    } else {
                        *args.get(2)?
                    }
                }
                "and" => {
                    if *args.get(0)? == 1.0 && *args.get(1)? == 1.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "or" => {
                    if *args.get(0)? == 1.0 || *args.get(1)? == 1.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "not" => {
                    if *args.get(0)? == 1.0 {
                        0.0
                    } else {
                        1.0
                    }
                }
                "deg2rad" => args.get(0)? / 180. * std::f64::consts::PI,
                "rad2deg" => args.get(0)? * 180. / std::f64::consts::PI,
                "switch" => *args.get(*args.get(0)? as usize)?,

                // Free variables
                _ => match f(name) {
                    GetEnum::Ok(x) => x.into(),
                    _ => None?,
                },
            })
        };

        use fasteval::*;

        match &self.0 {
            AnyUniform::Bool(b) => GetEnum::Ok(AnyUniformResult::Bool(*b)),
            AnyUniform::Int { value, .. } => GetEnum::Ok(AnyUniformResult::Int(*value)),
            AnyUniform::Angle(a) => GetEnum::Ok(AnyUniformResult::Float(*a)),
            AnyUniform::Float { value, .. } => GetEnum::Ok(AnyUniformResult::Float(*value)),
            AnyUniform::Formula(f) => match formulas_cache.get(&f.0) {
                Some(x) => match x.eval(&formulas_cache.slab, &mut cb) {
                    Ok(x) => GetEnum::Ok(AnyUniformResult::Float(x)),
                    Err(_) => GetEnum::NotFound,
                },
                None => GetEnum::NotFound,
            },
        }
    }

    fn egui(&mut self, ui: &mut Ui, _: usize, data: &mut Self::Input, _: &[String]) -> WhatChanged {
        let mut changed =
            WhatChanged::from_uniform(egui_combo_label(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, data);
        changed
    }

    fn errors_count(&self, _: usize, formulas_cache: &Self::Input, _: &[String]) -> usize {
        match &self.0 {
            AnyUniform::Formula(text) => formulas_cache.has_errors(&text.0) as usize,
            _ => 0,
        }
    }
}
