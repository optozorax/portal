use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::glsl::*;
use crate::gui::matrix::Matrix;
use crate::gui::matrix::MatrixId;
use crate::gui::storage2::*;
use crate::gui::uniform::*;
use crate::gui::unique_id::UniqueId;
use std::borrow::Cow;

use egui::*;

use serde::{Deserialize, Serialize};

use crate::hlist;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
pub struct MatrixName<'a>(pub Cow<'a, str>);

impl<'a> MatrixName<'a> {
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
    Simple(Option<MatrixId>),
    Portal(Option<MatrixId>, Option<MatrixId>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Object {
    DebugMatrix(Option<MatrixId>),
    Flat {
        kind: ObjectType,
        is_inside: IsInsideCode, // gets current position (vec4), surface x y, must return material number. if this is portal, then additionally gets `first`, `back`
    },
    Complex {
        kind: ObjectType,
        intersect: IntersectCode, // gets transformed Ray, must return SurfaceIntersect
    },
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::Simple(None)
    }
}

impl Default for Object {
    fn default() -> Self {
        Object::DebugMatrix(None)
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
            0 => Simple(None),
            1 => Portal(None, None),
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
            0 => DebugMatrix(None),
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
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        (matrices, input): &mut hlist![Storage2<Matrix>, Storage2<AnyUniform>, FormulasCache],
        data_id: egui::Id,
    ) -> WhatChanged {
        use ObjectType::*;
        let mut changed = WhatChanged::default();
        match self {
            Simple(a) => {
                changed |= matrices.inline("Matrix:", 45., a, ui, input, data_id.with("first"));
            }
            Portal(a, b) => {
                changed |= matrices.inline("First:", 45., a, ui, input, data_id.with("first"));
                changed |= matrices.inline("Second:", 45., b, ui, input, data_id.with("second"));
            }
        }
        changed
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ObjectId(UniqueId);

impl Wrapper for ObjectId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl Object {
    pub fn get_name<'a>(id: MatrixId, storage: &'a Storage2<Matrix>) -> Option<MatrixName<'a>> {
        if let Some(name) = storage.get_name(id) {
            Some(
                name.map(|name| MatrixName(Cow::Borrowed(name)))
                    .unwrap_or_else(|| MatrixName(Cow::Owned(format!("id{}", id.un_wrap())))),
            )
        } else {
            None
        }
    }
}

impl StorageElem2 for Object {
    type IdWrapper = ObjectId;
    type GetType = Object;

    const SAFE_TO_RENAME: bool = true;

    type Input = hlist![
        ShaderErrors,
        Storage2<Matrix>,
        Storage2<AnyUniform>,
        FormulasCache
    ];
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        input: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        data_id: egui::Id,
        self_id: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::from_shader(egui_combo_label(ui, "Type:", 45., self));
        ui.separator();

        use Object::*;
        let (errors, input) = input;
        let has_errors = errors.get(self_id).is_some();
        match self {
            DebugMatrix(a) => {
                let (matrices, input) = input;
                changed |= matrices.inline("Matrix:", 45., a, ui, input, data_id.with(0));
            }
            Flat { kind, is_inside } => {
                changed.shader |= egui_combo_label(ui, "Kind:", 45., kind);
                changed |= kind.egui(ui, input, data_id.with(0));
                ui.separator();
                if matches!(kind, ObjectType::Portal { .. }) {
                    ui.horizontal_wrapped(|ui| {
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
                    ui.horizontal_wrapped(|ui| {
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
                    changed |= is_inside.0.egui(ui);
                });
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = errors.get(self_id) {
                    egui_errors(ui, local_errors);
                }
            }
            Complex { kind, intersect } => {
                changed.shader |= egui_combo_label(ui, "Kind:", 45., kind);
                changed |= kind.egui(ui, input, data_id.with(0));
                ui.separator();

                ui.horizontal_wrapped(|ui| {
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
                    changed |= intersect.0.egui(ui);
                });
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = errors.get(self_id) {
                    egui_errors(ui, local_errors);
                }
            }
        }

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
        use Object::*;
        use ObjectType::*;
        match self {
            DebugMatrix(a)
            | Flat {
                kind: Simple(a), ..
            }
            | Complex {
                kind: Simple(a), ..
            } => {
                if let Some(id) = a {
                    matrices.remove_as_field(*id, input);
                }
            }
            Flat {
                kind: Portal(a, b), ..
            }
            | Complex {
                kind: Portal(a, b), ..
            } => {
                if let Some(id) = a {
                    matrices.remove_as_field(*id, input);
                }
                if let Some(id) = b {
                    matrices.remove_as_field(*id, input);
                }
            }
        }
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        (errors, (matrices, input)): &Self::Input,
        self_id: Self::IdWrapper,
    ) -> usize {
        let mut result = if let Some(local_errors) = errors.get(self_id) {
            local_errors.len()
        } else {
            0
        };

        use Object::*;
        use ObjectType::*;
        result += match self {
            DebugMatrix(a) => a.map(|id| matrices.errors_inline(id, input)).unwrap_or(1),
            Flat { kind, .. } | Complex { kind, .. } => match kind {
                Simple(a) => a.map(|id| matrices.errors_inline(id, input)).unwrap_or(1),
                Portal(a, b) => {
                    a.map(|id| matrices.errors_inline(id, input)).unwrap_or(1)
                        + b.map(|id| matrices.errors_inline(id, input)).unwrap_or(1)
                }
            },
        };

        result
    }
}
