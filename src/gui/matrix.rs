use std::cell::RefCell;
use crate::gui::storage2::StorageElem2;
use crate::gui::storage2::Wrapper;
use crate::gui::unique_id::UniqueId;
use crate::gui::storage2::Storage2;
use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::object::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;

use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

use crate::hlist;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Matrix {
    Mul {
        to: Option<MatrixId>,
        what: Option<MatrixId>,
    },
    Teleport {
        first_portal: Option<MatrixId>,
        second_portal: Option<MatrixId>,
        what: Option<MatrixId>,
    },
    Simple {
        offset: DVec3,
        scale: f64,
        rotate: DVec3,
        mirror: (bool, bool, bool),
    },
    Parametrized {
        offset: TVec3<ParametrizeOrNot>,
        rotate: TVec3<ParametrizeOrNot>,
        mirror: TVec3<ParametrizeOrNot>,
        scale: ParametrizeOrNot,
    },
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix::Simple {
            offset: DVec3::default(),
            scale: 1.0,
            rotate: DVec3::default(),
            mirror: (false, false, false),
        }
    }
}

impl ComboBoxChoosable for Matrix {
    fn variants() -> &'static [&'static str] {
        &["Simple", "Mul", "Teleport", "Param."]
    }
    fn get_number(&self) -> usize {
        use Matrix::*;
        match self {
            Simple { .. } => 0,
            Mul { .. } => 1,
            Teleport { .. } => 2,
            Parametrized { .. } => 3,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Matrix::*;
        *self = match number {
            0 => match self {
                Simple { .. } => self.clone(),
                Parametrized {
                    offset,
                    rotate,
                    mirror,
                    scale,
                } => Simple {
                    offset: DVec3::new(
                        offset.x.freeget().unwrap_or(0.0),
                        offset.y.freeget().unwrap_or(0.0),
                        offset.z.freeget().unwrap_or(0.0),
                    ),
                    rotate: DVec3::new(
                        rotate.x.freeget().unwrap_or(0.0),
                        rotate.y.freeget().unwrap_or(0.0),
                        rotate.z.freeget().unwrap_or(0.0),
                    ),
                    mirror: (
                        (mirror.x.freeget().unwrap_or(0.0) - 1.0).abs() < 1e-6,
                        (mirror.y.freeget().unwrap_or(0.0) - 1.0).abs() < 1e-6,
                        (mirror.z.freeget().unwrap_or(0.0) - 1.0).abs() < 1e-6,
                    ),
                    scale: scale.freeget().unwrap_or(1.0),
                },
                _ => Self::default(),
            },
            1 => Mul {
                to: None,
                what: None,
            },
            2 => Teleport {
                first_portal: None,
                second_portal: None,

                what: None,
            },
            3 => match self {
                Parametrized { .. } => self.clone(),
                Simple {
                    offset,
                    rotate,
                    mirror,
                    scale,
                } => Parametrized {
                    offset: TVec3 {
                        x: ParametrizeOrNot::No(offset.x),
                        y: ParametrizeOrNot::No(offset.y),
                        z: ParametrizeOrNot::No(offset.z),
                    },
                    rotate: TVec3 {
                        x: ParametrizeOrNot::No(rotate.x),
                        y: ParametrizeOrNot::No(rotate.y),
                        z: ParametrizeOrNot::No(rotate.z),
                    },
                    mirror: TVec3 {
                        x: ParametrizeOrNot::No(mirror.0 as i32 as f64),
                        y: ParametrizeOrNot::No(mirror.1 as i32 as f64),
                        z: ParametrizeOrNot::No(mirror.2 as i32 as f64),
                    },
                    scale: ParametrizeOrNot::No(*scale),
                },
                _ => Parametrized {
                    offset: TVec3 {
                        x: ParametrizeOrNot::No(0.),
                        y: ParametrizeOrNot::No(0.),
                        z: ParametrizeOrNot::No(0.),
                    },
                    rotate: TVec3 {
                        x: ParametrizeOrNot::No(0.),
                        y: ParametrizeOrNot::No(0.),
                        z: ParametrizeOrNot::No(0.),
                    },
                    mirror: TVec3 {
                        x: ParametrizeOrNot::No(0.),
                        y: ParametrizeOrNot::No(0.),
                        z: ParametrizeOrNot::No(0.),
                    },
                    scale: ParametrizeOrNot::No(1.),
                },
            },
            _ => unreachable!(),
        };
    }
}

