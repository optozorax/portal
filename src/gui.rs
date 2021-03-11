use crate::megaui::Ui;
use crate::megaui::*;
use glam::*;
use macroquad::prelude::Material;
use macroquad::prelude::UniformType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f32::consts::PI;

fn mymax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

pub fn ui_any_number(ui: &mut Ui, id: Id, label: &str, number: &mut f32) -> bool {
    let previous = *number;
    ui.slider(id, label, *number - 0.1..*number + 0.1, number);
    previous != *number
}

pub fn ui_any_vector(ui: &mut Ui, id: Id, number: &mut Vec3) -> bool {
    let mut is_changed = false;
    is_changed |= ui_any_number(ui, hash!(id, 0), "x", &mut number.x);
    is_changed |= ui_any_number(ui, hash!(id, 1), "y", &mut number.y);
    is_changed |= ui_any_number(ui, hash!(id, 2), "z", &mut number.z);
    is_changed
}

pub fn ui_angle(ui: &mut Ui, id: Id, label: &str, angle: &mut f32) -> bool {
    let mut current = *angle / PI;
    let previous = current;
    ui.slider(id, label, 0.0..2.0, &mut current);
    if previous != current {
        *angle = current * PI;
        true
    } else {
        false
    }
}

pub fn ui_positive_number(ui: &mut Ui, id: Id, label: &str, number: &mut f32) -> bool {
    let previous = *number;
    ui.slider(id, label, mymax(*number - 0.1, 0.0)..*number + 0.1, number);
    previous != *number
}

pub fn ui_bool(ui: &mut Ui, id: Id, label: &str, flag: &mut bool) -> bool {
    let previous = *flag;
    ui.checkbox(id, label, flag);
    previous != *flag
}

pub trait Uiable {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> bool;
}

pub trait ComboBoxChoosable {
    fn variants() -> &'static [&'static str];
    fn get_number(&self) -> usize;
    fn set_number(&mut self, number: usize);
}

pub fn ui_combo_box<T: ComboBoxChoosable + Uiable>(
    ui: &mut Ui,
    id: Id,
    label: &str,
    t: &mut T,
) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    ui.combo_box(hash!(id, 0), label, &T::variants(), Some(&mut current_type));

    if current_type != previous_type {
        t.set_number(current_type);
        is_changed = true;
    }

    is_changed |= t.ui(ui, hash!(id, 1));

    is_changed
}

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
        &["Multiplication", "Teleport", "Customizable"]
    }
    fn get_number(&self) -> usize {
        use Matrix::*;
        match self {
            Mul { .. } => 0,
            Teleport { .. } => 1,
            Simple { .. } => 2,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Matrix::*;
        *self = match number {
            0 => Mul {
                to: "id".to_owned(),
                what: "id".to_owned(),
            },
            1 => Teleport {
                first_portal: "id".to_owned(),
                second_portal: "id".to_owned(),

                what: "id".to_owned(),
            },
            2 => Self::default(),
            _ => unreachable!(),
        };
    }
}

pub trait UiWithNames {
    fn ui_with_names_inner(&mut self, ui: &mut Ui, id: Id, names: &[String]) -> bool;
}

impl UiWithNames for Matrix {
    fn ui_with_names_inner(&mut self, ui: &mut Ui, id: Id, names: &[String]) -> bool {
        use Matrix::*;
        let mut is_changed = false;
        match self {
            Mul { to, what } => {
                is_changed |= ui_existing_name(ui, hash!(id, 0), "Mul to", to, names);
                is_changed |= ui_existing_name(ui, hash!(id, 1), "What", what, names);
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                is_changed |=
                    ui_existing_name(ui, hash!(id, 2), "First portal", first_portal, names);
                is_changed |=
                    ui_existing_name(ui, hash!(id, 3), "Second portal", second_portal, names);
                is_changed |= ui_existing_name(ui, hash!(id, 4), "What", what, names);
            }
            Simple {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                is_changed |= ui_any_vector(ui, hash!(id, 5), offset);
                is_changed |= ui_positive_number(ui, hash!(id, 6), "Scale", scale);
                is_changed |= ui_angle(ui, hash!(id, 7), "Rotate X", &mut rotate.x);
                is_changed |= ui_angle(ui, hash!(id, 8), "Rotate Y", &mut rotate.y);
                is_changed |= ui_angle(ui, hash!(id, 9), "Rotate Z", &mut rotate.z);
                is_changed |= ui_bool(ui, hash!(id, 10), "Mirror X", &mut mirror.0);
                is_changed |= ui_bool(ui, hash!(id, 11), "Mirror Y", &mut mirror.1);
                is_changed |= ui_bool(ui, hash!(id, 12), "Mirror Z", &mut mirror.2);
            }
        }
        is_changed
    }
}

