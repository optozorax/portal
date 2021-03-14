use egui::*;
use glam::*;
use macroquad::prelude::UniformType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::f32::consts::PI;
use std::ops::Range;

pub fn mymax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

pub fn deg2rad(deg: f32) -> f32 {
    deg / 180. * PI
}

pub fn rad2deg(rad: f32) -> f32 {
    rad * 180. / PI
}

// ----------------------------------------------------------------------------------------------------------
// Common UI things
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Default)]
pub struct Data {
    pub names: Option<Vec<String>>,
    pub pos: Option<u64>,
    pub to_export: Option<String>,
    pub errors: Option<BTreeMap<String, Vec<(usize, String)>>>,
    pub show_error_window: bool,
}

pub trait Eguiable {
    #[must_use]
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged;
}

pub fn egui_bool(ui: &mut Ui, flag: &mut bool) -> bool {
    check_changed(flag, |flag| drop(ui.add(Checkbox::new(flag, ""))))
}

pub fn egui_angle(ui: &mut Ui, angle: &mut f32) -> bool {
    let mut current = rad2deg(*angle) as i32;
    let previous = current;
    ui.add(
        DragValue::i32(&mut current)
            .speed(1)
            .suffix("°")
            .clamp_range(0.0..=360.0),
    );
    if previous != current {
        *angle = deg2rad(current as f32);
        true
    } else {
        false
    }
}

pub fn egui_f32(ui: &mut Ui, value: &mut f32) -> bool {
    check_changed(value, |value| {
        ui.add(
            DragValue::f32(value)
                .speed(0.01)
                .min_decimals(0)
                .max_decimals(2),
        );
    })
}

pub fn egui_f32_positive(ui: &mut Ui, value: &mut f32) -> bool {
    check_changed(value, |value| {
        ui.add(
            DragValue::f32(value)
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
        Color32::WHITE,
    );
}

pub fn egui_existing_name(
    ui: &mut Ui,
    label: &str,
    size: f32,
    current: &mut String,
    names: &[String],
) -> bool {
    check_changed(current, |current| {
        ui.horizontal(|ui| {
            egui_label(ui, label, size);
            ui.text_edit_singleline(current);
        });
        if !names.contains(current) {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error: ").text_color(Color32::RED));
                ui.label(format!("name '{}' not found", current));
            });
        }
    })
}

pub fn egui_errors(ui: &mut Ui, errors: &[(usize, String)]) {
    ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
        ui.spacing_mut().item_spacing.x = 0.;
        for (line_no, message) in errors {
            ui.add(
                Label::new(format!("ERR:{}: ", line_no))
                    .text_color(COLOR_ERROR)
                    .monospace(),
            );
            ui.add(Label::new(message).monospace());
            ui.add(Label::new("\n").monospace());
        }
    });
}

// ----------------------------------------------------------------------------------------------------------
// Combo box things
// ----------------------------------------------------------------------------------------------------------

pub trait ComboBoxChoosable {
    fn variants() -> &'static [&'static str];
    fn get_number(&self) -> usize;
    fn set_number(&mut self, number: usize);
}

pub fn egui_combo_label<T: ComboBoxChoosable>(
    ui: &mut Ui,
    label: &str,
    size: f32,
    t: &mut T,
) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    ui.horizontal(|ui| {
        egui_label(ui, label, size);
        for (pos, name) in T::variants().iter().enumerate() {
            ui.selectable_value(&mut current_type, pos, *name);
        }
    });

    if current_type != previous_type {
        t.set_number(current_type);
        is_changed = true;
    }

    is_changed
}

pub fn egui_combo_box<T: ComboBoxChoosable>(
    ui: &mut Ui,
    label: &str,
    size: f32,
    t: &mut T,
    id: u64,
) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    let id = ui.make_persistent_id(id);
    ui.horizontal(|ui| {
        egui_label(ui, label, size);
        egui::combo_box(ui, id, T::variants()[current_type], |ui| {
            for (pos, name) in T::variants().iter().enumerate() {
                ui.selectable_value(&mut current_type, pos, *name);
            }
        });
    });

    if current_type != previous_type {
        t.set_number(current_type);
        is_changed = true;
    }

    is_changed
}

// ----------------------------------------------------------------------------------------------------------
// Matrix
// ----------------------------------------------------------------------------------------------------------

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
        &["Simple", "Mul", "Teleport"]
    }
    fn get_number(&self) -> usize {
        use Matrix::*;
        match self {
            Simple { .. } => 0,
            Mul { .. } => 1,
            Teleport { .. } => 2,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Matrix::*;
        *self = match number {
            1 => Mul {
                to: "id".to_owned(),
                what: "id".to_owned(),
            },
            2 => Teleport {
                first_portal: "id".to_owned(),
                second_portal: "id".to_owned(),

                what: "id".to_owned(),
            },
            0 => Self::default(),
            _ => unreachable!(),
        };
    }
}

impl Eguiable for Matrix {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use Matrix::*;
        let names = data.names.as_ref().unwrap();
        let mut is_changed = false;
        match self {
            Mul { to, what } => {
                is_changed |= egui_existing_name(ui, "Mul to:", 45., to, names);
                is_changed |= egui_existing_name(ui, "What:", 45., what, names);
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                is_changed |= egui_existing_name(ui, "From:", 45., first_portal, names);
                is_changed |= egui_existing_name(ui, "To:", 45., second_portal, names);
                is_changed |= egui_existing_name(ui, "What:", 45., what, names);
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
        }
        WhatChanged::from_uniform(is_changed)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatrixComboBox(pub Matrix);

impl Eguiable for MatrixComboBox {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed =
            WhatChanged::from_uniform(egui_combo_label(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, data);
        changed
    }
}

impl StorageElem for MatrixComboBox {
    type GetType = Mat4;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, mut f: F) -> Option<Self::GetType> {
        use Matrix::*;
        Some(match &self.0 {
            Mul { to, what } => {
                let to = f(&to)?;
                let what = f(&what)?;
                what * to
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let first_portal = f(&first_portal)?;
                let second_portal = f(&second_portal)?;
                let what = f(&what)?;
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
        })
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (
            vec!["id".to_owned(), "a".to_owned()],
            vec![MatrixComboBox::default(), MatrixComboBox::default()],
        )
    }
}

// ----------------------------------------------------------------------------------------------------------
// Errors handling
// ----------------------------------------------------------------------------------------------------------

pub trait ErrorCount {
    fn errors(&self, data: &mut Data) -> usize;
}

// ----------------------------------------------------------------------------------------------------------
// Gui named storage
// ----------------------------------------------------------------------------------------------------------

pub trait StorageElem: Sized + Default + Eguiable {
    type GetType;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, f: F) -> Option<Self::GetType>;

    fn defaults() -> (Vec<String>, Vec<Self>);
}

// Checks if this name is used, sends name to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageWithNames<T: StorageElem> {
    names: Vec<String>,
    storage: Vec<T>,
}