pub fn simple_matrix_egui(
    ui: &mut Ui, 
    offset: &mut DVec3, 
    rotate: &mut DVec3,
    mirror: &mut (bool, bool, bool),
    scale: &mut f64,
) -> bool {
    let mut is_changed = false;
     Grid::new("matrix")
        .striped(true)
        .min_col_width(45.)
        .max_col_width(45.)
        .show(ui, |ui| {
            ui.label("");
            ui.centered_and_justified(|ui| ui.label("X"));
            ui.centered_and_justified(|ui| ui.label("Y"));
            ui.centered_and_justified(|ui| ui.label("Z"));
            ui.end_row();

            ui.label("Offset: ");
            ui.centered_and_justified(|ui| is_changed |= egui_f64(ui, &mut offset.x));
            ui.centered_and_justified(|ui| is_changed |= egui_f64(ui, &mut offset.y));
            ui.centered_and_justified(|ui| is_changed |= egui_f64(ui, &mut offset.z));
            ui.end_row();

            ui.label("Rotate: ");
            ui.centered_and_justified(|ui| is_changed |= egui_angle(ui, &mut rotate.x));
            ui.centered_and_justified(|ui| is_changed |= egui_angle(ui, &mut rotate.y));
            ui.centered_and_justified(|ui| is_changed |= egui_angle(ui, &mut rotate.z));
            ui.end_row();

            ui.label("Mirror: ");
            ui.centered_and_justified(|ui| is_changed |= egui_bool(ui, &mut mirror.0));
            ui.centered_and_justified(|ui| is_changed |= egui_bool(ui, &mut mirror.1));
            ui.centered_and_justified(|ui| is_changed |= egui_bool(ui, &mut mirror.2));
            ui.end_row();

            ui.label("Scale: ");
            is_changed |= egui_f64_positive(ui, scale);
            ui.end_row();
        });
    is_changed
}

impl Matrix {
    pub fn simple_egui(&mut self, ui: &mut Ui) -> WhatChanged {
        match self {
            Matrix::Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => WhatChanged::from_uniform(simple_matrix_egui(ui, offset, rotate, mirror, scale)),
            _ => {
                drop(ui.label(
                    "Internal error, other types of matrices are not allowed to be accessed by user.",
                ));
                WhatChanged::default()
            },
        }
    }
}

impl Matrix {
    pub fn errors_count(
        &self,
        pos: usize,
        input: &hlist![Vec<String>, MatrixRecursionError],
        names: &[String],
    ) -> usize {
        use Matrix::*;
        let mut errors_count = 0;
        let hpat!(formulas_names, matrix_recursion_error) = input;
        match self {
            Mul { to, what } => {
                errors_count += to.is_none() as usize + 
                what.is_none() as usize;
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                errors_count += first_portal.is_none() as usize + 
                second_portal.is_none() as usize + 
                what.is_none() as usize;
            }
            Simple { .. } => {}
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                errors_count += offset.x.errors_count(formulas_names)
                    + offset.y.errors_count(formulas_names)
                    + offset.z.errors_count(formulas_names);
                errors_count += rotate.x.errors_count(formulas_names)
                    + rotate.y.errors_count(formulas_names)
                    + rotate.z.errors_count(formulas_names);
                errors_count += mirror.x.errors_count(formulas_names)
                    + mirror.y.errors_count(formulas_names)
                    + mirror.z.errors_count(formulas_names);
                errors_count += scale.errors_count(formulas_names);
            }
        }
        if matrix_recursion_error
            .0
            .get(&MatrixName(names[pos].clone()))
            .copied()
            .unwrap_or(false)
        {
            errors_count += 1;
        }
        errors_count
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MatrixId(UniqueId);

impl Wrapper<UniqueId> for MatrixId {
    fn wrap(id: UniqueId) -> Self { Self(id) }
    fn un_wrap(self) -> UniqueId { self.0 }
}

impl StorageElem2 for Matrix {
    type IdWrapper = MatrixId;
    type GetType = DMat4;