struct ObjectAndNames<'a, 'b, T>(&'a mut T, &'b [String]);

impl<'a, 'b, T: ComboBoxChoosable> ComboBoxChoosable for ObjectAndNames<'a, 'b, T> {
    fn variants() -> &'static [&'static str] {
        T::variants()
    }
    fn get_number(&self) -> usize {
        self.0.get_number()
    }
    fn set_number(&mut self, number: usize) {
        self.0.set_number(number);
    }
}

impl<'a, 'b, T: UiWithNames> Uiable for ObjectAndNames<'a, 'b, T> {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> bool {
        let ObjectAndNames(t, names) = &mut *self;
        UiWithNames::ui_with_names_inner(*t, ui, id, names)
    }
}

pub fn ui_existing_name(
    ui: &mut Ui,
    id: Id,
    label: &str,
    current: &mut String,
    names: &[String],
) -> bool {
    let previous = current.clone();
    ui.input_text(hash!(id, 0), label, current);
    if !names.contains(current) {
        ui.label(None, &format!("Error: name `{}` not found", current));
    }
    previous != *current
}

pub fn ui_name(ui: &mut Ui, id: Id, label: &str, names: &mut [String], pos: usize) -> bool {
    let previous = names[pos].clone();
    ui.input_text(hash!(id, 0), label, &mut names[pos]);
    if names[..pos].contains(&names[pos]) {
        ui.label(None, &format!("Error: name `{}` already used", names[pos]));
    }
    previous != names[pos]
}

pub trait StorageElem: Sized + Default {
    type GetType;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, f: F) -> Option<Self::GetType>;

    fn ui_with_names(&mut self, ui: &mut Ui, id: Id, names: &[String]) -> bool;

    fn defaults() -> (Vec<String>, Vec<Self>);
}

