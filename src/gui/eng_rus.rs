use crate::gui::common::egui_label;
use crate::gui::common::view_edit;
use egui::Ui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngRusText {
    pub eng: String,
    pub rus: String,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub enum EngRusSettings {
    Eng,
    Rus,
}

impl Default for EngRusSettings {
    fn default() -> Self {
        EngRusSettings::Eng
    }
}

impl EngRusSettings {
    pub fn egui(ui: &mut Ui) {
        let id = egui::Id::new("EngRusSettings");
        let mut state = *ui.memory().data.get_persisted_mut_or_default::<EngRusSettings>(id) == EngRusSettings::Rus;

        if ui
            .add(egui::SelectableLabel::new(state, "🔤 Русский язык"))
            .clicked()
        {
            state = !state;

            if state {
                ui.memory().data.insert_persisted(id, EngRusSettings::Rus);
            } else {
                ui.memory().data.insert_persisted(id, EngRusSettings::Eng);
            }
        }
    }
}

impl EngRusText {
    pub fn egui_view_edit(&mut self, ui: &mut Ui, id: egui::Id) {
        ui.vertical(|ui| {
            ui.label("Eng:");
            view_edit(ui, &mut self.eng, id.with("en"));
            ui.separator();
            ui.label("Rus:");
            view_edit(ui, &mut self.rus, id.with("ru"));
        });
    }

    pub fn egui_multiline(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label("Eng:");
            ui.text_edit_multiline(&mut self.eng);
            ui.separator();
            ui.label("Rus:");
            ui.text_edit_multiline(&mut self.rus);
        });
    }

    pub fn egui_singleline(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                egui_label(ui, "Eng:", 45.);
                ui.text_edit_singleline(&mut self.eng);
            });
            ui.horizontal(|ui| {
                egui_label(ui, "Rus:", 45.);
                ui.text_edit_singleline(&mut self.rus);
            });
        });
    }

    pub fn text<'a>(&'a self, ui: &mut Ui) -> &'a str {
        let state = *ui.memory().data.get_persisted_mut_or_default::<EngRusSettings>(egui::Id::new("EngRusSettings"));
        match state {
            EngRusSettings::Eng => &self.eng,
            EngRusSettings::Rus => &self.rus,
        }
    }
}
