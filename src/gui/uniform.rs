use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::storage2::*;
use crate::gui::unique_id::UniqueId;
use core::cell::RefCell;
use egui::*;
use glam::*;
use std::fmt::{self, Debug, Formatter};
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Formula(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnyUniform {
    Bool(bool),
    Int(ClampedValue<i32>),
    Float(ClampedValue<f64>),
    Angle(f64),
    Formula(Formula),
}

impl AnyUniform {
    pub fn int(int: i32) -> AnyUniform {
        AnyUniform::Int(ClampedValue::new(int))
    }

    pub fn float(float: f64) -> AnyUniform {
        AnyUniform::Float(ClampedValue::new(float))
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
            AnyUniformResult::Float(f) => f,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TVec3 {
    pub x: ParametrizeOrNot,
    pub y: ParametrizeOrNot,
    pub z: ParametrizeOrNot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParametrizeOrNot {
    Yes(Option<UniformId>),
    No(f64),
}

impl TVec3 {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        f: impl Fn(&mut Ui, &mut f64) -> bool,
        uniforms: &mut Storage2<AnyUniform>,
        formulas_cache: &mut FormulasCache,
        data_id: egui::Id,
    ) -> bool {
        let mut changed = false;
        changed |= self
            .x
            .egui(ui, "X", 0.0, &f, uniforms, formulas_cache, data_id.with(0));
        changed |= self
            .y
            .egui(ui, "Y", 0.0, &f, uniforms, formulas_cache, data_id.with(1));
        changed |= self
            .z
            .egui(ui, "Z", 0.0, &f, uniforms, formulas_cache, data_id.with(2));
        changed
    }

    pub fn remove_as_field(
        &self,
        uniforms: &mut Storage2<AnyUniform>,
        formulas_cache: &mut FormulasCache,
    ) {
        self.x.remove_as_field(uniforms, formulas_cache);
        self.y.remove_as_field(uniforms, formulas_cache);
        self.z.remove_as_field(uniforms, formulas_cache);
    }

    pub fn errors_count(
        &self,
        uniforms: &Storage2<AnyUniform>,
        formulas_cache: &FormulasCache,
    ) -> usize {
        self.x.errors_count(uniforms, formulas_cache)
            + self.y.errors_count(uniforms, formulas_cache)
            + self.z.errors_count(uniforms, formulas_cache)
    }
}

impl ParametrizeOrNot {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        label: &str,
        default: f64,
        f: impl FnOnce(&mut Ui, &mut f64) -> bool,
        uniforms: &mut Storage2<AnyUniform>,
        formulas_cache: &mut FormulasCache,
        data_id: egui::Id,
    ) -> bool {
        use ParametrizeOrNot::*;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(label);
            let mut current = matches!(self, Yes { .. });
            changed |= egui_bool(ui, &mut current);
            if changed {
                *self = if current { Yes(None) } else { No(default) };
            }
            changed |= match self {
                Yes(current) => {
                    uniforms
                        .inline("", 0.0, current, ui, formulas_cache, data_id)
                        .uniform
                }
                No(float) => f(ui, float),
            };
        });
        changed
    }

    pub fn get(
        &self,
        uniforms: &Storage2<AnyUniform>,
        formulas_cache: &FormulasCache,
    ) -> Option<f64> {
        use ParametrizeOrNot::*;
        Some(match self {
            Yes(f) => uniforms.get((*f)?, formulas_cache)?.into(),
            No(f) => (*f).into(),
        })
    }

    pub fn freeget(&self) -> Option<f64> {
        use ParametrizeOrNot::*;
        match self {
            Yes(_) => None,
            No(f) => Some(*f),
        }
    }

    pub fn remove_as_field(
        &self,
        uniforms: &mut Storage2<AnyUniform>,
        formulas_cache: &mut FormulasCache,
    ) {
        if let ParametrizeOrNot::Yes(Some(id)) = self {
            uniforms.remove_as_field(*id, formulas_cache);
        }
    }

    pub fn errors_count(
        &self,
        uniforms: &Storage2<AnyUniform>,
        formulas_cache: &FormulasCache,
    ) -> usize {
        use ParametrizeOrNot::*;
        match self {
            Yes(Some(id)) => uniforms.errors_inline(*id, formulas_cache),
            Yes(None) => 1,
            No(_) => 0,
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
        let has_errors = formulas_cache.has_errors(&self.0);
        let edit = formulas_cache.with_edit(&mut self.0, |text| {
            let response = egui_with_red_field(ui, has_errors, |ui| {
                ui.add(
                    TextEdit::multiline(text)
                        .text_style(TextStyle::Monospace)
                        .desired_rows(1),
                )
            });
            if has_errors {
                response.on_hover_text("Error in this formula");
            }
        });
        WhatChanged::from_uniform(edit)
    }
}

struct FormulasCacheInner {
    parser: fasteval::Parser,
    slab: fasteval::Slab,
    cache: BTreeMap<String, Option<fasteval::Instruction>>,
}

impl Default for FormulasCacheInner {
    fn default() -> Self {
        Self {
            parser: fasteval::Parser::new(),
            slab: fasteval::Slab::new(),
            cache: Default::default(),
        }
    }
}

impl FormulasCacheInner {
    pub fn get<'a, 'b>(&'a mut self, text: &'b str) -> Option<&'a fasteval::Instruction> {
        self.compile(text)?;
        Some(self.get_unsafe(text))
    }

    /// You must call `self.compile(text)?;` before
    fn get_unsafe<'a, 'b>(&'a self, text: &'b str) -> &'a fasteval::Instruction {
        self.cache.get(text).unwrap().as_ref().unwrap()
    }

    /// Returns `None` when text is wrong formula
    #[must_use]
    pub fn compile(&mut self, text: &str) -> Option<()> {
        let FormulasCacheInner {
            parser,
            slab,
            cache,
        } = self;
        if let Some(result) = cache.get(text) {
            result.as_ref().map(|_| ())
        } else {
            use fasteval::*;
            let compiled = || -> Option<_> {
                Some(
                    parser
                        .parse(text, &mut slab.ps)
                        .ok()?
                        .from(&slab.ps)
                        .compile(&slab.ps, &mut slab.cs),
                )
            }();
            let result = compiled.as_ref().map(|_| ());
            cache.insert(text.to_owned(), compiled);
            result
        }
    }

    pub fn eval(
        &mut self,
        text: &str,
        ns: &mut impl FnMut(&str, Vec<f64>) -> Option<f64>,
    ) -> Option<Result<f64, fasteval::Error>> {
        use fasteval::*;
        self.compile(text)?;
        Some(self.get_unsafe(text).eval(&self.slab, ns))
    }
}

