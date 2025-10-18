use crate::gui::common::*;
use crate::gui::glsl::*;
use crate::gui::storage2::*;
use crate::gui::unique_id::UniqueId;

use crate::gui::common::ShaderErrors;
use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

// gets ray, returns SceneIntersectionWithMaterial
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntersectionMaterial(pub IntersectionMaterialCode);

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct IntersectionMaterialId(UniqueId);

impl Wrapper for IntersectionMaterialId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for IntersectionMaterial {
    type IdWrapper = IntersectionMaterialId;
    type GetType = IntersectionMaterial;

    const SAFE_TO_RENAME: bool = false;

    type Input = ShaderErrors;
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        errors: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        _: egui::Id,
        self_id: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = false;

        let has_errors = errors.get(self_id).is_some();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            ui.add(Label::new(
                egui::RichText::new("SceneIntersectionWithMaterial ")
                    .color(COLOR_TYPE)
                    .monospace(),
            ));
            ui.add(Label::new(
                egui::RichText::new("process_intersection_with_material")
                    .color(COLOR_FUNCTION)
                    .monospace(),
            ));
            ui.add(Label::new(egui::RichText::new("(").monospace()));
            ui.add(Label::new(
                egui::RichText::new("Ray ").color(COLOR_TYPE).monospace(),
            ));
            ui.add(Label::new(egui::RichText::new("r) {").monospace()));
        });

        egui_with_red_field(ui, has_errors, |ui| {
            changed |= self.0 .0.egui(ui).shader;
        });
        ui.add(Label::new(egui::RichText::new("}").monospace()));

        if let Some(local_errors) = errors.get(self_id) {
            egui_errors(ui, local_errors);
        }

        WhatChanged::from_shader(changed)
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(self.clone())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, _: F, _: &mut Self::Input) {}

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        errors: &Self::Input,
        self_id: Self::IdWrapper,
    ) -> usize {
        errors.get(self_id).map(|x| x.len()).unwrap_or(0)
    }

    fn duplicate_inline<F>(&self, _map_self: &mut F, _input: &mut Self::Input) -> Self
    where
        F: FnMut(Self::IdWrapper, &mut Self::Input) -> Self::IdWrapper,
    {
        self.clone()
    }
}
