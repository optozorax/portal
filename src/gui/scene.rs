use crate::gui::camera::CurrentCam;
use crate::gui::glsl::*;
use crate::gui::storage2::Storage2;

use crate::code_generation::*;
use crate::gui::animation::*;
use crate::gui::common::*;
use crate::gui::eng_rus::EngRusText;
use crate::gui::material::*;
use crate::gui::matrix::*;
use crate::gui::object::*;
use crate::gui::texture::*;
use crate::gui::uniform::*;
use crate::gui::camera::Cam;
use crate::shader_error_parser::*;

use egui::*;
use glam::*;
use macroquad::prelude::UniformType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CamSettings {
    pub look_at: DVec3,
    pub alpha: f64,
    pub beta: f64,
    pub r: f64,
    pub offset_after_material: f64,
}

impl Default for CamSettings {
    fn default() -> Self {
        Self {
            look_at: DVec3::default(),
            alpha: 0.,
            beta: 0.,
            r: 3.5,
            offset_after_material: 0.000025,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scene {
    pub desc: EngRusText,

    pub cam: CamSettings,

    pub uniforms: Storage2<AnyUniform>,

    pub matrices: Storage2<Matrix>,
    objects: Storage2<Object>,

    pub cameras: Storage2<Cam>,

    pub textures: Storage2<TextureName>,

    materials: Storage2<Material>,
    library: Storage2<LibraryCode>,

    animations_filters: AnimationFilters,

    elements_descriptions: ElementsDescriptions,

    user_uniforms: GlobalUserUniforms,
    animation_stages: Storage2<AnimationStage>,

    current_stage: Option<AnimationId>,

    dev_stage: DevStage,
}

// In case of panic
impl Drop for Scene {
    fn drop(&mut self) {
        match ron::to_string(self) {
            Ok(result) => crate::error!(format, "scene:\n\n{}", result),
            Err(err) => crate::error!(format, "errors while serializing scene: {:?}", err),
        }
    }
}

/*
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OldScene {
    pub description_en: String,
    pub description_ru: String,

    pub cam: CamSettings,

    pub uniforms: Storage2<AnyUniform>,

    pub matrices: StorageWithNames<Matrix>,
    objects: StorageWithNames<ObjectComboBox>,

    pub textures: StorageWithNames<TextureName>,

    materials: StorageWithNames<MaterialComboBox>,
    library: StorageWithNames<LibraryCode>,

    user_uniforms: GlobalUserUniforms,
    animation_stages: StorageWithNames<AnimationStage>,

    current_stage: usize,
}

impl From<OldScene> for Scene {
    fn from(old: OldScene) -> Scene {
        Scene {
            description_en: old.description_en,
            description_ru: old.description_ru,

            cam: old.cam,

            uniforms: ,

            matrices: old.matrices.into(),
            objects: old.objects,

            textures: old.textures,

            materials: old.materials,
            library: old.library,

            user_uniforms: old.user_uniforms,
            animation_stages: old.animation_stages,

            current_stage: old.current_stage,
        }
    }
}
*/

impl Scene {
    pub fn init(&mut self, data: &mut Data, memory: &mut egui::Memory) {
        data.errors = Default::default();
        data.show_error_window = false;
        drop(self.init_stage(self.current_stage, memory));
    }

    pub fn dev_stage_button(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let current_selected = self.current_stage.is_none();
        let response = ui.radio(current_selected, "dev");
        if response.clicked() && !current_selected {
            self.current_stage = None;
            changed |= self.init_stage(self.current_stage, &mut ui.memory());
        }
        changed
    }

    #[allow(clippy::type_complexity)] // TODO: reduce type complexity
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        data: &mut Data,
        should_recompile: &mut bool,
    ) -> (
        WhatChanged,
        Option<Result<macroquad::material::Material, (String, String, ShaderErrors)>>,
    ) {
        let mut changed = WhatChanged::default();
        let mut material = None;

        ui.horizontal(|ui| {
            if ui.button("Export").clicked() {
                let s = ron::to_string(self).unwrap();
                data.to_export = Some(s);
            }
            if ui
                .add(Button::new("Recompile").enabled(*should_recompile))
                .clicked()
            {
                match self.get_new_material(&data) {
                    Some(Ok(m)) => {
                        data.reload_textures = true;
                        material = Some(Ok(m));
                        *should_recompile = false;
                        changed.uniform = true;
                        data.errors = Default::default();
                        data.show_error_window = false;
                    }
                    Some(Err(err)) => {
                        material = Some(Err(err));
                    }
                    None => {
                        material = None;
                    }
                }
            }
            let response = ui.radio(self.current_stage.is_none(), "dev");
            if response.clicked() && self.current_stage.is_some() {
                self.current_stage = None;
                changed |= self.init_stage(self.current_stage, &mut ui.memory());
            }
        });

        ui.separator();

        // other ui

        CollapsingHeader::new("Description")
            .default_open(false)
            .show(ui, |ui| {
                self.desc.egui_view_edit(ui, egui::Id::new("description"));
            });

        if self.current_stage.is_none() {
            let changed_uniforms = self.uniforms.egui(ui, &mut data.formulas_cache, "Uniforms");
            if changed_uniforms.uniform {
                self.dev_stage.uniforms.copy(&self.uniforms);
            }
            changed |= changed_uniforms;

            ui.collapsing("Calculated uniforms", |ui| {
                for (id, name) in self.uniforms.visible_elements() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        ui.label(format!("{} = ", name));
                        use AnyUniformResult::*;
                        match self.uniforms.get(id, &data.formulas_cache) {
                            Some(x) => match x {
                                Bool(b) => ui.label(b.to_string()),
                                Int(b) => ui.label(b.to_string()),
                                Float(b) => ui.label(b.to_string()),
                            },
                            None => ui.label("NotFound"),
                        }
                    });
                }
            });

            let changed_matrices = with_swapped!(x => (self.uniforms, data.formulas_cache);
                self.matrices.egui(ui, &mut x, "Matrices"));
            if changed_matrices.uniform {
                self.dev_stage.matrices.copy(&self.matrices);
            }
            changed |= changed_matrices;
        } else {
            ui.label("You can edit uniforms and matrices only when `dev` stage is enabled");
        }

