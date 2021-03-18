use crate::get_try;
use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::object::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;

use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

use crate::megatuple;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Matrix {
    Mul {
        to: String,

        what: String,
    },
    Teleport {
        first_portal: String,
        second_portal: String,

        what: String,
    },
    Simple {
        offset: Vec3,
        scale: f32,
        rotate: Vec3,
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
            offset: Vec3::default(),
            scale: 1.0,
            rotate: Vec3::default(),
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
                    offset: Vec3::new(
                        offset.x.freeget().unwrap_or(0.0),
                        offset.y.freeget().unwrap_or(0.0),
                        offset.z.freeget().unwrap_or(0.0),
                    ),
                    rotate: Vec3::new(
                        rotate.x.freeget().unwrap_or(0.0),
                        rotate.y.freeget().unwrap_or(0.0),
                        rotate.z.freeget().unwrap_or(0.0),
                    ),
                    mirror: (
                        mirror.x.freeget().unwrap_or(0.0) == 1.0,
                        mirror.y.freeget().unwrap_or(0.0) == 1.0,
                        mirror.z.freeget().unwrap_or(0.0) == 1.0,
                    ),
                    scale: scale.freeget().unwrap_or(1.0),
                },
                _ => Self::default(),
            },
            1 => Mul {
                to: "id".to_owned(),
                what: "id".to_owned(),
            },
            2 => Teleport {
                first_portal: "id".to_owned(),
                second_portal: "id".to_owned(),

                what: "id".to_owned(),
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
                        x: ParametrizeOrNot::No(mirror.0 as i32 as f32),
                        y: ParametrizeOrNot::No(mirror.1 as i32 as f32),
                        z: ParametrizeOrNot::No(mirror.2 as i32 as f32),
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

impl Matrix {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut megatuple!(Vec<String>, MatrixRecursionError),
        names: &[String],
    ) -> WhatChanged {
        use Matrix::*;
        let megapattern!(formulas_names, matrix_recursion_error) = input;
        let mut is_changed = false;
        let mut errors_count = 0;
        match self {
            Mul { to, what } => {
                is_changed |= egui_existing_name(ui, "Mul to:", 45., to, names, &mut errors_count);
                is_changed |= egui_existing_name(ui, "What:", 45., what, names, &mut errors_count);
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                is_changed |=
                    egui_existing_name(ui, "From:", 45., first_portal, names, &mut errors_count);
                is_changed |=
                    egui_existing_name(ui, "To:", 45., second_portal, names, &mut errors_count);
                is_changed |= egui_existing_name(ui, "What:", 45., what, names, &mut errors_count);
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => {
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
                        ui.centered_and_justified(|ui| is_changed |= egui_f32(ui, &mut offset.x));
                        ui.centered_and_justified(|ui| is_changed |= egui_f32(ui, &mut offset.y));
                        ui.centered_and_justified(|ui| is_changed |= egui_f32(ui, &mut offset.z));
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
                        is_changed |= egui_f32_positive(ui, scale);
                        ui.end_row();
                    });
            }
            Parametrized {
                offset,
                rotate,
                mirror,
                scale,
            } => {
                ui.label("Offset: ");
                is_changed |= offset
                    .x
                    .egui(ui, formulas_names, "X", 0.0, |ui, x| egui_f32(ui, x));
                is_changed |= offset
                    .y
                    .egui(ui, formulas_names, "Y", 0.0, |ui, x| egui_f32(ui, x));
                is_changed |= offset
                    .z
                    .egui(ui, formulas_names, "Z", 0.0, |ui, x| egui_f32(ui, x));
                ui.separator();
                ui.label("Rotate: ");
                is_changed |= rotate
                    .x
                    .egui(ui, formulas_names, "X", 0.0, |ui, x| egui_angle(ui, x));
                is_changed |= rotate
                    .y
                    .egui(ui, formulas_names, "Y", 0.0, |ui, x| egui_angle(ui, x));
                is_changed |= rotate
                    .z
                    .egui(ui, formulas_names, "Z", 0.0, |ui, x| egui_angle(ui, x));
                ui.separator();
                ui.label("Mirror: ");
                is_changed |= mirror
                    .x
                    .egui(ui, formulas_names, "X", 0.0, |ui, x| egui_0_1(ui, x));
                is_changed |= mirror
                    .y
                    .egui(ui, formulas_names, "Y", 0.0, |ui, x| egui_0_1(ui, x));
                is_changed |= mirror
                    .z
                    .egui(ui, formulas_names, "Z", 0.0, |ui, x| egui_0_1(ui, x));
                ui.separator();
                is_changed |= scale.egui(ui, formulas_names, "Scale:", 1.0, |ui, x| {
                    egui_f32_positive(ui, x)
                });
            }
        }
        if matrix_recursion_error
            .0
            .get(&MatrixName(names[pos].clone()))
            .copied()
            .unwrap_or(false)
        {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error: ").text_color(Color32::RED));
                ui.label("this matrix has recursion");
            });
        }
        WhatChanged::from_uniform(is_changed)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatrixComboBox(pub Matrix);

