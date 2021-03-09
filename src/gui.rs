use crate::megaui::Ui;
use crate::megaui::Vector2;
use crate::megaui::*;
use macroquad::prelude::*;
use macroquad_profiler as profiler;
use std::f32::consts::PI;

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

fn ui_any_number(ui: &mut Ui, hash: Id, name: &str, number: &mut f32) -> bool {
    let previous = *number;
    ui.slider(hash, name, *number - 0.1..*number + 0.1, number);
    previous != *number
}

fn ui_any_vector(ui: &mut Ui, hash: Id, number: &mut Vec3) -> bool {
    let mut is_changed = false;
    is_changed |= ui_any_number(ui, hash!(hash, 0), "x", &mut number.x);
    is_changed |= ui_any_number(ui, hash!(hash, 1), "y", &mut number.y);
    is_changed |= ui_any_number(ui, hash!(hash, 2), "z", &mut number.z);
    is_changed
}

fn ui_angle(ui: &mut Ui, hash: Id, name: &str, angle: &mut f32) -> bool {
    let mut current = *angle / (2. * PI);
    let previous = current;
    ui.slider(hash, name, 0.0..2.0, &mut current);
    if previous != current {
        *angle = current * 2. * PI;
        true
    } else {
        false
    }
}

fn ui_positive_number(ui: &mut Ui, hash: Id, name: &str, number: &mut f32) -> bool {
    let previous = *number;
    ui.slider(hash, name, mymax(*number - 0.1, 0.0)..*number + 0.1, number);
    previous != *number
}

pub trait ComboBoxChoosable {
    fn variants() -> &'static [&'static str];
    fn get_number(&self) -> usize;
    fn set_number(&mut self, number: usize);

    fn ui(&mut self, ui: &mut Ui, hash: Id) -> bool;
}

fn ui_combo_box<T: ComboBoxChoosable>(ui: &mut Ui, hash: Id, name: &str, t: &mut T) -> bool {
    let mut is_changed = false;

    let mut current_type = t.get_number();
    let previous_type = current_type;

    ui.combo_box(
        hash!(hash, 0),
        name,
        &T::variants(),
        Some(&mut current_type),
    );

    if current_type != previous_type {
        t.set_number(current_type);
        is_changed = true;
    }

    is_changed |= t.ui(ui, hash!(hash, 1));

    is_changed
}

// struct Matrix {
// 	name: String,
// 	inner: MatrixEnum,
// }

// enum MatrixEnum {
// 	Mul {
// 		to: String,

// 		what: String,
// 	},
// 	Teleport {
// 		first_portal: String,
// 		second_portal: String,

// 		what: String,
// 	},
// 	Simple {
// 		offset: Vec3,
// 		scale: f32,
// 		rotate: Vec3,
// 		mirror: (bool, bool, bool),
// 	},
// }

#[derive(Debug, Clone)]
pub enum Matrix {
    Mul(Vec<Matrix>),
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
    MirrorX,
    MirrorY,
    MirrorZ,
    Scale(f32),
    Offset(Vec3),
    // Teleported {
    //  portal_name: String,
    //  matrix: Box<Matrix>,
    // },
}

impl ComboBoxChoosable for Matrix {
    fn variants() -> &'static [&'static str] {
        &[
            "Multiplication",
            "Rotate X",
            "Rotate Y",
            "Rotate Z",
            "Mirror X",
            "Mirror Y",
            "Mirror Z",
            "Scale",
            "Offset",
        ]
    }
    fn get_number(&self) -> usize {
        use Matrix::*;
        match self {
            Mul(_) => 0,
            RotateX(_) => 1,
            RotateY(_) => 2,
            RotateZ(_) => 3,
            MirrorX => 4,
            MirrorY => 5,
            MirrorZ => 6,
            Scale(_) => 7,
            Offset(_) => 8,
        }
    }

    fn set_number(&mut self, number: usize) {
        use Matrix::*;
        *self = match number {
            0 => Mul(vec![]),
            1 => RotateX(0.),
            2 => RotateY(0.),
            3 => RotateZ(0.),
            4 => MirrorX,
            5 => MirrorY,
            6 => MirrorZ,
            7 => Scale(1.),
            8 => Offset(Vec3::default()),
            _ => panic!(),
        };
    }

    fn ui(&mut self, ui: &mut Ui, hash: Id) -> bool {
        use Matrix::*;
        match self {
            Mul(matrices) => {
                let mut is_changed = false;
                if ui.button(None, "Add") {
                    matrices.push(Self::Offset(Vec3::default()));
                    is_changed = true;
                }
                ui.same_line(0.0);
                if ui.button(None, "Remove") {
                    matrices.pop().unwrap();
                    is_changed = true;
                }
                for (pos, i) in matrices.iter_mut().enumerate() {
                    ui.tree_node(hash!(hash, 100, pos), &pos.to_string(), |ui| {
                        is_changed |= i.full_ui(ui, hash!(hash, 0, pos));
                    });
                }
                is_changed
            }
            RotateX(angle) => ui_angle(ui, hash!(hash, 1), "angle", angle),
            RotateY(angle) => ui_angle(ui, hash!(hash, 2), "angle", angle),
            RotateZ(angle) => ui_angle(ui, hash!(hash, 3), "angle", angle),
            MirrorX => false,
            MirrorY => false,
            MirrorZ => false,
            Scale(size) => ui_positive_number(ui, hash!(hash, 7), "size", size),
            Offset(vec) => ui_any_vector(ui, hash!(hash, 8), vec),
        }
    }
}

impl From<Matrix> for Mat4 {
    fn from(mat: Matrix) -> Mat4 {
        use Matrix::*;
        match mat {
            Mul(matrices) => matrices
                .into_iter()
                .map(|m| Mat4::from(m))
                .fold(Mat4::identity(), |acc, x| acc * x),
            RotateX(angle) => Mat4::from_rotation_x(angle),
            RotateY(angle) => Mat4::from_rotation_y(angle),
            RotateZ(angle) => Mat4::from_rotation_z(angle),
            MirrorX => Mat4::from_scale(Vec3::new(-1., 1., 1.)),
            MirrorY => Mat4::from_scale(Vec3::new(1., -1., 1.)),
            MirrorZ => Mat4::from_scale(Vec3::new(1., 1., -1.)),
            Scale(size) => Mat4::from_scale(Vec3::new(size, size, size)),
            Offset(vec) => Mat4::from_translation(vec),
        }
    }
}

impl Matrix {
    pub fn full_ui(&mut self, ui: &mut Ui, hash: Id) -> bool {
        ui_combo_box(ui, hash, "Type", self)
    }
}
