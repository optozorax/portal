use crate::gui::animation::ElementsDescription;
use crate::gui::common::almost_identity;
use crate::gui::common::check_changed;
use crate::gui::common::deg2rad;
use crate::gui::common::egui_bool_named;
use crate::gui::common::egui_f64;
use crate::gui::common::matrix_hash;
use crate::gui::common::rad2deg;
use crate::gui::common::WhatChanged;
use crate::gui::matrix::*;
use crate::gui::storage2::*;
use crate::gui::uniform::AnyUniform;
use crate::gui::uniform::FormulasCache;
use crate::gui::unique_id::UniqueId;
use egui::Button;
use egui::DragValue;
use egui::Ui;
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CalculatedCam {
    pub look_at: DVec3,
    pub alpha: f64,
    pub beta: f64,
    pub r: f64,
    pub free_movement: bool,
    pub in_subspace: bool,
    pub matrix: DMat4,
    pub override_matrix: bool,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CurrentCam(pub Option<CameraId>); // if this is None, then camera is original

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct OriginalCam(pub CalculatedCam);

impl Default for CalculatedCam {
    fn default() -> Self {
        Self {
            look_at: DVec3::new(0., 0., 0.),
            alpha: deg2rad(81.),
            beta: deg2rad(64.),
            r: 3.5,
            in_subspace: false,
            free_movement: false,
            matrix: DMat4::IDENTITY,
            override_matrix: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CamLookAt {
    MatrixCenter(Option<MatrixId>), // uses inline_only_name
    Coordinate(DVec3),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cam {
    look_at: CamLookAt,
    alpha: f64,
    beta: f64,
    r: f64,

    #[serde(default)]
    in_subspace: bool,

    #[serde(default)]
    free_movement: bool,

    #[serde(default)]
    matrix: DMat4,
}

impl Default for CamLookAt {
    fn default() -> Self {
        Self::Coordinate(DVec3::default())
    }
}

impl Default for Cam {
    fn default() -> Self {
        Self {
            look_at: Default::default(),
            alpha: 0.0,
            beta: 0.0,
            r: 3.5,
            in_subspace: false,
            free_movement: false,
            matrix: DMat4::IDENTITY,
        }
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CameraId(UniqueId);

impl Wrapper for CameraId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl Cam {
    pub fn get_pos(
        &self,
        matrices: &Storage2<Matrix>,
        input: &hlist![Storage2<AnyUniform>, FormulasCache],
    ) -> Option<DVec3> {
        Some(match self.look_at {
            CamLookAt::MatrixCenter(id) => {
                matrices.get(id?, input)?.project_point3(DVec3::ZERO)
                    + DVec3::new(0.001, 0.001, 0.001)
            }
            CamLookAt::Coordinate(pos) => pos,
        })
    }

    pub fn get(
        &self,
        matrices: &Storage2<Matrix>,
        input: &hlist![Storage2<AnyUniform>, FormulasCache],
    ) -> Option<CalculatedCam> {
        Some(CalculatedCam {
            look_at: self.get_pos(matrices, input)?,
            alpha: self.alpha,
            beta: self.beta,
            r: self.r,
            in_subspace: self.in_subspace,
            free_movement: self.free_movement,
            matrix: self.matrix,
            override_matrix: true,
        })
    }

    pub fn get_easy(&self) -> Option<CalculatedCam> {
        if let CamLookAt::Coordinate(pos) = self.look_at {
            Some(CalculatedCam {
                look_at: pos,
                alpha: self.alpha,
                beta: self.beta,
                r: self.r,
                in_subspace: self.in_subspace,
                free_movement: self.free_movement,
                matrix: self.matrix,
                override_matrix: true,
            })
        } else {
            None
        }
    }

    pub fn set_this_cam(&mut self, ui: &mut Ui, self_id: CameraId) {
        ui.memory_mut(|memory| {
            memory
                .data
                .insert_persisted(egui::Id::new("CurrentCam"), CurrentCam(Some(self_id)))
        });
    }

    pub fn set_original_cam(ui: &mut Ui) {
        ui.memory_mut(|memory| {
            memory
                .data
                .insert_persisted(egui::Id::new("CurrentCam"), CurrentCam(None))
        });
    }

    pub fn user_egui(
        &mut self,
        ui: &mut Ui,
        names: &mut ElementsDescription<Cam>,
        self_id: CameraId,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let id = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                .0
        });
        let selected = id == Some(self_id);
        let name = names.get(self_id).clone();
        ui.horizontal(|ui| {
            if ui.radio(selected, name.overrided_name.clone()).clicked() && !selected {
                changed.uniform = true;
                self.set_this_cam(ui, self_id);
            }
            name.description(ui);
        });
        changed
    }
}

impl StorageElem2 for Cam {
    type IdWrapper = CameraId;
    type GetType = ();

    const SAFE_TO_RENAME: bool = true;

    type Input = Storage2<Matrix>;
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        matrices: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        data_id: egui::Id,
        self_id: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut from_matrix = matches!(self.look_at, CamLookAt::MatrixCenter(_));
        if ui.checkbox(&mut from_matrix, "From matrix").clicked() {
            if from_matrix {
                self.look_at = CamLookAt::MatrixCenter(None);
            } else {
                self.look_at = CamLookAt::Coordinate(Default::default());
            }
            changed.uniform = true;
        }
        match &mut self.look_at {
            CamLookAt::MatrixCenter(id) => {
                changed |= matrices.inline_only_name("Name:", 45., id, ui, data_id)
            }
            CamLookAt::Coordinate(coord) => {
                ui.horizontal(|ui| {
                    ui.monospace("X");
                    changed.uniform |= egui_f64(ui, &mut coord.x);
                    ui.separator();
                    ui.label("Y");
                    changed.uniform |= egui_f64(ui, &mut coord.y);
                    ui.separator();
                    ui.label("Z");
                    changed.uniform |= egui_f64(ui, &mut coord.z);
                });
            }
        }
        const BETA_MIN: f64 = 0.01;
        const BETA_MAX: f64 = PI - 0.01;
        ui.horizontal(|ui| {
            ui.label("α");
            changed.uniform |= check_changed(&mut self.alpha, |alpha| {
                let mut current = rad2deg(*alpha);
                ui.add(
                    DragValue::new(&mut current)
                        .speed(1.0)
                        .suffix("°")
                        .min_decimals(0)
                        .max_decimals(1),
                );
                *alpha = deg2rad(current);
            });
            ui.separator();
            ui.label("β");
            changed.uniform |= check_changed(&mut self.beta, |beta| {
                let mut current = rad2deg(*beta);
                ui.add(
                    DragValue::new(&mut current)
                        .speed(1.0)
                        .range(rad2deg(BETA_MIN)..=rad2deg(BETA_MAX))
                        .suffix("°")
                        .min_decimals(0)
                        .max_decimals(1),
                );
                *beta = deg2rad(current);
            });
            ui.separator();
            ui.label("R");
            changed.uniform |= check_changed(&mut self.r, |r| {
                ui.add(
                    DragValue::new(r)
                        .speed(0.01)
                        .range(0.01..=1000.0)
                        .min_decimals(0)
                        .max_decimals(2),
                );
            });
        });
        if almost_identity(&self.matrix) {
            ui.monospace("Matrix: IDENTITY");
        } else {
            ui.horizontal(|ui| {
                ui.monospace(format!("Matrix: {:.3}", matrix_hash(&self.matrix)));
                if ui.button("Make IDENTITY").clicked() {
                    self.matrix = DMat4::IDENTITY;
                    changed.uniform = true;
                }
            });
        }
        if self.free_movement {
            ui.monospace("FREE movement");
        } else {
            ui.monospace("Rotate around");
        }
        egui_bool_named(ui, &mut self.in_subspace, "In subspace");
        ui.separator();

        let current_cam = ui.memory_mut(|memory| {
            *memory
                .data
                .get_persisted_mut_or_default::<CalculatedCam>(egui::Id::new("CalculatedCam"))
        });

        ui.monospace("Current:");
        ui.monospace(format!(
            "X: {:.1}, Y: {:.1}, Z: {:.1}",
            current_cam.look_at.x, current_cam.look_at.y, current_cam.look_at.z
        ));
        ui.monospace(format!(
            "α: {:.1}, β: {:.1}, r: {:.1}",
            rad2deg(current_cam.alpha),
            rad2deg(current_cam.beta),
            current_cam.r
        ));
        if almost_identity(&current_cam.matrix) {
            ui.monospace("Matrix: IDENTITY");
        } else {
            ui.monospace(format!("Matrix: {:.3}", matrix_hash(&current_cam.matrix)));
        }
        if current_cam.free_movement {
            ui.monospace("FREE movement");
        } else {
            ui.monospace("Rotate around");
        }
        if current_cam.in_subspace {
            ui.monospace("IN subspace");
        } else {
            ui.monospace("Not in subspace");
        }

        ui.separator();

        let id = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                .0
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(id != Some(self_id), Button::new("Set this cam as current"))
                .clicked()
            {
                self.set_this_cam(ui, self_id);
                changed.uniform = true;
            }
            if ui
                .add_enabled(id.is_some(), Button::new("Return original camera"))
                .clicked()
            {
                Self::set_original_cam(ui);
                changed.uniform = true;
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Copy from current cam").clicked() {
                self.alpha = current_cam.alpha;
                self.beta = current_cam.beta;
                self.r = current_cam.r;
                self.matrix = current_cam.matrix;
                self.free_movement = current_cam.free_movement;
                self.in_subspace = current_cam.in_subspace;
                if matches!(self.look_at, CamLookAt::Coordinate(_)) {
                    self.look_at = CamLookAt::Coordinate(current_cam.look_at);
                }
                changed.uniform = true;
            }
            if ui.button("Copy to current cam").clicked() {
                ui.memory_mut(|memory| {
                    memory
                        .data
                        .insert_persisted(egui::Id::new("OverrideCam"), self.get_easy().unwrap());
                });
            }
        });

        changed
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, _: F, _: &mut Self::Input) {
        // здесь не надо удалять матрицу, потому что мы не создаём инлайн матрицы
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        _: &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        matches!(self.look_at, CamLookAt::MatrixCenter(None)) as usize
    }

    fn duplicate_inline<F>(&self, _map_self: &mut F, _input: &mut Self::Input) -> Self
    where
        F: FnMut(Self::IdWrapper, &mut Self::Input) -> Self::IdWrapper,
    {
        self.clone()
    }
}