        with_swapped!(x => (data.errors, self.matrices, self.uniforms, data.formulas_cache);
            changed |= self.objects.egui(ui, &mut x, "Objects"));

        changed |= self.cameras.egui(ui, &mut self.matrices, "Cameras");

        changed |= self.materials.egui(ui, &mut data.errors, "Materials");

        changed |= self.textures.egui(ui, &mut data.texture_errors, "Textures");

        changed |= self.library.egui(ui, &mut data.errors, "User GLSL code");

        ui.collapsing("Filter to animation stages", |ui| {
            self.animations_filters
                .egui(ui, &self.uniforms, &self.matrices, &self.cameras);
        });

        ui.collapsing("Elements descriptions", |ui| {
            self.elements_descriptions
                .egui(ui, &self.uniforms, &self.matrices, &self.cameras, &mut self.animations_filters);
        });

        ui.collapsing("Global user uniforms", |ui| {
            changed |= self
                .user_uniforms
                .egui(ui, &mut self.uniforms, &mut self.matrices, &mut self.animations_filters);
        });

        with_swapped!(x => (self.cameras, self.animations_filters, self.user_uniforms, self.matrices, self.uniforms, data.formulas_cache);
            changed |= self
                .animation_stages
                .egui(ui, &mut x, "Animation stages"));

        ui.collapsing("Select stage", |ui| {
            changed |= self.dev_stage_button(ui);
            ui.separator();
            changed |= self.select_stage_ui(ui);
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("View GLSL library").clicked() {
                data.show_glsl_library = true;
            }
            if ui.button("View generated GLSL code").clicked() {
                data.show_compiled_code = self.generate_shader_code(&data).map(|x| x.storage);
            }
        });

        let errors = &data.errors;
        let show_error_window = &mut data.show_error_window;
        if let Some(local_errors) = errors.get::<()>(()) {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Other errors:");
                if ui.button("Show full code and errors").clicked() {
                    *show_error_window = true;
                }
            });
            egui_errors(ui, &local_errors);
        }

        match ron::to_string(self) {
            Ok(result) => std::fs::write("scene_dump.ron", result).unwrap(),
            Err(err) => crate::error!(format, "errors while serializing scene: {:?}", err),
        }

        (changed, material)
    }
}

impl Scene {
    pub fn errors_count(&mut self, _: usize, data: &mut Data) -> usize {
        with_swapped!(x => (self.uniforms, data.formulas_cache);
            self.matrices.errors_count_all(&x))
            + with_swapped!(x => (data.errors, self.matrices, self.uniforms, data.formulas_cache);
                self.objects.errors_count_all(&x))
            + self.materials.errors_count_all(&data.errors)
            + self.library.errors_count_all(&data.errors)
            + if let Some(local_errors) = data.errors.get::<()>(()) {
                local_errors.len()
            } else {
                0
            }
    }
}

pub trait UniformStruct {
    fn uniforms(&self) -> Vec<(String, UniformType)>;
    fn set_uniforms(&self, material: macroquad::material::Material);
}

