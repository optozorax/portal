use std::hash::Hash;
use crate::code_generation::ErrId;
use crate::code_generation::ErrorId;
use crate::gui::object::MatrixName;
use crate::gui::uniform::FormulasCache;
use egui::*;
use glam::*;

use std::collections::BTreeMap;

use std::f32::consts::PI;

pub fn mymax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

pub fn deg2rad(deg: f32) -> f32 {
    deg / 180. * PI
}

pub fn rad2deg(rad: f32) -> f32 {
    rad * 180. / PI
}

#[derive(Debug, Default)]
pub struct MatrixRecursionError(pub BTreeMap<MatrixName, bool>);

#[derive(Debug, Default)]
pub struct ShaderErrors(pub BTreeMap<ErrId, Vec<(usize, String)>>);

#[derive(Debug, Default)]
pub struct TextureErrors(pub BTreeMap<String, macroquad::file::FileError>);

impl ShaderErrors {
    pub fn get_errors<'a, T: ErrorId>(
        &'a self,
        t: &T,
        pos: usize,
    ) -> Option<&'a [(usize, String)]> {
        self.0.get(&t.identifier(pos)).map(|x| &x[..])
    }
}

#[derive(Debug, Default)]
pub struct Data {
    pub to_export: Option<String>,
    pub errors: ShaderErrors,
    pub matrix_recursion_error: MatrixRecursionError,
    pub show_error_window: bool,
    pub show_glsl_library: bool,
    pub show_compiled_code: Option<String>,
    pub description_en_edit: bool,
    pub description_ru_edit: bool,

    pub formulas_cache: FormulasCache,

    pub reload_textures: bool,
    pub texture_errors: TextureErrors,

    pub read_ru: bool,

    pub generated_code_show_text: bool,
}

pub fn add_line_numbers(s: &str) -> String {
    s.split('\n')
        .enumerate()
        .map(|(line, text)| format!("{}|{}", line + 1, text))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct WhatChanged {
    pub uniform: bool,
    pub shader: bool,
}

impl WhatChanged {
    pub fn from_uniform(uniform: bool) -> Self {
        Self {
            uniform,
            shader: false,
        }
    }

    pub fn from_shader(shader: bool) -> Self {
        Self {
            uniform: false,
            shader,
        }
    }
}

impl std::ops::BitOrAssign for WhatChanged {
    fn bitor_assign(&mut self, rhs: Self) {
        self.uniform |= rhs.uniform;
        self.shader |= rhs.shader;
    }
}

pub fn check_changed<T: PartialEq + Clone, F: FnOnce(&mut T)>(t: &mut T, f: F) -> bool {
    let previous = t.clone();
    f(t);
    previous != *t
}

pub fn egui_bool(ui: &mut Ui, flag: &mut bool) -> bool {
    check_changed(flag, |flag| drop(ui.add(Checkbox::new(flag, ""))))
}

pub fn egui_angle(ui: &mut Ui, angle: &mut f32) -> bool {
    let mut current = rad2deg(*angle);
    let previous = current;
    ui.add(
        DragValue::from_get_set(|v| {
            if let Some(v) = v {
                if v > 360. {
                    current = v as f32 % 360.;
                } else if v < 0. {
                    current = 360. + (v as f32 % 360.);
                } else {
                    current = v as f32;
                }
            }
            current.into()
        })
        .speed(1)
        .suffix("°"),
    );
    if (previous - current).abs() > 1e-6 {
        *angle = deg2rad(current);
        true
    } else {
        false
    }
}

pub fn egui_angle_f64(ui: &mut Ui, angle: &mut f64) -> bool {
    use std::f64::consts::PI;
    let mut current = *angle / PI * 180.;
    let previous = current;
    ui.add(
        DragValue::from_get_set(|v| {
            if let Some(v) = v {
                if v > 360. {
                    current = v % 360.;
                } else if v < 0. {
                    current = 360. + (v % 360.);
                } else {
                    current = v;
                }
            }
            current
        })
        .speed(1)
        .suffix("°"),
    );
    if (previous - current).abs() > 1e-6 {
        *angle = current * PI / 180.;
        true
    } else {
        false
    }
}

pub fn egui_f32(ui: &mut Ui, value: &mut f32) -> bool {
    check_changed(value, |value| {
        ui.add(
            DragValue::new(value)
                .speed(0.01)
                .min_decimals(0)
                .max_decimals(2),
        );
    })
}

pub fn egui_0_1(ui: &mut Ui, value: &mut f32) -> bool {
    check_changed(value, |value| {
        ui.add(
            Slider::new(value, 0.0..=1.0)
                .clamp_to_range(true)
                .min_decimals(0)
                .max_decimals(2),
        );
    })
}

pub fn egui_f32_positive(ui: &mut Ui, value: &mut f32) -> bool {
    check_changed(value, |value| {
        ui.add(
            DragValue::new(value)
                .speed(0.01)
                .prefix("×")
                .clamp_range(0.0..=1000.0)
                .min_decimals(0)
                .max_decimals(2),
        );
    })
}

pub fn egui_label(ui: &mut Ui, label: &str, size: f32) {
    let (rect, _) = ui.allocate_at_least(egui::vec2(size, 0.), Sense::hover());
    ui.painter().text(
        rect.max,
        Align2::RIGHT_CENTER,
        label,
        TextStyle::Body,
        ui.visuals().text_color(),
    );
}

pub fn egui_existing_name(
    ui: &mut Ui,
    label: &str,
    size: f32,
    current: &mut String,
    names: &[String],
    add_to_errors_count: &mut usize,
) -> bool {
    check_changed(current, |current| {
        ui.horizontal(|ui| {
            egui_label(ui, label, size);
            ui.text_edit_singleline(current);
        });
        if !names.contains(current) {
            *add_to_errors_count += 1;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.add(Label::new("Error: ").text_color(Color32::RED));
                ui.label(format!("name '{}' not found", current));
            });
        }
    })
}

