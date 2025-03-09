use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::glsl::*;
use crate::gui::storage2::*;
use crate::gui::unique_id::UniqueId;

use crate::gui::common::ShaderErrors;
use egui::*;
use glam::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Material {
    Simple {
        color: [f64; 3],
        normal_coef: f64, // 0..1
        grid: bool,
        grid_scale: f64,
        grid_coef: f64, // 0..1

        #[serde(default)]
        grid2: bool,
    },
    Reflect {
        add_to_color: [f64; 3],
    },
    Refract {
        refractive_index: f64,
        add_to_color: [f64; 3],
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
            grid2: false,
        }
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MaterialId(UniqueId);

impl Wrapper for MaterialId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for Material {
    type IdWrapper = MaterialId;
    type GetType = Material;

    const SAFE_TO_RENAME: bool = false;

    type Input = ShaderErrors;
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        errors: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        data_id: egui::Id,
        self_id: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = egui_combo_box(ui, "Type:", 45., self, data_id);
        ui.separator();

        use Material::*;
        let has_errors = errors.get(self_id).is_some();
        match self {
            Simple {
                color,
                normal_coef,
                grid,
                grid_scale,
                grid_coef,
                grid2,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Color");
                    changed |= egui_color_f64(ui, color);
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
                    ui.separator();
                    changed |=
                        check_changed(grid2, |grid| drop(ui.add(Checkbox::new(grid, "Grid 2"))));
                });
            }
            Reflect { add_to_color } => {
                ui.horizontal(|ui| {
                    ui.label("Add to color");
                    changed |= egui_color_f64(ui, add_to_color);
                });
            }
            Refract {
                refractive_index,
                add_to_color,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Add to color");
                    changed |= egui_color_f64(ui, add_to_color);
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
                    ui.add(Label::new(
                        egui::RichText::new("MaterialProcessing ")
                            .color(COLOR_TYPE)
                            .monospace(),
                    ));
                    ui.add(Label::new(
                        egui::RichText::new("process_material")
                            .color(COLOR_FUNCTION)
                            .monospace(),
                    ));
                    ui.add(Label::new(egui::RichText::new("(\n  ").monospace()));
                    ui.add(Label::new(
                        egui::RichText::new("SurfaceIntersect ")
                            .color(COLOR_TYPE)
                            .monospace(),
                    ));
                    ui.add(Label::new(egui::RichText::new("hit,\n  ").monospace()));
                    ui.add(Label::new(
                        egui::RichText::new("Ray ").color(COLOR_TYPE).monospace(),
                    ));
                    ui.add(Label::new(egui::RichText::new("r\n) {").monospace()));
                });

                egui_with_red_field(ui, has_errors, |ui| {
                    changed |= code.0.egui(ui).shader;
                });
                ui.add(Label::new(egui::RichText::new("}").monospace()));

                if let Some(local_errors) = errors.get(self_id) {
                    egui_errors(ui, local_errors);
                }
            }
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