impl Scene {
    pub fn textures(&self) -> Vec<String> {
        self.textures
            .visible_elements()
            .map(|(_, name)| TextureName::name(name))
            .collect()
    }

    pub fn uniforms(&self, data: &Data) -> Option<Vec<(String, UniformType)>> {
        let mut result = Vec::new();
        use Object::*;
        use ObjectType::*;
        for (id, _) in self.objects.visible_elements() {
            let object = self.objects.get(id, &()).unwrap();
            match &object {
                DebugMatrix(matrix) => {
                    let matrix = Object::get_name((*matrix)?, &self.matrices).unwrap();
                    result.push(matrix.normal_name());
                    result.push(matrix.inverse_name());
                }
                Flat { kind, is_inside: _ } | Complex { kind, intersect: _ } => match kind {
                    Simple(matrix) => {
                        let matrix = Object::get_name((*matrix)?, &self.matrices)?;
                        result.push(matrix.normal_name());
                        result.push(matrix.inverse_name());
                    }
                    Portal(a, b) => {
                        let a = Object::get_name((*a)?, &self.matrices)?;
                        let b = Object::get_name((*b)?, &self.matrices)?;
                        result.push(a.normal_name());
                        result.push(a.inverse_name());
                        result.push(b.normal_name());
                        result.push(b.inverse_name());
                        result.push(a.teleport_to_name(&b));
                        if *b.0 != *a.0 {
                            result.push(b.teleport_to_name(&a));
                        }
                    }
                },
            }
        }

        for (_, name) in self.matrices.visible_elements() {
            let matrix = MatrixName(std::borrow::Cow::Borrowed(name));
            result.push(matrix.normal_name());
            result.push(matrix.inverse_name());
        }

        let mut result = result
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|name| (name, UniformType::Mat4))
            .collect::<Vec<_>>();

        for (id, name) in self.uniforms.visible_elements() {
            let name = format!("{}_u", name);
            match self.uniforms.get(id, &data.formulas_cache) {
                Some(AnyUniformResult::Bool(_)) => result.push((name, UniformType::Int1)),
                Some(AnyUniformResult::Int { .. }) => result.push((name, UniformType::Int1)),
                Some(AnyUniformResult::Float { .. }) => result.push((name, UniformType::Float1)),
                None => {}
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

        Some(result)
    }

    pub fn set_uniforms(&mut self, material: macroquad::material::Material, data: &mut Data) {
        let objects = &self.objects;
        let uniforms = &mut self.uniforms;
        let matrices = &self.matrices;
        let passed_matrices = self
            .objects
            .visible_elements()
            .filter_map(|(id, _)| {
                use Object::*;
                use ObjectType::*;
                Some(
                    match &objects.get(id, &())? {
                        DebugMatrix(matrix) => vec![(*matrix)?],
                        Flat { kind, is_inside: _ } | Complex { kind, intersect: _ } => {
                            match kind {
                                Simple(matrix) => vec![(*matrix)?],
                                Portal(a, b) => vec![(*a)?, (*b)?],
                            }
                        }
                    }
                    .into_iter()
                    .filter_map(|id| Some((id, Object::get_name(id, &matrices)?))),
                )
            })
            .flatten()
            .chain(
                self.matrices
                    .visible_elements()
                    .map(|(id, name)| (id, MatrixName(std::borrow::Cow::Borrowed(name)))),
            );
        for (id, name) in passed_matrices {
            let matrix = with_swapped!(x => (*uniforms, data.formulas_cache); matrices.get(id, &x));
            if let Some(matrix) = matrix {
                material.set_uniform(&name.normal_name(), matrix.as_f32());
                material.set_uniform(&name.inverse_name(), matrix.inverse().as_f32());
            } else {
                crate::error!(format, "matrix `{}` can't be getted", name.0);
            }
        }

        let teleport_matrices = self.objects.visible_elements().filter_map(|(id, _)| {
            use Object::*;
            use ObjectType::*;
            match &objects.get(id, &())? {
                DebugMatrix(_) => None,
                Flat { kind, is_inside: _ } | Complex { kind, intersect: _ } => match kind {
                    Simple(_) => None,
                    Portal(a, b) => {
                        let a = (*a)?;
                        let b = (*b)?;
                        let a = (a, Object::get_name(a, &matrices)?);
                        let b = (b, Object::get_name(b, &matrices)?);
                        Some((a, b))
                    }
                },
            }
        });
        for ((ida, namea), (idb, nameb)) in teleport_matrices {
            let a = with_swapped!(x => (*uniforms, data.formulas_cache); matrices.get(ida, &x));
            let b = with_swapped!(x => (*uniforms, data.formulas_cache); matrices.get(idb, &x));
            if let Some((ma, mb)) = a.zip(b) {
                material.set_uniform(
                    &namea.teleport_to_name(&nameb),
                    (mb * ma.inverse()).as_f32(),
                );
                if namea.0 != nameb.0 {
                    material.set_uniform(
                        &nameb.teleport_to_name(&namea),
                        (ma * mb.inverse()).as_f32(),
                    );
                }
            }
        }

        for (id, name) in self.uniforms.visible_elements() {
            let name_u = format!("{}_u", name);
            match self.uniforms.get(id, &data.formulas_cache) {
                Some(result) => match result {
                    AnyUniformResult::Bool(b) => material.set_uniform(&name_u, b as i32),
                    AnyUniformResult::Int(i) => material.set_uniform(&name_u, i),
                    AnyUniformResult::Float(f) => material.set_uniform(&name_u, f as f32),
                },
                _ => {
                    crate::error!(format, "Error getting `{}` uniform", name);
                }
            }
        }
    }
}

impl Scene {
    pub fn generate_uniforms_declarations(&self, data: &Data) -> Option<StringStorage> {
        let mut result = StringStorage::default();
        for (name, kind) in self
            .uniforms(data)?
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
        Some(result)
    }

