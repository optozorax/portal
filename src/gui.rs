use egui::*;
use glam::*;
use macroquad::prelude::Material;
use macroquad::prelude::UniformType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f32::consts::PI;
use std::sync::Arc;

fn mymax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
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

pub trait Eguiable {
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged;
}

pub fn egui_bool(ui: &mut Ui, flag: &mut bool) -> bool {
    let previous = *flag;
    ui.add(Checkbox::new(flag, ""));
    previous != *flag
}

pub fn egui_angle(ui: &mut Ui, angle: &mut f32) -> bool {
    let mut current = (*angle / PI * 180.) as i32;
    let previous = current;
    ui.add(
        DragValue::i32(&mut current)
            .speed(1)
            .suffix("°")
            .clamp_range(0.0..=360.0),
    );
    if previous != current {
        *angle = current as f32 * PI / 180.;
        true
    } else {
        false
    }
}

pub fn egui_f32(ui: &mut Ui, value: &mut f32) -> bool {
    let previous = *value;
    ui.add(
        DragValue::f32(value)
            .speed(0.01)
            .min_decimals(0)
            .max_decimals(2),
    );
    if previous != *value { true } else { false }
}

pub fn egui_f32_positive(ui: &mut Ui, value: &mut f32) -> bool {
    let previous = *value;
    ui.add(
        DragValue::f32(value)
            .speed(0.01)
            .prefix("×")
            .clamp_range(0.0..=1000.0)
            .min_decimals(0)
            .max_decimals(2),
    );
    if previous != *value { true } else { false }
}

pub fn egui_label(ui: &mut Ui, label: &str, size: f32) {
    ui.put(
        Rect::from_min_size(ui.min_rect().min, egui::vec2(size, 0.)),
        Label::new(label),
    );
}