pub fn egui_errors(ui: &mut Ui, errors: &[(usize, String)]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.;
        for (line_no, message) in errors {
            if *line_no == usize::MAX {
                ui.add(
                    Label::new("UNKNOWN ERR: ")
                        .text_color(COLOR_ERROR)
                        .monospace(),
                );
            } else {
                ui.add(
                    Label::new(format!("ERR:{}: ", line_no))
                        .text_color(COLOR_ERROR)
                        .monospace(),
                );
            }
            ui.add(Label::new(message).monospace());
            ui.add(Label::new("\n").monospace());
        }
    });
}

pub fn egui_with_red_field<Res>(ui: &mut Ui, has_errors: bool, f: impl FnOnce(&mut Ui) -> Res) -> Res {
    let previous = ui.visuals().clone();
    if has_errors {
        ui.visuals_mut().selection.stroke.color = Color32::RED;
        ui.visuals_mut().widgets.inactive.bg_stroke.color = Color32::from_rgb_additive(128, 0, 0);
        ui.visuals_mut().widgets.inactive.bg_stroke.width = 1.0;
        ui.visuals_mut().widgets.hovered.bg_stroke.color =
            Color32::from_rgb_additive(255, 128, 128);
    }
    let result = f(ui);
    if has_errors {
        *ui.visuals_mut() = previous;
    }
    result
}

pub fn egui_with_enabled_by(ui: &mut Ui, by: bool, f: impl FnOnce(&mut Ui)) {
    let previous = ui.enabled();
    ui.set_enabled(by);
    f(ui);
    ui.set_enabled(previous);
}

pub fn view_edit(ui: &mut Ui, text: &mut String, id_source: impl Hash) -> egui::Response {
    #[derive(Clone, Copy, Eq, PartialEq, Debug)]
    #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
    enum State {
        View,
        Edit,
    }

    impl Default for State {
        fn default() -> Self {
            State::View
        }
    }

    ui.vertical(|ui| {
        let id = Id::new(id_source);

        let mut state = *ui.memory().id_data.get_or_default::<State>(id);

        ui.horizontal(|ui| {
            ui.selectable_value(&mut state, State::View, "View");
            ui.selectable_value(&mut state, State::Edit, "Edit");
        });

        ui.memory().id_data.insert(id, state);

        match state {
            State::View => {
                egui::experimental::easy_mark(ui, &text);
            }
            State::Edit => {
                ui.add(
                    TextEdit::multiline(text)
                        .text_style(TextStyle::Monospace),
                );
            }
        }
    })
    .response
}

pub const COLOR_TYPE: Color32 = Color32::from_rgb(0x2d, 0xbf, 0xb8);
pub const COLOR_FUNCTION: Color32 = Color32::from_rgb(0x2B, 0xAB, 0x63);
pub const COLOR_ERROR: Color32 = Color32::RED;
