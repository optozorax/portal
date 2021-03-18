use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::glsl::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;

use egui::*;

use serde::{Deserialize, Serialize};

use crate::megatuple;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
pub struct MatrixName(pub String);

impl MatrixName {
    pub fn normal_name(&self) -> String {
        format!("{}_mat", self.0)
    }

    pub fn inverse_name(&self) -> String {
        format!("{}_mat_inv", self.0)
    }

    pub fn teleport_to_name(&self, to: &MatrixName) -> String {
        format!("{}_to_{}_mat_teleport", self.0, to.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Simple(MatrixName),
    Portal(MatrixName, MatrixName),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Object {
    DebugMatrix(MatrixName),
    Flat {
        kind: ObjectType,
        is_inside: IsInsideCode, // gets current position (vec4), surface x y, must return material number. if this is portal, then additionally gets `first`, `back`
    },
    Complex {
        kind: ObjectType,
        intersect: IntersectCode, // gets transformed Ray, must return SurfaceIntersect
    },
}

impl Default for MatrixName {
    fn default() -> Self {
        Self("id".into())
    }
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::Simple(Default::default())
    }
}

impl Default for Object {
    fn default() -> Self {
        Object::DebugMatrix(Default::default())
    }
}

impl ComboBoxChoosable for ObjectType {
    fn variants() -> &'static [&'static str] {
        &["Simple", "Portal"]
    }
    fn get_number(&self) -> usize {
        use ObjectType::*;
        match self {
            Simple { .. } => 0,
            Portal { .. } => 1,
        }
    }
    fn set_number(&mut self, number: usize) {
        use ObjectType::*;
        *self = match number {
            0 => Simple(Default::default()),
            1 => Portal(Default::default(), Default::default()),
            _ => unreachable!(),
        };
    }
}

impl ComboBoxChoosable for Object {
    fn variants() -> &'static [&'static str] {
        &["Debug", "Flat", "Complex"]
    }
    fn get_number(&self) -> usize {
        use Object::*;
        match self {
            DebugMatrix { .. } => 0,
            Flat { .. } => 1,
            Complex { .. } => 2,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Object::*;
        *self = match number {
            0 => DebugMatrix(Default::default()),
            1 => Flat {
                kind: Default::default(),
                is_inside: Default::default(),
            },
            2 => Complex {
                kind: Default::default(),
                intersect: Default::default(),
            },
            _ => unreachable!(),
        };
    }
}

impl ObjectType {
    pub fn egui(&mut self, ui: &mut Ui, names: &mut Vec<String>) -> WhatChanged {
        use ObjectType::*;
        let mut is_changed = false;
        let mut errors_count = 0;
        match self {
            Simple(a) => {
                is_changed |=
                    egui_existing_name(ui, "Matrix:", 45., &mut a.0, names, &mut errors_count)
            }
            Portal(a, b) => {
                is_changed |=
                    egui_existing_name(ui, "First:", 45., &mut a.0, names, &mut errors_count);
                is_changed |=
                    egui_existing_name(ui, "Second:", 45., &mut b.0, names, &mut errors_count);
            }
        }
        WhatChanged::from_shader(is_changed)
    }
}

impl Object {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut megatuple!(Vec<String>, ShaderErrors),
    ) -> WhatChanged {
        use Object::*;
        let megapattern!(names, errors) = input;
        let mut is_changed = WhatChanged::default();
        let has_errors = errors.get_errors(self, pos).is_some();
        let mut errors_count = 0;
        match self {
            DebugMatrix(a) => {
                is_changed.shader |=
                    egui_existing_name(ui, "Matrix:", 45., &mut a.0, names, &mut errors_count);
            }
            Flat { kind, is_inside } => {
                is_changed.shader |= egui_combo_label(ui, "Kind:", 45., kind);
                is_changed |= kind.egui(ui, names);
                ui.separator();
                if matches!(kind, ObjectType::Portal { .. }) {
                    ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        ui.add(Label::new("int ").text_color(COLOR_TYPE).monospace());
                        ui.add(
                            Label::new("is_inside")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("vec4 ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("pos, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("x, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("y, \n              ").monospace());
                        ui.add(Label::new("bool ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("back, ").monospace());
                        ui.add(Label::new("bool ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("first) {").monospace());
                    });
                } else {
                    ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        ui.add(Label::new("int ").text_color(COLOR_TYPE).monospace());
                        ui.add(
                            Label::new("is_inside")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("vec4 ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("pos, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("x, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("y) {").monospace());
                    });
                }
                egui_with_red_field(ui, has_errors, |ui| {
                    is_changed |= is_inside.0.egui(ui);
                });
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = errors.get_errors(self, pos) {
                    egui_errors(ui, local_errors);
                }
            }
            Complex { kind, intersect } => {
                is_changed.shader |= egui_combo_label(ui, "Kind:", 45., kind);
                is_changed |= kind.egui(ui, names);
                ui.separator();

                ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.;

                    if matches!(kind, ObjectType::Portal { .. }) {
                        ui.add(
                            Label::new("SceneIntersection ")
                                .text_color(COLOR_TYPE)
                                .monospace(),
                        );
                        ui.add(
                            Label::new("intersect")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("Ray ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("r, ").monospace());
                        ui.add(Label::new("bool ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("first) {").monospace());
                    } else {
                        ui.add(
                            Label::new("SceneIntersection ")
                                .text_color(COLOR_TYPE)
                                .monospace(),
                        );
                        ui.add(
                            Label::new("intersect")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("Ray ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("r) {").monospace());
                    }
                });
                egui_with_red_field(ui, has_errors, |ui| {
                    is_changed |= intersect.0.egui(ui);
                });
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = errors.get_errors(self, pos) {
                    egui_errors(ui, local_errors);
                }
            }
        }
        is_changed
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectComboBox(pub Object);

impl ObjectType {
    pub fn errors_count(&self, names: &[String]) -> usize {
        let mut result = 0;

        use ObjectType::*;
        match self {
            Simple(a) => {
                if !names.contains(&a.0) {
                    result += 1;
                }
            }
            Portal(a, b) => {
                if !names.contains(&a.0) {
                    result += 1;
                }
                if !names.contains(&b.0) {
                    result += 1;
                }
            }
        }

        result
    }
}

impl Object {
    pub fn errors_count(
        &self,
        pos: usize,
        megapattern!(names, errors): &megatuple!(Vec<String>, ShaderErrors),
    ) -> usize {
        let mut result = if let Some(local_errors) = errors.get_errors(self, pos) {
            local_errors.len()
        } else {
            0
        };

        use Object::*;
        match self {
            DebugMatrix(a) => {
                if !names.contains(&a.0) {
                    result += 1;
                }
            }
            Flat { kind, is_inside: _ } => {
                result += kind.errors_count(names);
            }
            Complex { kind, intersect: _ } => {
                result += kind.errors_count(names);
            }
        }

        result
    }
}

impl StorageElem for ObjectComboBox {
    type GetType = Object;
    type Input = megatuple!(Vec<String>, ShaderErrors);

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.0.clone())
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["my object".to_owned()], vec![Default::default()])
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let mut changed = WhatChanged::from_shader(egui_combo_label(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, pos, input);
        changed
    }

    fn errors_count(&self, pos: usize, data: &Self::Input, _: &[String]) -> usize {
        self.0.errors_count(pos, data)
    }
}
