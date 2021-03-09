use crate::megaui::Ui;
use crate::megaui::*;
use std::f32::consts::PI;
use glam::*;
use serde::{Serialize, Deserialize};

/*
struct Scene {
    parameters: BTreeMap<String, Parameter>,
    matrices: BTreeMap<String, Matrix>,
    objects: Vec<Object>,
    materials: BTreeMap<String, Material>,
    functions: Vec<GlslCode>,
}

struct Object {
    Flat {
        plane: Matrix,
        is_inside: GlslCode,
    },
    FlatPortal {
        name: String,
        first: String,
        second: String,
        is_inside: GlslCode,
        disabled_teleportation_material: String,
    },
    // ComplexPortal {
    // 	name: String,
    // 	crd: Matrix,
    // 	intersect: GlslCode,
    // 	disabled_teleportation_material: String,
    // },
}

struct GlslCode(String);

struct Material {
    // Code must return vec3 at the end
    code: GlslCode,
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
    let mut current = *angle / (2. * PI);
    let previous = current;
    ui.slider(id, label, 0.0..2.0, &mut current);
    if previous != current {
        *angle = current * 2. * PI;
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

pub fn ui_combo_box<T: ComboBoxChoosable + Uiable>(ui: &mut Ui, id: Id, label: &str, t: &mut T) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    ui.combo_box(
        hash!(id, 0),
        label,
        &T::variants(),
        Some(&mut current_type),
    );

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
            Mul {..} => 0,
            Teleport {..} => 1,
            Simple {..} => 2,
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

struct MatrixAndNames<'a, 'b>(&'a mut Matrix, &'b [String]);

impl<'a, 'b> ComboBoxChoosable for MatrixAndNames<'a, 'b> {
    fn variants() -> &'static [&'static str] {
        Matrix::variants()
    }
    fn get_number(&self) -> usize {
        self.0.get_number()
    }
    fn set_number(&mut self, number: usize) {
        self.0.set_number(number);
    }
}

impl<'a, 'b> Uiable for MatrixAndNames<'a, 'b> {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> bool {
        use Matrix::*;
        let MatrixAndNames(matrix, names) = &mut *self;
        match matrix {
            Mul { to, what } => {
                let mut is_changed = false;
                is_changed |= ui_existing_name(ui, hash!(id, 0), "Mul to", to, names);
                is_changed |= ui_existing_name(ui, hash!(id, 1), "What", what, names);
                is_changed
            },
            Teleport { first_portal, second_portal, what } =>  {
                let mut is_changed = false;
                is_changed |= ui_existing_name(ui, hash!(id, 2), "First portal", first_portal, names);
                is_changed |= ui_existing_name(ui, hash!(id, 3), "Second portal", second_portal, names);
                is_changed |= ui_existing_name(ui, hash!(id, 4), "What", what, names);
                is_changed  
            },
            Simple { offset, scale, rotate, mirror } => {
                let mut is_changed = false;
                is_changed |= ui_any_vector(ui, hash!(id, 5), offset);
                is_changed |= ui_positive_number(ui, hash!(id, 6), "Scale", scale);
                is_changed |= ui_angle(ui, hash!(id, 7), "Rotate X", &mut rotate.x);
                is_changed |= ui_angle(ui, hash!(id, 8), "Rotate Y", &mut rotate.y);
                is_changed |= ui_angle(ui, hash!(id, 9), "Rotate Z", &mut rotate.z);
                is_changed |= ui_bool(ui, hash!(id, 10), "Mirror X", &mut mirror.0);
                ui.same_line(0.0);
                is_changed |= ui_bool(ui, hash!(id, 11), "Mirror Y", &mut mirror.1);
                ui.same_line(0.0);
                is_changed |= ui_bool(ui, hash!(id, 12), "Mirror Z", &mut mirror.2);
                is_changed
            },
        }
    }
}

pub fn ui_existing_name(ui: &mut Ui, id: Id, label: &str, current: &mut String, names: &[String]) -> bool {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrices {
    names: Vec<String>,
    matrices: Vec<Matrix>,
}

impl Default for Matrices {
    fn default() -> Self {
        Matrices {
            names: vec!["id".to_owned(), "portal1".to_owned()],
            matrices: vec![Matrix::default(), Matrix::default()],
        }
    }
}

impl Matrices {
    pub fn get_matrix(&self, name: &str) -> Option<Mat4> {
        let mut visited = vec![];
        self.get_matrix_inner(name, &mut visited)
    }

    pub fn add(&mut self, name: String, matrix: Matrix) {
        self.names.push(name);
        self.matrices.push(matrix);
    }

    pub fn remove(&mut self, pos: usize) {
        self.names.remove(pos);
        self.matrices.remove(pos);
    }

    fn get_matrix_inner<'a>(&'a self, name: &'a str, visited: &mut Vec<&'a str>) -> Option<Mat4> {
        if visited.iter().any(|x| *x == name) {
            return None;
        }

        visited.push(name);
        let pos = self.names.iter().position(|x| x == name)?;

        use Matrix::*;
        let result = match &self.matrices[pos] {
            Mul { to, what } => {
                let to = self.get_matrix(&to)?;
                let what = self.get_matrix(&what)?;
                what * to
            },
            Teleport { first_portal, second_portal, what } =>  {
                let first_portal = self.get_matrix(&first_portal)?;
                let second_portal = self.get_matrix(&second_portal)?;
                let what = self.get_matrix(&what)?;
                second_portal * first_portal.inverse() * what
            },
            Simple { offset, scale, rotate, mirror } => {
                let mut result = Mat4::identity();

                result = result *  Mat4::from_translation(*offset);

                result = result *  Mat4::from_scale(Vec3::new(*scale, *scale, *scale));

                result = result *  Mat4::from_rotation_x(rotate.x);
                result = result *  Mat4::from_rotation_y(rotate.y);
                result = result *  Mat4::from_rotation_z(rotate.z);

                if mirror.0 { result = result * Mat4::from_scale(Vec3::new(-1., 1., 1.)); }
                if mirror.1 { result = result * Mat4::from_scale(Vec3::new(1., -1., 1.)); }
                if mirror.2 { result = result * Mat4::from_scale(Vec3::new(1., 1., -1.)); }

                result
            },
        };
        visited.pop().unwrap();
        Some(result)
    }
}

impl Uiable for Matrices {
    fn ui(&mut self, ui: &mut Ui, id: Id) -> bool {
        let mut is_changed = false;
        let mut to_delete = None;
        if ui.button(None, "Add matrix") {
            is_changed = true;
            self.add("new_matrix".to_owned(), Matrix::default());
        }
        for (pos, matrix) in self.matrices.iter_mut().enumerate().skip(1) {
            ui.separator();
            if ui.button(None, "Delete") {
                to_delete = Some(pos);
            }
            is_changed |= ui_name(ui, hash!(id, 0, pos), "Name", &mut self.names, pos);
            is_changed |= ui_combo_box(ui, hash!(id, 1, pos), "Type", &mut MatrixAndNames(matrix, &self.names));
        }
        if let Some(pos) = to_delete {
            self.remove(pos);
        }
        is_changed
    }   
}