pub fn egui_existing_name(
    ui: &mut Ui,
    label: &str,
    size: f32,
    current: &mut String,
    names: &[String],
) -> bool {
    let previous = current.clone();
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
    previous != *current
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
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged {
        use Matrix::*;
        let names = data.get::<Arc<Vec<String>>>();
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
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged {
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
            } => {
                let mut result = Mat4::IDENTITY;

                result = result * Mat4::from_translation(*offset);

                result = result * Mat4::from_scale(Vec3::new(*scale, *scale, *scale));

                result = result * Mat4::from_rotation_x(rotate.x);
                result = result * Mat4::from_rotation_y(rotate.y);
                result = result * Mat4::from_rotation_z(rotate.z);

                if mirror.0 {
                    result = result * Mat4::from_scale(Vec3::new(-1., 1., 1.));
                }
                if mirror.1 {
                    result = result * Mat4::from_scale(Vec3::new(1., -1., 1.));
                }
                if mirror.2 {
                    result = result * Mat4::from_scale(Vec3::new(1., 1., -1.));
                }

                result
            }
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
    fn errors(&self, data: &mut state::Container) -> usize;
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
    data: &mut state::Container,
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

                data.set(pos as u64);
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
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged {
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

                    data.set(pos as u64);
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
// Glsl code things
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlslCode(pub String);

// Code must return vec3 at the end
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCode(pub GlslCode);

impl Default for MaterialCode {
    fn default() -> Self {
        MaterialCode(GlslCode(
            "return plane_process_material(hit, r, color(0.2, 0.6, 0.6));".to_owned(),
        ))
    }
}

impl StorageElem for MaterialCode {
    type GetType = MaterialCode;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, _: F) -> Option<Self::GetType> {
        Some(self.clone())
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["black".to_owned()], vec![MaterialCode::default()])
    }
}

impl Eguiable for GlslCode {
    fn egui(&mut self, ui: &mut Ui, _: &mut state::Container) -> WhatChanged {
        WhatChanged::from_shader(ui.add(TextEdit::multiline(&mut self.0).text_style(TextStyle::Monospace)).changed())
    }
}

impl Eguiable for MaterialCode {
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged {
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

// ----------------------------------------------------------------------------------------------------------
// Scene object
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Object {
    Flat {
        plane: String,
        is_inside: IsInsideCode,
    },
    FlatPortal {
        first: String,
        second: String,
        is_inside: IsInsideCode,
    },
}

impl Default for Object {
    fn default() -> Self {
        Object::Flat {
            plane: "id".to_owned(),
            is_inside: Default::default(),
        }
    }
}

impl ComboBoxChoosable for Object {
    fn variants() -> &'static [&'static str] {
        &["Flat object", "Flat portal"]
    }
    fn get_number(&self) -> usize {
        use Object::*;
        match self {
            Flat { .. } => 0,
            FlatPortal { .. } => 1,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Object::*;
        *self = match number {
            0 => Flat {
                plane: "id".to_owned(),
                is_inside: Default::default(),
            },
            1 => FlatPortal {
                first: "id".to_owned(),
                second: "id".to_owned(),

                is_inside: Default::default(),
            },
            _ => unreachable!(),
        };
    }
}

impl Eguiable for Object {
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged {
        use Object::*;
        let names = data.get::<Arc<Vec<String>>>();
        let mut is_changed = WhatChanged::default();
        match self {
            Flat { plane, is_inside } => {
                is_changed.shader |= egui_existing_name(ui, "Plane:", 45., plane, names);
                is_changed |= is_inside.0.egui(ui, data);
            }
            FlatPortal {
                first,
                second,
                is_inside,
            } => {
                is_changed.shader |= egui_existing_name(ui, "First:", 45., first, names);
                is_changed.shader |= egui_existing_name(ui, "Second:", 45., second, names);
                is_changed |= is_inside.0.egui(ui, data);
            }
        }
        is_changed
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectComboBox(pub Object);

impl Eguiable for ObjectComboBox {
    fn egui(&mut self, ui: &mut Ui, data: &mut state::Container) -> WhatChanged {
        let mut changed =
            WhatChanged::from_uniform(egui_combo_box(ui, "Type:", 45., &mut self.0, *data.get()));
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
    objects: Vec<Object>,

    materials: StorageWithNames<MaterialCode>,
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
            objects: vec![Object::default()],
            materials: Default::default(),
            library: GlslCode("".to_owned()),
        }
    }

    pub fn egui(
        &mut self,
        ui: &mut Ui,
        should_recompile: &mut bool,
    ) -> (WhatChanged, Option<macroquad::material::Material>) {
        let mut changed = WhatChanged::default();
        let mut material = None;

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                let s = serde_json::to_string(self).unwrap();
                std::fs::write("scene.json", s).unwrap();
            }
            if ui.button("Load").clicked() {
                let s = std::fs::read_to_string("scene.json").unwrap();
                *self = serde_json::from_str(&s).unwrap();
                changed.shader = true;
            }
            if ui
                .add(Button::new("Recompile").enabled(*should_recompile))
                .clicked()
            {
                match self.get_new_material() {
                    Ok(m) => {
                        material = Some(m);
                        *should_recompile = false;
                        changed.uniform = true;
                    }
                    Err(err) => {
                        println!("code:\n{}\n\nmessage:\n{}", add_line_numbers(&err.0), err.1);
                    }
                }
            }
        });

        // other ui

        let mut data = state::Container::new();

        CollapsingHeader::new("Matrices")
            .default_open(false)
            .show(ui, |ui| {
                data.set(Arc::new(self.matrices.names.clone()));
                changed |= self.matrices.egui(ui, &mut data);
            });
        CollapsingHeader::new("Objects")
            .default_open(false)
            .show(ui, |ui| {
                data.set(Arc::new(self.matrices.names.clone()));
                changed |= egui_collection(ui, &mut self.objects, &mut data);
            });
        CollapsingHeader::new("Materials")
            .default_open(false)
            .show(ui, |ui| {
                changed |= self.materials.egui(ui, &mut data);
            });
        CollapsingHeader::new("Glsl Library")
            .default_open(false)
            .show(ui, |ui| {
                self.library.egui(ui, &mut data);
            });

        (changed, material)
    }
}

// ----------------------------------------------------------------------------------------------------------
// Uniforms
// ----------------------------------------------------------------------------------------------------------

pub trait UniformStruct {
    fn uniforms(&self) -> Vec<(String, UniformType)>;
    fn set_uniforms(&self, material: Material);
}

impl UniformStruct for Scene {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        self.objects
            .iter()
            .map(|object| match object {
                Object::Flat {
                    plane,
                    is_inside: _,
                } => vec![format!("{}_mat", plane), format!("{}_mat_inv", plane)].into_iter(),
                Object::FlatPortal {
                    first,
                    second,
                    is_inside: _,
                } => vec![
                    format!("{}_mat", first),
                    format!("{}_mat_inv", first),
                    format!("{}_mat", second),
                    format!("{}_mat_inv", second),
                    format!("{}_to_{}_mat_teleport", first, second),
                    format!("{}_to_{}_mat_teleport", second, first),
                ]
                .into_iter(),
            })
            .flatten()
            .map(|name| (name, UniformType::Mat4))
            .chain(
                vec![
                    ("_camera".to_owned(), UniformType::Mat4),
                    ("_resolution".to_owned(), UniformType::Float2),
                ]
                .into_iter(),
            )
            .collect()
    }

    fn set_uniforms(&self, material: Material) {
        for i in &self.objects {
            match i {
                Object::Flat {
                    plane,
                    is_inside: _,
                } => {
                    if let Some(m) = self.matrices.get(&plane) {
                        material.set_uniform(&format!("{}_mat_inv", plane), m.inverse());
                        material.set_uniform(&format!("{}_mat", plane), m);
                    }
                }
                Object::FlatPortal {
                    first,
                    second,
                    is_inside: _,
                } => {
                    if let Some((m1, m2)) =
                        self.matrices.get(&first).zip(self.matrices.get(&second))
                    {
                        material.set_uniform(
                            &format!("{}_to_{}_mat_teleport", first, second),
                            m2 * m1.inverse(),
                        );
                        material.set_uniform(
                            &format!("{}_to_{}_mat_teleport", second, first),
                            m1 * m2.inverse(),
                        );
                        material.set_uniform(&format!("{}_mat_inv", first), m1.inverse());
                        material.set_uniform(&format!("{}_mat", first), m1);
                        material.set_uniform(&format!("{}_mat_inv", second), m2.inverse());
                        material.set_uniform(&format!("{}_mat", second), m2);
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------
// Code generation
// ----------------------------------------------------------------------------------------------------------

impl Scene {
    pub fn generate_shader_code(&self) -> GlslCode {
        /*
           %%uniforms%%
           %%materials_defines%%
           %%intersection_functions%%
           %%intersections%%
           %%material_processing%%
        */

        let uniforms = {
            self.uniforms()
                .into_iter()
                .map(|x| x.0)
                .filter(|name| !name.starts_with("_"))
                .map(|name| format!("uniform mat4 {};", name))
                .collect::<Vec<_>>()
                .join("\n")
        };

        const MATERIAL_OFFSET: usize = 10;

        let materials = {
            let mut result = BTreeMap::new();
            for name in self.materials.names_iter() {
                result.insert(format!("{}_M", name), self.materials.get(&name).unwrap());
            }
            for (pos, first, second) in
                self.objects
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, x)| match x {
                        Object::Flat { .. } => None,
                        Object::FlatPortal { first, second, .. } => Some((pos, first, second)),
                    })
            {
                result.insert(
                    format!("teleport_{}_1_M", pos),
                    MaterialCode(GlslCode(format!(
                        "return teleport(i.hit.t, {}_to_{}_mat_teleport, r);",
                        first, second
                    ))),
                );
                result.insert(
                    format!("teleport_{}_2_M", pos),
                    MaterialCode(GlslCode(format!(
                        "return teleport(i.hit.t, {}_to_{}_mat_teleport, r);",
                        second, first
                    ))),
                );
            }
            result
        };

        let materials_defines = {
            materials
                .iter()
                .enumerate()
                .map(|(pos, (name, _))| format!("#define {} {}", name, pos + MATERIAL_OFFSET))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let intersection_functions = {
            let mut result = String::new();
            for (pos, i) in self.objects.iter().enumerate() {
                match i {
                    Object::Flat {
                        plane: _,
                        is_inside,
                    } => {
                        result += &format!("int is_inside_{}(float x, float y) {{\n", pos);
                        result += &is_inside.0.0;
                        result += "\n}\n";
                    }
                    Object::FlatPortal {
                        first: _,
                        second: _,
                        is_inside,
                    } => {
                        result += &format!(
                            "int is_inside_{}(float x, float y, bool first, bool back) {{\n",
                            pos
                        );
                        result += &is_inside.0.0;
                        result += "\n}\n";
                    }
                }
            }
            result
        };

        let intersections = {
            let mut result = String::new();
            for (pos, i) in self.objects.iter().enumerate() {
                match i {
                    Object::Flat {
                        plane,
                        is_inside: _,
                    } => {
                        result += &format!(
                            "hit = plane_intersect(r, {}_mat_inv, {}_mat[2].xyz);\n",
                            plane, plane
                        );
                        result += &format!(
                            "if (hit.hit && hit.t < i.hit.t) {{ inside = is_inside_{}(hit.u, hit.v); }}\n",
                            pos
                        );
                        result += "i = process_plane_intersection(i, hit, inside);\n";
                        result += "\n";
                        result += "\n";
                    }
                    Object::FlatPortal {
                        first,
                        second,
                        is_inside: _,
                    } => {
                        result += &format!(
                            "hit = plane_intersect(r, {}_mat_inv, {}_mat[2].xyz);\n",
                            first, first
                        );
                        result += &format!(
                            "if (hit.hit && hit.t < i.hit.t) {{ inside = is_inside_{}(hit.u, hit.v, true, !is_collinear(hit.n, {}_mat[2].xyz)); }}\n",
                            pos, first
                        );
                        result += &format!(
                            "i = process_portal_intersection(i, hit, inside, teleport_{}_1_M);\n",
                            pos
                        );
                        result += "\n";

                        result += &format!(
                            "hit = plane_intersect(r, {}_mat_inv, {}_mat[2].xyz);\n",
                            second, second
                        );
                        result += &format!(
                            "if (hit.hit && hit.t < i.hit.t) {{ inside = is_inside_{}(hit.u, hit.v, false, !is_collinear(hit.n, -{}_mat[2].xyz)); }}\n",
                            pos, second
                        );
                        result += &format!(
                            "i = process_portal_intersection(i, hit, inside, teleport_{}_2_M);\n",
                            pos,
                        );
                        result += "\n";
                        result += "\n";
                    }
                }
            }
            result
        };

        let material_processing = {
            let mut result = String::new();
            for (name, code) in &materials {
                result += &format!("}} else if (i.material == {}) {{\n", name);
                result += &code.0.0;
                result += "\n";
            }
            result
        };

        let mut result = FRAGMENT_SHADER.to_owned();
        result = result.replace("%%uniforms%%", &uniforms);
        result = result.replace("%%materials_defines%%", &materials_defines);
        result = result.replace("%%intersection_functions%%", &intersection_functions);
        result = result.replace("%%intersections%%", &intersections);
        result = result.replace("%%material_processing%%", &material_processing);
        GlslCode(result)
    }

    pub fn get_new_material(&self) -> Result<macroquad::prelude::Material, (String, String)> {
        let code = self.generate_shader_code();

        // println!("{}", code.0);

        use macroquad::prelude::load_material;
        use macroquad::prelude::MaterialParams;

        load_material(
            VERTEX_SHADER,
            &code.0,
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
                (code.0, error_message)
            } else {
                panic!(err);
            }
        })
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