impl<T: StorageElem> Default for StorageWithNames<T> {
    fn default() -> Self {
        let (names, storage) = T::defaults();
        StorageWithNames { names, storage }
    }
}

impl<T: StorageElem> StorageWithNames<T> {
    pub fn get(&self, name: &str) -> Option<T::GetType> {
        let mut visited = vec![];
        self.get_inner(name, &mut visited)
    }

    pub fn add(&mut self, name: String, t: T) {
        self.names.push(name);
        self.storage.push(t);
    }

    pub fn remove(&mut self, pos: usize) {
        self.names.remove(pos);
        self.storage.remove(pos);
    }

    pub fn names_iter(&self) -> std::slice::Iter<String> {
        self.names.iter()
    }

    fn get_inner<'a>(&'a self, name: &'a str, visited: &mut Vec<String>) -> Option<T::GetType> {
        if visited.iter().any(|x| *x == name) {
            return None;
        }

        visited.push(name.to_owned());
        let pos = self.names.iter().position(|x| x == name)?;
        let result = self.storage[pos].get(|name| self.get_inner(name, visited))?;
        visited.pop().unwrap();
        Some(result)
    }
}

pub fn egui_collection(
    ui: &mut Ui,
    collection: &mut Vec<impl Eguiable + Default>,
    data: &mut Data,
) -> WhatChanged {
    let mut changed = WhatChanged::default();
    let mut to_delete = None;
    for (pos, elem) in collection.iter_mut().enumerate() {
        CollapsingHeader::new(pos.to_string())
            .id_source(pos)
            .show(ui, |ui| {
                if ui
                    .add(Button::new("Delete").text_color(Color32::RED))
                    .clicked()
                {
                    to_delete = Some(pos);
                }

                data.pos = Some(pos as u64);
                changed |= elem.egui(ui, data);
            });
    }
    if let Some(pos) = to_delete {
        changed.shader = true;
        collection.remove(pos);
    }
    if ui
        .add(Button::new("Add").text_color(Color32::GREEN))
        .clicked()
    {
        collection.push(Default::default());
    }
    changed
}

impl<T: StorageElem> Eguiable for StorageWithNames<T> {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut to_delete = None;
        let storage = &mut self.storage;
        let names = &mut self.names;
        for (pos, elem) in storage.iter_mut().enumerate() {
            CollapsingHeader::new(&names[pos])
                .id_source(pos)
                .show(ui, |ui| {
                    let previous = names[pos].clone();
                    ui.horizontal(|ui| {
                        egui_label(ui, "Name:", 45.);
                        ui.put(
                            Rect::from_min_size(
                                ui.min_rect().min + egui::vec2(49., 0.),
                                egui::vec2(ui.available_width() - 65., 0.),
                            ),
                            TextEdit::singleline(&mut names[pos]),
                        );
                        if ui
                            .add(Button::new("Delete").text_color(Color32::RED))
                            .clicked()
                        {
                            to_delete = Some(pos);
                        }
                    });
                    if names[..pos].contains(&names[pos]) {
                        ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                            ui.add(Label::new("Error: ").text_color(Color32::RED));
                            ui.label(format!("name '{}' already used", names[pos]));
                        });
                    }
                    changed.shader |= previous != names[pos];

                    data.pos = Some(pos as u64);
                    changed |= elem.egui(ui, data);
                });
        }
        if let Some(pos) = to_delete {
            changed.shader = true;
            self.remove(pos);
        }
        if ui
            .add(Button::new("Add").text_color(Color32::GREEN))
            .clicked()
        {
            self.add(format!("_{}", self.names.len()), Default::default());
        }
        changed
    }
}

// ----------------------------------------------------------------------------------------------------------
// Material
// ----------------------------------------------------------------------------------------------------------

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

impl StorageElem for MaterialComboBox {
    type GetType = Material;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, _: F) -> Option<Self::GetType> {
        Some(self.0.clone())
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (
            vec!["black".to_owned()],
            vec![MaterialComboBox(Material::default())],
        )
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

const COLOR_TYPE: Color32 = Color32::from_rgb(0x2d, 0xbf, 0xb8);
const COLOR_FUNCTION: Color32 = Color32::from_rgb(0x2B, 0xAB, 0x63);
const COLOR_ERROR: Color32 = Color32::RED;

impl Eguiable for Material {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use Material::*;
        let mut changed = false;
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
                                DragValue::f32(normal_coef)
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
                                DragValue::f32(grid_scale)
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
                                DragValue::f32(grid_coef)
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
                                DragValue::f32(r)
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
                ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
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
                changed |= code.0.egui(ui, data).shader;
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = data
                    .errors
                    .as_ref()
                    .and_then(|map| map.get(&self.identifier(data.pos.unwrap() as usize).unwrap()))
                {
                    egui_errors(ui, local_errors);
                }
            }
        }
        WhatChanged::from_shader(changed)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialComboBox(pub Material);

impl Eguiable for MaterialComboBox {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::from_shader(egui_combo_box(
            ui,
            "Type:",
            45.,
            &mut self.0,
            data.pos.unwrap(),
        ));
        ui.separator();
        changed |= self.0.egui(ui, data);
        changed
    }
}

