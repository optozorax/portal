use crate::gui::common::*;
use crate::gui::storage2::*;
use crate::gui::unique_id::UniqueId;

use egui::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureName(pub String);

impl TextureName {
    pub fn name(s: &str) -> String {
        format!("{}_tex", s)
    }
}

impl Default for TextureName {
    fn default() -> Self {
        Self("scenes/monoportal.png".into())
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TextureId(UniqueId);

impl Wrapper for TextureId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for TextureName {
    type IdWrapper = TextureId;
    type GetType = ();

    const SAFE_TO_RENAME: bool = true;

    type Input = TextureErrors;
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        texture_errors: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        _: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let errors = texture_errors.0.get(&self.0);
        WhatChanged::from_shader(check_changed(&mut self.0, |text| {
            let response =
                egui_with_red_field(ui, errors.is_some(), |ui| ui.text_edit_singleline(text));
            if let Some(err) = errors {
                response.on_hover_text(format!("Error while loading this file: {:?}", err));
            }
        }))
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, _: F, _: &mut Self::Input) {}

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        texture_errors: &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        texture_errors.0.get(&self.0).is_some() as usize
    }

    fn duplicate_inline<F>(&self, _map_self: &mut F, _input: &mut Self::Input) -> Self
    where
        F: FnMut(Self::IdWrapper, &mut Self::Input) -> Self::IdWrapper,
    {
        self.clone()
    }
}
