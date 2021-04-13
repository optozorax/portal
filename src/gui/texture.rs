use crate::gui::common::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;

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

impl TextureName {}

impl StorageElem for TextureName {
    type GetType = TextureName;
    type Input = TextureErrors;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.clone())
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        _: usize,
        texture_errors: &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let result = WhatChanged::from_shader(check_changed(&mut self.0, |text| {
            drop(ui.text_edit_singleline(text))
        }));

        if let Some(err) = texture_errors.0.get(&self.0) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.add(Label::new("Error:").text_color(Color32::RED));
                ui.label(format!("error while loading file: {:?}", err));
            });
        }

        result
    }

    fn errors_count(&self, _: usize, texture_errors: &Self::Input, _: &[String]) -> usize {
        texture_errors.0.get(&self.0).is_some() as usize
    }
}
