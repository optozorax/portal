use crate::gui::camera::CalculatedCam;
use crate::gui::camera::CameraId;
use crate::gui::camera::CurrentCam;
use crate::gui::glsl::*;
use crate::gui::scenes::ShowHiddenScenes;
use crate::gui::storage2::Storage2;
use macroquad::prelude::UniformDesc;

use super::scene_serialized::{
    deserialize_scene_new_format, serialize_scene_new_format, SerializedScene,
};
use crate::code_generation::*;
use crate::gui::animation::*;
use crate::gui::camera::Cam;
use crate::gui::common::*;
use crate::gui::eng_rus::EngRusText;
use crate::gui::intersection_material::*;
use crate::gui::material::*;
use crate::gui::matrix::*;
use crate::gui::object::*;
use crate::gui::texture::*;
use crate::gui::uniform::*;
use crate::gui::video::*;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Copy)]
pub enum CurrentStage {
    #[serde(alias = "None")]
    #[default]
    Dev,

    #[serde(alias = "Some")]
    Animation(AnimationId),

    RealAnimation(RealAnimationId),
}

impl CurrentStage {
    pub fn is_dev(&self) -> bool {
        CurrentStage::Dev == *self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scene {
    pub desc: EngRusText,

    pub cam: CamSettings,

    pub uniforms: Storage2<AnyUniform>,

    pub matrices: Storage2<Matrix>,
    pub objects: Storage2<Object>,

    pub cameras: Storage2<Cam>,

    pub textures: Storage2<TextureName>,

    #[serde(default)]
    pub videos: Storage2<Video>,

    pub materials: Storage2<Material>,

    #[serde(default)]
    pub intersection_materials: Storage2<IntersectionMaterial>,

    pub library: Storage2<LibraryCode>,

    pub animations_filters: AnimationFilters,

    pub elements_descriptions: ElementsDescriptions,

    pub user_uniforms: GlobalUserUniforms,
    pub animation_stages: Storage2<AnimationStage>,

    pub current_stage: CurrentStage,

    pub dev_stage: DevStage,

    #[serde(default)]
    pub animations: Storage2<RealAnimation>,

    #[serde(default)]
    pub use_time: bool,

    #[serde(skip)]
    pub run_animations: bool,

    #[serde(skip)]
    pub animation_stage_edit_state: bool,

    #[serde(skip)]
    prev_t_raw: f64,

    #[serde(default)]
    pub skybox: Option<String>,
}

// In case of panic
impl Drop for Scene {
    fn drop(&mut self) {
        // match ron::to_string(self) {
        //     Ok(result) => crate::error!(format, "scene:\n\n{}", result),
        //     Err(err) => crate::error!(format, "errors while serializing scene: {:?}", err),
        // }
    }
}

impl Scene {
    pub fn to_serialized(&self) -> SerializedScene {
        serialize_scene_new_format(self)
    }

    pub fn from_serialized(ser: SerializedScene) -> Self {
        let mut scene: Scene = Default::default();
        deserialize_scene_new_format(ser, &mut scene);
        scene
    }
    pub fn init(&mut self, data: &mut Data, memory: &mut egui::Memory) {
        data.errors = Default::default();
        data.show_error_window = false;
        drop(self.init_stage(self.current_stage, memory));
    }

    pub fn dev_stage_button(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let current_selected = self.current_stage.is_dev();
        let response = ui.radio(current_selected, "dev");
        if response.clicked() && !current_selected {
            self.current_stage = CurrentStage::Dev;
            changed |= ui.memory_mut(|memory| self.init_stage(self.current_stage, memory));
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
                .add_enabled(*should_recompile, Button::new("Recompile"))
                .clicked()
            {
                match self.get_new_material(data) {
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
            changed |= self.dev_stage_button(ui);

            ui.checkbox(&mut self.use_time, "Use time");

            ui.checkbox(
                &mut self.animation_stage_edit_state,
                "Animation stage edit state",
            );
        });

        ui.separator();

        // other ui

        CollapsingHeader::new("Description")
            .default_open(false)
            .show(ui, |ui| {
                self.desc.egui_view_edit(ui, egui::Id::new("description"));
            });

        if self.current_stage.is_dev() {
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
                                Bool(b) => drop(ui.label(b.to_string())),
                                Int(b) => drop(ui.label(b.to_string())),
                                Float(b) => drop(ui.label(b.to_string())),
                                TrefoilSpecial(_) => {}
                            },
                            None => drop(ui.label("NotFound")),
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

        changed |=
            self.intersection_materials
                .egui(ui, &mut data.errors, "Intersection with material");

        changed |= self.textures.egui(ui, &mut data.texture_errors, "Textures");

        with_swapped!(x => (data.video_errors, self.uniforms, data.formulas_cache);
            changed |= self.videos.egui(ui, &mut x, "Videos"));

        ui.collapsing("Skybox", |ui| {
            changed.shader |= egui_option(
                ui,
                &mut self.skybox,
                "Skybox texture",
                String::new,
                |ui, t| ui.text_edit_singleline(t).changed(),
            );
        });

        changed |= self.library.egui(ui, &mut data.errors, "User GLSL code");

        ui.collapsing("Filter to animation stages", |ui| {
            self.animations_filters
                .egui(ui, &self.uniforms, &self.matrices, &self.cameras);
        });

        ui.collapsing("Elements descriptions", |ui| {
            self.elements_descriptions.egui(
                ui,
                &self.uniforms,
                &self.matrices,
                &self.cameras,
                &mut self.animations_filters,
            );
        });

        ui.collapsing("Global user uniforms", |ui| {
            changed |= self.user_uniforms.egui(
                ui,
                &mut self.uniforms,
                &mut self.matrices,
                &mut self.animations_filters,
            );
        });

        with_swapped!(x => (self.cameras, self.animations_filters, self.user_uniforms, self.matrices, self.uniforms, data.formulas_cache);
            changed |= self
                .animation_stages
                .egui(ui, &mut x, "Animation stages"));

        with_swapped!(x => (self.animation_stages.visible_elements_vec(), self.animations.visible_elements_vec(), self.cameras, self.animations_filters, self.user_uniforms, self.matrices, self.uniforms, data.formulas_cache);
            changed |= self
                .animations
                .egui(ui, &mut x, "Animations"));

        ui.collapsing("Select stage", |ui| {
            changed |= self.dev_stage_button(ui);
            ui.separator();
            changed |= self.select_stage_ui(ui, true);
            ui.separator();
            changed |= self.control_animation_parameters(ui);
            ui.separator();
            changed |= self.select_animation_ui(ui);
            ui.separator();
            changed.uniform |= egui_bool_named(ui, &mut self.run_animations, "Run all animations");
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("View GLSL library").clicked() {
                data.show_glsl_library = true;
            }
            if ui.button("View generated GLSL code").clicked() {
                data.show_compiled_code = self.generate_shader_code(data).map(|x| x.storage);
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
            egui_errors(ui, local_errors);
        }

        #[cfg(not(target_arch = "wasm32"))]
        match ron::to_string(self) {
            Ok(result) => {
                drop(std::fs::write("scene_dump_temp.ron", result));
                drop(std::fs::remove_file("scene_dump.ron"));
                drop(std::fs::rename("scene_dump_temp.ron", "scene_dump.ron"));
            }
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
            + self.intersection_materials.errors_count_all(&data.errors)
            + self.library.errors_count_all(&data.errors)
            + if let Some(local_errors) = data.errors.get::<()>(()) {
                local_errors.len()
            } else {
                0
            }
    }
}

pub trait UniformStruct {
    fn uniforms(&self) -> Vec<macroquad::prelude::UniformDesc>;
    fn set_uniforms(&self, material: &mut macroquad::material::Material);
}

impl Scene {
    pub fn textures(&self) -> Vec<String> {
        use std::collections::BTreeSet;
        let mut names = BTreeSet::new();
        let mut result = Vec::new();

        for (_, name) in self.textures.visible_elements() {
            if names.insert(name.to_owned()) {
                result.push(TextureName::name(name));
            }
        }

        for (_, name) in self.videos.visible_elements() {
            if names.insert(name.to_owned()) {
                result.push(TextureName::name(name));
            }
        }

        result
    }

    pub fn compile_all_formulas(&self, cache: &FormulasCache) {
        for id in self.uniforms.all_ids() {
            if let AnyUniform::Formula(f) | AnyUniform::FormulaInt(f) =
                self.uniforms.get_original(id).unwrap()
            {
                cache.compile(&f.0);
            }
        }
    }

    pub fn uniforms(&self, data: &Data) -> Option<Vec<macroquad::prelude::UniformDesc>> {
        self.compile_all_formulas(&data.formulas_cache);

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
                Flat {
                    kind,
                    is_inside: _,
                    in_subspace: _,
                }
                | Complex {
                    kind,
                    intersect: _,
                    in_subspace: _,
                } => match kind {
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
                Some(AnyUniformResult::TrefoilSpecial(x)) => {
                    for (i, _) in x.0.iter().enumerate() {
                        result.push((format!("ts_{}_{}", i, name), UniformType::Int1))
                    }
                }
                None => {}
            }
        }

        result.extend(vec![
            ("_camera".to_owned(), UniformType::Mat4),
            ("_camera_mul_inv".to_owned(), UniformType::Mat4),
            ("_camera_in_subspace".to_owned(), UniformType::Int1),
            ("_resolution".to_owned(), UniformType::Float2),
            ("_ray_tracing_depth".to_owned(), UniformType::Int1),
            ("_aa_count".to_owned(), UniformType::Int1),
            ("_aa_start".to_owned(), UniformType::Int1),
            ("_offset_after_material".to_owned(), UniformType::Float1),
            ("_t_start".to_owned(), UniformType::Float1),
            ("_t_end".to_owned(), UniformType::Float1),
            ("_view_angle".to_owned(), UniformType::Float1),
            ("_use_panini_projection".to_owned(), UniformType::Int1),
            ("_use_360_camera".to_owned(), UniformType::Int1),
            ("_angle_color_disable".to_owned(), UniformType::Int1),
            ("_darken_by_distance".to_owned(), UniformType::Int1),
            ("_grid_disable".to_owned(), UniformType::Int1),
            ("_black_border_disable".to_owned(), UniformType::Int1),
            ("_panini_param".to_owned(), UniformType::Float1),
            ("_teleport_external_ray".to_owned(), UniformType::Int1),
            ("_external_ray_a".to_owned(), UniformType::Float3),
            ("_external_ray_b".to_owned(), UniformType::Float3),
        ]);

        let result = result
            .into_iter()
            .map(|x| macroquad::prelude::UniformDesc::new(&x.0, x.1))
            .collect();

        Some(result)
    }

    pub fn set_uniforms(&mut self, material: &mut macroquad::material::Material, data: &mut Data) {
        self.compile_all_formulas(&data.formulas_cache);

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
                        Flat {
                            kind,
                            is_inside: _,
                            in_subspace: _,
                        }
                        | Complex {
                            kind,
                            intersect: _,
                            in_subspace: _,
                        } => match kind {
                            Simple(matrix) => vec![(*matrix)?],
                            Portal(a, b) => vec![(*a)?, (*b)?],
                        },
                    }
                    .into_iter()
                    .filter_map(|id| Some((id, Object::get_name(id, matrices)?))),
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
                Flat {
                    kind,
                    is_inside: _,
                    in_subspace: _,
                }
                | Complex {
                    kind,
                    intersect: _,
                    in_subspace: _,
                } => match kind {
                    Simple(_) => None,
                    Portal(a, b) => {
                        let a = (*a)?;
                        let b = (*b)?;
                        let a = (a, Object::get_name(a, matrices)?);
                        let b = (b, Object::get_name(b, matrices)?);
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
                    AnyUniformResult::TrefoilSpecial(x) => {
                        for (i, (enabled, value, color)) in x.0.iter().enumerate() {
                            let compressed_value =
                                *value as u32 + (*enabled as u32 * 10000) + (*color as u32 * 1000);
                            material.set_uniform(&format!("ts_{}_{}", i, name_u), compressed_value);
                        }
                    }
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
        for UniformDesc {
            name, uniform_type, ..
        } in self
            .uniforms(data)?
            .into_iter()
            .filter(|UniformDesc { name, .. }| !name.starts_with('_'))
        {
            #[allow(unreachable_patterns)]
            result.add_string(format!(
                "uniform {} {};\n",
                match uniform_type {
                    UniformType::Mat4 => "mat4",
                    UniformType::Float1 => "float",
                    UniformType::Int1 => "int",

                    UniformType::Float2 => unreachable!(),
                    UniformType::Float3 => unreachable!(),
                    UniformType::Float4 => unreachable!(),
                    UniformType::Int2 => unreachable!(),
                    UniformType::Int3 => unreachable!(),
                    UniformType::Int4 => unreachable!(),

                    _ => todo!(),
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
            use std::collections::BTreeSet;
            let mut names = BTreeSet::new();

            for (_, name) in self.textures.visible_elements() {
                names.insert(name.to_owned());
            }
            for (_, name) in self.videos.visible_elements() {
                names.insert(name.to_owned());
            }

            for name in names {
                result.add_string(format!("uniform sampler2D {};\n", TextureName::name(&name)));
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
                        grid2,
                        grid3,
                    } => {
                        material_processing.add_string(
                            format!(
                                "return material_simple2(hit, r, vec3({:e}, {:e}, {:e}), {:e}, {}, {:e}, {:e}, {}, {});\n",
                                color[0], color[1], color[2], normal_coef, grid, grid_scale, grid_coef, grid2, grid3
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
                    Flat { kind, is_inside, in_subspace: _ } => {
                        if matches!(kind, Portal { .. }) {
                            result.add_string(format!(
                                "int is_inside_{}(vec4 pos, float x, float y, bool back, bool first) {{\n",
                                pos
                            ));
                        } else {
                            result.add_string(format!("int is_inside_{}(vec4 pos, float x, float y, bool back) {{\n", pos));
                        }
                        result.add_identifier_string(id, &is_inside.0.0);
                        result.add_string("\n}\n");
                    }
                    Complex { kind, intersect, in_subspace: _ } => {
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
                            "transformed_ray = transform({}, r);\nlen = length(transformed_ray.d);\ntransformed_ray = normalize_ray(transformed_ray);",
                            matrix.inverse_name()
                        ));
                        result.add_string("ihit = debug_intersect(transformed_ray);\nihit.hit.t /= len;\n");
                        result.add_string(format!(
                            "if (nearer(i, ihit)) {{ i = ihit; i.hit.n = normalize(adjugate({}) * i.hit.n); }}\n\n",
                            matrix.inverse_name()
                        ));
                    }
                    Flat { kind, is_inside: _, in_subspace } => {
                        match in_subspace {
                            SubspaceType::Normal => result.add_string("if (r.in_subspace == false) {"),
                            SubspaceType::Subspace => result.add_string("if (r.in_subspace == true) {"),
                            SubspaceType::Both => {},
                        }
                        match kind {
                            Simple(matrix) => {
                                let matrix = Object::get_name(matrix?, &self.matrices)?;
                                result.add_string(format!(
                                    "normal = -get_normal({});\n",
                                    matrix.normal_name()
                                ));
                                result.add_string(format!(
                                    "hit = plane_intersect(r, {}, get_normal({}));\n",
                                    matrix.inverse_name(),
                                    matrix.normal_name()
                                ));
                                result.add_string(format!(
                                    "if (nearer(i, hit)) {{ i = process_plane_intersection(i, hit, is_inside_{}(r.o + r.d * hit.t, hit.u, hit.v, is_collinear(hit.n, normal))); }}\n\n",
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
                        };
                        match in_subspace {
                            SubspaceType::Normal | SubspaceType::Subspace => result.add_string("}"),
                            SubspaceType::Both => {},
                        }
                    },
                    Complex { kind, intersect: _, in_subspace } => {
                        match in_subspace {
                            SubspaceType::Normal => result.add_string("if (r.in_subspace == false) {"),
                            SubspaceType::Subspace => result.add_string("if (r.in_subspace == true) {"),
                            SubspaceType::Both => {},
                        }
                        match kind {
                            Simple(matrix) => {
                                let matrix = Object::get_name(matrix?, &self.matrices)?;
                                result.add_string(format!(
                                    "transformed_ray = transform({}, r);\nlen = length(transformed_ray.d);\ntransformed_ray = normalize_ray(transformed_ray);",
                                    matrix.inverse_name()
                                ));
                                result.add_string(format!(
                                    "ihit = intersect_{}(transformed_ray);\nihit.hit.t /= len;\n",
                                    pos,
                                ));
                                result.add_string(format!(
                                    "if (nearer(i, ihit)) {{ i = ihit; i.hit.n = normalize(adjugate({}) * i.hit.n); }}\n\n",
                                    matrix.normal_name()
                                ));
                            }
                            Portal(a, b) => {
                                let mut add = |matrix: &MatrixName, first, material| {
                                    result.add_string(format!(
                                        "transformed_ray = transform({}, r);\nlen = length(transformed_ray.d);\ntransformed_ray = normalize_ray(transformed_ray);",
                                        matrix.inverse_name()
                                    ));
                                    result.add_string(format!(
                                        "ihit = intersect_{}(transformed_ray, {});\nihit.hit.t /= len;\n",
                                        pos, first
                                    ));
                                    result.add_string(format!(
                                        "if (nearer(i, ihit) && ihit.material != NOT_INSIDE) {{ if (ihit.material == TELEPORT) {{ ihit.material = {}; }} if (ihit.material == TELEPORT_SUBSPACE) {{ ihit.material = {}; ihit.in_subspace = true; }} i = ihit; i.hit.n = normalize(adjugate({}) * i.hit.n); }}\n\n",
                                        material,
                                        material,
                                        matrix.normal_name()
                                    ));
                                };
                                let a = Object::get_name(a?, &self.matrices)?;
                                let b = Object::get_name(b?, &self.matrices)?;
                                add(&a, true, format!("teleport_{}_1_M", pos));
                                add(&b, false, format!("teleport_{}_2_M", pos));
                            }
                        };
                        match in_subspace {
                            SubspaceType::Normal | SubspaceType::Subspace => result.add_string("}"),
                            SubspaceType::Both => {},
                        }
                    },
                }
                result.add_string("\n");
            }
            result
        });

        storages.insert("intersection_material_functions".to_owned(), {
            let mut result = StringStorage::default();

            for (pos, (id, _)) in self.intersection_materials.visible_elements().enumerate() {
                let object = self.intersection_materials.get(id, &()).unwrap();
                result.add_string(format!(
                    "SceneIntersectionWithMaterial intersect_material_{}(Ray r) {{\n",
                    pos
                ));
                result.add_identifier_string(id, &object.0 .0 .0);
                result.add_string("\n}\n");
            }
            result
        });

        storages.insert("intersection_material_processing".to_owned(), {
            let mut result = StringStorage::default();
            for (pos, (_, _)) in self.intersection_materials.visible_elements().enumerate() {
                result.add_string(format!("hit = intersect_material_{}(r);\n", pos));
                result.add_string(
                    "if (nearer(result.scene.hit, hit.scene.hit)) { result = hit; }\n\n",
                );
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

        storages.insert("skybox_processing".to_owned(), {
            let mut result = StringStorage::default();
            if let Some(skybox_texture) = &self.skybox {
                result.add_string("vec4 rd2 = _camera_mul_inv * r.d;");
                result.add_string("float u = atan(rd2.z, rd2.x);");
                result.add_string("float v = atan(sqrt(rd2.x * rd2.x + rd2.z * rd2.z), rd2.y);");
                result.add_string(format!("vec3 not_found_color = sqrvec(texture({skybox_texture}_tex, vec2((u/PI+1.)/2., v/PI)).rgb);"));
            } else {
                result.add_string("vec3 not_found_color = color(0.6, 0.6, 0.6);");
            }
            result
        });

        // Choose different for cycles: one uses variables, other uses numbers (variable one not compiles on some systems)
        let mut res = apply_template(FRAGMENT_SHADER, storages);
        let mut res_storage = String::new();
        for line in res.storage.lines() {
            let number_for = line.contains("!FOR_NUMBER!");
            let variable_for = line.contains("!FOR_VARIABLE!");
            let antialiasing_line = line.contains("!ANTIALIASING!");
            let camera_teleportation_line = line.contains("!CAMERA_TELEPORTATION!");
            let glsl_100 = line.contains("!GLSL100!");
            let glsl_300 = line.contains("!GLSL300!");
            if (number_for && data.for_prefer_variable)
                || (variable_for && !data.for_prefer_variable)
                || (antialiasing_line && data.disable_antialiasing)
                || (camera_teleportation_line && data.disable_camera_teleportation)
                || (glsl_100 && data.use_300_version)
                || (glsl_300 && !data.use_300_version)
            {
                // skip line
            } else {
                res_storage += line;
            }
            res_storage += "\n";
        }
        res.storage = res_storage.trim().to_string();

        Some(res)
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
                macroquad::prelude::ShaderSource::Glsl {
                    vertex: if data.use_300_version {
                        VERTEX_SHADER_300
                    } else {
                        VERTEX_SHADER_100
                    },
                    fragment: &code.storage,
                },
                MaterialParams {
                    uniforms: self.uniforms(data)?,
                    textures: self.textures(),
                    ..Default::default()
                },
            )
            .map_err(|err| {
                let error_message = match err {
                    macroquad::Error::ShaderError(
                        macroquad::prelude::ShaderError::CompilationError { error_message, .. },
                    ) => error_message,
                    macroquad::Error::ShaderError(macroquad::prelude::ShaderError::LinkError(
                        msg,
                    )) => msg,
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
    fn init_stage(&mut self, stage: CurrentStage, memory: &mut egui::Memory) -> WhatChanged {
        match stage {
            CurrentStage::Animation(id) => {
                let stage = self.animation_stages.get_original(id).unwrap();
                stage
                    .uniforms
                    .init_stage(&mut self.uniforms, &self.dev_stage.uniforms);
                stage
                    .matrices
                    .init_stage(&mut self.matrices, &self.dev_stage.matrices);

                if let Some(cam) = stage.set_cam {
                    memory
                        .data
                        .insert_persisted(egui::Id::new("CurrentCam"), CurrentCam(cam));
                } else {
                    memory
                        .data
                        .insert_persisted(egui::Id::new("CurrentCam"), CurrentCam(None));
                }
            }
            CurrentStage::Dev => {
                self.dev_stage.uniforms.init_stage(&mut self.uniforms);
                self.dev_stage.matrices.init_stage(&mut self.matrices);
                memory
                    .data
                    .insert_persisted(egui::Id::new("CurrentCam"), CurrentCam(None));
            }
            CurrentStage::RealAnimation(id) => {
                let animation = self.animations.get_original(id).unwrap();
                if animation.animation_stage != CurrentStage::RealAnimation(id) {
                    drop(self.init_stage(animation.animation_stage, memory));
                } else {
                    crate::error!(format, "Initialization recursion!",);
                }
                let animation = self.animations.get_original(id).unwrap().clone();
                animation.uniforms.init_stage(&mut self.uniforms);
                animation.matrices.init_stage(&mut self.matrices);

                let disable_cam_interp_id = egui::Id::new("RealAnimationDisableCamInterpolation");
                let disable_cam_interp = memory
                    .data
                    .get_persisted::<bool>(disable_cam_interp_id)
                    .unwrap_or(false);

                if !disable_cam_interp {
                    let cam_start = self.get_start_cam(&animation, id);
                    if let Some(cam) = cam_start {
                        memory
                            .data
                            .insert_persisted(egui::Id::new("CurrentCam"), CurrentCam(Some(cam)));
                    }
                }
            }
        }
        self.current_stage = stage;
        WhatChanged::from_uniform(true)
    }

    pub fn init_stage_by_name(&mut self, name: &str, memory: &mut egui::Memory) -> Option<()> {
        let elements = self
            .animation_stages
            .visible_elements()
            .map(|(x, name)| (x, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name2) in elements {
            if name2 == name {
                drop(self.init_stage(CurrentStage::Animation(id), memory));
                return Some(());
            }
        }
        None
    }

    pub fn init_animation_by_name(&mut self, name: &str, memory: &mut egui::Memory) -> Option<()> {
        let elements = self
            .animations
            .visible_elements()
            .map(|(x, name)| (x, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name2) in elements {
            if name2 == name {
                drop(self.init_stage(CurrentStage::RealAnimation(id), memory));
                return Some(());
            }
        }
        None
    }

    pub fn init_animation_by_position(
        &mut self,
        position: usize,
        memory: &mut egui::Memory,
    ) -> Option<()> {
        let element = self.animations.visible_elements().nth(position)?.0;
        drop(self.init_stage(CurrentStage::RealAnimation(element), memory));
        Some(())
    }

    pub fn animations_len(&mut self) -> usize {
        self.animations.len()
    }

    pub fn is_current_stage_real_animation(&self) -> bool {
        matches!(self.current_stage, CurrentStage::RealAnimation(_))
    }

    pub fn get_current_animation_name(&self) -> Option<&str> {
        if let CurrentStage::RealAnimation(id) = self.current_stage {
            Some(self.animations.get_name(id)??)
        } else {
            None
        }
    }

    pub fn get_current_animation_duration(&self) -> Option<f64> {
        if let CurrentStage::RealAnimation(id) = self.current_stage {
            Some(self.animations.get_original(id)?.duration)
        } else {
            None
        }
    }

    pub fn get_start_cam(&self, anim: &RealAnimation, id: RealAnimationId) -> Option<CameraId> {
        if anim.use_prev_cam {
            for (a, b) in self
                .animations
                .visible_elements()
                .map(|(id, _)| id)
                .zip(self.animations.visible_elements().map(|(id, _)| id).skip(1))
            {
                if b == id {
                    let prev = self.animations.get_original(a).unwrap();
                    return self.get_end_cam(prev, a);
                }
            }
            None
        } else if let Some(get_end_cam) = anim.use_any_cam_as_start {
            let any_id = anim.cam_any_start?;
            let any = self.animations.get_original(any_id).unwrap();
            if get_end_cam {
                self.get_end_cam(any, any_id)
            } else {
                self.get_start_cam(any, any_id)
            }
        } else {
            anim.cam_start
        }
    }

    pub fn get_end_cam(&self, anim: &RealAnimation, id: RealAnimationId) -> Option<CameraId> {
        if anim.use_start_cam_as_end {
            self.get_start_cam(anim, id)
        } else if let Some(get_end_cam) = anim.use_any_cam_as_end {
            let any_id = anim.cam_any_end?;
            let any = self.animations.get_original(any_id).unwrap();
            if get_end_cam {
                self.get_end_cam(any, any_id)
            } else {
                self.get_start_cam(any, any_id)
            }
        } else {
            anim.cam_end
        }
    }

    pub fn total_animation_duration(&self) -> f64 {
        self.animations
            .visible_elements()
            .map(|(id, _)| self.animations.get_original(id).unwrap().duration)
            .sum()
    }

    pub fn update(&mut self, memory: &mut egui::Memory, data: &mut Data, mut time: f64) {
        let total_time;
        if self.animation_stage_edit_state {
            drop(self.init_stage(self.current_stage, memory));
        }

        if self.run_animations {
            let total_duration = self.total_animation_duration();
            if total_duration > 0.0 {
                time %= total_duration;
                total_time = time;
            } else {
                time = 0.0;
                total_time = 0.0;
            }
            for id in self
                .animations
                .visible_elements()
                .map(|(id, _)| id)
                .collect::<Vec<_>>()
            {
                let duration = self.animations.get_original(id).unwrap().duration;
                if time < duration {
                    if self.current_stage != CurrentStage::RealAnimation(id) {
                        drop(self.init_stage(CurrentStage::RealAnimation(id), memory));
                    }
                    time /= duration;
                    break;
                } else {
                    time -= duration;
                }
            }
        } else if let CurrentStage::RealAnimation(id) = self.current_stage {
            let duration = self.animations.get_original(id).unwrap().duration;
            if duration > 0.0 {
                let mut local_seconds = time % duration;

                // Override time with manual value from UI slider if enabled.
                let enabled_id = egui::Id::new("RealAnimationManualTimeEnabled");
                let value_id = egui::Id::new("RealAnimationManualTimeValue");

                if memory
                    .data
                    .get_persisted::<bool>(enabled_id)
                    .unwrap_or(false)
                {
                    if let Some(value) = memory.data.get_persisted::<f64>(value_id) {
                        let clamped = value.clamp(0.0, 1.0);
                        time = clamped;
                        local_seconds = clamped * duration;
                    } else {
                        time = local_seconds / duration;
                    }
                } else {
                    time = local_seconds / duration;
                }

                // Sum durations of all previous real animations in visible order.
                let prefix = self
                    .animations
                    .visible_elements()
                    .map(|(aid, _)| aid)
                    .take_while(|aid| *aid != id)
                    .map(|aid| self.animations.get_original(aid).unwrap().duration)
                    .sum::<f64>();

                total_time = prefix + local_seconds;
            } else {
                time = 0.0;
                total_time = 0.0;
            }
        } else {
            total_time = time;
        }
        data.formulas_cache.set_time(time);
        data.formulas_cache.set_total_time(total_time);

        if let CurrentStage::RealAnimation(id) = self.current_stage {
            let disable_cam_interp_id = egui::Id::new("RealAnimationDisableCamInterpolation");
            let disable_cam_interp = memory
                .data
                .get_persisted::<bool>(disable_cam_interp_id)
                .unwrap_or(false);
            let apply_once_id = egui::Id::new("RealAnimationDisableCamInterpolationOnce");
            let apply_once = memory
                .data
                .get_persisted::<bool>(apply_once_id)
                .unwrap_or(false);

            if !disable_cam_interp || apply_once {
                let animation = self.animations.get_original(id).unwrap();
                let cam_start = self.get_start_cam(animation, id);
                let cam_end = self.get_end_cam(animation, id);
                if let Some((cam1id, cam2id)) = cam_start.zip(cam_end) {
                    let cam1 = with_swapped!(x => (self.uniforms, data.formulas_cache);
                        self.cameras.get_original(cam1id).unwrap().get(&self.matrices, &x).unwrap());
                    let cam2 = with_swapped!(x => (self.uniforms, data.formulas_cache);
                        self.cameras.get_original(cam2id).unwrap().get(&self.matrices, &x).unwrap());

                    let t_raw = data.formulas_cache.get_time() % 1.;
                    let t = if let Some(opt_uid) = animation.cam_easing_uniform {
                        if let Some(uid) = opt_uid {
                            if let Some(value) = self.uniforms.get(uid, &data.formulas_cache) {
                                let mut v: f64 = value.into();
                                if !v.is_finite() {
                                    v = 0.0;
                                }
                                v.clamp(0.0, 1.0)
                            } else {
                                animation.cam_easing.ease(t_raw)
                            }
                        } else {
                            animation.cam_easing.ease(t_raw)
                        }
                    } else {
                        animation.cam_easing.ease(t_raw)
                    };

                    let override_matrix = t_raw < self.prev_t_raw || t_raw == 0.;

                    let cam = CalculatedCam {
                        look_at: cam1.look_at.lerp(cam2.look_at, t),
                        alpha: lerp(cam1.alpha..=cam2.alpha, t),
                        beta: lerp(cam1.beta..=cam2.beta, t),
                        r: lerp(cam1.r..=cam2.r, t),
                        in_subspace: cam1.in_subspace,
                        free_movement: cam1.free_movement,
                        matrix: cam1.matrix,
                        override_matrix,
                    };

                    memory
                        .data
                        .insert_persisted(egui::Id::new("OverrideCam"), cam);

                    self.prev_t_raw = t_raw;
                }

                if apply_once {
                    memory.data.insert_persisted(apply_once_id, false);
                }
            }
        }
    }

    pub fn select_stage_ui(&mut self, ui: &mut Ui, show_hidden_override: bool) -> WhatChanged {
        let show_hidden =
            ui.memory_mut(|memory| {
                *memory
                    .data
                    .get_persisted_mut_or_default::<ShowHiddenScenes>(egui::Id::new(
                        "ShowHiddenScenes",
                    ))
            })
            .0 || show_hidden_override;

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
                if !stage_value.hidden || show_hidden {
                    let mut text = stage_value.name.text(ui).to_owned();
                    if stage_value.hidden {
                        text = "* ".to_owned() + &text;
                    }
                    ui.radio_value(stage, CurrentStage::Animation(id), text);
                    if *stage != previous && *stage == CurrentStage::Animation(id) {
                        changed |= ui.memory_mut(|memory| self.init_stage(*stage, memory));
                    }
                }
            }
        });
        self.current_stage = current_stage;
        changed
    }

    pub fn control_animation_parameters(&mut self, ui: &mut Ui) -> WhatChanged {
        ui.label("Control animations:");

        let mut changed = WhatChanged::default();

        let enabled_id = egui::Id::new("RealAnimationManualTimeEnabled");
        let value_id = egui::Id::new("RealAnimationManualTimeValue");
        let free_cam_id = egui::Id::new("RealAnimationDisableCamInterpolation");

        ui.horizontal(|ui| {
            let mut manual_enabled = ui
                .memory_mut(|memory| *memory.data.get_persisted_mut_or_default::<bool>(enabled_id));
            changed.uniform |= ui.checkbox(&mut manual_enabled, "Manual time").changed();
            ui.memory_mut(|memory| {
                memory.data.insert_persisted(enabled_id, manual_enabled);
            });

            if manual_enabled {
                let mut manual_value = ui.memory_mut(|memory| {
                    *memory.data.get_persisted_mut_or_default::<f64>(value_id)
                });
                changed.uniform |= ui
                    .add(
                        egui::Slider::new(&mut manual_value, 0.0..=1.0)
                            .clamping(SliderClamping::Always),
                    )
                    .changed();
                ui.memory_mut(|memory| {
                    memory.data.insert_persisted(value_id, manual_value);
                });
            }
        });

        let mut free_cam = ui.memory_mut(|memory| {
            *memory
                .data
                .get_persisted_mut_or_default::<bool>(free_cam_id)
        });
        let prev_free_cam = free_cam;
        changed.uniform |= ui.checkbox(&mut free_cam, "Free cam").changed();
        ui.memory_mut(|memory| {
            memory.data.insert_persisted(free_cam_id, free_cam);
            if !prev_free_cam && free_cam {
                memory.data.insert_persisted(
                    egui::Id::new("RealAnimationDisableCamInterpolationOnce"),
                    true,
                );
            }
        });

        changed
    }

    pub fn select_animation_ui(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let mut current_stage = self.current_stage;
        changed.uniform |= check_changed(&mut current_stage, |stage| {
            let previous = *stage;
            let elements = self
                .animations
                .visible_elements()
                .map(|(id, name)| (id, name.to_owned()))
                .collect::<Vec<_>>();
            for (id, name) in elements {
                ui.radio_value(stage, CurrentStage::RealAnimation(id), name);
                if *stage != previous && *stage == CurrentStage::RealAnimation(id) {
                    changed |= ui.memory_mut(|memory| self.init_stage(*stage, memory));
                }
            }
        });
        self.current_stage = current_stage;
        changed
    }

    pub fn control_egui(&mut self, ui: &mut Ui, data: &mut Data) -> WhatChanged {
        let mut changed = WhatChanged::default();
        changed |= self.user_uniforms.user_egui(
            ui,
            &mut self.uniforms,
            &mut self.matrices,
            &mut self.elements_descriptions,
            egui::Id::new("control_egui"),
        );

        if self.animation_stages.len() != 0 {
            changed |= self.select_stage_ui(ui, false);
            ui.separator();
            match self.current_stage {
                CurrentStage::Animation(stage) => {
                    if let Some(stage) = self.animation_stages.get_original(stage) {
                        with_swapped!(x => (self.uniforms, data.formulas_cache);
                            changed |= stage.user_egui(ui, &mut x, &mut self.matrices, &mut self.cameras, &mut self.elements_descriptions, egui::Id::new("control_egui 2")));
                    } else {
                        self.current_stage = CurrentStage::Dev;
                    }
                }
                CurrentStage::Dev => {
                    ui.label("Select any stage");
                }
                CurrentStage::RealAnimation(_id) => {
                    ui.label("Animation is being played");
                }
            }
        }

        changed
    }
}

const FRAGMENT_SHADER: &str = include_str!("../frag.glsl");

pub const LIBRARY: &str = include_str!("../library.glsl");

const VERTEX_SHADER_100: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;

varying vec2 uv;
varying vec2 uv_screen;
varying float pixel_size;

uniform mat4 Model;
uniform mat4 Projection;

uniform vec2 Center;
uniform vec2 _resolution;

void main() {
    vec4 res = Projection * Model * vec4(position, 1);

    float coef = min(_resolution.x, _resolution.y);
    uv_screen = (position.xy - _resolution/2.) / coef * 2.;
    uv = position.xy;
    pixel_size = 1. / coef;

    gl_Position = res;
}
";

const VERTEX_SHADER_300: &str = "#version 300 es
in vec3 position;
in vec2 texcoord;

out vec2 uv;
out vec2 uv_screen;
out float pixel_size;

uniform mat4 Model;
uniform mat4 Projection;

uniform vec2 Center;
uniform vec2 _resolution;

void main() {
    vec4 res = Projection * Model * vec4(position, 1);

    float coef = min(_resolution.x, _resolution.y);
    uv_screen = (position.xy - _resolution/2.) / coef * 2.;
    uv = position.xy;
    pixel_size = 1. / coef;

    gl_Position = res;
}
";