impl StorageElem for Matrix {
    type GetType = Mat4;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, mut f: F) -> Option<Self::GetType> {
        use Matrix::*;
        Some(match self {
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

    fn ui_with_names(&mut self, ui: &mut Ui, id: Id, names: &[String]) -> bool {
        ui_combo_box(ui, id, "Type", &mut ObjectAndNames(self, &names))
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (
            vec!["id".to_owned(), "portal1".to_owned()],
            vec![Matrix::default(), Matrix::default()],
        )
    }
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
        self.get_matrix_inner(name, &mut visited)
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

    fn get_matrix_inner<'a>(
        &'a self,
        name: &'a str,
        visited: &mut Vec<String>,
    ) -> Option<T::GetType> {
        if visited.iter().any(|x| *x == name) {
            return None;
        }

        visited.push(name.to_owned());
        let pos = self.names.iter().position(|x| x == name)?;
        let result = self.storage[pos].get(|name| self.get_matrix_inner(name, visited))?;
        visited.pop().unwrap();
        Some(result)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WhatChanged {
    pub inner: bool,
    pub outer: bool,
}

pub trait ComplexUiable {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> WhatChanged;
}

impl<T: StorageElem> ComplexUiable for StorageWithNames<T> {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut to_delete = None;
        if ui.button(None, "Add") {
            changed.outer = true;
            self.add("new".to_owned(), T::default());
        }
        let mut names = &mut self.names;
        for (pos, t) in self.storage.iter_mut().enumerate() {
            ui.tree_node(hash!(id, 0, pos), &names[pos].clone(), |ui| {
                if ui.button(None, "Delete") {
                    to_delete = Some(pos);
                }
                changed.outer |= ui_name(ui, hash!(id, 1, pos), "Name", &mut names, pos);
                changed.inner |= t.ui_with_names(ui, hash!(id, 2, pos), &names);
            });
        }
        if let Some(pos) = to_delete {
            changed.outer = true;
            self.remove(pos);
        }
        changed
    }
}

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

pub fn ui_editbox(ui: &mut Ui, id: Id, data: &mut String) -> bool {
    ui.editbox(id, Vector2::new(300., 200.), data)
}

impl StorageElem for MaterialCode {
    type GetType = MaterialCode;

    fn get<F: FnMut(&str) -> Option<Self::GetType>>(&self, _: F) -> Option<Self::GetType> {
        Some(self.clone())
    }

    fn ui_with_names(&mut self, ui: &mut Ui, id: Id, _: &[String]) -> bool {
        ui_editbox(ui, id, &mut self.0.0)
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["black".to_owned()], vec![MaterialCode::default()])
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

impl UiWithNames for Object {
    fn ui_with_names_inner(&mut self, ui: &mut Ui, id: Id, names: &[String]) -> bool {
        use Object::*;
        let mut is_changed = false;
        match self {
            Flat { plane, is_inside } => {
                is_changed |= ui_existing_name(ui, hash!(id, 0), "Plane", plane, names);
                is_changed |= ui_editbox(ui, hash!(id, 1), &mut is_inside.0.0);
            }
            FlatPortal {
                first,
                second,
                is_inside,
            } => {
                is_changed |= ui_existing_name(ui, hash!(id, 0), "First portal", first, names);
                is_changed |= ui_existing_name(ui, hash!(id, 1), "Second portal", second, names);
                is_changed |= ui_editbox(ui, hash!(id, 2), &mut is_inside.0.0);
            }
        }
        is_changed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    matrices: StorageWithNames<Matrix>,
    objects: Vec<Object>,

    materials: StorageWithNames<MaterialCode>,
    library: GlslCode,
}

impl ComplexUiable for Scene {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> WhatChanged {
        let mut changed = WhatChanged::default();
        if ui.button(None, "Save") {
            let s = serde_json::to_string(self).unwrap();
            std::fs::write("scene.json", s).unwrap();
        }
        ui.same_line(0.0);
        if ui.button(None, "Load") {
            let s = std::fs::read_to_string("scene.json").unwrap();
            *self = serde_json::from_str(&s).unwrap();
            changed.outer = true;
        }
        ui.tree_node(hash!(id, 0), "Matrices", |ui| {
            let WhatChanged { inner, outer } = self.matrices.ui(ui, hash!(id, 1));
            changed.inner |= inner;
            changed.outer |= outer;
        });
        ui.tree_node(hash!(id, 2), "Objects", |ui| {
            let mut to_delete = None;
            if ui.button(None, "Add object") {
                changed.outer = true;
                self.objects.push(Object::default());
            }
            let names = &self.matrices.names;
            for (pos, t) in self.objects.iter_mut().enumerate() {
                ui.tree_node(hash!(id, 3, pos), &pos.to_string(), |ui| {
                    if ui.button(None, "Delete") {
                        to_delete = Some(pos);
                    }
                    changed.outer |= ui_combo_box(
                        ui,
                        hash!(id, 4, pos),
                        "Type",
                        &mut ObjectAndNames(t, &names),
                    );
                });
            }
            if let Some(pos) = to_delete {
                changed.outer = true;
                self.objects.remove(pos);
            }
        });
        ui.tree_node(hash!(id, 5), "Materials", |ui| {
            let WhatChanged { inner, outer } = self.materials.ui(ui, hash!(id, 6));
            changed.outer |= inner;
            changed.outer |= outer;
        });
        ui.tree_node(hash!(id, 7), "Library", |ui| {
            changed.outer |= ui_editbox(ui, hash!(id, 7), &mut self.library.0);
        });
        changed
    }
}

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

impl Scene {
    pub fn new() -> Self {
        Self {
            matrices: Default::default(),
            objects: vec![Object::default()],
            materials: Default::default(),
            library: GlslCode("".to_owned()),
        }
    }

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
                    },
                    Object::FlatPortal {
                        first: _,
                        second: _,
                        is_inside,
                    } => {
                        result += &format!("int is_inside_{}(float x, float y, bool first, bool back) {{\n", pos);
                        result += &is_inside.0.0;
                        result += "\n}\n";
                    },
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

/*
    // ComplexPortal {
    //  name: String,
    //  crd: Matrix,
    //  intersect: GlslCode,
    //  disabled_teleportation_material: String,
    // },
}



enum Parameter {
    /// [0..1]
    Progress(f32),

    AnyFloat(f32),

    // [0..2] * PI
    Angle(f32),

    Boolean(bool),
}
*/