#[derive(Default)]
pub struct FormulasCache(RefCell<FormulasCacheInner>);

impl Debug for FormulasCache {
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl FormulasCache {
    pub fn has_errors(&self, text: &str) -> bool {
        self.0.borrow_mut().get(text).is_none()
    }

    pub fn with_edit(&self, text: &mut String, f: impl FnOnce(&mut String)) -> bool {
        let previous = text.clone();
        f(text);
        if previous == *text {
            false
        } else {
            self.0.borrow_mut().cache.remove(&previous);
            true
        }
    }

    pub fn eval(
        &self,
        text: &str,
        ns: &mut impl FnMut(&str, Vec<f64>) -> Option<f64>,
    ) -> Option<Result<f64, fasteval::Error>> {
        self.0.borrow_mut().eval(text, ns)
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
                Int(value) => value.get_value() >= 1,
                Float(value) => value.get_value() >= 1.0,
                Angle(a) => a >= &mut 1.0,
                Formula { .. } => false,
            }),
            1 => match self {
                Bool(b) => AnyUniform::int(*b as i32),
                Int { .. } => self.clone(),
                Float(value) => AnyUniform::int(value.get_value() as i32),
                Angle(a) => AnyUniform::int(rad2deg(*a as f64) as i32),
                Formula { .. } => AnyUniform::int(0),
            },
            2 => match self {
                Bool(b) => AnyUniform::float(*b as i32 as f64),
                Int(value) => AnyUniform::float(value.get_value() as f64),
                Angle(a) => AnyUniform::float(*a),
                Float { .. } => self.clone(),
                Formula { .. } => AnyUniform::float(0.0),
            },
            3 => Angle(match self {
                Bool(b) => (*b as i32 as f64) * std::f64::consts::PI,
                Int(value) => macroquad::math::clamp(
                    deg2rad(value.get_value() as f64),
                    0.,
                    std::f64::consts::TAU,
                ) as f64,
                Angle(a) => *a,
                Float(value) => {
                    macroquad::math::clamp(value.get_value(), 0., std::f64::consts::TAU)
                }
                Formula { .. } => 0.0,
            }),
            4 => Formula(match self {
                Bool(b) => F((*b as i32).to_string()),
                Int(value) => F(value.get_value().to_string()),
                Angle(a) => F(a.to_string()),
                Float(value) => F(value.get_value().to_string()),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClampedValue<T> {
    min: Option<T>,
    max: Option<T>,
    value: T,
}

impl<T: egui::emath::Numeric> ClampedValue<T> {
    pub fn new(value: T) -> Self {
        ClampedValue {
            min: None,
            max: None,
            value,
        }
    }

    pub fn get_value(&self) -> T {
        self.value
    }

    pub fn egui(
        &mut self,
        ui: &mut Ui,
        speed: f64,
        decimals: RangeInclusive<usize>,
        default_range: RangeInclusive<T>,
    ) -> WhatChanged {
        fn egui_edge<T: egui::emath::Numeric>(
            ui: &mut Ui,
            edge: &mut Option<T>,
            value: T,
            is_min: bool,
            default_value: T,
            speed: f64,
            decimals: RangeInclusive<usize>,
        ) -> bool {
            let mut changed = false;
            let mut current = edge.is_some();
            ui.checkbox(&mut current, if is_min { "min" } else { "max" });
            if let Some(edge) = edge {
                changed |= check_changed(edge, |edge| {
                    ui.add(
                        DragValue::from_get_set(|v| {
                            if let Some(v) = v {
                                let condition = if is_min {
                                    v > value.to_f64()
                                } else {
                                    v < value.to_f64()
                                };
                                if condition {
                                    *edge = value;
                                } else {
                                    *edge = T::from_f64(v);
                                }
                            }
                            edge.to_f64()
                        })
                        .speed(speed)
                        .min_decimals(*decimals.start())
                        .max_decimals(*decimals.end()),
                    );
                });
            } else {
                ui.label(if is_min { "-inf" } else { "+inf" });
            }
            if current && edge.is_none() {
                changed = true;
                let condition = if is_min {
                    value < default_value
                } else {
                    value > default_value
                };
                *edge = Some(if condition { value } else { default_value });
            }
            if !current && edge.is_some() {
                changed = true;
                *edge = None;
            }
            changed
        }

        let mut result = WhatChanged::default();
        ui.horizontal(|ui| {
            let ClampedValue { min, max, value } = self;
            result.uniform |= egui_edge(
                ui,
                min,
                *value,
                true,
                *default_range.start(),
                speed,
                decimals.clone(),
            );
            ui.separator();
            result.uniform |= egui_edge(
                ui,
                max,
                *value,
                false,
                *default_range.end(),
                speed,
                decimals.clone(),
            );
            ui.separator();
            ui.centered_and_justified(|ui| result |= self.user_egui(ui, speed, decimals));
        });
        result
    }

    pub fn user_egui(
        &mut self,
        ui: &mut Ui,
        speed: f64,
        decimals: RangeInclusive<usize>,
    ) -> WhatChanged {
        let mut result = WhatChanged::default();
        let ClampedValue { min, max, value } = self;
        if let Some((min, max)) = min.as_ref().zip(max.as_ref()) {
            result.uniform |= check_changed(value, |value| {
                ui.add(Slider::new(value, *min..=*max).clamp_to_range(true));
            });
        } else {
            result.uniform |= check_changed(value, |value| {
                ui.add(
                    DragValue::from_get_set(|v| {
                        if let Some(v) = v {
                            *value = T::from_f64(v);
                            if let Some(min) = min {
                                if *value < *min {
                                    *value = *min;
                                }
                            }
                            if let Some(max) = max {
                                if *value > *max {
                                    *value = *max;
                                }
                            }
                        }
                        value.to_f64()
                    })
                    .speed(speed)
                    .min_decimals(*decimals.start())
                    .max_decimals(*decimals.end()),
                );
            });
        }
        result
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UniformId(UniqueId);

impl Wrapper for UniformId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for AnyUniform {
    type IdWrapper = UniformId;
    type GetType = AnyUniformResult;

    const SAFE_TO_RENAME: bool = false;

    type Input = FormulasCache;
    type GetInput = Self::Input;

    fn egui(
        &mut self,
        ui: &mut Ui,
        formulas_cache: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        data_id: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let mut result = WhatChanged::from_uniform(egui_combo_box(
            ui,
            "Type:",
            45.,
            self,
            data_id.with("combo"),
        ));
        ui.separator();
        use AnyUniform::*;
        match self {
            Int(value) => result |= value.egui(ui, 1.0, 0..=0, -10..=10),
            Float(value) => result |= value.egui(ui, 0.01, 0..=2, -10.0..=10.0),
            Bool(x) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_bool(ui, x))),
            Angle(a) => {
                drop(ui.centered_and_justified(|ui| result.uniform |= egui_angle_f64(ui, a)))
            }
            Formula(x) => {
                drop(ui.centered_and_justified(|ui| result |= x.egui(ui, formulas_cache)))
            }
        }
        result
    }

    fn get(
        &self,
        get_helper: &GetHelper<Self>,
        formulas_cache: &Self::GetInput,
    ) -> Option<Self::GetType> {
        let mut cb = |name: &str, args: Vec<f64>| -> Option<f64> {
            Some(match name {
                // Custom functions
                "if" => {
                    if (*args.get(0)? - 1.0).abs() < 1e-6 {
                        *args.get(1)?
                    } else {
                        *args.get(2)?
                    }
                }
                "and" => {
                    if (*args.get(0)? - 1.0).abs() < 1e-6 && (*args.get(1)? - 1.0).abs() < 1e-6 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "or" => {
                    if (*args.get(0)? - 1.0).abs() < 1e-6 || (*args.get(1)? - 1.0).abs() < 1e-6 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "not" => {
                    if (*args.get(0)? - 1.0).abs() < 1e-6 {
                        0.0
                    } else {
                        1.0
                    }
                }
                "deg2rad" => args.get(0)? / 180. * std::f64::consts::PI,
                "rad2deg" => args.get(0)? * 180. / std::f64::consts::PI,
                "switch" => *args.get(*args.get(0)? as usize)?,

                // Free variables
                _ => get_helper.get(get_helper.find_id(name)?)?.into(),
            })
        };

        Some(match &self {
            AnyUniform::Bool(b) => AnyUniformResult::Bool(*b),
            AnyUniform::Int(value) => AnyUniformResult::Int(value.get_value()),
            AnyUniform::Angle(a) => AnyUniformResult::Float(*a),
            AnyUniform::Float(value) => AnyUniformResult::Float(value.get_value()),
            AnyUniform::Formula(f) => {
                AnyUniformResult::Float(formulas_cache.eval(&f.0, &mut cb)?.ok()?)
            }
        })
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, _: F, _: &mut Self::Input) {}

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        formulas_cache: &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        if let AnyUniform::Formula(formula) = self {
            formulas_cache.has_errors(&formula.0) as usize
        } else {
            0
        }
    }
}