    const SAFE_TO_RENAME: bool = false;

    type Input = hlist![StorageWithNames<AnyUniformComboBox>, RefCell<FormulasCache>];

    fn egui(
        &mut self,
        ui: &mut Ui,
        input: &mut Self::Input,
        self_storage: &mut Storage2<Self>,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed =
            WhatChanged::from_uniform(egui_combo_label(ui, "Type:", 45., self));
        ui.separator();

        use Matrix::*;
        
        match self {
            Mul { to, what } => {
                changed |= self_storage.inline("Mul to:", 45., to, ui, input, data_id.with(0));
                changed |= self_storage.inline("What:", 45., what, ui, input, data_id.with(1));
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                changed |= self_storage.inline("From:", 45., first_portal, ui, input, data_id.with(0));
                changed |= self_storage.inline("To:", 45., second_portal, ui, input, data_id.with(1));
                changed |= self_storage.inline("What:", 45., what, ui, input, data_id.with(2));
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                changed.uniform |= simple_matrix_egui(ui, offset, rotate, mirror, scale);
            }
            Parametrized {
                offset,
                rotate,
                mirror,
                scale,
            } => {
                let hpat![uniforms, formulas_cache] = input;
                /*
                // TODO
                ui.label("Offset: ");
                changed.uniform |= offset
                    .x
                    .egui(ui, formulas_names, "X", 0.0, |ui, x| egui_f64(ui, x));
                changed.uniform |= offset
                    .y
                    .egui(ui, formulas_names, "Y", 0.0, |ui, x| egui_f64(ui, x));
                changed.uniform |= offset
                    .z
                    .egui(ui, formulas_names, "Z", 0.0, |ui, x| egui_f64(ui, x));
                ui.separator();
                ui.label("Rotate: ");
                changed.uniform |= rotate
                    .x
                    .egui(ui, formulas_names, "X", 0.0, |ui, x| egui_angle(ui, x));
                changed.uniform |= rotate
                    .y
                    .egui(ui, formulas_names, "Y", 0.0, |ui, x| egui_angle(ui, x));
                changed.uniform |= rotate
                    .z
                    .egui(ui, formulas_names, "Z", 0.0, |ui, x| egui_angle(ui, x));
                ui.separator();
                ui.label("Mirror: ");
                changed.uniform |= mirror
                    .x
                    .egui(ui, formulas_names, "X", 0.0, |ui, x| egui_0_1(ui, x));
                changed |= mirror
                    .y
                    .egui(ui, formulas_names, "Y", 0.0, |ui, x| egui_0_1(ui, x));
                changed |= mirror
                    .z
                    .egui(ui, formulas_names, "Z", 0.0, |ui, x| egui_0_1(ui, x));
                ui.separator();
                changed |= scale.egui(ui, formulas_names, "Scale:", 1.0, |ui, x| {
                    egui_f64_positive(ui, x)
                });
                */
            }
        }
        /*
        // TODO
        if matrix_recursion_error
            .0
            .get(&MatrixName(names[pos].clone()))
            .copied()
            .unwrap_or(false)
        {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.add(Label::new("Error: ").text_color(Color32::RED));
                ui.label("this matrix has recursion");
            });
        }
        */
        changed
    }