impl StorageElem for MatrixComboBox {
    type GetType = Mat4;
    type Input = megatuple!(Vec<String>, MatrixRecursionError);

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        mut f: F,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        use Matrix::*;
        GetEnum::Ok(match &self.0 {
            Mul { to, what } => {
                let to = get_try!(f(&to));
                let what = get_try!(f(&what));
                what * to
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let first_portal = get_try!(f(&first_portal));
                let second_portal = get_try!(f(&second_portal));
                let what = get_try!(f(&what));
                second_portal * first_portal.inverse() * what
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => Mat4::from_scale_rotation_translation(
                Vec3::new(
                    *scale * if mirror.0 { -1. } else { 1. },
                    *scale * if mirror.1 { -1. } else { 1. },
                    *scale * if mirror.2 { -1. } else { 1. },
                ),
                Quat::from_rotation_x(rotate.x)
                    * Quat::from_rotation_y(rotate.y)
                    * Quat::from_rotation_z(rotate.z),
                *offset,
            ),
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                let scale = scale.get(uniforms, formulas_cache) as f32;
                Mat4::from_scale_rotation_translation(
                    Vec3::new(
                        scale * (1. - 2.0 * mirror.x.get(uniforms, formulas_cache) as f32),
                        scale * (1. - 2.0 * mirror.y.get(uniforms, formulas_cache) as f32),
                        scale * (1. - 2.0 * mirror.z.get(uniforms, formulas_cache) as f32),
                    ),
                    Quat::from_rotation_x(rotate.x.get(uniforms, formulas_cache) as f32)
                        * Quat::from_rotation_y(rotate.y.get(uniforms, formulas_cache) as f32)
                        * Quat::from_rotation_z(rotate.z.get(uniforms, formulas_cache) as f32),
                    Vec3::new(
                        offset.x.get(uniforms, formulas_cache) as f32,
                        offset.y.get(uniforms, formulas_cache) as f32,
                        offset.z.get(uniforms, formulas_cache) as f32,
                    ),
                )
            }
        })
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (
            vec!["id".to_owned(), "a".to_owned()],
            vec![MatrixComboBox::default(), MatrixComboBox::default()],
        )
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut Self::Input,
        names: &[String],
    ) -> WhatChanged {
        let mut changed =
            WhatChanged::from_uniform(egui_combo_label(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, pos, input, names);
        changed
    }

    fn errors_count(&self, pos: usize, input: &Self::Input, names: &[String]) -> usize {
        self.0.errors_count(pos, input, names)
    }
}

impl Matrix {
    pub fn errors_count(
        &self,
        pos: usize,
        input: &megatuple!(Vec<String>, MatrixRecursionError),
        names: &[String],
    ) -> usize {
        use Matrix::*;
        let mut errors_count = 0;
        let megapattern!(formulas_names, matrix_recursion_error) = input;
        match self {
            Mul { to, what } => {
                if !names.contains(to) {
                    errors_count += 1;
                }
                if !names.contains(what) {
                    errors_count += 1;
                }
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                if !names.contains(first_portal) {
                    errors_count += 1;
                }
                if !names.contains(second_portal) {
                    errors_count += 1;
                }
                if !names.contains(what) {
                    errors_count += 1;
                }
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