    pub fn generate_shader_code(&self, data: &Data) -> Option<StringStorage> {
        let mut storages: BTreeMap<String, StringStorage> = BTreeMap::new();

        storages.insert(
            "uniforms".to_owned(),
            self.generate_uniforms_declarations(data)?,
        );

        storages.insert("textures".to_owned(), {
            let mut result = StringStorage::default();
            for (_, name) in self.textures.visible_elements() {
                result.add_string(format!("uniform sampler2D {};\n", TextureName::name(name)));
            }
            result
        });

        let (material_processing, material_defines) = {
            let mut material_processing = StringStorage::default();
            let mut material_defines = StringStorage::default();
            let mut counter = 0;

            use Material::*;
            for (id, name) in self.materials.visible_elements() {
                let material = self.materials.get(id, &()).unwrap();
                let name_m = format!("{}_M", name);

                material_defines.add_string(format!(
                    "#define {} (USER_MATERIAL_OFFSET + {})\n",
                    name_m, counter
                ));
                counter += 1;

                material_processing
                    .add_string(format!("}} else if (i.material == {}) {{\n", name_m));

                match &material {
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
                        material_processing.add_identifier_string(id, &code.0 .0);
                        material_processing.add_string("\n");
                    }
                };
            }
            for (pos, first, second) in self
                .objects
                .visible_elements()
                .map(|(id, _)| self.objects.get(id, &()).unwrap())
                .enumerate()
                .filter_map(|(pos, x)| match x {
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
                .filter_map(|(pos, first, second)| {
                    Some((
                        pos,
                        Object::get_name(first?, &self.matrices)?,
                        Object::get_name(second?, &self.matrices)?,
                    ))
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
                    first.teleport_to_name(&second)
                ));

                material_processing
                    .add_string(format!("}} else if (i.material == {}) {{\n", name_m_2));
                material_processing.add_string(format!(
                    "return material_teleport(hit, r, {});",
                    second.teleport_to_name(&first)
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

            for (pos, (id, _)) in self.objects.visible_elements().enumerate() {
                let object = self.objects.get(id, &()).unwrap();
                match object {
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
                        result.add_identifier_string(id, &is_inside.0.0);
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
                        result.add_identifier_string(id, &intersect.0.0);
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

            for (pos, (id, _)) in self.objects.visible_elements().enumerate() {
                let object = self.objects.get(id, &()).unwrap();
                match object {
                    DebugMatrix(matrix) => {
                        let matrix = Object::get_name(matrix?, &self.matrices)?;
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
                            let matrix = Object::get_name(matrix?, &self.matrices)?;
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
                            let a = Object::get_name(a?, &self.matrices)?;
                            let b = Object::get_name(b?, &self.matrices)?;
                            add(&a, true, format!("teleport_{}_1_M", pos));
                            add(&b, false, format!("teleport_{}_2_M", pos));
                        }
                    },
                    Complex { kind, intersect: _ } => match kind {
                        Simple(matrix) => {
                            let matrix = Object::get_name(matrix?, &self.matrices)?;
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
                            let a = Object::get_name(a?, &self.matrices)?;
                            let b = Object::get_name(b?, &self.matrices)?;
                            add(&a, true, format!("teleport_{}_1_M", pos));
                            add(&b, false, format!("teleport_{}_2_M", pos));
                        }
                    },
                }
                result.add_string("\n");
            }
            result
        });

        storages.insert("library".to_owned(), {
            let mut result = StringStorage::default();
            for (id, _) in self.library.visible_elements() {
                let code = self.library.get(id, &()).unwrap();
                result.add_identifier_string(id, code.0 .0);
            }
            result
        });

        storages.insert("predefined_library".to_owned(), {
            let mut result = StringStorage::default();
            result.add_string(LIBRARY);
            result
        });

        Some(apply_template(FRAGMENT_SHADER, storages))
    }

    pub fn get_new_material(
        &self,
        data: &Data,
    ) -> Option<Result<macroquad::prelude::Material, (String, String, ShaderErrors)>> {
        let code = self.generate_shader_code(data)?;

        use macroquad::prelude::load_material;
        use macroquad::prelude::MaterialParams;

        Some(
            load_material(
                VERTEX_SHADER,
                &code.storage,
                MaterialParams {
                    uniforms: self.uniforms(data)?,
                    textures: self.textures(),
                    ..Default::default()
                },
            )
            .map_err(|err| {
                let error_message = match err {
                    macroquad::prelude::miniquad::graphics::ShaderError::CompilationError {
                        error_message,
                        ..
                    } => error_message,
                    macroquad::prelude::miniquad::graphics::ShaderError::LinkError(msg) => msg,
                    other => {
                        crate::error!(format, "unknown material compilation error: {:?}", other);
                        Default::default()
                    }
                };
                let mut errors: ShaderErrors = Default::default();
                for x in shader_error_parser(&error_message) {
                    match x {
                        Ok((line_no, message)) => match code.line_numbers.get_identifier(line_no) {
                            Some((id, local_line_no)) => {
                                errors.push(id, (local_line_no, message.to_owned()));
                            }
                            None => {
                                errors.push_t((), (line_no, message.to_owned()));
                            }
                        },
                        Err(message) => {
                            errors.push_t((), (usize::MAX, message.to_owned()));
                        }
                    }
                }
                (code.storage, error_message, errors)
            }),
        )
    }
}

impl Scene {
    fn init_stage(&mut self, stage: Option<AnimationId>, memory: &mut egui::Memory) -> WhatChanged {
        if let Some(id) = stage {
            let stage = self.animation_stages.get_original(id).unwrap();
            stage
                .uniforms
                .init_stage(&mut self.uniforms, &self.dev_stage.uniforms);
            stage
                .matrices
                .init_stage(&mut self.matrices, &self.dev_stage.matrices);

            if let Some(cam) = stage.set_cam {
                memory.data.insert(CurrentCam(cam));
            } else {
                memory.data.insert(CurrentCam(None));
            }
        } else {
            self.dev_stage.uniforms.init_stage(&mut self.uniforms);
            self.dev_stage.matrices.init_stage(&mut self.matrices);
            memory.data.insert(CurrentCam(None));
        }
        WhatChanged::from_uniform(true)
    }

    pub fn select_stage_ui(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut current_stage = self.current_stage;
        changed.uniform |= check_changed(&mut current_stage, |stage| {
            let previous = *stage;
            let elements = self
                .animation_stages
                .visible_elements()
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            for id in elements {
                let stage_value = self.animation_stages.get_original(id).unwrap();
                let text = stage_value.name.text(ui);
                ui.radio_value(stage, Some(id), text);
                if *stage != previous && *stage == Some(id) {
                    changed |= self.init_stage(*stage, &mut ui.memory());
                }
            }
        });
        self.current_stage = current_stage;
        changed
    }

    pub fn control_egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::default();
        changed |= self.user_uniforms.user_egui(ui, &mut self.uniforms, &mut self.matrices, &mut self.elements_descriptions);

        if self.animation_stages.len() != 0 {
            changed |= self.select_stage_ui(ui);
            ui.separator();
            if let Some(stage) = self.current_stage.clone() {
                if let Some(stage) = self.animation_stages.get_original(stage) {
                    with_swapped!(x => (self.uniforms, data.formulas_cache);
                        changed |= stage.user_egui(ui, &mut x, &mut self.matrices, &mut self.cameras, &mut self.elements_descriptions));

                } else {
                    self.current_stage = None;
                }
            } else {
                ui.label("Select any stage");
            }
        }

        changed
    }
}

const FRAGMENT_SHADER: &str = include_str!("../frag.glsl");

pub const LIBRARY: &str = include_str!("../library.glsl");

const VERTEX_SHADER: &str = "#version 100
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