// ----------------------------------------------------------------------------------------------------------
// Glsl code in objects and materials
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCode(pub GlslCode);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlslCode(pub String);

impl Default for MaterialCode {
    fn default() -> Self {
        MaterialCode(GlslCode(
            "return plane_process_material(hit, r, color(0.2, 0.6, 0.6));".to_owned(),
        ))
    }
}

impl Eguiable for GlslCode {
    fn egui(&mut self, ui: &mut Ui, _: &mut Data) -> WhatChanged {
        WhatChanged::from_shader(
            ui.add(TextEdit::multiline(&mut self.0).text_style(TextStyle::Monospace))
                .changed(),
        )
    }
}

impl Eguiable for MaterialCode {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        self.0.egui(ui, data)
    }
}

// Code must return integer - material. NOT_INSIDE if not inside. TELEPORT is should be teleported by current matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsInsideCode(pub GlslCode);

impl Default for IsInsideCode {
    fn default() -> Self {
        IsInsideCode(GlslCode(
            "if (x*x + y*y < 1.) {\n  return black_M;\n} else {\n  return NOT_INSIDE;\n}"
                .to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntersectCode(pub GlslCode);

impl Default for IntersectCode {
    fn default() -> Self {
        Self(GlslCode(
            r#"vec3 op = -r.o.xyz;
float b = dot(op, r.d.xyz);
float det = b*b - dot(op, op) + 1.0;
if (det < 0.) return scene_intersection_none;

det = sqrt(det);
float t = b - det;
if (t < 0.) t = b + det;
if (t < 0.) return scene_intersection_none;

vec4 pos = r.o + r.d * t;
vec3 n = normalize(pos.xyz);

float u = atan(pos.z, pos.x);
float v = atan(sqrt(pos.x * pos.x + pos.z * pos.z), pos.y);

return SceneIntersection(black_M, SurfaceIntersection(true, t, u, v, n));"#
                .to_owned(),
        ))
    }
}

// ----------------------------------------------------------------------------------------------------------
// Scene object
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixName(String);

impl MatrixName {
    pub fn normal_name(&self) -> String {
        format!("{}_mat", self.0)
    }

    pub fn inverse_name(&self) -> String {
        format!("{}_mat_inv", self.0)
    }

    pub fn teleport_to_name(&self, to: &MatrixName) -> String {
        format!("{}_to_{}_mat_teleport", self.0, to.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Simple(MatrixName),
    Portal(MatrixName, MatrixName),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Object {
    DebugMatrix(MatrixName),
    Flat {
        kind: ObjectType,
        is_inside: IsInsideCode, // gets current position (vec4), surface x y, must return material number. if this is portal, then additionally gets `first`, `back`
    },
    Complex {
        kind: ObjectType,
        intersect: IntersectCode, // gets transformed Ray, must return SurfaceIntersect
    },
}

impl Default for MatrixName {
    fn default() -> Self {
        Self("id".into())
    }
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::Simple(Default::default())
    }
}

impl Default for Object {
    fn default() -> Self {
        Object::DebugMatrix(Default::default())
    }
}

impl ComboBoxChoosable for ObjectType {
    fn variants() -> &'static [&'static str] {
        &["Simple", "Portal"]
    }
    fn get_number(&self) -> usize {
        use ObjectType::*;
        match self {
            Simple { .. } => 0,
            Portal { .. } => 1,
        }
    }
    fn set_number(&mut self, number: usize) {
        use ObjectType::*;
        *self = match number {
            0 => Simple(Default::default()),
            1 => Portal(Default::default(), Default::default()),
            _ => unreachable!(),
        };
    }
}

impl ComboBoxChoosable for Object {
    fn variants() -> &'static [&'static str] {
        &["Debug", "Flat", "Complex"]
    }
    fn get_number(&self) -> usize {
        use Object::*;
        match self {
            DebugMatrix { .. } => 0,
            Flat { .. } => 1,
            Complex { .. } => 2,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Object::*;
        *self = match number {
            0 => DebugMatrix(Default::default()),
            1 => Flat {
                kind: Default::default(),
                is_inside: Default::default(),
            },
            2 => Complex {
                kind: Default::default(),
                intersect: Default::default(),
            },
            _ => unreachable!(),
        };
    }
}

impl Eguiable for ObjectType {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use ObjectType::*;
        let names = data.names.as_ref().unwrap();
        let mut is_changed = false;
        match self {
            Simple(a) => is_changed |= egui_existing_name(ui, "Matrix:", 45., &mut a.0, names),
            Portal(a, b) => {
                is_changed |= egui_existing_name(ui, "First:", 45., &mut a.0, names);
                is_changed |= egui_existing_name(ui, "Second:", 45., &mut b.0, names);
            }
        }
        WhatChanged::from_shader(is_changed)
    }
}

impl Eguiable for Object {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use Object::*;
        let names = data.names.as_ref().unwrap();
        let mut is_changed = WhatChanged::default();
        match self {
            DebugMatrix(a) => {
                is_changed.shader |= egui_existing_name(ui, "Matrix:", 45., &mut a.0, names);
            }
            Flat { kind, is_inside } => {
                is_changed.shader |= egui_combo_label(ui, "Kind:", 45., kind);
                is_changed |= kind.egui(ui, data);
                ui.separator();
                if matches!(kind, ObjectType::Portal { .. }) {
                    ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        ui.add(Label::new("int ").text_color(COLOR_TYPE).monospace());
                        ui.add(
                            Label::new("is_inside")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("vec4 ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("pos, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("x, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("y, \n              ").monospace());
                        ui.add(Label::new("bool ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("back, ").monospace());
                        ui.add(Label::new("bool ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("first) {").monospace());
                    });
                } else {
                    ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        ui.add(Label::new("int ").text_color(COLOR_TYPE).monospace());
                        ui.add(
                            Label::new("is_inside")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("vec4 ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("pos, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("x, ").monospace());
                        ui.add(Label::new("float ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("y) {").monospace());
                    });
                }
                is_changed |= is_inside.0.egui(ui, data);
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = data
                    .errors
                    .as_ref()
                    .and_then(|map| map.get(&self.identifier(data.pos.unwrap() as usize).unwrap()))
                {
                    egui_errors(ui, local_errors);
                }
            }
            Complex { kind, intersect } => {
                is_changed.shader |= egui_combo_label(ui, "Kind:", 45., kind);
                is_changed |= kind.egui(ui, data);
                ui.separator();

                ui.horizontal_wrapped_for_text(TextStyle::Monospace, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.;

                    if matches!(kind, ObjectType::Portal { .. }) {
                        ui.add(
                            Label::new("SceneIntersection ")
                                .text_color(COLOR_TYPE)
                                .monospace(),
                        );
                        ui.add(
                            Label::new("intersect")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("Ray ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("r, ").monospace());
                        ui.add(Label::new("bool ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("first) {").monospace());
                    } else {
                        ui.add(
                            Label::new("SceneIntersection ")
                                .text_color(COLOR_TYPE)
                                .monospace(),
                        );
                        ui.add(
                            Label::new("intersect")
                                .text_color(COLOR_FUNCTION)
                                .monospace(),
                        );
                        ui.add(Label::new("(").monospace());
                        ui.add(Label::new("Ray ").text_color(COLOR_TYPE).monospace());
                        ui.add(Label::new("r) {").monospace());
                    }
                });
                is_changed |= intersect.0.egui(ui, data);
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = data
                    .errors
                    .as_ref()
                    .and_then(|map| map.get(&self.identifier(data.pos.unwrap() as usize).unwrap()))
                {
                    egui_errors(ui, local_errors);
                }
            }
        }
        is_changed
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectComboBox(pub Object);

impl Eguiable for ObjectComboBox {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::from_shader(egui_combo_label(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, data);
        changed
    }
}

// ----------------------------------------------------------------------------------------------------------
// Scene
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    matrices: StorageWithNames<MatrixComboBox>,
    objects: Vec<ObjectComboBox>,

    materials: StorageWithNames<MaterialComboBox>,
    library: GlslCode,
}

pub fn add_line_numbers(s: &str) -> String {
    s.split("\n")
        .enumerate()
        .map(|(line, text)| format!("{}|{}", line + 1, text))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Scene {
    pub fn new() -> Self {
        Self {
            matrices: Default::default(),
            objects: vec![Default::default()],
            materials: Default::default(),
            library: GlslCode("".to_owned()),
        }
    }

    pub fn egui(
        &mut self,
        ui: &mut Ui,
        data: &mut Data,
        should_recompile: &mut bool,
    ) -> (
        WhatChanged,
        Option<
            Result<
                macroquad::material::Material,
                (String, String, BTreeMap<String, Vec<(usize, String)>>),
            >,
        >,
    ) {
        let mut changed = WhatChanged::default();
        let mut material = None;

        ui.horizontal(|ui| {
            if ui.button("Export").clicked() {
                let s = serde_json::to_string(self).unwrap();
                data.to_export = Some(s);
            }
            if ui.button("Load mobius").clicked() {
                let s = include_str!("../scene.json");
                *self = serde_json::from_str(&s).unwrap();
                changed.shader = true;
            }
            if ui.button("Load monoportal").clicked() {
                let s = include_str!("../scene_monoportal_offset.json");
                *self = serde_json::from_str(&s).unwrap();
                changed.shader = true;
            }
            if ui
                .add(Button::new("Recompile").enabled(*should_recompile))
                .clicked()
            {
                match self.get_new_material() {
                    Ok(m) => {
                        material = Some(Ok(m));
                        *should_recompile = false;
                        changed.uniform = true;
                        data.errors = None;
                        data.show_error_window = false;
                    }
                    Err(err) => {
                        material = Some(Err(err));
                    }
                }
            }
        });

        // other ui

        CollapsingHeader::new("Matrices")
            .default_open(false)
            .show(ui, |ui| {
                data.names = Some(self.matrices.names.clone());
                changed |= self.matrices.egui(ui, data);
            });
        CollapsingHeader::new("Objects")
            .default_open(false)
            .show(ui, |ui| {
                data.names = Some(self.matrices.names.clone());
                changed |= egui_collection(ui, &mut self.objects, data);
            });
        CollapsingHeader::new("Materials")
            .default_open(false)
            .show(ui, |ui| {
                changed |= self.materials.egui(ui, data);
            });
        CollapsingHeader::new("Glsl Library")
            .default_open(false)
            .show(ui, |ui| {
                changed |= self.library.egui(ui, data);
                if let Some(local_errors) = data.errors.as_ref().and_then(|map| map.get("library"))
                {
                    egui_errors(ui, local_errors);
                }
            });

        if let Some(local_errors) = data.errors.as_ref().and_then(|map| map.get("")).cloned() {
            ui.horizontal(|ui| {
                ui.label("Other errors:");
                if ui.button("Show full code and errors").clicked() {
                    data.show_error_window = true;
                }
            });
            egui_errors(ui, &local_errors);
        }

        (changed, material)
    }
}

// ----------------------------------------------------------------------------------------------------------
// Uniforms
// ----------------------------------------------------------------------------------------------------------

pub trait UniformStruct {
    fn uniforms(&self) -> Vec<(String, UniformType)>;
    fn set_uniforms(&self, material: macroquad::material::Material);
}

impl UniformStruct for Scene {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        use Object::*;
        use ObjectType::*;

        let mut result = Vec::new();
        for object in &self.objects {
            match &object.0 {
                DebugMatrix(matrix) => {
                    result.push(matrix.normal_name());
                }
                Flat { kind, is_inside: _ } | Complex { kind, intersect: _ } => match kind {
                    Simple(matrix) => {
                        result.push(matrix.normal_name());
                        result.push(matrix.inverse_name());
                    }
                    Portal(a, b) => {
                        result.push(a.normal_name());
                        result.push(a.inverse_name());
                        result.push(b.normal_name());
                        result.push(b.inverse_name());
                        result.push(a.teleport_to_name(b));
                        if *b != *a {
                            result.push(b.teleport_to_name(a));
                        }
                    }
                },
            }
        }

        let mut result = result
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|name| (name, UniformType::Mat4))
            .collect::<Vec<_>>();

        // TODO move this out from scene, and set all this parameters outside of scene
        result.extend(vec![
            ("_camera".to_owned(), UniformType::Mat4),
            ("_resolution".to_owned(), UniformType::Float2),
            ("_ray_tracing_depth".to_owned(), UniformType::Int1),
            ("_offset_after_material".to_owned(), UniformType::Float1),
        ]);

        result
    }

    fn set_uniforms(&self, material: macroquad::material::Material) {
        use Object::*;
        use ObjectType::*;
        for object in &self.objects {
            match &object.0 {
                DebugMatrix(matrix) => {
                    if let Some(m) = self.matrices.get(&matrix.0) {
                        material.set_uniform(&matrix.normal_name(), m);
                    }
                }
                Flat { kind, is_inside: _ } | Complex { kind, intersect: _ } => {
                    match kind {
                        Simple(matrix) => {
                            if let Some(m) = self.matrices.get(&matrix.0) {
                                material.set_uniform(&matrix.normal_name(), m);
                                material.set_uniform(&matrix.inverse_name(), m.inverse());
                            } else {
                                // todo add error processing in this case
                            }
                        }
                        Portal(a, b) => {
                            if let Some((ma, mb)) =
                                self.matrices.get(&a.0).zip(self.matrices.get(&b.0))
                            {
                                material.set_uniform(&a.normal_name(), ma);
                                material.set_uniform(&a.inverse_name(), ma.inverse());
                                material.set_uniform(&b.normal_name(), mb);
                                material.set_uniform(&b.inverse_name(), mb.inverse());
                                material.set_uniform(&a.teleport_to_name(b), mb * ma.inverse());
                                if a != b {
                                    material.set_uniform(&b.teleport_to_name(a), ma * mb.inverse());
                                }
                            } else {
                                // todo add error processing in this case
                            }
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------
// Code generation
// ----------------------------------------------------------------------------------------------------------

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct LineNumbersByKey(pub BTreeMap<String, Range<usize>>);

impl LineNumbersByKey {
    pub fn offset(&mut self, lines: usize) {
        self.0
            .iter_mut()
            .for_each(|(_, line)| *line = line.start + lines..line.end + lines);
    }

    pub fn add(&mut self, identifier: String, lines: Range<usize>) {
        assert!(self.0.get(&identifier).is_none());
        self.0.insert(identifier, lines);
    }

    pub fn extend(&mut self, other: LineNumbersByKey) {
        for (k, v) in other.0 {
            self.add(k, v);
        }
    }

    // Returns identifier and local line position
    pub fn get_identifier(&self, line_no: usize) -> Option<(&str, usize)> {
        self.0
            .iter()
            .find(|(_, range)| range.contains(&line_no))
            .map(|(name, range)| (name.as_ref(), line_no - range.start + 1))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StringStorage {
    pub storage: String,
    pub line_numbers: LineNumbersByKey,
    current_line_no: usize,
}

impl Default for StringStorage {
    fn default() -> Self {
        Self {
            storage: Default::default(),
            current_line_no: 1,
            line_numbers: Default::default(),
        }
    }
}

impl StringStorage {
    pub fn add_string<T: AsRef<str>>(&mut self, s: T) {
        self.current_line_no += s.as_ref().chars().filter(|c| *c == '\n').count();
        self.storage += s.as_ref();
    }

    pub fn add_identifier_string<T: AsRef<str>>(&mut self, identifier: String, s: T) {
        let start = self.current_line_no;
        self.add_string(s);
        let end = self.current_line_no;
        self.line_numbers.add(identifier, start..end + 1);
    }

    pub fn add_string_storage(&mut self, mut other: StringStorage) {
        other.line_numbers.offset(self.current_line_no - 1);
        self.add_string(other.storage);
        self.line_numbers.extend(other.line_numbers);
    }
}

fn apply_template(template: &str, mut storages: BTreeMap<String, StringStorage>) -> StringStorage {
    let mut result = StringStorage::default();
    for (is_name, s) in template
        .split("%%")
        .enumerate()
        .map(|(pos, s)| (pos % 2 == 1, s))
    {
        if is_name {
            result.add_string_storage(storages.remove(s).expect(s));
        } else {
            result.add_string(s);
        }
    }
    result
}

#[cfg(test)]
mod tests_string_storage {
    use super::*;

    #[test]
    fn test() {
        let mut s1 = StringStorage::default();
        s1.add_string("1\n2\n3\n");
        s1.add_identifier_string("x".to_owned(), "\n4\n5\n");

        assert_eq!(
            s1,
            StringStorage {
                storage: "1\n2\n3\n\n4\n5\n".to_owned(),
                current_line_no: 7,
                line_numbers: LineNumbersByKey(vec![("x".to_owned(), 4..8)].into_iter().collect()),
            }
        );

        let mut s2 = StringStorage::default();
        s2.add_string("a\nb");
        s2.add_identifier_string("y".to_owned(), "c\nd");

        assert_eq!(
            s2,
            StringStorage {
                storage: "a\nbc\nd".to_owned(),
                current_line_no: 3,
                line_numbers: LineNumbersByKey(vec![("y".to_owned(), 2..4)].into_iter().collect()),
            }
        );

        let storages = vec![("s1".to_owned(), s1), ("s2".to_owned(), s2)]
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let s = apply_template("abc\n%%s2%%\n\ne\nf\n%%s1%%\n9", storages);

        assert_eq!(
            s,
            StringStorage {
                storage: "abc\na\nbc\nd\n\ne\nf\n1\n2\n3\n\n4\n5\n\n9".to_owned(),
                current_line_no: 15,
                line_numbers: LineNumbersByKey(
                    vec![("x".to_owned(), 11..15), ("y".to_owned(), 3..5)]
                        .into_iter()
                        .collect()
                ),
            }
        );
    }
}

pub trait Identifier {
    fn identifier(&self, pos: usize) -> Option<String>;
}

impl Identifier for Material {
    fn identifier(&self, pos: usize) -> Option<String> {
        if matches!(self, Material::Complex { .. }) {
            Some(format!("m_complex_{}", pos))
        } else {
            None
        }
    }
}

impl Identifier for Object {
    fn identifier(&self, pos: usize) -> Option<String> {
        use Object::*;
        match self {
            DebugMatrix { .. } => None,
            Flat { .. } => Some(format!("o_flat_{}", pos)),
            Complex { .. } => Some(format!("o_complex_{}", pos)),
        }
    }
}

impl Scene {
    pub fn generate_shader_code(&self) -> StringStorage {
        let mut storages: BTreeMap<String, StringStorage> = BTreeMap::new();

        storages.insert("uniforms".to_owned(), {
            let mut result = StringStorage::default();
            for name in self
                .uniforms()
                .into_iter()
                .map(|x| x.0)
                .filter(|name| !name.starts_with("_"))
            {
                result.add_string(format!("uniform mat4 {};\n", name))
            }
            result
        });

        let (material_processing, material_defines) = {
            let mut material_processing = StringStorage::default();
            let mut material_defines = StringStorage::default();
            let mut counter = 0;

            use Material::*;
            for (pos, name) in self.materials.names_iter().enumerate() {
                let name_m = format!("{}_M", name);

                material_defines.add_string(format!(
                    "#define {} (USER_MATERIAL_OFFSET + {})\n",
                    name_m, counter
                ));
                counter += 1;

                material_processing
                    .add_string(format!("}} else if (i.material == {}) {{\n", name_m));

                match &self.materials.get(name).unwrap() {
                    Simple {
                        color,
                        normal_coef,
                        grid,
                        grid_scale,
                        grid_coef,
                    } => {
                        material_processing.add_string(
                            format!(
                                "return material_simple(hit, r, color({:e}, {:e}, {:e}), {:e}, {}, {:e}, {:e});\n",
                                color[0], color[1], color[2], normal_coef, grid, grid_scale, grid_coef,
                            )
                        );
                    }
                    Reflect { add_to_color } => {
                        material_processing.add_string(format!(
                            "return material_reflect(hit, r, color({:e}, {:e}, {:e}));\n",
                            add_to_color[0], add_to_color[1], add_to_color[2],
                        ));
                    }
                    Refract {
                        refractive_index,
                        add_to_color,
                    } => {
                        material_processing.add_string(format!(
                            "return material_refract(hit, r, color({:e}, {:e}, {:e}), {:e});\n",
                            add_to_color[0], add_to_color[1], add_to_color[2], refractive_index,
                        ));
                    }
                    x @ Complex { .. } => {
                        let code = match x {
                            Complex { code } => code,
                            _ => unreachable!(),
                        };
                        material_processing
                            .add_identifier_string(x.identifier(pos).unwrap(), &code.0.0);
                        material_processing.add_string("\n");
                    }
                };
            }
            for (pos, first, second) in
                self.objects
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, x)| match &x.0 {
                        Object::DebugMatrix { .. }
                        | Object::Flat {
                            kind: ObjectType::Simple { .. },
                            ..
                        }
                        | Object::Complex {
                            kind: ObjectType::Simple { .. },
                            ..
                        } => None,
                        Object::Flat {
                            kind: ObjectType::Portal(first, second),
                            ..
                        }
                        | Object::Complex {
                            kind: ObjectType::Portal(first, second),
                            ..
                        } => Some((pos, first, second)),
                    })
            {
                let name_m_1 = format!("teleport_{}_1_M", pos);
                let name_m_2 = format!("teleport_{}_2_M", pos);

                material_defines.add_string(format!(
                    "#define {} (USER_MATERIAL_OFFSET + {})\n",
                    name_m_1, counter
                ));
                counter += 1;
                material_defines.add_string(format!(
                    "#define {} (USER_MATERIAL_OFFSET + {})\n",
                    name_m_2, counter
                ));
                counter += 1;

                material_processing
                    .add_string(format!("}} else if (i.material == {}) {{\n", name_m_1));
                material_processing.add_string(format!(
                    "return material_teleport(hit, r, {});",
                    first.teleport_to_name(second)
                ));

                material_processing
                    .add_string(format!("}} else if (i.material == {}) {{\n", name_m_2));
                material_processing.add_string(format!(
                    "return material_teleport(hit, r, {});",
                    second.teleport_to_name(first)
                ));
            }
            (material_processing, material_defines)
        };

        storages.insert("material_processing".to_owned(), material_processing);
        storages.insert("materials_defines".to_owned(), material_defines);

        storages.insert("intersection_functions".to_owned(), {
            use Object::*;
            use ObjectType::*;
            let mut result = StringStorage::default();

            for (pos, i) in self.objects.iter().enumerate() {
                match &i.0 {
                    DebugMatrix(_) => {}
                    Flat { kind, is_inside } => {
                        if matches!(kind, Portal { .. }) {
                            result.add_string(format!(
                                "int is_inside_{}(vec4 pos, float x, float y, bool back, bool first) {{\n",
                                pos
                            ));
                        } else {
                            result.add_string(format!("int is_inside_{}(vec4 pos, float x, float y) {{\n", pos));
                        }
                        result.add_identifier_string(i.0.identifier(pos).unwrap(), &is_inside.0.0);
                        result.add_string("\n}\n");
                    }
                    Complex { kind, intersect } => {
                        if matches!(kind, Portal { .. }) {
                            result.add_string(format!(
                                "SceneIntersection intersect_{}(Ray r, bool first) {{\n",
                                pos
                            ));
                        } else {
                            result.add_string(format!("SceneIntersection intersect_{}(Ray r) {{\n", pos));
                        }
                        result.add_identifier_string(i.0.identifier(pos).unwrap(), &intersect.0.0);
                        result.add_string("\n}\n");
                    }
                }
            }
            result
        });

        storages.insert("intersections".to_owned(), {
            use Object::*;
            use ObjectType::*;
            let mut result = StringStorage::default();

            for (pos, i) in self.objects.iter().enumerate() {
                match &i.0 {
                    DebugMatrix(matrix) => {
                        // todo add normalize of r.d
                        result.add_string(format!(
                            "ihit = debug_intersect(transform({}, r));\n",
                            matrix.normal_name()
                        ));
                        result.add_string("if (nearer(i, ihit)) { i = ihit; }\n\n");
                    }
                    Flat { kind, is_inside: _ } => match kind {
                        Simple(matrix) => {
                            result.add_string(format!(
                                "hit = plane_intersect(r, {}, get_normal({}));\n",
                                matrix.inverse_name(),
                                matrix.normal_name()
                            ));
                            result.add_string(format!(
                                "if (nearer(i, hit)) {{ i = process_plane_intersection(i, hit, is_inside_{}(r.o + r.d * hit.t, hit.u, hit.v)); }}\n\n",
                                pos
                            ));
                        }
                        Portal(a, b) => {
                            let mut add = |matrix: &MatrixName, first, material| {
                                result.add_string(format!(
                                    "normal = {}get_normal({});\n",
                                    if first { "-" } else { "" },
                                    matrix.normal_name()
                                ));
                                result.add_string(format!(
                                    "hit = plane_intersect(r, {}, normal);\n",
                                    matrix.inverse_name()
                                ));
                                result.add_string(format!(
                                    "if (nearer(i, hit)) {{ i = process_portal_intersection(i, hit, is_inside_{}(r.o + r.d * hit.t, hit.u, hit.v, is_collinear(hit.n, normal), {}), {}); }}\n\n",
                                    pos, first, material
                                ));
                            };
                            add(a, true, format!("teleport_{}_1_M", pos));
                            add(b, false, format!("teleport_{}_2_M", pos));
                        }
                    },
                    Complex { kind, intersect: _ } => match kind {
                        Simple(matrix) => {
                            result.add_string(format!(
                                "transformed_ray = transform({}, r);\nlen = length(transformed_ray.d);\ntransformed_ray.d = normalize(transformed_ray.d);",
                                matrix.inverse_name()
                            ));
                            result.add_string(format!(
                                "ihit = intersect_{}(transformed_ray);\nihit.hit.t /= len;\n",
                                pos,
                            ));
                            result.add_string(format!(
                                "if (nearer(i, ihit)) {{ i = ihit; i.hit.n = normalize(({} * vec4(i.hit.n, 0.)).xyz); }}\n\n",
                                matrix.normal_name()
                            ));
                        }
                        Portal(a, b) => {
                            let mut add = |matrix: &MatrixName, first, material| {
                                result.add_string(format!(
                                    "transformed_ray = transform({}, r);\nlen = length(transformed_ray.d);\ntransformed_ray.d = normalize(transformed_ray.d);",
                                    matrix.inverse_name()
                                ));
                                result.add_string(format!(
                                    "ihit = intersect_{}(transformed_ray, {});\nihit.hit.t /= len;\n",
                                    pos, first
                                ));
                                result.add_string(format!(
                                    "if (nearer(i, ihit) && ihit.material != NOT_INSIDE) {{ if (ihit.material == TELEPORT) {{ ihit.material = {}; }} i = ihit; i.hit.n = normalize(({} * vec4(i.hit.n, 0.)).xyz); }}\n\n",
                                    material,
                                    matrix.normal_name()
                                ));
                            };
                            add(a, true, format!("teleport_{}_1_M", pos));
                            add(b, false, format!("teleport_{}_2_M", pos));
                        }
                    },
                }
                result.add_string("\n");
            }
            result
        });

        storages.insert("library".to_owned(), {
            let mut result = StringStorage::default();
            result.add_identifier_string("library".to_owned(), &self.library.0);
            result
        });

        apply_template(FRAGMENT_SHADER, storages)
    }

    pub fn get_new_material(
        &self,
    ) -> Result<
        macroquad::prelude::Material,
        (String, String, BTreeMap<String, Vec<(usize, String)>>),
    > {
        let code = self.generate_shader_code();

        dbg!(&code.line_numbers);

        use macroquad::prelude::load_material;
        use macroquad::prelude::MaterialParams;

        load_material(
            VERTEX_SHADER,
            &code.storage,
            MaterialParams {
                uniforms: self.uniforms(),
                ..Default::default()
            },
        )
        .map_err(|err| {
            if let macroquad::prelude::miniquad::graphics::ShaderError::CompilationError {
                error_message,
                ..
            } = err
            {
                let mut errors: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
                for x in shader_error_parser(&error_message) {
                    match x {
                        Ok((line_no, message)) => match code.line_numbers.get_identifier(line_no) {
                            Some((identifier, local_line_no)) => {
                                errors
                                    .entry(identifier.to_owned())
                                    .or_insert_with(|| Default::default())
                                    .push((local_line_no, message.to_owned()));
                            }
                            None => {
                                errors
                                    .entry("".to_owned())
                                    .or_insert_with(|| Default::default())
                                    .push((line_no, message.to_owned()));
                            }
                        },
                        Err(message) => {
                            errors
                                .entry("".to_owned())
                                .or_insert_with(|| Default::default())
                                .push((usize::MAX, message.to_owned()));
                        }
                    }
                }
                (code.storage, error_message, errors)
            } else {
                panic!(err);
            }
        })
    }
}

pub fn shader_error_parser(error: &str) -> Vec<Result<(usize, &str), &str>> {
    fn expect_str(input: &mut &str, to_expect: &str) -> Option<()> {
        if to_expect.chars().count() > input.chars().count() {
            return None;
        }

        if input.chars().zip(to_expect.chars()).any(|(a, b)| a != b) {
            return None;
        }

        *input = &input[to_expect.len()..];
        Some(())
    }

    fn expect_int(input: &mut &str) -> Option<usize> {
        let pos = input
            .char_indices()
            .take_while(|(_, c)| c.is_digit(10))
            .last()
            .map(|(i, c)| i + c.len_utf8())?;
        let lineno: usize = input[..pos].parse().ok()?;
        *input = &input[pos..];
        Some(lineno)
    }

    // Try parse format `0(270) : error C0000: syntax error, unexpected '}' at token "}"`
    // This format is noticed on native Linux
    fn try_parse_1(mut line: &str) -> Option<(usize, &str)> {
        expect_str(&mut line, "0(")?;
        let lineno = expect_int(&mut line)?;
        expect_str(&mut line, ") : error ")?;
        Some((lineno, line))
    }

    // Try parse format `ERROR: 0:586: 'pos' : redefinition`
    // This format is noticed on Firefox + Linux
    fn try_parse_2(mut line: &str) -> Option<(usize, &str)> {
        expect_str(&mut line, "ERROR: 0:")?;
        let lineno = expect_int(&mut line)?;
        expect_str(&mut line, ": ")?;
        Some((lineno, line))
    }

    error
        .split("\n")
        .map(|line| {
            if let Some(r) = try_parse_1(line) {
                Ok(r)
            } else if let Some(r) = try_parse_2(line) {
                Ok(r)
            } else {
                crate::miniquad::error!("can't parse line: `{}`", line);
                Err(line)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_error_parser_test() {
        let linux1 = r#"0(270) : error C0000: syntax error, unexpected '}' at token "}"
0(286) : error C1503: undefined variable "a"
0(286) : error C1503: undefined variable "n"
0(287) : error C1503: undefined variable "b"
0(287) : error C1503: undefined variable "n"
0(288) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(327) : error C1503: undefined variable "two_lines_nearest_points"
0(327) : error C1503: undefined variable "l"
0(327) : error C1503: undefined variable "r"
0(329) : error C1503: undefined variable "l"
0(329) : error C1503: undefined variable "l"
0(330) : error C1503: undefined variable "r"
0(330) : error C1503: undefined variable "r"
0(332) : error C1059: non constant expression in initialization
0(334) : error C0000: syntax error, unexpected reserved word "if" at token "if"
0(347) : error C1503: undefined variable "u"
0(348) : error C1503: undefined variable "u"
0(349) : error C1503: undefined variable "u"
0(350) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(359) : error C1503: undefined variable "u"
0(359) : error C1038: declaration of "b" conflicts with previous declaration at 0(347)
0(360) : error C1503: undefined variable "u"
0(360) : error C1038: declaration of "c" conflicts with previous declaration at 0(348)
0(361) : error C1503: undefined variable "u"
0(361) : error C1038: declaration of "d" conflicts with previous declaration at 0(349)
0(362) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(373) : error C0000: syntax error, unexpected '}' at token "}"
0(375) : error C0000: syntax error, unexpected '(', expecting "::" at token "("
0(378) : error C1503: undefined variable "mobius_step"
0(378) : error C1503: undefined variable "r"
0(379) : error C0000: syntax error, unexpected reserved word "for" at token "for"
0(433) : error C1503: undefined variable "op"
0(433) : error C1503: undefined variable "r"
0(433) : error C1038: declaration of "b" conflicts with previous declaration at 0(347)
0(434) : error C1503: undefined variable "op"
0(434) : error C1503: undefined variable "op"
0(435) : error C0000: syntax error, unexpected reserved word "return" at token "return"
0(550) : error C1503: undefined variable "is_inside_triangle"
0(555) : error C1503: undefined variable "is_inside_triangle"
0(631) : error C1503: undefined variable "process_plane_intersection"
0(635) : error C1503: undefined variable "process_plane_intersection"
0(639) : error C1503: undefined variable "process_plane_intersection"
0(643) : error C1503: undefined variable "process_plane_intersection"
0(647) : error C1503: undefined variable "process_plane_intersection"
0(651) : error C1503: undefined variable "process_plane_intersection"
0(655) : error C1503: undefined variable "process_plane_intersection"
0(659) : error C1503: undefined variable "process_plane_intersection"
0(664) : error C1503: undefined variable "process_portal_intersection"
0(668) : error C1503: undefined variable "process_portal_intersection"
0(673) : error C1503: undefined variable "process_portal_intersection"
0(677) : error C1503: undefined variable "process_portal_intersection"
0(680) : error C1503: undefined variable "a2_mat"
0(682) : error C1503: undefined variable "process_portal_intersection"
0(686) : error C1503: undefined variable "process_portal_intersection""#;
        assert!(shader_error_parser(linux1).iter().all(|x| x.is_ok()));
        let linux2 = r#"0(277) : error C1503: undefined variable "borer_m"
0(292) : error C0000: syntax error, unexpected '}', expecting ',' or ';' at token "}"
0(284) : error C1110: function "two_lines_nearest_points" has no return statement
0(295) : error C1115: unable to find compatible overloaded function "dot(mat3, vec3)"
0(299) : error C1102: incompatible type for parameter #1 ("a.84")"#;
        assert!(shader_error_parser(linux2).iter().all(|x| x.is_ok()));
        let web_linux = r#"ERROR: 0:565: 'pos' : redefinition
ERROR: 0:586: 'pos' : redefinition
ERROR: 0:606: 'pos' : redefinition
ERROR: 0:607: '<' : comparison operator only defined for scalars
ERROR: 0:607: '<' : wrong operand types - no operation '<' exists that takes a left-hand operand of type 'in highp 4-component vector of float' and a right operand of type 'const float' (or there is no acceptable conversion)
ERROR: 0:613: '<' : comparison operator only defined for scalars
ERROR: 0:613: '<' : wrong operand types - no operation '<' exists that takes a left-hand operand of type 'in highp 4-component vector of float' and a right operand of type 'const float' (or there is no acceptable conversion)"#;
        assert!(shader_error_parser(web_linux).iter().all(|x| x.is_ok()));
    }
}

const FRAGMENT_SHADER: &'static str = include_str!("frag.glsl");

const VERTEX_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;

varying lowp vec2 uv;
varying lowp vec2 uv_screen;

uniform mat4 Model;
uniform mat4 Projection;

uniform vec2 Center;
uniform vec2 _resolution;

void main() {
    vec4 res = Projection * Model * vec4(position, 1);

    uv_screen = (position.xy - _resolution/2.) / min(_resolution.x, _resolution.y) * 2.;
    uv_screen.y *= -1.;
    uv = texcoord;

    gl_Position = res;
}
";
