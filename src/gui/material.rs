use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::glsl::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;

use crate::gui::common::ShaderErrors;
use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Material {
    Simple {
        color: [f32; 3],
        normal_coef: f32, // 0..1
        grid: bool,
        grid_scale: f32,
        grid_coef: f32, // 0..1
    },
    Reflect {
        add_to_color: [f32; 3],
    },
    Refract {
        refractive_index: f32,
        add_to_color: [f32; 3],
    },
    Complex {
        code: MaterialCode, // gets (SphereIntersection hit, Ray r) -> MaterialProcessing, must use material_next or material_final
    },
}

impl Default for Material {
    fn default() -> Self {
        Material::Simple {
            color: [0.5, 0.2, 0.2],
            normal_coef: 0.5,
            grid: true,
            grid_scale: 4.0,
            grid_coef: 0.3,
        }
    }
}

impl Material {
    pub fn errors_count(&self, pos: usize, errors: &ShaderErrors) -> usize {
        if let Some(local_errors) = errors.get_errors(self, pos) {
            local_errors.len()
        } else {
            0
        }
    }
}

impl StorageElem for MaterialComboBox {
    type GetType = Material;
    type Input = ShaderErrors;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.0.clone())
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let mut changed = WhatChanged::from_shader(egui_combo_box(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, pos, input);
        changed
    }

    fn errors_count(&self, pos: usize, input: &Self::Input, _: &[String]) -> usize {
        self.0.errors_count(pos, &input)
    }
}

impl ComboBoxChoosable for Material {
    fn variants() -> &'static [&'static str] {
        &["Simple", "Reflect", "Refract", "Complex"]
    }
    fn get_number(&self) -> usize {
        use Material::*;
        match self {
            Simple { .. } => 0,
            Reflect { .. } => 1,
            Refract { .. } => 2,
            Complex { .. } => 3,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Material::*;
        *self = match number {
            0 => Default::default(),
            1 => Reflect {
                add_to_color: [1.0, 1.0, 1.0],
            },
            2 => Refract {
                add_to_color: [1.0, 1.0, 1.0],
                refractive_index: 1.5,
            },
            3 => Complex {
                code: Default::default(),
            },
            _ => unreachable!(),
        };
    }
}

impl Material {
    fn egui(&mut self, ui: &mut Ui, pos: usize, errors: &mut ShaderErrors) -> WhatChanged {
        use Material::*;
        let mut changed = false;
        let has_errors = errors.get_errors(&*self, pos).is_some();
        match self {
            Simple {
                color,
                normal_coef,
                grid,
                grid_scale,
                grid_coef,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Color");
                    changed |= check_changed(color, |color| drop(ui.color_edit_button_rgb(color)));
                    ui.separator();
                    ui.label("Normal coef");
                    changed |= check_changed(normal_coef, |normal_coef| {
                        drop(
                            ui.add(
                                DragValue::new(normal_coef)
                                    .speed(0.01)
                                    .clamp_range(0.0..=1.0)
                                    .min_decimals(0)
                                    .max_decimals(2),
                            ),
                        )
                    });
                });
                ui.horizontal(|ui| {
                    changed |=
                        check_changed(grid, |grid| drop(ui.add(Checkbox::new(grid, "Grid"))));
                    ui.set_enabled(*grid);
                    ui.separator();
                    ui.label("Grid scale");
                    changed |= check_changed(grid_scale, |grid_scale| {
                        drop(
                            ui.add(
                                DragValue::new(grid_scale)
                                    .speed(0.01)
                                    .clamp_range(0.0..=1000.0)
                                    .min_decimals(0)
                                    .max_decimals(2),
                            ),
                        )
                    });
                    ui.separator();
                    ui.label("Grid coef");
                    changed |= check_changed(grid_coef, |grid_coef| {
                        drop(
                            ui.add(
                                DragValue::new(grid_coef)
                                    .speed(0.01)
                                    .clamp_range(0.0..=1.0)
                                    .min_decimals(0)
                                    .max_decimals(2),
                            ),
                        )
                    });
                });
            }
            Reflect { add_to_color } => {
                ui.horizontal(|ui| {
                    ui.label("Add to color");
                    changed |=
                        check_changed(add_to_color, |color| drop(ui.color_edit_button_rgb(color)));
                });
            }
            Refract {
                refractive_index,
                add_to_color,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Add to color");
                    changed |=
                        check_changed(add_to_color, |color| drop(ui.color_edit_button_rgb(color)));
                });
                ui.horizontal(|ui| {
                    ui.label("Refractive index");
                    changed |= check_changed(refractive_index, |r| {
                        drop(
                            ui.add(
                                DragValue::new(r)
                                    .speed(0.01)
                                    .clamp_range(0.0..=10.0)
                                    .min_decimals(0)
                                    .max_decimals(2),
                            ),
                        )
                    });
                });
            }
            Complex { code } => {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.;
                    ui.add(
                        Label::new("MaterialProcessing ")
                            .text_color(COLOR_TYPE)
                            .monospace(),
                    );
                    ui.add(
                        Label::new("process_material")
                            .text_color(COLOR_FUNCTION)
                            .monospace(),
                    );
                    ui.add(Label::new("(\n  ").monospace());
                    ui.add(
                        Label::new("SurfaceIntersect ")
                            .text_color(COLOR_TYPE)
                            .monospace(),
                    );
                    ui.add(Label::new("hit,\n  ").monospace());
                    ui.add(Label::new("Ray ").text_color(COLOR_TYPE).monospace());
                    ui.add(Label::new("r\n) {").monospace());
                });

                egui_with_red_field(ui, has_errors, |ui| {
                    changed |= code.0.egui(ui).shader;
                });
                ui.add(Label::new("}").monospace());

                if let Some(local_errors) = errors.get_errors(self, pos) {
                    egui_errors(ui, local_errors);
                }
            }
        }
        WhatChanged::from_shader(changed)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialComboBox(pub Material);