    fn get<F: FnMut(Self::IdWrapper) -> Option<Self::GetType>>(
        &self,
        mut f: F,
        hpat![uniforms, formulas_cache]: &Self::Input,
    ) -> Option<Self::GetType> {
        use Matrix::*;
        Some(match &self {
            Mul { to, what } => {
                let to = f((*to)?)?;
                let what = f((*what)?)?;
                what * to
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let first_portal = f((*first_portal)?)?;
                let second_portal = f((*second_portal)?)?;
                let what = f((*what)?)?;
                second_portal * first_portal.inverse() * what
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => DMat4::from_scale_rotation_translation(
                DVec3::new(
                    *scale * if mirror.0 { -1. } else { 1. },
                    *scale * if mirror.1 { -1. } else { 1. },
                    *scale * if mirror.2 { -1. } else { 1. },
                ),
                DQuat::from_rotation_x(rotate.x)
                    * DQuat::from_rotation_y(rotate.y)
                    * DQuat::from_rotation_z(rotate.z),
                *offset,
            ),
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                let formulas_cache = &*formulas_cache.borrow();
                let scale = scale.get(uniforms, formulas_cache);
                DMat4::from_scale_rotation_translation(
                    DVec3::new(
                        scale * (1. - 2.0 * mirror.x.get(uniforms, formulas_cache)),
                        scale * (1. - 2.0 * mirror.y.get(uniforms, formulas_cache)),
                        scale * (1. - 2.0 * mirror.z.get(uniforms, formulas_cache)),
                    ),
                    DQuat::from_rotation_x(rotate.x.get(uniforms, formulas_cache))
                        * DQuat::from_rotation_y(rotate.y.get(uniforms, formulas_cache))
                        * DQuat::from_rotation_z(rotate.z.get(uniforms, formulas_cache)),
                    DVec3::new(
                        offset.x.get(uniforms, formulas_cache),
                        offset.y.get(uniforms, formulas_cache),
                        offset.z.get(uniforms, formulas_cache),
                    ),
                )
            }
        })
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, mut f: F, input: &mut Self::Input) {
        use Matrix::*;
        match &self {
            Mul { to, what } => {
                if let Some(x) = to {
                    f(*x, input);
                }
                if let Some(x) = what {
                    f(*x, input);
                }
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                if let Some(x) = first_portal {
                    f(*x, input);
                }
                if let Some(x) = second_portal {
                    f(*x, input);
                }
                if let Some(x) = what {
                    f(*x, input);
                }
            }
            Simple { .. } => {},
            Parametrized { .. } => {},
        }
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(&self, f: F, input: &Self::Input) -> usize {
        use Matrix::*;
        /*
        // TODO
        match &self {
            Mul { to, what } => {
                to.map(|x| )
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let first_portal = f((*first_portal)?)?;
                let second_portal = f((*second_portal)?)?;
                let what = f((*what)?)?;
                second_portal * first_portal.inverse() * what
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => DMat4::from_scale_rotation_translation(
                DVec3::new(
                    *scale * if mirror.0 { -1. } else { 1. },
                    *scale * if mirror.1 { -1. } else { 1. },
                    *scale * if mirror.2 { -1. } else { 1. },
                ),
                DQuat::from_rotation_x(rotate.x)
                    * DQuat::from_rotation_y(rotate.y)
                    * DQuat::from_rotation_z(rotate.z),
                *offset,
            ),
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                let formulas_cache = &*formulas_cache.borrow();
                let scale = scale.get(uniforms, formulas_cache);
                DMat4::from_scale_rotation_translation(
                    DVec3::new(
                        scale * (1. - 2.0 * mirror.x.get(uniforms, formulas_cache)),
                        scale * (1. - 2.0 * mirror.y.get(uniforms, formulas_cache)),
                        scale * (1. - 2.0 * mirror.z.get(uniforms, formulas_cache)),
                    ),
                    DQuat::from_rotation_x(rotate.x.get(uniforms, formulas_cache))
                        * DQuat::from_rotation_y(rotate.y.get(uniforms, formulas_cache))
                        * DQuat::from_rotation_z(rotate.z.get(uniforms, formulas_cache)),
                    DVec3::new(
                        offset.x.get(uniforms, formulas_cache),
                        offset.y.get(uniforms, formulas_cache),
                        offset.z.get(uniforms, formulas_cache),
                    ),
                )
            }
        }
        */
        0
    }
}

impl StorageElem for Matrix {
    type GetType = ();
    type Input = ();

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(())
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        Default::default()
    }

    fn errors_count(&self, pos: usize, data: &Self::Input, _: &[String]) -> usize {
        0
    }
}
