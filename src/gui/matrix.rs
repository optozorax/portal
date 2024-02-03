use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::storage2::GetHelper;
use crate::gui::storage2::InlineHelper;
use crate::gui::storage2::Storage2;
use crate::gui::storage2::StorageElem2;
use crate::gui::storage2::Wrapper;
use crate::gui::uniform::*;
use crate::gui::unique_id::UniqueId;

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
        offset: TVec3,
        rotate: TVec3,
        mirror: TVec3,
        scale: ParametrizeOrNot,
    },
    Exact {
        i: TVec3,
        j: TVec3,
        k: TVec3,
        pos: TVec3,
    },
    If {
        condition: ParametrizeOrNot,
        then: Option<MatrixId>,
        otherwise: Option<MatrixId>,
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
        &["Simple", "Mul", "Teleport", "Param.", "Exact", "If"]
    }
    fn get_number(&self) -> usize {
        use Matrix::*;
        match self {
            Simple { .. } => 0,
            Mul { .. } => 1,
            Teleport { .. } => 2,
            Parametrized { .. } => 3,
            Exact { .. } => 4,
            If { .. } => 5,
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
            4 => Exact {
                i: TVec3 {
                    x: ParametrizeOrNot::No(1.0),
                    y: ParametrizeOrNot::No(0.0),
                    z: ParametrizeOrNot::No(0.0),
                },
                j: TVec3 {
                    x: ParametrizeOrNot::No(0.0),
                    y: ParametrizeOrNot::No(1.0),
                    z: ParametrizeOrNot::No(0.0),
                },
                k: TVec3 {
                    x: ParametrizeOrNot::No(0.0),
                    y: ParametrizeOrNot::No(0.0),
                    z: ParametrizeOrNot::No(1.0),
                },
                pos: TVec3 {
                    x: ParametrizeOrNot::No(0.0),
                    y: ParametrizeOrNot::No(0.0),
                    z: ParametrizeOrNot::No(0.0),
                },
            },
            5 => If {
                condition: ParametrizeOrNot::No(1.0),
                then: None,
                otherwise: None,
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
    id: egui::Id,
) -> bool {
    let mut is_changed = false;
    Grid::new(id)
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
    pub fn user_egui(&mut self, ui: &mut Ui, id: egui::Id) -> WhatChanged {
        match self {
            Matrix::Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                WhatChanged::from_uniform(simple_matrix_egui(ui, offset, rotate, mirror, scale, id))
            }
            _ => {
                drop(ui.label(
                    "Internal error, other types of matrices are not allowed to be accessed by user.",
                ));
                WhatChanged::default()
            }
        }
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MatrixId(UniqueId);

impl Wrapper for MatrixId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for Matrix {
    type IdWrapper = MatrixId;
    type GetType = DMat4;

    const SAFE_TO_RENAME: bool = false;

    type Input = hlist![Storage2<AnyUniform>, FormulasCache];
    type GetInput = Self::Input;

    fn egui(
        &mut self,
        ui: &mut Ui,
        input: &mut Self::Input,
        inline_helper: &mut InlineHelper<Self>,
        data_id: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::from_uniform(egui_combo_box(
            ui,
            "Type:",
            45.,
            self,
            data_id.with("combo"),
        ));
        ui.separator();

        use Matrix::*;

        match self {
            Mul { to, what } => {
                changed |= inline_helper.inline("First:", 45., what, ui, input, data_id.with(1));
                changed |= inline_helper.inline("Then:", 45., to, ui, input, data_id.with(0));
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                changed |=
                    inline_helper.inline("From:", 45., first_portal, ui, input, data_id.with(0));
                changed |=
                    inline_helper.inline("To:", 45., second_portal, ui, input, data_id.with(1));
                changed |= inline_helper.inline("What:", 45., what, ui, input, data_id.with(2));
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                changed.uniform |=
                    simple_matrix_egui(ui, offset, rotate, mirror, scale, data_id.with("matrix"));
            }
            Parametrized {
                offset,
                rotate,
                mirror,
                scale,
            } => {
                let hpat![uniforms, formulas_cache] = input;
                ui.label("Offset: ");
                changed.uniform |=
                    offset.egui(ui, egui_f64, uniforms, formulas_cache, data_id.with(0));
                ui.separator();
                ui.label("Rotate: ");
                changed.uniform |=
                    rotate.egui(ui, egui_angle, uniforms, formulas_cache, data_id.with(1));
                ui.separator();
                ui.label("Mirror: ");
                changed.uniform |=
                    mirror.egui(ui, egui_0_1, uniforms, formulas_cache, data_id.with(2));
                ui.separator();
                changed.uniform |= scale.egui(
                    ui,
                    "Scale:",
                    1.0,
                    egui_f64_positive,
                    uniforms,
                    formulas_cache,
                    data_id.with(3),
                );
            }
            Exact { i, j, k, pos } => {
                let hpat![uniforms, formulas_cache] = input;
                ui.label("i: ");
                changed.uniform |= i.egui(ui, egui_f64, uniforms, formulas_cache, data_id.with(0));
                ui.separator();
                ui.label("j: ");
                changed.uniform |= j.egui(ui, egui_f64, uniforms, formulas_cache, data_id.with(0));
                ui.separator();
                ui.label("k: ");
                changed.uniform |= k.egui(ui, egui_f64, uniforms, formulas_cache, data_id.with(0));
                ui.separator();
                ui.label("pos: ");
                changed.uniform |=
                    pos.egui(ui, egui_f64, uniforms, formulas_cache, data_id.with(0));
            }
            If {
                condition,
                then,
                otherwise,
            } => {
                let hpat![uniforms, formulas_cache] = input;
                changed.uniform |= condition.egui(
                    ui,
                    "If:",
                    1.0,
                    egui_f64_positive,
                    uniforms,
                    formulas_cache,
                    data_id.with(3),
                );
                changed |= inline_helper.inline("Then:", 45., then, ui, input, data_id.with(0));
                changed |=
                    inline_helper.inline("Else:", 45., otherwise, ui, input, data_id.with(1));
            }
        }
        /*
        // POSTPONE
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

    fn get(
        &self,
        get_helper: &GetHelper<Self>,
        hpat![uniforms, formulas_cache]: &Self::Input,
    ) -> Option<Self::GetType> {
        use Matrix::*;
        Some(match &self {
            Mul { to, what } => {
                let to = get_helper.get((*to)?)?;
                let what = get_helper.get((*what)?)?;
                what * to
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let first_portal = get_helper.get((*first_portal)?)?;
                let second_portal = get_helper.get((*second_portal)?)?;
                let what = get_helper.get((*what)?)?;
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
                let scale = scale.get(uniforms, formulas_cache)?;
                DMat4::from_scale_rotation_translation(
                    DVec3::new(
                        scale * (1. - 2.0 * mirror.x.get(uniforms, formulas_cache)?),
                        scale * (1. - 2.0 * mirror.y.get(uniforms, formulas_cache)?),
                        scale * (1. - 2.0 * mirror.z.get(uniforms, formulas_cache)?),
                    ),
                    DQuat::from_rotation_x(rotate.x.get(uniforms, formulas_cache)?)
                        * DQuat::from_rotation_y(rotate.y.get(uniforms, formulas_cache)?)
                        * DQuat::from_rotation_z(rotate.z.get(uniforms, formulas_cache)?),
                    DVec3::new(
                        offset.x.get(uniforms, formulas_cache)?,
                        offset.y.get(uniforms, formulas_cache)?,
                        offset.z.get(uniforms, formulas_cache)?,
                    ),
                )
            }
            Exact { i, j, k, pos } => {
                macro_rules! get {
                    ($($x:tt)*) => {$($x)*.get(uniforms, formulas_cache)?};
                }
                let matrix = [
                    [get!(i.x), get!(i.y), get!(i.z), 0.],
                    [get!(j.x), get!(j.y), get!(j.z), 0.],
                    [get!(k.x), get!(k.y), get!(k.z), 0.],
                    [get!(pos.x), get!(pos.y), get!(pos.z), 1.],
                ];
                DMat4::from_cols_array_2d(&matrix)
            }
            If {
                condition,
                then,
                otherwise,
            } => {
                if condition.get(uniforms, formulas_cache)? > 0.5 {
                    get_helper.get((*then)?)?
                } else {
                    get_helper.get((*otherwise)?)?
                }
            }
        })
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(
        &self,
        mut f: F,
        input: &mut Self::Input,
    ) {
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
            Simple { .. } => {}
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                let hpat![uniforms, formulas_cache] = input;
                offset.remove_as_field(uniforms, formulas_cache);
                rotate.remove_as_field(uniforms, formulas_cache);
                mirror.remove_as_field(uniforms, formulas_cache);
                scale.remove_as_field(uniforms, formulas_cache);
            }
            Exact { i, j, k, pos } => {
                let hpat![uniforms, formulas_cache] = input;
                i.remove_as_field(uniforms, formulas_cache);
                j.remove_as_field(uniforms, formulas_cache);
                k.remove_as_field(uniforms, formulas_cache);
                pos.remove_as_field(uniforms, formulas_cache);
            }
            If {
                condition,
                then,
                otherwise,
            } => {
                let hpat![uniforms, formulas_cache] = input;
                condition.remove_as_field(uniforms, formulas_cache);
                if let Some(x) = then {
                    f(*x, input);
                }
                if let Some(x) = otherwise {
                    f(*x, input);
                }
            }
        }
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        mut f: F,
        hpat![uniforms, formulas_cache]: &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        use Matrix::*;
        match self {
            Mul { to, what } => to.map(&mut f).unwrap_or(1) + what.map(&mut f).unwrap_or(1),
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                first_portal.map(&mut f).unwrap_or(1)
                    + second_portal.map(&mut f).unwrap_or(1)
                    + what.map(&mut f).unwrap_or(1)
            }
            Simple { .. } => 0,
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                offset.errors_count(uniforms, formulas_cache)
                    + rotate.errors_count(uniforms, formulas_cache)
                    + mirror.errors_count(uniforms, formulas_cache)
                    + scale.errors_count(uniforms, formulas_cache)
            }
            Exact { i, j, k, pos } => {
                i.errors_count(uniforms, formulas_cache)
                    + j.errors_count(uniforms, formulas_cache)
                    + k.errors_count(uniforms, formulas_cache)
                    + pos.errors_count(uniforms, formulas_cache)
            }
            If {
                condition,
                then,
                otherwise,
            } => {
                condition.errors_count(uniforms, formulas_cache)
                    + then.map(&mut f).unwrap_or(1)
                    + otherwise.map(f).unwrap_or(1)
            }
        }
        // POSTPONE
        /*
        if matrix_recursion_error
            .0
            .get(&MatrixName(names[pos].clone()))
            .copied()
            .unwrap_or(false)
        {
            errors_count += 1;
        }
        */
    }
}
