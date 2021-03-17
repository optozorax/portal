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
// Ugly data for UI. I decided to use such an ugly approach to store data for ui for fast development.
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Data {
    pub pos: usize,
    pub names: Vec<String>,
    pub to_export: Option<String>,
    pub errors: BTreeMap<ErrId, Vec<(usize, String)>>,
    pub matrix_recursion_error: BTreeMap<MatrixName, bool>,
    pub show_error_window: bool,
    pub show_glsl_library: bool,
    pub show_compiled_code: Option<String>,
    pub description_en_edit: bool,
    pub description_ru_edit: bool,

    pub formulas_cache: FormulasCache,
    pub formulas_names: Vec<String>,

    pub reload_textures: bool,
    pub texture_errors: BTreeMap<String, macroquad::file::FileError>,
}

impl Data {
    pub fn get_errors<'a, T: ErrorId>(
        &'a self,
        t: &T,
        pos: usize,
    ) -> Option<&'a [(usize, String)]> {
        let errors = &self.errors;
        let identifier = t.identifier(pos);
        let result = errors.get(&identifier)?;
        Some(&result[..])
    }
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

pub trait Eguiable {
    #[must_use]
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged;
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
                    current = 360. - (v as f32 % 360.);
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
    if previous != current {
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
                    current = 360. - (v % 360.);
                } else if v < 0. {
                    current = 360. + (v % 360.);
                } else {
                    current = v;
                }
            }
            current.into()
        })
        .speed(1)
        .suffix("°"),
    );
    if previous != current {
        *angle = *angle * PI / 180.;
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

pub fn egui_0_1(ui: &mut Ui, value: &mut f32) -> bool {
    check_changed(value, |value| {
        ui.add(
            Slider::f32(value, 0.0..=1.0)
                .clamp_to_range(true)
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

pub fn egui_with_red_field(ui: &mut Ui, has_errors: bool, f: impl FnOnce(&mut Ui)) {
    let previous = ui.visuals().clone();
    if has_errors {
        ui.visuals_mut().selection.stroke.color = Color32::RED;
        ui.visuals_mut().widgets.inactive.bg_stroke.color = Color32::from_rgb_additive(128, 0, 0);
        ui.visuals_mut().widgets.inactive.bg_stroke.width = 1.0;
        ui.visuals_mut().widgets.hovered.bg_stroke.color =
            Color32::from_rgb_additive(255, 128, 128);
    }
    f(ui);
    if has_errors {
        *ui.visuals_mut() = previous;
    }
}

pub fn egui_with_enabled_by(ui: &mut Ui, by: bool, f: impl FnOnce(&mut Ui)) {
    let previous = ui.enabled();
    ui.set_enabled(by);
    f(ui);
    ui.set_enabled(previous);
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
    id: usize,
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
// Gui named storage
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GetEnum<T> {
    Ok(T),
    NotFound,
    Recursion,
}

macro_rules! get_try {
    ($x:expr) => {
        match $x {
            GetEnum::Ok(t) => t,
            other => return other,
        }
    };
}

pub trait StorageElem: Sized + Default + Eguiable + ErrorsCount {
    type GetType;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        f: F,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<Self::GetType>;

    fn defaults() -> (Vec<String>, Vec<Self>);
}

// Checks if this name is used, sends name to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageWithNames<T> {
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
    pub fn get(
        &self,
        name: &str,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<T::GetType> {
        let mut visited = vec![];
        self.get_inner(name, &mut visited, uniforms, formulas_cache)
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

    pub fn iter(&self) -> std::iter::Zip<std::slice::Iter<String>, std::slice::Iter<T>> {
        self.names.iter().zip(self.storage.iter())
    }

    fn get_inner<'a>(
        &'a self,
        name: &'a str,
        visited: &mut Vec<String>,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<T::GetType> {
        if visited.iter().any(|x| *x == name) {
            return GetEnum::Recursion;
        }

        visited.push(name.to_owned());
        let pos = if let Some(pos) = self.names.iter().position(|x| x == name) {
            pos
        } else {
            return GetEnum::NotFound;
        };
        let result = get_try!(self.storage[pos].get(
            |name| self.get_inner(name, visited, uniforms, formulas_cache),
            uniforms,
            formulas_cache
        ));
        visited.pop().unwrap();
        GetEnum::Ok(result)
    }

    pub fn rich_egui(&mut self, ui: &mut Ui, data: &mut Data, name: &str) -> WhatChanged {
        use std::borrow::Cow;

        let errors_count = self.errors_count(0, data);
        let header = if errors_count > 0 {
            Cow::Owned(format!("{} ({} err)", name, errors_count))
        } else {
            Cow::Borrowed(name)
        };
        let mut changed = WhatChanged::default();
        CollapsingHeader::new(header)
            .id_source(name)
            .default_open(false)
            .show(ui, |ui| {
                changed |= self.egui(ui, data);
            });
        changed
    }
}

impl<T: StorageElem> Eguiable for StorageWithNames<T> {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut to_delete = None;
        let mut to_move_up = None;
        let mut to_move_down = None;
        let storage = &mut self.storage;
        let names = &mut self.names;
        let len = storage.len();
        for (pos, elem) in storage.iter_mut().enumerate() {
            data.pos = pos;

            let errors_count =
                elem.errors_count(pos, data) + names[..pos].contains(&names[pos]) as usize;
            CollapsingHeader::new(if errors_count > 0 {
                format!("{} ({} err)", names[pos], errors_count)
            } else {
                names[pos].to_owned()
            })
            .id_source(pos)
            .show(ui, |ui| {
                let previous = names[pos].clone();
                ui.horizontal(|ui| {
                    egui_label(ui, "Name:", 45.);
                    ui.put(
                        Rect::from_min_size(
                            ui.min_rect().min + egui::vec2(49., 0.),
                            egui::vec2(ui.available_width() - 120., 0.),
                        ),
                        TextEdit::singleline(&mut names[pos]),
                    );
                    if ui
                        .add(
                            Button::new("⏶")
                                .text_color(ui.visuals().hyperlink_color)
                                .enabled(pos != 0),
                        )
                        .clicked()
                    {
                        to_move_up = Some(pos);
                    }
                    if ui
                        .add(
                            Button::new("⏷")
                                .text_color(ui.visuals().hyperlink_color)
                                .enabled(pos + 1 != len),
                        )
                        .clicked()
                    {
                        to_move_down = Some(pos);
                    }
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

                changed |= elem.egui(ui, data);
            });
        }
        if let Some(pos) = to_delete {
            changed.shader = true;
            self.remove(pos);
        } else if let Some(pos) = to_move_up {
            self.storage.swap(pos, pos - 1);
            self.names.swap(pos, pos - 1);
        } else if let Some(pos) = to_move_down {
            self.storage.swap(pos, pos + 1);
            self.names.swap(pos, pos + 1);
        }
        if ui
            .add(Button::new("Add").text_color(Color32::GREEN))
            .clicked()
        {
            self.add(format!("_{}", self.names.len()), Default::default());
            changed.shader = true;
        }
        changed
    }
}

impl<T: StorageElem> ErrorsCount for StorageWithNames<T> {
    fn errors_count(&self, _: usize, data: &mut Data) -> usize {
        self.storage
            .iter()
            .enumerate()
            .map(|(pos, x)| {
                data.pos = pos;
                x.errors_count(pos, data) + self.names[..pos].contains(&self.names[pos]) as usize
            })
            .sum()
    }
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
    Parametrized {
        offset: TVec3<ParametrizeOrNot>,
        rotate: TVec3<ParametrizeOrNot>,
        mirror: TVec3<ParametrizeOrNot>,
        scale: ParametrizeOrNot,
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
        &["Simple", "Mul", "Teleport", "Param."]
    }
    fn get_number(&self) -> usize {
        use Matrix::*;
        match self {
            Simple { .. } => 0,
            Mul { .. } => 1,
            Teleport { .. } => 2,
            Parametrized { .. } => 3,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Matrix::*;
        *self = match number {
            0 => match self {
                Simple { .. } => self.clone(),
                Parametrized {
                    offset,
                    rotate,
                    mirror,
                    scale,
                } => Simple {
                    offset: Vec3::new(
                        offset.x.freeget().unwrap_or(0.0),
                        offset.y.freeget().unwrap_or(0.0),
                        offset.z.freeget().unwrap_or(0.0),
                    ),
                    rotate: Vec3::new(
                        rotate.x.freeget().unwrap_or(0.0),
                        rotate.y.freeget().unwrap_or(0.0),
                        rotate.z.freeget().unwrap_or(0.0),
                    ),
                    mirror: (
                        mirror.x.freeget().unwrap_or(0.0) == 1.0,
                        mirror.y.freeget().unwrap_or(0.0) == 1.0,
                        mirror.z.freeget().unwrap_or(0.0) == 1.0,
                    ),
                    scale: scale.freeget().unwrap_or(1.0),
                },
                _ => Self::default(),
            },
            1 => Mul {
                to: "id".to_owned(),
                what: "id".to_owned(),
            },
            2 => Teleport {
                first_portal: "id".to_owned(),
                second_portal: "id".to_owned(),

                what: "id".to_owned(),
            },
            3 => match self {
                Parametrized { .. } => self.clone(),
                Simple {
                    offset,
                    rotate,
                    mirror,
                    scale,
                } => Parametrized {
                    offset: TVec3 {
                        x: ParametrizeOrNot::No(offset.x),
                        y: ParametrizeOrNot::No(offset.y),
                        z: ParametrizeOrNot::No(offset.z),
                    },
                    rotate: TVec3 {
                        x: ParametrizeOrNot::No(rotate.x),
                        y: ParametrizeOrNot::No(rotate.y),
                        z: ParametrizeOrNot::No(rotate.z),
                    },
                    mirror: TVec3 {
                        x: ParametrizeOrNot::No(mirror.0 as i32 as f32),
                        y: ParametrizeOrNot::No(mirror.1 as i32 as f32),
                        z: ParametrizeOrNot::No(mirror.2 as i32 as f32),
                    },
                    scale: ParametrizeOrNot::No(*scale),
                },
                _ => Parametrized {
                    offset: TVec3 {
                        x: ParametrizeOrNot::No(0.),
                        y: ParametrizeOrNot::No(0.),
                        z: ParametrizeOrNot::No(0.),
                    },
                    rotate: TVec3 {
                        x: ParametrizeOrNot::No(0.),
                        y: ParametrizeOrNot::No(0.),
                        z: ParametrizeOrNot::No(0.),
                    },
                    mirror: TVec3 {
                        x: ParametrizeOrNot::No(0.),
                        y: ParametrizeOrNot::No(0.),
                        z: ParametrizeOrNot::No(0.),
                    },
                    scale: ParametrizeOrNot::No(1.),
                },
            },
            _ => unreachable!(),
        };
    }
}

impl Eguiable for Matrix {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use Matrix::*;
        let mut is_changed = false;
        let mut errors_count = 0;
        match self {
            Mul { to, what } => {
                let names = &data.names;
                is_changed |= egui_existing_name(ui, "Mul to:", 45., to, names, &mut errors_count);
                is_changed |= egui_existing_name(ui, "What:", 45., what, names, &mut errors_count);
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let names = &data.names;
                is_changed |=
                    egui_existing_name(ui, "From:", 45., first_portal, names, &mut errors_count);
                is_changed |=
                    egui_existing_name(ui, "To:", 45., second_portal, names, &mut errors_count);
                is_changed |= egui_existing_name(ui, "What:", 45., what, names, &mut errors_count);
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
            Parametrized {
                offset,
                rotate,
                mirror,
                scale,
            } => {
                ui.label("Offset: ");
                is_changed |= offset.x.egui(ui, data, "X", 0.0, |ui, x| egui_f32(ui, x));
                is_changed |= offset.y.egui(ui, data, "Y", 0.0, |ui, x| egui_f32(ui, x));
                is_changed |= offset.z.egui(ui, data, "Z", 0.0, |ui, x| egui_f32(ui, x));
                ui.separator();
                ui.label("Rotate: ");
                is_changed |= rotate.x.egui(ui, data, "X", 0.0, |ui, x| egui_angle(ui, x));
                is_changed |= rotate.y.egui(ui, data, "Y", 0.0, |ui, x| egui_angle(ui, x));
                is_changed |= rotate.z.egui(ui, data, "Z", 0.0, |ui, x| egui_angle(ui, x));
                ui.separator();
                ui.label("Mirror: ");
                is_changed |= mirror.x.egui(ui, data, "X", 0.0, |ui, x| egui_0_1(ui, x));
                is_changed |= mirror.y.egui(ui, data, "Y", 0.0, |ui, x| egui_0_1(ui, x));
                is_changed |= mirror.z.egui(ui, data, "Z", 0.0, |ui, x| egui_0_1(ui, x));
                ui.separator();
                is_changed |= scale.egui(ui, data, "Scale:", 1.0, |ui, x| egui_f32_positive(ui, x));
            }
        }
        if data
            .matrix_recursion_error
            .get(&MatrixName(data.names[data.pos].clone()))
            .copied()
            .unwrap_or(false)
        {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error: ").text_color(Color32::RED));
                ui.label("this matrix has recursion");
            });
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

impl ErrorsCount for MatrixComboBox {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        self.0.errors_count(pos, data)
    }
}

impl StorageElem for MatrixComboBox {
    type GetType = Mat4;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        mut f: F,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        use Matrix::*;
        GetEnum::Ok(match &self.0 {
            Mul { to, what } => {
                let to = get_try!(f(&to));
                let what = get_try!(f(&what));
                what * to
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let first_portal = get_try!(f(&first_portal));
                let second_portal = get_try!(f(&second_portal));
                let what = get_try!(f(&what));
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
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                let scale = scale.get(uniforms, formulas_cache) as f32;
                Mat4::from_scale_rotation_translation(
                    Vec3::new(
                        scale * (1. - 2.0 * mirror.x.get(uniforms, formulas_cache) as f32),
                        scale * (1. - 2.0 * mirror.y.get(uniforms, formulas_cache) as f32),
                        scale * (1. - 2.0 * mirror.z.get(uniforms, formulas_cache) as f32),
                    ),
                    Quat::from_rotation_x(rotate.x.get(uniforms, formulas_cache) as f32)
                        * Quat::from_rotation_y(rotate.y.get(uniforms, formulas_cache) as f32)
                        * Quat::from_rotation_z(rotate.z.get(uniforms, formulas_cache) as f32),
                    Vec3::new(
                        offset.x.get(uniforms, formulas_cache) as f32,
                        offset.y.get(uniforms, formulas_cache) as f32,
                        offset.z.get(uniforms, formulas_cache) as f32,
                    ),
                )
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

impl ErrorsCount for Matrix {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        use Matrix::*;
        let mut errors_count = 0;
        match self {
            Mul { to, what } => {
                let names = &data.names;
                if !names.contains(to) {
                    errors_count += 1;
                }
                if !names.contains(what) {
                    errors_count += 1;
                }
            }
            Teleport {
                first_portal,
                second_portal,
                what,
            } => {
                let names = &data.names;
                if !names.contains(first_portal) {
                    errors_count += 1;
                }
                if !names.contains(second_portal) {
                    errors_count += 1;
                }
                if !names.contains(what) {
                    errors_count += 1;
                }
            }
            Simple { .. } => {}
            Parametrized {
                offset,
                scale,
                rotate,
                mirror,
            } => {
                errors_count += offset.x.errors_count(pos, data)
                    + offset.y.errors_count(pos, data)
                    + offset.z.errors_count(pos, data);
                errors_count += rotate.x.errors_count(pos, data)
                    + rotate.y.errors_count(pos, data)
                    + rotate.z.errors_count(pos, data);
                errors_count += mirror.x.errors_count(pos, data)
                    + mirror.y.errors_count(pos, data)
                    + mirror.z.errors_count(pos, data);
                errors_count += scale.errors_count(pos, data);
            }
        }
        if data
            .matrix_recursion_error
            .get(&MatrixName(data.names[data.pos].clone()))
            .copied()
            .unwrap_or(false)
        {
            errors_count += 1;
        }
        errors_count
    }
}

// ----------------------------------------------------------------------------------------------------------
// Errors handling
// ----------------------------------------------------------------------------------------------------------

pub trait ErrorsCount {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct ErrId(pub usize);

// Used to find errors source
pub trait ErrorId {
    fn identifier(&self, pos: usize) -> ErrId;
}

impl ErrorId for Material {
    fn identifier(&self, pos: usize) -> ErrId {
        ErrId(1000 + pos)
    }
}

impl ErrorId for Object {
    fn identifier(&self, pos: usize) -> ErrId {
        ErrId(2000 + pos)
    }
}

impl ErrorId for LibraryCode {
    fn identifier(&self, pos: usize) -> ErrId {
        ErrId(3000 + pos)
    }
}

impl ErrorId for Matrix {
    fn identifier(&self, pos: usize) -> ErrId {
        ErrId(4000 + pos)
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

impl ErrorsCount for Material {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        if let Some(local_errors) = data.get_errors(self, pos) {
            local_errors.len()
        } else {
            0
        }
    }
}

impl ErrorsCount for MaterialComboBox {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        self.0.errors_count(pos, data)
    }
}

impl StorageElem for MaterialComboBox {
    type GetType = Material;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.0.clone())
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
        let has_errors = data.get_errors(&*self, data.pos).is_some();
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

                egui_with_red_field(ui, has_errors, |ui| {
                    changed |= code.0.egui(ui, data).shader;
                });
                ui.add(Label::new("}").monospace());

                if let Some(local_errors) = data.get_errors(self, data.pos) {
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
        let mut changed =
            WhatChanged::from_shader(egui_combo_box(ui, "Type:", 45., &mut self.0, data.pos));
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlslCode(pub String);

impl ErrorsCount for LibraryCode {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        if let Some(local_errors) = data.get_errors(self, pos) {
            local_errors.len()
        } else {
            0
        }
    }
}

impl StorageElem for LibraryCode {
    type GetType = LibraryCode;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.clone())
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["my functions".to_owned()], vec![Default::default()])
    }
}

impl Default for MaterialCode {
    fn default() -> Self {
        MaterialCode(GlslCode(
            "return material_simple(hit, r, vec3(9.21e-2, 7.28e-1, 6.81e-2), 5e-1, true, 4e0, 3e-1);".to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LibraryCode(GlslCode);

impl Eguiable for GlslCode {
    fn egui(&mut self, ui: &mut Ui, _: &mut Data) -> WhatChanged {
        WhatChanged::from_shader(
            ui.add(TextEdit::multiline(&mut self.0).text_style(TextStyle::Monospace))
                .changed(),
        )
    }
}

impl Eguiable for LibraryCode {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::default();
        egui_with_red_field(ui, data.get_errors(self, data.pos).is_some(), |ui| {
            changed = WhatChanged::from_shader(
                ui.add(TextEdit::multiline(&mut self.0.0).text_style(TextStyle::Monospace))
                    .changed(),
            );
            if let Some(local_errors) = data.get_errors(self, data.pos) {
                egui_errors(ui, local_errors);
            }
        });
        changed
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
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
        let names = &data.names;
        let mut is_changed = false;
        let mut errors_count = 0;
        match self {
            Simple(a) => {
                is_changed |=
                    egui_existing_name(ui, "Matrix:", 45., &mut a.0, names, &mut errors_count)
            }
            Portal(a, b) => {
                is_changed |=
                    egui_existing_name(ui, "First:", 45., &mut a.0, names, &mut errors_count);
                is_changed |=
                    egui_existing_name(ui, "Second:", 45., &mut b.0, names, &mut errors_count);
            }
        }
        WhatChanged::from_shader(is_changed)
    }
}

impl Eguiable for Object {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use Object::*;
        let names = &data.names;
        let mut is_changed = WhatChanged::default();
        let has_errors = data.get_errors(self, data.pos).is_some();
        let mut errors_count = 0;
        match self {
            DebugMatrix(a) => {
                is_changed.shader |=
                    egui_existing_name(ui, "Matrix:", 45., &mut a.0, names, &mut errors_count);
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
                egui_with_red_field(ui, has_errors, |ui| {
                    is_changed |= is_inside.0.egui(ui, data);
                });
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = data.get_errors(self, data.pos) {
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
                egui_with_red_field(ui, has_errors, |ui| {
                    is_changed |= intersect.0.egui(ui, data);
                });
                ui.add(Label::new("}").monospace());
                if let Some(local_errors) = data.get_errors(self, data.pos) {
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

impl ErrorsCount for ObjectType {
    fn errors_count(&self, _: usize, data: &mut Data) -> usize {
        let mut result = 0;

        use ObjectType::*;
        let names = &data.names;
        match self {
            Simple(a) => {
                if !names.contains(&a.0) {
                    result += 1;
                }
            }
            Portal(a, b) => {
                if !names.contains(&a.0) {
                    result += 1;
                }
                if !names.contains(&b.0) {
                    result += 1;
                }
            }
        }

        result
    }
}

impl ErrorsCount for Object {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        let mut result = if let Some(local_errors) = data.get_errors(self, pos) {
            local_errors.len()
        } else {
            0
        };

        use Object::*;
        let names = &data.names;
        match self {
            DebugMatrix(a) => {
                if !names.contains(&a.0) {
                    result += 1;
                }
            }
            Flat { kind, is_inside: _ } => {
                result += kind.errors_count(pos, data);
            }
            Complex { kind, intersect: _ } => {
                result += kind.errors_count(pos, data);
            }
        }

        result
    }
}

impl ErrorsCount for ObjectComboBox {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        self.0.errors_count(pos, data)
    }
}

impl StorageElem for ObjectComboBox {
    type GetType = Object;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.0.clone())
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["my object".to_owned()], vec![Default::default()])
    }
}

// ----------------------------------------------------------------------------------------------------------
// Texture
// ----------------------------------------------------------------------------------------------------------

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

impl ErrorsCount for TextureName {
    fn errors_count(&self, _: usize, data: &mut Data) -> usize {
        data.texture_errors.get(&self.0).is_some() as usize
    }
}

impl Eguiable for TextureName {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let result = WhatChanged::from_shader(check_changed(&mut self.0, |text| drop(ui.text_edit_singleline(text))));

        if let Some(err) = data.texture_errors.get(&self.0) {
             ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error:").text_color(Color32::RED));
                ui.label(format!("error while loading file: {:?}", err));
            });
        } 

        result
    }
}

impl StorageElem for TextureName {
    type GetType = TextureName;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.clone())
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["texture".to_owned()], vec![Default::default()])
    }
}

// ----------------------------------------------------------------------------------------------------------
// Scene
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    description_en: String,
    description_ru: String,

    pub uniforms: StorageWithNames<AnyUniformComboBox>,

    pub matrices: StorageWithNames<MatrixComboBox>,
    objects: StorageWithNames<ObjectComboBox>,

    pub textures: StorageWithNames<TextureName>,

    materials: StorageWithNames<MaterialComboBox>,
    library: StorageWithNames<LibraryCode>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OldScene {
    description_en: String,
    description_ru: String,

    pub uniforms: StorageWithNames<AnyUniformComboBox>,

    pub matrices: StorageWithNames<MatrixComboBox>,
    objects: StorageWithNames<ObjectComboBox>,

    materials: StorageWithNames<MaterialComboBox>,
    library: StorageWithNames<LibraryCode>,
}

impl From<OldScene> for Scene {
    fn from(old: OldScene) -> Scene {
        Scene {
            description_en: old.description_en,
            description_ru: old.description_ru,

            uniforms: old.uniforms,

            matrices: old.matrices,
            objects: old.objects,

            textures: Default::default(),

            materials: old.materials,
            library: old.library,
        }
    }
}

pub fn add_line_numbers(s: &str) -> String {
    s.split("\n")
        .enumerate()
        .map(|(line, text)| format!("{}|{}", line + 1, text))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Scene {
    pub fn new(data: &mut Data) -> Self {
        let mut result = Self {
            description_en: Default::default(),
            description_ru: Default::default(),
            uniforms: Default::default(),
            matrices: Default::default(),
            objects: Default::default(),
            materials: Default::default(),
            textures: Default::default(),
            library: Default::default(),
        };
        result.init(data);
        result
    }

    pub fn init(&mut self, data: &mut Data) {
        for (_, object) in self.uniforms.iter() {
            if let AnyUniform::Formula(f) = &object.0 {
                data.formulas_cache.compile(&f.0);
            }
        }
        data.errors = Default::default();
        data.show_error_window = false;
        data.names = self.matrices.names.clone();
        data.formulas_names = self.uniforms.names.clone();
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
                (String, String, BTreeMap<ErrId, Vec<(usize, String)>>),
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
            if ui
                .add(Button::new("Recompile").enabled(*should_recompile))
                .clicked()
            {
                match self.get_new_material() {
                    Ok(m) => {
                        data.reload_textures = true;
                        material = Some(Ok(m));
                        *should_recompile = false;
                        changed.uniform = true;
                        data.errors = Default::default();
                        data.show_error_window = false;
                    }
                    Err(err) => {
                        material = Some(Err(err));
                    }
                }
            }
        });

        ui.separator();

        // other ui

        CollapsingHeader::new("Description")
            .default_open(false)
            .show(ui, |ui| {
                CollapsingHeader::new("English")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut data.description_en_edit, false, "View");
                            ui.selectable_value(&mut data.description_en_edit, true, "Edit");
                        });
                        if data.description_en_edit {
                            ui.add(
                                TextEdit::multiline(&mut self.description_en)
                                    .text_style(TextStyle::Monospace),
                            );
                        } else {
                            egui::experimental::easy_mark(ui, &self.description_en);
                        }
                    });
                CollapsingHeader::new("Яussiaи")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut data.description_ru_edit, false, "View");
                            ui.selectable_value(&mut data.description_ru_edit, true, "Edit");
                        });
                        if data.description_ru_edit {
                            ui.add(
                                TextEdit::multiline(&mut self.description_ru)
                                    .text_style(TextStyle::Monospace),
                            );
                        } else {
                            egui::experimental::easy_mark(ui, &self.description_ru);
                        }
                    });
            });

        changed |= self.uniforms.rich_egui(ui, data, "Uniforms");

        ui.collapsing("Calculated uniforms", |ui| {
            for name in self.uniforms.names_iter() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.;
                    ui.label(format!("{} = ", name));
                    use AnyUniformResult::*;
                    match self
                        .uniforms
                        .get(name, &self.uniforms, &data.formulas_cache)
                    {
                        GetEnum::Ok(x) => match x {
                            Bool(b) => ui.label(b.to_string()),
                            Int(b) => ui.label(b.to_string()),
                            Float(b) => ui.label(b.to_string()),
                        },
                        GetEnum::NotFound => ui.label("NotFound"),
                        GetEnum::Recursion => ui.label("Recursion"),
                    }
                });
            }
        });

        data.names = self.matrices.names.clone();
        data.formulas_names = self.uniforms.names.clone();

        changed |= self.matrices.rich_egui(ui, data, "Matrices");
        changed |= self.objects.rich_egui(ui, data, "Objects");
        changed |= self.materials.rich_egui(ui, data, "Materials");
        changed |= self.textures.rich_egui(ui, data, "Textures");
        changed |= self.library.rich_egui(ui, data, "User GLSL code");

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("View GLSL library").clicked() {
                data.show_glsl_library = true;
            }
            if ui.button("View generated GLSL code").clicked() {
                data.show_compiled_code = Some(self.generate_shader_code().storage);
            }
        });

         if let Some(local_errors) = data.errors.get(&ErrId::default()).cloned() {
            ui.separator();
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

impl ErrorsCount for Scene {
    fn errors_count(&self, _: usize, data: &mut Data) -> usize {
        self.matrices.errors_count(0, data)
            + self.objects.errors_count(0, data)
            + self.materials.errors_count(0, data)
            + self.library.errors_count(0, data)
            + if let Some(local_errors) = data.errors.get(&ErrId::default()).cloned() {
                local_errors.len()
            } else {
                0
            }
    }
}

// ----------------------------------------------------------------------------------------------------------
// Uniforms
// ----------------------------------------------------------------------------------------------------------

pub trait UniformStruct {
    fn uniforms(&self) -> Vec<(String, UniformType)>;
    fn set_uniforms(&self, material: macroquad::material::Material);
}

impl Scene {
    pub fn textures(&self) -> Vec<String> {
        self.textures.names_iter().cloned().map(|x| TextureName::name(&x)).collect()
    }

    pub fn uniforms(&self) -> Vec<(String, UniformType)> {
        use Object::*;
        use ObjectType::*;

        let mut result = Vec::new();
        for (_, object) in self.objects.iter() {
            match &object.0 {
                DebugMatrix(matrix) => {
                    result.push(matrix.normal_name());
                    result.push(matrix.inverse_name());
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

        for (name, uniform) in self.uniforms.iter() {
            let name = format!("{}_u", name);
            match uniform.0 {
                AnyUniform::Bool(_) => result.push((name, UniformType::Int1)),
                AnyUniform::Int { .. } => result.push((name, UniformType::Int1)),
                AnyUniform::Float { .. } => result.push((name, UniformType::Float1)),
                AnyUniform::Angle { .. } => result.push((name, UniformType::Float1)),
                AnyUniform::Formula(_) => result.push((name, UniformType::Float1)),
            }
        }

        result.extend(vec![
            ("_camera".to_owned(), UniformType::Mat4),
            ("_resolution".to_owned(), UniformType::Float2),
            ("_ray_tracing_depth".to_owned(), UniformType::Int1),
            ("_offset_after_material".to_owned(), UniformType::Float1),
            ("_view_angle".to_owned(), UniformType::Float1),
            ("_use_panini_projection".to_owned(), UniformType::Int1),
            ("_panini_param".to_owned(), UniformType::Float1),
        ]);

        result
    }

    pub fn set_uniforms(
        &self,
        material: macroquad::material::Material,
        data: &mut Data,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
    ) {
        data.matrix_recursion_error.clear();
        macro_rules! local_try {
            ($a:expr, $c:ident, $b: expr) => {
                match self.matrices.get(&$a.0, uniforms, &data.formulas_cache) {
                    GetEnum::Ok($c) => {
                        *data
                            .matrix_recursion_error
                            .entry($a.clone())
                            .or_insert(false) = false;
                        $b
                    }
                    GetEnum::Recursion => {
                        *data
                            .matrix_recursion_error
                            .entry($a.clone())
                            .or_insert(false) = true;
                    }
                    _ => {}
                }
            };
        }
        use Object::*;
        use ObjectType::*;
        for (_, object) in self.objects.iter() {
            match &object.0 {
                DebugMatrix(matrix) => {
                    local_try!(matrix, m, {
                        material.set_uniform(&matrix.normal_name(), m);
                        material.set_uniform(&matrix.inverse_name(), m.inverse());
                    })
                }
                Flat { kind, is_inside: _ } | Complex { kind, intersect: _ } => match kind {
                    Simple(matrix) => {
                        local_try!(matrix, m, {
                            material.set_uniform(&matrix.normal_name(), m);
                            material.set_uniform(&matrix.inverse_name(), m.inverse());
                        })
                    }
                    Portal(a, b) => {
                        local_try!(a, ma, {
                            local_try!(b, mb, {
                                material.set_uniform(&a.normal_name(), ma);
                                material.set_uniform(&a.inverse_name(), ma.inverse());
                                material.set_uniform(&b.normal_name(), mb);
                                material.set_uniform(&b.inverse_name(), mb.inverse());
                                material.set_uniform(&a.teleport_to_name(b), mb * ma.inverse());
                                if a != b {
                                    material.set_uniform(&b.teleport_to_name(a), ma * mb.inverse());
                                }
                            })
                        })
                    }
                },
            }
        }

        for name in self.uniforms.names_iter() {
            let name_u = format!("{}_u", name);
            match self.uniforms.get(&name, uniforms, &data.formulas_cache) {
                GetEnum::Ok(result) => match result {
                    AnyUniformResult::Bool(b) => material.set_uniform(&name_u, b as i32),
                    AnyUniformResult::Int(i) => material.set_uniform(&name_u, i),
                    AnyUniformResult::Float(f) => material.set_uniform(&name_u, f as f32),
                },
                _ => {
                    println!("Error getting `{}` uniform", name);
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------
// Code generation
// ----------------------------------------------------------------------------------------------------------

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct LineNumbersByKey(pub BTreeMap<ErrId, Range<usize>>);

impl LineNumbersByKey {
    pub fn offset(&mut self, lines: usize) {
        self.0
            .iter_mut()
            .for_each(|(_, line)| *line = line.start + lines..line.end + lines);
    }

    pub fn add(&mut self, identifier: ErrId, lines: Range<usize>) {
        assert!(self.0.get(&identifier).is_none());
        self.0.insert(identifier, lines);
    }

    pub fn extend(&mut self, other: LineNumbersByKey) {
        for (k, v) in other.0 {
            self.add(k, v);
        }
    }

    // Returns identifier and local line position
    pub fn get_identifier(&self, line_no: usize) -> Option<(ErrId, usize)> {
        self.0
            .iter()
            .find(|(_, range)| range.contains(&line_no))
            .map(|(id, range)| (*id, line_no - range.start + 1))
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

    pub fn add_identifier_string<T: AsRef<str>>(&mut self, identifier: ErrId, s: T) {
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
        .split("//%")
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
        s1.add_identifier_string(ErrId(1), "\n4\n5\n");

        assert_eq!(
            s1,
            StringStorage {
                storage: "1\n2\n3\n\n4\n5\n".to_owned(),
                current_line_no: 7,
                line_numbers: LineNumbersByKey(vec![(ErrId(1), 4..8)].into_iter().collect()),
            }
        );

        let mut s2 = StringStorage::default();
        s2.add_string("a\nb");
        s2.add_identifier_string(ErrId(2), "c\nd");

        assert_eq!(
            s2,
            StringStorage {
                storage: "a\nbc\nd".to_owned(),
                current_line_no: 3,
                line_numbers: LineNumbersByKey(vec![(ErrId(2), 2..4)].into_iter().collect()),
            }
        );

        let storages = vec![("s1".to_owned(), s1), ("s2".to_owned(), s2)]
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let s = apply_template("abc\n//%s2//%\n\ne\nf\n//%s1//%\n9", storages);

        assert_eq!(
            s,
            StringStorage {
                storage: "abc\na\nbc\nd\n\ne\nf\n1\n2\n3\n\n4\n5\n\n9".to_owned(),
                current_line_no: 15,
                line_numbers: LineNumbersByKey(
                    vec![(ErrId(1), 11..15), (ErrId(2), 3..5)]
                        .into_iter()
                        .collect()
                ),
            }
        );
    }
}

impl Scene {
    pub fn generate_shader_code(&self) -> StringStorage {
        let mut storages: BTreeMap<String, StringStorage> = BTreeMap::new();

        storages.insert("uniforms".to_owned(), {
            let mut result = StringStorage::default();
            for (name, kind) in self
                .uniforms()
                .into_iter()
                .filter(|(name, _)| !name.starts_with("_"))
            {
                result.add_string(format!(
                    "uniform {} {};\n",
                    match kind {
                        UniformType::Mat4 => "mat4",
                        UniformType::Float1 => "float",
                        UniformType::Int1 => "int",

                        UniformType::Float2 => unreachable!(),
                        UniformType::Float3 => unreachable!(),
                        UniformType::Float4 => unreachable!(),
                        UniformType::Int2 => unreachable!(),
                        UniformType::Int3 => unreachable!(),
                        UniformType::Int4 => unreachable!(),
                    },
                    name
                ))
            }
            result
        });

        storages.insert("textures".to_owned(), {
            let mut result = StringStorage::default();
            for name in self.textures.names_iter() {
                result.add_string(format!("uniform sampler2D {};\n", TextureName::name(name)));
            }
            result
        });

        let (material_processing, material_defines) = {
            let mut material_processing = StringStorage::default();
            let mut material_defines = StringStorage::default();
            let mut counter = 0;

            use Material::*;
            for (pos, (name, material)) in self.materials.iter().enumerate() {
                let name_m = format!("{}_M", name);

                material_defines.add_string(format!(
                    "#define {} (USER_MATERIAL_OFFSET + {})\n",
                    name_m, counter
                ));
                counter += 1;

                material_processing
                    .add_string(format!("}} else if (i.material == {}) {{\n", name_m));

                match &material.0 {
                    Simple {
                        color,
                        normal_coef,
                        grid,
                        grid_scale,
                        grid_coef,
                    } => {
                        material_processing.add_string(
                            format!(
                                "return material_simple(hit, r, vec3({:e}, {:e}, {:e}), {:e}, {}, {:e}, {:e});\n",
                                color[0], color[1], color[2], normal_coef, grid, grid_scale, grid_coef,
                            )
                        );
                    }
                    Reflect { add_to_color } => {
                        material_processing.add_string(format!(
                            "return material_reflect(hit, r, vec3({:e}, {:e}, {:e}));\n",
                            add_to_color[0], add_to_color[1], add_to_color[2],
                        ));
                    }
                    Refract {
                        refractive_index,
                        add_to_color,
                    } => {
                        material_processing.add_string(format!(
                            "return material_refract(hit, r, vec3({:e}, {:e}, {:e}), {:e});\n",
                            add_to_color[0], add_to_color[1], add_to_color[2], refractive_index,
                        ));
                    }
                    x @ Complex { .. } => {
                        let code = match x {
                            Complex { code } => code,
                            _ => unreachable!(),
                        };
                        material_processing.add_identifier_string(x.identifier(pos), &code.0.0);
                        material_processing.add_string("\n");
                    }
                };
            }
            for (pos, first, second) in
                self.objects
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, (_, x))| match &x.0 {
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

            for (pos, (_, i)) in self.objects.iter().enumerate() {
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
                        result.add_identifier_string(i.0.identifier(pos), &is_inside.0.0);
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
                        result.add_identifier_string(i.0.identifier(pos), &intersect.0.0);
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

            for (pos, (_, i)) in self.objects.iter().enumerate() {
                match &i.0 {
                    DebugMatrix(matrix) => {
                        result.add_string(format!(
                            "transformed_ray = transform({}, r);\nlen = length(transformed_ray.d);\ntransformed_ray.d = normalize(transformed_ray.d);",
                            matrix.inverse_name()
                        ));
                        result.add_string("ihit = debug_intersect(transformed_ray);\nihit.hit.t /= len;\n");
                        result.add_string(format!(
                            "if (nearer(i, ihit)) {{ i = ihit; i.hit.n = normalize(({} * vec4(i.hit.n, 0.)).xyz); }}\n\n",
                            matrix.normal_name()
                        ));
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
            for (pos, (_, i)) in self.library.iter().enumerate() {
                result.add_identifier_string(i.identifier(pos), &i.0.0);
            }
            result
        });

        storages.insert("predefined_library".to_owned(), {
           let mut result = StringStorage::default();
            result.add_string(LIBRARY);
            result 
        });

        apply_template(FRAGMENT_SHADER, storages)
    }

    pub fn get_new_material(
        &self,
    ) -> Result<macroquad::prelude::Material, (String, String, BTreeMap<ErrId, Vec<(usize, String)>>)>
    {
        let code = self.generate_shader_code();

        use macroquad::prelude::load_material;
        use macroquad::prelude::MaterialParams;

        load_material(
            VERTEX_SHADER,
            &code.storage,
            MaterialParams {
                uniforms: self.uniforms(),
                textures: self.textures(),
                ..Default::default()
            },
        )
        .map_err(|err| {
            if let macroquad::prelude::miniquad::graphics::ShaderError::CompilationError {
                error_message,
                ..
            } = err
            {
                let mut errors: BTreeMap<ErrId, Vec<(usize, String)>> = BTreeMap::new();
                for x in shader_error_parser(&error_message) {
                    match x {
                        Ok((line_no, message)) => match code.line_numbers.get_identifier(line_no) {
                            Some((identifier, local_line_no)) => {
                                errors
                                    .entry(identifier)
                                    .or_insert_with(|| Default::default())
                                    .push((local_line_no, message.to_owned()));
                            }
                            None => {
                                errors
                                    .entry(ErrId::default())
                                    .or_insert_with(|| Default::default())
                                    .push((line_no, message.to_owned()));
                            }
                        },
                        Err(message) => {
                            errors
                                .entry(ErrId::default())
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

    fn try_parse_2(mut line: &str) -> Option<(usize, &str)> {
        expect_str(&mut line, "0(")?;
        let lineno = expect_int(&mut line)?;
        expect_str(&mut line, ") : warning ")?;
        Some((lineno, line))
    }

    // Try parse format `ERROR: 0:586: 'pos' : redefinition`
    // This format is noticed on Firefox + Linux
    fn try_parse_3(mut line: &str) -> Option<(usize, &str)> {
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
            } else if let Some(r) = try_parse_3(line) {
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
        let linux3 = r#"0(365) : warning C7022: unrecognized profile specifier "a""#;
        assert!(shader_error_parser(linux3).iter().all(|x| x.is_ok()));
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

pub const LIBRARY: &'static str = include_str!("library.glsl");

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
    uv = texcoord;

    gl_Position = res;
}
";

// ----------------------------------------------------------------------------------------------------------
// Processing formulase and parametrized uniforms
// ----------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnyUniform {
    Bool(bool),
    Int {
        min: Option<i32>,
        max: Option<i32>,
        value: i32,
    },
    Float {
        min: Option<f64>,
        max: Option<f64>,
        value: f64,
    },
    Angle(f64),
    Formula(Formula),
}

impl AnyUniform {
    pub fn int(int: i32) -> AnyUniform {
        AnyUniform::Int {
            min: None,
            max: None,
            value: int,
        }
    }

    pub fn float(float: f64) -> AnyUniform {
        AnyUniform::Float {
            min: None,
            max: None,
            value: float,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnyUniformResult {
    Bool(bool),
    Int(i32),
    Float(f64),
}

impl From<AnyUniformResult> for f64 {
    fn from(u: AnyUniformResult) -> f64 {
        match u {
            AnyUniformResult::Bool(b) => {
                if b {
                    1.0
                } else {
                    0.0
                }
            }
            AnyUniformResult::Int(i) => i as f64,
            AnyUniformResult::Float(f) => f.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TVec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParametrizeOrNot {
    Yes(FormulaName),
    No(f32),
}

impl ErrorsCount for ParametrizeOrNot {
    fn errors_count(&self, _: usize, data: &mut Data) -> usize {
        match self {
            ParametrizeOrNot::Yes(name) => !data.formulas_names.contains(&name.0) as usize,
            ParametrizeOrNot::No { .. } => 0,
        }
    }
}

impl ParametrizeOrNot {
    fn egui(
        &mut self,
        ui: &mut Ui,
        data: &mut Data,
        label: &str,
        default: f32,
        f: impl FnOnce(&mut Ui, &mut f32) -> bool,
    ) -> bool {
        use ParametrizeOrNot::*;
        let mut not_found = false;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(label);
            let mut current = matches!(self, Yes { .. });
            changed |= egui_bool(ui, &mut current);
            if changed {
                *self = if current {
                    Yes(FormulaName("a".to_owned()))
                } else {
                    No(default)
                };
            }
            changed |= match self {
                Yes(current) => {
                    not_found = !data.formulas_names.contains(&current.0);
                    check_changed(&mut current.0, |text| drop(ui.text_edit_singleline(text)))
                }
                No(float) => f(ui, float),
            };
        });
        if not_found {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error:").text_color(Color32::RED));
                ui.label("uniform with this name is not found");
            });
        }
        changed
    }

    pub fn get(
        &self,
        uniforms: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> f64 {
        use ParametrizeOrNot::*;
        match self {
            Yes(f) => match uniforms.get(&f.0, uniforms, formulas_cache) {
                GetEnum::Ok(x) => x.into(),
                _ => {
                    eprintln!("can't find uniform {}", f.0);
                    0.0
                }
            },
            No(f) => (*f).into(),
        }
    }

    pub fn freeget(&self) -> Option<f32> {
        use ParametrizeOrNot::*;
        match self {
            Yes(_) => None,
            No(f) => Some(*f),
        }
    }
}

impl Default for Formula {
    fn default() -> Self {
        Formula("sin(pi())".to_owned())
    }
}

impl Eguiable for Formula {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let edit = data.formulas_cache.with_edit(&mut self.0, |text| {
            ui.add(TextEdit::singleline(text).text_style(TextStyle::Monospace));
        });
        if edit.has_errors {
            ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
                ui.add(Label::new("Error with this formula").text_color(Color32::RED));
            });
        }
        WhatChanged::from_uniform(edit.changed)
    }
}

pub struct FormulasCache {
    parser: fasteval::Parser,
    slab: fasteval::Slab,
    cache: BTreeMap<String, Option<fasteval::Instruction>>,
}

impl Default for FormulasCache {
    fn default() -> Self {
        let mut result = FormulasCache {
            parser: fasteval::Parser::new(),
            slab: fasteval::Slab::new(),
            cache: Default::default(),
        };
        result.compile("0");
        result
    }
}

use std::fmt::{self, Debug, Formatter};

impl Debug for FormulasCache {
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[derive(Default)]
pub struct EditResult {
    changed: bool,
    has_errors: bool,
}

impl FormulasCache {
    pub fn has_errors(&self, text: &str) -> bool {
        self.cache.get(text).map(|x| x.is_none()).unwrap_or(false)
    }

    pub fn compile(&mut self, text: &str) -> bool {
        use fasteval::*;

        let compiled = || -> Option<_> {
            Some(
                self.parser
                    .parse(text, &mut self.slab.ps)
                    .ok()?
                    .from(&self.slab.ps)
                    .compile(&self.slab.ps, &mut self.slab.cs),
            )
        }();

        let is_compiled = compiled.is_some();

        self.cache.insert(text.to_owned(), compiled);

        is_compiled
    }

    pub fn with_edit(&mut self, text: &mut String, f: impl FnOnce(&mut String)) -> EditResult {
        let previous = text.clone();
        f(text);
        if previous == *text && self.cache.get(text).is_some() {
            EditResult {
                changed: false,
                has_errors: self.cache.get(text).unwrap().is_none(),
            }
        } else {
            self.cache.remove(&previous);
            EditResult {
                changed: true,
                has_errors: !self.compile(text),
            }
        }
    }

    pub fn get<'a, 'b>(&'a self, text: &'b str) -> Option<&'a fasteval::Instruction> {
        self.cache.get(text).and_then(|x| x.as_ref())
    }
}

impl ComboBoxChoosable for AnyUniform {
    fn variants() -> &'static [&'static str] {
        &["bool", "int", "float", "angle", "formula"]
    }
    fn get_number(&self) -> usize {
        use AnyUniform::*;
        match self {
            Bool { .. } => 0,
            Int { .. } => 1,
            Float { .. } => 2,
            Angle { .. } => 3,
            Formula { .. } => 4,
        }
    }
    fn set_number(&mut self, number: usize) {
        use crate::gui::Formula as F;
        use AnyUniform::*;
        *self = match number {
            0 => Bool(match self {
                Bool(b) => *b,
                Int { value, .. } => value >= &mut 1,
                Float { value, .. } => value >= &mut 1.0,
                Angle(a) => a >= &mut 1.0,
                Formula { .. } => false,
            }),
            1 => match self {
                Bool(b) => AnyUniform::int(*b as i32),
                Int { .. } => self.clone(),
                Float { value, .. } => AnyUniform::int(*value as i32),
                Angle(a) => AnyUniform::int(rad2deg(*a as f32) as i32),
                Formula { .. } => AnyUniform::int(0),
            },
            2 => match self {
                Bool(b) => AnyUniform::float(*b as i32 as f64),
                Int { value, .. } => AnyUniform::float(*value as f64),
                Angle(a) => AnyUniform::float(*a),
                Float { .. } => self.clone(),
                Formula { .. } => AnyUniform::float(0.0),
            },
            3 => Angle(match self {
                Bool(b) => (*b as i32 as f64) * std::f64::consts::PI,
                Int { value, .. } => {
                    macroquad::math::clamp(deg2rad(*value as f32), 0., std::f32::consts::TAU) as f64
                }
                Angle(a) => *a,
                Float { value, .. } => macroquad::math::clamp(*value, 0., std::f64::consts::TAU),
                Formula { .. } => 0.0,
            }),
            4 => Formula(match self {
                Bool(b) => F((*b as i32).to_string()),
                Int { value, .. } => F(value.to_string()),
                Angle(a) => F(a.to_string()),
                Float { value, .. } => F(value.to_string()),
                Formula(f) => f.clone(),
            }),
            _ => unreachable!(),
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnyUniformComboBox(pub AnyUniform);

impl Default for AnyUniform {
    fn default() -> Self {
        AnyUniform::Formula(Default::default())
    }
}

impl ErrorsCount for AnyUniform {
    fn errors_count(&self, _: usize, data: &mut Data) -> usize {
        match self {
            AnyUniform::Formula(text) => data.formulas_cache.has_errors(&text.0) as usize,
            _ => 0,
        }
    }
}

impl ErrorsCount for AnyUniformComboBox {
    fn errors_count(&self, pos: usize, data: &mut Data) -> usize {
        self.0.errors_count(pos, data)
    }
}

impl Eguiable for AnyUniform {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        use AnyUniform::*;
        let mut result = WhatChanged::default();
        match self {
            Bool(x) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_bool(ui, x))),
            Int { min, max, value } => {
                ui.horizontal(|ui| {
                    let mut current = min.is_some();
                    if let Some(min) = min {
                        ui.checkbox(&mut current, "min");
                        result.uniform |= check_changed(min, |min| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if v as i32 > *value {
                                            *min = *value;
                                        } else {
                                            *min = v as i32;
                                        }
                                    }
                                    *min as f64
                                })
                                .speed(1),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "min");
                        ui.label("-inf");
                    }
                    if current && min.is_none() {
                        result.uniform = true;
                        let min_i = -100;
                        *min = Some(if *value < min_i { *value } else { min_i });
                    }
                    if !current && min.is_some() {
                        result.uniform = true;
                        *min = None;
                    }
                    ui.separator();
                    let mut current = max.is_some();
                    if let Some(max) = max {
                        ui.checkbox(&mut current, "max");
                        result.uniform |= check_changed(max, |max| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if (v as i32) < *value {
                                            *max = *value;
                                        } else {
                                            *max = v as i32;
                                        }
                                    }
                                    *max as f64
                                })
                                .speed(1),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "max");
                        ui.label("+inf");
                    }
                    if current && max.is_none() {
                        result.uniform = true;
                        let max_i = 100;
                        *max = Some(if *value > max_i { *value } else { max_i });
                    }
                    if !current && max.is_some() {
                        result.uniform = true;
                        *max = None;
                    }
                    ui.separator();
                    ui.centered_and_justified(|ui| {
                        if let Some((min, max)) = min.as_ref().zip(max.as_ref()) {
                            result.uniform |= check_changed(value, |value| {
                                ui.add(Slider::i32(value, *min..=*max).clamp_to_range(true));
                            })
                        } else {
                            result.uniform |= check_changed(value, |value| {
                                ui.add(
                                    DragValue::from_get_set(|v| {
                                        if let Some(v) = v {
                                            *value = v as i32;
                                            if let Some(min) = min {
                                                if value < min {
                                                    *value = *min;
                                                }
                                            }
                                            if let Some(max) = max {
                                                if value > max {
                                                    *value = *max;
                                                }
                                            }
                                        }
                                        (*value).into()
                                    })
                                    .speed(1),
                                );
                            });
                        }
                    });
                });
            }
            Angle(a) => {
                drop(ui.centered_and_justified(|ui| result.uniform |= egui_angle_f64(ui, a)))
            }
            Float { min, max, value } => {
                ui.horizontal(|ui| {
                    let mut current = min.is_some();
                    if let Some(min) = min {
                        ui.checkbox(&mut current, "min");
                        result.uniform |= check_changed(min, |min| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if v > *value {
                                            *min = *value;
                                        } else {
                                            *min = v;
                                        }
                                    }
                                    *min as f64
                                })
                                .speed(0.01)
                                .min_decimals(0)
                                .max_decimals(2),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "min");
                        ui.label("-inf");
                    }
                    if current && min.is_none() {
                        result.uniform = true;
                        let min_i = -10.0;
                        *min = Some(if *value < min_i { *value } else { min_i });
                    }
                    if !current && min.is_some() {
                        result.uniform = true;
                        *min = None;
                    }
                    ui.separator();
                    let mut current = max.is_some();
                    if let Some(max) = max {
                        ui.checkbox(&mut current, "max");
                        result.uniform |= check_changed(max, |max| {
                            ui.add(
                                DragValue::from_get_set(|v| {
                                    if let Some(v) = v {
                                        if v < *value {
                                            *max = *value;
                                        } else {
                                            *max = v;
                                        }
                                    }
                                    *max
                                })
                                .speed(0.01)
                                .min_decimals(0)
                                .max_decimals(2),
                            );
                        });
                    } else {
                        ui.checkbox(&mut current, "max");
                        ui.label("+inf");
                    }
                    if current && max.is_none() {
                        result.uniform = true;
                        let max_i = 10.0;
                        *max = Some(if *value > max_i { *value } else { max_i });
                    }
                    if !current && max.is_some() {
                        result.uniform = true;
                        *max = None;
                    }
                    ui.separator();
                    ui.centered_and_justified(|ui| {
                        if let Some((min, max)) = min.as_ref().zip(max.as_ref()) {
                            result.uniform |= check_changed(value, |value| {
                                ui.add(Slider::f64(value, *min..=*max).clamp_to_range(true));
                            });
                        } else {
                            result.uniform |= check_changed(value, |value| {
                                ui.add(
                                    DragValue::from_get_set(|v| {
                                        if let Some(v) = v {
                                            *value = v;
                                            if let Some(min) = min {
                                                if value < min {
                                                    *value = *min;
                                                }
                                            }
                                            if let Some(max) = max {
                                                if value > max {
                                                    *value = *max;
                                                }
                                            }
                                        }
                                        (*value).into()
                                    })
                                    .speed(0.01)
                                    .min_decimals(0)
                                    .max_decimals(2),
                                );
                            });
                        }
                    });
                });
            }
            Formula(x) => drop(ui.centered_and_justified(|ui| result |= x.egui(ui, data))),
        }
        result
    }
}

impl Eguiable for AnyUniformComboBox {
    fn egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed =
            WhatChanged::from_uniform(egui_combo_label(ui, "Type:", 45., &mut self.0));
        ui.separator();
        changed |= self.0.egui(ui, data);
        changed
    }
}

impl StorageElem for AnyUniformComboBox {
    type GetType = AnyUniformResult;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        mut f: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        formulas_cache: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        let mut cb = |name: &str, args: Vec<f64>| -> Option<f64> {
            Some(match name {
                // Custom functions
                "if" => {
                    if *args.get(0)? == 1.0 {
                        *args.get(1)?
                    } else {
                        *args.get(2)?
                    }
                }
                "and" => {
                    if *args.get(0)? == 1.0 && *args.get(1)? == 1.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "or" => {
                    if *args.get(0)? == 1.0 || *args.get(1)? == 1.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "not" => {
                    if *args.get(0)? == 1.0 {
                        0.0
                    } else {
                        1.0
                    }
                }
                "deg2rad" => args.get(0)? / 180. * std::f64::consts::PI,
                "rad2deg" => args.get(0)? * 180. / std::f64::consts::PI,

                // Free variables
                _ => match f(name) {
                    GetEnum::Ok(x) => x.into(),
                    _ => None?,
                },
            })
        };

        use fasteval::*;

        match &self.0 {
            AnyUniform::Bool(b) => GetEnum::Ok(AnyUniformResult::Bool(*b)),
            AnyUniform::Int { value, .. } => GetEnum::Ok(AnyUniformResult::Int(*value)),
            AnyUniform::Angle(a) => GetEnum::Ok(AnyUniformResult::Float(*a)),
            AnyUniform::Float { value, .. } => GetEnum::Ok(AnyUniformResult::Float(*value)),
            AnyUniform::Formula(f) => match formulas_cache.get(&f.0) {
                Some(x) => match x.eval(&formulas_cache.slab, &mut cb) {
                    Ok(x) => GetEnum::Ok(AnyUniformResult::Float(x)),
                    Err(_) => GetEnum::NotFound,
                },
                None => GetEnum::NotFound,
            },
        }
    }

    fn defaults() -> (Vec<String>, Vec<Self>) {
        (vec!["a".to_owned()], vec![Default::default()])
    }
}

// ----------------------------------------------------------------------------------------------------------
// Animation stages
// ----------------------------------------------------------------------------------------------------------

enum AnimationUniform {
    ProvidedToUser,
    Remains,
    Changed(AnyUniform),
}

struct AnimationStage {
    uniforms: Vec<AnimationUniform>,
}

struct GlobalUserUniforms {
    uniforms: Vec<bool>,
}