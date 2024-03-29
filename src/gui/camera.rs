use crate::gui::animation::ElementsDescription;
use crate::gui::common::deg2rad;
use crate::gui::common::rad2deg;
use crate::gui::common::WhatChanged;
use crate::gui::matrix::*;
use crate::gui::storage2::*;
use crate::gui::uniform::AnyUniform;
use crate::gui::uniform::FormulasCache;
use crate::gui::unique_id::UniqueId;
use egui::Button;
use egui::Ui;
use glam::DVec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CalculatedCam {
    pub look_at: DVec3,
    pub alpha: f64,
    pub beta: f64,
    pub r: f64,
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
}

impl Default for CamLookAt {
    fn default() -> Self {
        Self::MatrixCenter(None)
    }
}

impl Default for Cam {
    fn default() -> Self {
        Self {
            look_at: Default::default(),
            alpha: 0.0,
            beta: 0.0,
            r: 3.5,
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
        })
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
                ui.monospace(format!(
                    "X: {:.1}, Y: {:.1}, Z: {:.1}",
                    coord.x, coord.y, coord.z
                ));
            }
        }
        ui.separator();
        ui.monospace(format!(
            "α: {:.1}, β: {:.1}, r: {:.1}",
            rad2deg(self.alpha),
            rad2deg(self.beta),
            self.r
        ));
        ui.separator();

        let id = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                .0
        });
        if ui
            .add_enabled(id != Some(self_id), Button::new("Set this cam as current"))
            .clicked()
        {
            self.set_this_cam(ui, self_id);
            changed.uniform = true;
        }

        let id = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                .0
        });
        if ui
            .add_enabled(id.is_some(), Button::new("Return original camera"))
            .clicked()
        {
            Self::set_original_cam(ui);
            changed.uniform = true;
        }

        ui.separator();

        if ui
            .add(Button::new("Set angles from current camera"))
            .clicked()
        {
            let current_cam = ui.memory_mut(|memory| {
                *memory
                    .data
                    .get_persisted_mut_or_default::<CalculatedCam>(egui::Id::new("CalculatedCam"))
            });
            self.alpha = current_cam.alpha;
            self.beta = current_cam.beta;
            self.r = current_cam.r;
            changed.uniform = true;
        }

        let id = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                .0
        });
        if ui
            .add_enabled(id.is_none(), Button::new("Set center from current camera"))
            .clicked()
        {
            let current_cam = ui.memory_mut(|memory| {
                *memory
                    .data
                    .get_persisted_mut_or_default::<CalculatedCam>(egui::Id::new("CalculatedCam"))
            });
            self.look_at = CamLookAt::Coordinate(current_cam.look_at);
            changed.uniform = true;
        }

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
}
