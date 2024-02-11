use gesture_recognizer::*;
use glam::{DMat4, DVec2, DVec3, DVec4};
use macroquad::prelude::is_key_pressed;
use portal::gui::camera::CalculatedCam;
use portal::gui::camera::CameraId;
use portal::gui::camera::CurrentCam;
use portal::gui::camera::OriginalCam;
use portal::gui::eng_rus::EngRusSettings;
use portal::gui::eng_rus::EngRusText;
use portal::gui::scenes::ShowHiddenScenes;
use portal::with_swapped;
use std::f64::consts::PI;

use macroquad::prelude::{
    clamp, clear_background, draw_rectangle, draw_texture_ex, get_screen_data,
    gl_use_default_material, gl_use_material, is_mouse_button_down, mouse_position_local,
    mouse_wheel, next_frame, screen_height, screen_width, set_default_camera, Conf,
    DrawTextureParams, MouseButton, Texture2D, BLACK, WHITE,
};
use portal::gui::scenes::Scenes;
use portal::gui::{common::*, scene::*, texture::*};

use egui::{DragValue, Ui};

struct RotateAroundCam {
    look_at: DVec3,
    alpha: f64,
    beta: f64,
    r: f64,
    previous_mouse: DVec2,

    from: Option<CameraId>,

    mouse_sensitivity: f64,
    scale_factor: f64,
    view_angle: f64,
    use_panini_projection: bool,
    panini_param: f64,

    inverse_x: bool,
    inverse_y: bool,

    // crutch variables, because touch processed in function where we can't pass parameters
    mouse_over_canvas_right_now: bool,
    is_something_changed: bool,
    current_dpi: f64,
    current_render_scale: f32,

    start_touch_scale: f64,

    dpi_change_start_pos: Point,
    dpi_change_start_dpi: f64,
    new_dpi: Option<f64>,

    render_scale_change_start_pos: Point,
    render_scale_change_start_value: f32,
    new_render_scale: Option<f32>,
}

impl RotateAroundCam {
    const BETA_MIN: f64 = 0.01;
    const BETA_MAX: f64 = PI - 0.01;

    fn new() -> Self {
        Self {
            look_at: DVec3::new(0., 0., 0.),
            alpha: deg2rad(81.),
            beta: deg2rad(64.),
            r: 3.5,
            previous_mouse: DVec2::default(),

            from: None,

            mouse_sensitivity: 1.4,
            scale_factor: 1.1,
            view_angle: deg2rad(90.),

            use_panini_projection: false,
            panini_param: 1.0,

            inverse_x: false,
            inverse_y: false,

            mouse_over_canvas_right_now: false,
            is_something_changed: false,
            start_touch_scale: 1.0,
            current_dpi: 1.0,
            current_render_scale: 1.0,

            dpi_change_start_dpi: 1.0,
            dpi_change_start_pos: Default::default(),
            new_dpi: None,

            render_scale_change_start_pos: Default::default(),
            render_scale_change_start_value: 1.0,
            new_render_scale: None,
        }
    }

    fn get_calculated_cam(&self) -> CalculatedCam {
        CalculatedCam {
            look_at: self.look_at,
            alpha: self.alpha,
            beta: self.beta,
            r: self.r,
        }
    }

    fn process_mouse_offset(&mut self, x: f64, y: f64) {
        let mut dalpha = x * self.mouse_sensitivity / 800.;
        let mut dbeta = -y * self.mouse_sensitivity / 800.;

        if self.inverse_x {
            dalpha *= -1.0;
        }
        if self.inverse_y {
            dbeta *= -1.0;
        }

        self.alpha += dalpha;
        if self.alpha > 2.0 * PI {
            self.alpha -= 2.0 * PI;
        }
        if self.alpha < 0. {
            self.alpha += 2.0 * PI;
        }
        self.beta = clamp(self.beta + dbeta, Self::BETA_MIN, Self::BETA_MAX);
    }

    fn process_mouse_and_keys(
        &mut self,
        mouse_over_canvas: bool,
        memory: &mut egui::Memory,
    ) -> bool {
        let mut is_something_changed = self.is_something_changed;

        let mouse_pos: DVec2 = glam::Vec2::from(<[f32; 2]>::from(mouse_position_local())).as_f64();

        if is_mouse_button_down(MouseButton::Left) && mouse_over_canvas {
            let size = mymax(screen_width().into(), screen_height().into());
            self.process_mouse_offset(
                (mouse_pos.x - self.previous_mouse.x) * size,
                (mouse_pos.y - self.previous_mouse.y) * size,
            );
            is_something_changed = true;
        }

        let wheel_value = mouse_wheel().1;
        if mouse_over_canvas {
            if wheel_value > 0. {
                self.r *= 1.0 / self.scale_factor;
                is_something_changed = true;
            } else if wheel_value < 0. {
                self.r *= self.scale_factor;
                is_something_changed = true;
            }
            self.r = clamp(self.r, 0.01, 100.);
        }

        if is_something_changed {
            memory
                .data
                .insert_persisted(egui::Id::new("CalculatedCam"), self.get_calculated_cam());
        }

        self.previous_mouse = mouse_pos;

        is_something_changed
    }

    fn get_matrix(&self) -> DMat4 {
        let pos = DVec3::new(
            self.beta.sin() * self.alpha.cos(),
            self.beta.cos(),
            self.beta.sin() * self.alpha.sin(),
        ) * self.r
            + self.look_at;

        let k = (self.look_at - pos).normalize();
        let i = k.cross(DVec3::new(0., 1., 0.)).normalize();
        let j = k.cross(i).normalize();

        DMat4::from_cols(
            DVec4::new(i.x, i.y, i.z, 0.),
            DVec4::new(j.x, j.y, j.z, 0.),
            DVec4::new(k.x, k.y, k.z, 0.),
            DVec4::new(pos.x, pos.y, pos.z, 1.),
        )
    }

    fn set_cam(&mut self, s: &CamSettings) {
        self.look_at = DVec3::new(s.look_at.x, s.look_at.y, s.look_at.z);
        self.alpha = s.alpha;
        self.beta = s.beta;
        self.r = s.r;
    }

    fn get_cam(&mut self, cam_settings: &mut CamSettings) {
        cam_settings.look_at = DVec3::new(self.look_at.x, self.look_at.y, self.look_at.z);
        cam_settings.alpha = self.alpha;
        cam_settings.beta = self.beta;
        cam_settings.r = self.r;
    }
}

impl macroquad::miniquad::EventHandler for Window {
    fn update(&mut self) {}

    fn draw(&mut self) {}

    fn touch_event(&mut self, phase: macroquad::miniquad::TouchPhase, id: u64, x: f32, y: f32) {
        use macroquad::miniquad::TouchPhase::*;
        use TouchType::*;
        let phase = match phase {
            Started => Start,
            Moved => Move,
            Ended => End,
            Cancelled => End,
        };
        self.gesture_recognizer
            .process(&mut self.cam, phase, id, x, y);
    }
}

impl GestureEvents for RotateAroundCam {
    fn touch_one_move(&mut self, _pos: &Point, offset: &Point) {
        if self.mouse_over_canvas_right_now {
            self.process_mouse_offset(offset.x as f64, offset.y as f64);
            self.is_something_changed = true;
        }
    }

    fn touch_scale_start(&mut self, _pos: &Point) {
        self.start_touch_scale = self.r;
    }
    fn touch_scale_change(&mut self, scale: f32, _pos: &Point, offset: &Point) {
        if self.mouse_over_canvas_right_now {
            self.process_mouse_offset(offset.x as f64, offset.y as f64);

            self.r = self.start_touch_scale / scale as f64;
            self.r = clamp(self.r, 0.01, 100.);

            self.is_something_changed = true;
        }
    }

    fn touch_three_start(&mut self, pos: &Point) {
        self.dpi_change_start_pos = pos.clone();
        self.dpi_change_start_dpi = self.current_dpi;
    }
    fn touch_three_move(&mut self, pos: &Point, _offset: &Point) {
        let offset = pos.clone() - &self.dpi_change_start_pos;
        self.new_dpi =
            Some(self.dpi_change_start_dpi * (1.2_f64).powf((offset.x + offset.y) as f64 / 500.));
    }

    fn touch_four_start(&mut self, pos: &Point) {
        self.render_scale_change_start_pos = pos.clone();
        self.render_scale_change_start_value = self.current_render_scale;
    }
    fn touch_four_move(&mut self, pos: &Point, _offset: &Point) {
        let offset = pos.clone() - &self.render_scale_change_start_pos;
        self.new_render_scale = Some(clamp(
            self.render_scale_change_start_value * (1.2_f32).powf((offset.x + offset.y) / 100.),
            0.01,
            1.0,
        ));
    }
}

impl RotateAroundCam {
    fn egui(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Look at: ");
            ui.label("X");
            changed |= egui_f64(ui, &mut self.look_at.x);
            ui.separator();
            ui.label("Y");
            changed |= egui_f64(ui, &mut self.look_at.y);
            ui.separator();
            ui.label("Z");
            changed |= egui_f64(ui, &mut self.look_at.z);
            ui.separator();
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("α");
            changed |= check_changed(&mut self.alpha, |alpha| {
                let mut current = rad2deg(*alpha);
                ui.add(
                    DragValue::new(&mut current)
                        .speed(1.0)
                        .suffix("°")
                        .min_decimals(0)
                        .max_decimals(1),
                );
                *alpha = deg2rad(current);
            });
            ui.separator();
            ui.label("β");
            changed |= check_changed(&mut self.beta, |beta| {
                let mut current = rad2deg(*beta);
                ui.add(
                    DragValue::new(&mut current)
                        .speed(1.0)
                        .clamp_range(rad2deg(Self::BETA_MIN)..=rad2deg(Self::BETA_MAX))
                        .suffix("°")
                        .min_decimals(0)
                        .max_decimals(1),
                );
                *beta = deg2rad(current);
            });
            ui.separator();
            ui.label("R");
            changed |= check_changed(&mut self.r, |r| {
                ui.add(
                    DragValue::new(r)
                        .speed(0.01)
                        .clamp_range(0.01..=1000.0)
                        .min_decimals(0)
                        .max_decimals(2),
                );
            });
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Panini projection:");
            changed |= check_changed(&mut self.use_panini_projection, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
            });
            if !self.use_panini_projection {
                self.view_angle = macroquad::math::clamp(self.view_angle, 0.0, deg2rad(140.0));
            }
        });
        ui.horizontal(|ui| {
            let is_use = self.use_panini_projection;
            ui.label("Panini parameter:");
            changed |= check_changed(&mut self.panini_param, |param| {
                egui_with_enabled_by(ui, is_use, |ui| {
                    ui.add(egui::Slider::new(param, 0.0..=1.0));
                });
            });
        });

        let is_use = self.use_panini_projection;
        changed |= check_changed(&mut self.view_angle, |m| {
            let mut current = rad2deg(*m);
            ui.add(
                egui::Slider::new(&mut current, if is_use { 2.0..=250.0 } else { 2.0..=140.0 })
                    .text("View angle")
                    .suffix("°")
                    .clamp_to_range(true),
            );
            *m = deg2rad(current);
        });

        ui.separator();

        changed |= check_changed(&mut self.mouse_sensitivity, |m| {
            ui.add(egui::Slider::new(m, 0.0..=3.0).text("Mouse sensivity"));
        });
        changed |= check_changed(&mut self.scale_factor, |m| {
            ui.add(egui::Slider::new(m, 1.0..=2.0).text("Wheel R multiplier"));
        });

        ui.separator();

        ui.horizontal(|ui| {
            changed |= check_changed(&mut self.inverse_x, |m| {
                ui.checkbox(m, "Invert X-axis");
            });
            ui.separator();
            changed |= check_changed(&mut self.inverse_y, |m| {
                ui.checkbox(m, "Invert Y-axis");
            });
        });

        WhatChanged::from_uniform(changed)
    }
}

struct Window {
    scene: Scene,
    cam: RotateAroundCam,
    material: macroquad::material::Material,
    should_recompile: bool,
    dpi_set: bool,

    draw_menu: bool,
    welcome_opened: bool,
    control_scene_opened: bool,
    edit_scene_opened: bool,
    camera_settings_opened: bool,
    render_options_opened: bool,
    about_opened: bool,
    import_window: Option<String>,
    import_window_errors: Option<String>,

    error_message: Option<(String, String)>,

    data: Data,

    offset_after_material: f64,
    render_depth: i32,
    aa_count: i32,
    angle_color_disable: bool,
    grid_disable: bool,
    black_border_disable: bool,

    available_scenes: Scenes,

    about: EngRusText,
    welcome: EngRusText,

    scene_initted: bool,

    scene_name: &'static str,

    gesture_recognizer: GestureRecognizer,
    input_subscriber_id: usize,

    render_target: macroquad::prelude::RenderTarget,
    render_scale: f32,
}

impl Window {
    async fn new() -> Self {
        let available_scenes: Scenes = Default::default();

        let required_scene = quad_url::get_program_parameters()
            .iter()
            .filter_map(|x| {
                let x = quad_url::easy_parse(x)?;
                Some((x.0, x.1?))
            })
            .find(|(name, _)| *name == "scene")
            .and_then(|(_, value)| available_scenes.get_by_link(value));

        let default_scene = "room";

        let (scene_content, scene_name) = if let Some(result) = required_scene {
            result
        } else {
            available_scenes.get_by_link(default_scene).unwrap()
        };

        let mut scene: Scene = ron::from_str(scene_content).unwrap();

        let mut data: Data = Data {
            reload_textures: true,
            ..Default::default()
        };

        let mut material = scene
            .get_new_material(&data)
            .unwrap()
            .unwrap_or_else(|err| {
                portal::error!(
                    format,
                    "code:\n{}\n\nmessage:\n{}",
                    add_line_numbers(&err.0),
                    err.1
                );
                std::process::exit(1)
            });
        scene.set_uniforms(&mut material, &mut data);
        let mut result = Window {
            should_recompile: false,
            dpi_set: false,
            scene,
            cam: RotateAroundCam::new(),

            material,

            draw_menu: true,
            welcome_opened: true,
            control_scene_opened: scene_name != "Room",
            edit_scene_opened: false,
            camera_settings_opened: false,
            render_options_opened: false,
            about_opened: false,
            import_window: None,
            import_window_errors: None,

            error_message: None,

            data,

            offset_after_material: 0.005,
            render_depth: 100,
            aa_count: 1,
            angle_color_disable: false,
            grid_disable: false,
            black_border_disable: false,

            available_scenes,

            about: EngRusText {
                eng: include_str!("description.easymarkup.en").to_string(),
                rus: include_str!("description.easymarkup.ru").to_string(),
            },

            welcome: EngRusText {
                eng: include_str!("welcome.easymarkup.en").to_string(),
                rus: include_str!("welcome.easymarkup.ru").to_string(),
            },

            scene_initted: false,

            scene_name,

            gesture_recognizer: Default::default(),
            input_subscriber_id: macroquad::input::utils::register_input_subscriber(),

            render_target: macroquad::prelude::render_target(4000, 4000),
            render_scale: 0.5,
        };
        result
            .render_target
            .texture
            .set_filter(macroquad::prelude::FilterMode::Nearest);
        result.cam.set_cam(&result.scene.cam);
        result.offset_after_material = result.scene.cam.offset_after_material;
        result.reload_textures().await;
        result
    }

    async fn reload_textures(&mut self) {
        if self.data.reload_textures {
            self.data.reload_textures = false;
            self.data.texture_errors.0.clear();
            for (id, name) in self.scene.textures.visible_elements() {
                let path = self.scene.textures.get_original(id).unwrap();
                match macroquad::file::load_file(&path.0).await {
                    Ok(bytes) => {
                        let texture = Texture2D::from_file_with_format(&bytes[..], None);

                        self.material.set_texture(&TextureName::name(name), texture);
                    }
                    Err(file) => {
                        self.data.texture_errors.0.insert(name.to_string(), file);
                    }
                }
            }
        }
    }

    fn process_mouse_and_keys(&mut self, ctx: &egui::Context) -> bool {
        let mut is_something_changed = false;

        if !self.dpi_set {
            ctx.set_pixels_per_point(1.7);
            self.dpi_set = true;
        }

        self.data
            .formulas_cache
            .set_time(macroquad::miniquad::date::now());

        if !self.scene_initted {
            self.scene_initted = true;
            ctx.memory_mut(|memory| self.scene.init(&mut self.data, memory));
            ctx.memory_mut(|memory| {
                memory.data.insert_persisted(
                    egui::Id::new("OriginalCam"),
                    OriginalCam(self.cam.get_calculated_cam()),
                )
            });
        }

        let mut changed = WhatChanged::default();

        if is_key_pressed(macroquad::input::KeyCode::Escape) {
            self.draw_menu = !self.draw_menu;
        }
        if self.draw_menu {
            egui::containers::panel::TopBottomPanel::top("my top").show(ctx, |ui| {
                use egui::menu;

                menu::bar(ui, |ui| {
                    ui.menu_button("🗋 Load", |ui| {
                        if let Some((content, link, name)) = self.available_scenes.egui(ui) {
                            if self.scene_name == "Room" {
                                self.control_scene_opened = true;
                            }
                            let s = content;
                            // let old: OldScene = serde_json::from_str(&s).unwrap();
                            // *self = old.into();
                            self.scene = ron::from_str(s).unwrap();
                            ctx.memory_mut(|memory| self.scene.init(&mut self.data, memory));
                            self.material = self
                                .scene
                                .get_new_material(&self.data)
                                .unwrap()
                                .unwrap_or_else(|err| {
                                    portal::error!(
                                        format,
                                        "code:\n{}\n\nmessage:\n{}",
                                        add_line_numbers(&err.0),
                                        err.1
                                    );
                                    std::process::exit(1)
                                });
                            changed.uniform = true;
                            self.data.reload_textures = true;
                            self.cam.set_cam(&self.scene.cam);
                            ui.ctx().memory_mut(|memory| {
                                memory.data.insert_persisted(
                                    egui::Id::new("OriginalCam"),
                                    OriginalCam(self.cam.get_calculated_cam()),
                                )
                            });
                            self.offset_after_material = self.scene.cam.offset_after_material;
                            quad_url::set_program_parameter("scene", link);
                            self.scene_name = name;
                        }
                        ui.separator();
                        if ui.button("Import...").clicked() && self.import_window.is_none() {
                            self.import_window = Some("".to_owned());
                        }
                    });
                    if ui.button("↔ Control scene").clicked() {
                        self.control_scene_opened = true;
                    };
                    if ui.button("✏ Edit scene").clicked() {
                        self.edit_scene_opened = true;
                    }
                    if ui.button("📸 Camera settings").clicked() {
                        self.camera_settings_opened = true;
                    }
                    if ui.button("⛭ Rendering options").clicked() {
                        self.render_options_opened = true;
                    }
                    if ui.button("❓ About").clicked() {
                        self.about_opened = true;
                    }
                    EngRusSettings::egui(ui);
                });
            });
        }
        let mut edit_scene_opened = self.edit_scene_opened;

        let errors_count = self.scene.errors_count(0, &mut self.data);
        egui::Window::new(if errors_count > 0 {
            format!("Edit scene ({} err)", errors_count)
        } else {
            "Edit scene".to_owned()
        })
        .id(egui::Id::new("Edit scene"))
        .open(&mut edit_scene_opened)
        .vscroll(true)
        .hscroll(true)
        .show(ctx, |ui| {
            let (changed1, material) =
                self.scene
                    .egui(ui, &mut self.data, &mut self.should_recompile);

            changed |= changed1;

            if changed.shader {
                self.should_recompile = true;
            }

            if let Some(material) = material {
                match material {
                    Ok(material) => {
                        self.material = material;
                        self.error_message = None;
                    }
                    Err(err) => {
                        self.error_message = Some((err.0, err.1));
                        self.data.errors = err.2;
                    }
                }
            }
        });
        if let Some((code, message)) = self.error_message.as_ref() {
            if self.data.show_error_window {
                egui::Window::new("Error message")
                    .vscroll(true)
                    .default_width(700.)
                    .show(ctx, |ui| {
                        egui::CollapsingHeader::new("code")
                            .id_source(0)
                            .show(ui, |ui| {
                                ui.monospace(add_line_numbers(code));
                            });
                        egui::CollapsingHeader::new("message")
                            .id_source(1)
                            .show(ui, |ui| {
                                ui.monospace(message);
                            });
                        egui::CollapsingHeader::new("message to copy")
                            .id_source(2)
                            .show(ui, |ui| {
                                let mut clone = message.clone();
                                ui.add(
                                    egui::TextEdit::multiline(&mut clone)
                                        .font(egui::TextStyle::Monospace),
                                );
                            });
                    });
            }
        }

        {
            let mut not_close = true;
            let show_compiled_code = &mut self.data.show_compiled_code;
            let generated_code_show_text = &mut self.data.generated_code_show_text;
            if let Some(code) = show_compiled_code {
                egui::Window::new("Generated GLSL code")
                    .vscroll(true)
                    .open(&mut not_close)
                    .default_width(700.)
                    .show(ctx, |ui| {
                        egui_demo_lib::easy_mark::easy_mark(ui, r#"# What is this

This code is generated automatically based on:
- Uniforms
- Matrices
- Objects
- Material
- Library
First, predefined library is included, then uniforms, then user library, then intersection functions. So, you can use uniforms and predefined library in any your code.

---

# Code

"#);
                        ui.horizontal(|ui| {
                            ui.selectable_value(generated_code_show_text, false, "View");
                            ui.selectable_value(generated_code_show_text, true, "To copy");
                        });
                        if *generated_code_show_text {
                            ui.add(
                                egui::TextEdit::multiline(code)
                                    .font(egui::TextStyle::Monospace),
                            );
                        } else {
                            ui.monospace(&*code);
                        }
                    });
            }
            if !not_close {
                self.data.show_compiled_code = None;
            }
        }

        if self.scene_name == "Room" {
            egui::Window::new("Welcome!")
                .open(&mut self.welcome_opened)
                .vscroll(true)
                .anchor(egui::Align2::CENTER_CENTER, (0., 0.))
                .show(ctx, |ui| {
                    let text = self.welcome.text(ui);
                    egui_demo_lib::easy_mark::easy_mark(ui, text);
                });
        }

        {
            let mut not_remove_export = true;
            if let Some(to_export) = self.data.to_export.as_ref() {
                egui::Window::new("Export scene")
                    .open(&mut not_remove_export)
                    .vscroll(true)
                    .show(ctx, |ui| {
                        let mut clone = to_export.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut clone).font(egui::TextStyle::Monospace),
                        );
                    });
            }
            if !not_remove_export {
                self.data.to_export = None;
            }
            self.edit_scene_opened = edit_scene_opened;
        }

        {
            let mut opened = self.import_window.is_some();
            let mut import_window = self.import_window.clone();
            if let Some(content) = &mut import_window {
                egui::Window::new("Import scene")
                    .open(&mut opened)
                    .vscroll(true)
                    .show(ctx, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(content)
                                .font(egui::TextStyle::Monospace),
                        );
                        if ui.button("Recompile").clicked() {
                            match ron::from_str::<Scene>(content) {
                                Ok(scene) => {
                                    self.scene = scene;
                                    ui.memory_mut(|memory| self.scene.init(&mut self.data, memory));
                                    self.cam.set_cam(&self.scene.cam);
                                    self.offset_after_material = self.scene.cam.offset_after_material;
                                    changed.uniform = true;
                                    self.data.reload_textures = true;
                                    match self.scene.get_new_material(&self.data) {
                                        Some(Ok(material)) => {
                                            self.material = material;
                                        },
                                        Some(Err(_)) | None => {
                                            self.should_recompile = true;
                                            self.import_window_errors = Some("Errors in shaders, look into `Edit scene` window after pressing `Recompile`.".to_owned());
                                        },
                                    }
                                },
                                Err(err) => {
                                    self.import_window_errors = Some(err.to_string());
                                }
                            }
                        }

                        if let Some(err) = &self.import_window_errors {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.;
                                ui.add(egui::Label::new(egui::RichText::new("Error: ").color(egui::Color32::RED)));
                                ui.label(err);
                            });
                        }
                    });
                self.import_window = import_window;
            }
            if !opened {
                self.import_window = None;
            }
        }

        {
            let mut control_scene_opened = self.control_scene_opened;
            egui::Window::new("Control scene")
                .open(&mut control_scene_opened)
                .vscroll(true)
                .hscroll(true)
                .show(ctx, |ui| {
                    ui.spacing_mut().slider_width = 200.;
                    ui.vertical_centered(|ui| {
                        ui.heading(self.scene_name);
                    });
                    ui.separator();
                    ui.collapsing("Description", |ui| {
                        let text = self.scene.desc.text(ui);
                        egui_demo_lib::easy_mark::easy_mark(ui, text);
                    });
                    changed |= self.scene.control_egui(ui, &mut self.data);
                });
            self.control_scene_opened = control_scene_opened;
        }

        {
            egui::Window::new("GLSL library")
                .open(&mut self.data.show_glsl_library)
                .vscroll(true)
                .default_width(600.)
                .show(ctx, |ui| {
                    egui_demo_lib::easy_mark::easy_mark(ui, "# What is this\n\nThis is predefined library GLSL code, it included before user code, so you can use functions and etc. from this.\n\n---\n\n# Code\n\n");
                    ui.separator();
                    ui.monospace(LIBRARY);
                });
        }

        {
            let mut camera_settings_opened = self.camera_settings_opened;
            egui::Window::new("Camera settings")
                .open(&mut camera_settings_opened)
                .vscroll(true)
                .show(ctx, |ui| {
                    changed |= self.cam.egui(ui);
                });
            self.camera_settings_opened = camera_settings_opened;
        }

        {
            let mut render_options_opened = self.render_options_opened;
            egui::Window::new("Render options")
                .open(&mut render_options_opened)
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label("Offset after material:");
                    changed.uniform |= check_changed(&mut self.offset_after_material, |offset| {
                        const MIN: f64 = 0.0000001;
                        const MAX: f64 = 0.1;
                        ui.add(
                            egui::Slider::new(offset, MIN..=MAX)
                                .logarithmic(true)
                                .clamp_to_range(true)
                                .largest_finite(MAX)
                                .smallest_positive(MIN),
                        );
                    });
                    ui.label("(Ofsetting after ray being teleported, reflected, refracted)");
                    ui.separator();
                    ui.label("Render depth:");
                    changed.uniform |= check_changed(&mut self.render_depth, |depth| {
                        ui.add(egui::Slider::new(depth, 0..=1000).clamp_to_range(true));
                    });
                    ui.label("(Max count of ray bounce after portal, reflect, refract)");
                    ui.separator();
                    ui.label("Disable darkening by angle with normal:");
                    changed.uniform |= egui_bool(ui, &mut self.angle_color_disable);
                    ui.label("(This increases resulting gif size if you capturing screen)");
                    ui.separator();
                    ui.label("Disable grid:");
                    changed.uniform |= egui_bool(ui, &mut self.grid_disable);
                    ui.label("(If you want extreme small gif)");
                    ui.separator();
                    ui.label("Antialiasing count:");
                    changed.uniform |= check_changed(&mut self.aa_count, |count| {
                        ui.add(egui::Slider::new(count, 1..=16).clamp_to_range(true));
                    });
                    ui.separator();
                    ui.label("Lower rendering resolution ratio (can significantly increase FPS):");
                    changed.uniform |= check_changed(&mut self.render_scale, |scale| {
                        ui.add(egui::Slider::new(scale, 0.01..=1.0).clamp_to_range(true));
                    });
                    ui.label("(Render scale can be changed using four fingers, add one finger at a time to prevent triggering system four-finger gestures)");
                    ui.separator();
                    ui.label("Disable small black border:");
                    changed.uniform |= egui_bool(ui, &mut self.black_border_disable);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Interface DPI:");
                        if ui.button("⏶").clicked() {
                            ui.ctx()
                                .set_pixels_per_point(ui.ctx().pixels_per_point() * 1.2);
                        }
                        if ui.button("⏷").clicked() {
                            ui.ctx()
                                .set_pixels_per_point(ui.ctx().pixels_per_point() / 1.2);
                        }
                        ui.label(format!(" ({})", ui.ctx().pixels_per_point()));
                    });
                    ui.label("(DPI can be changed using three fingers, add one finger at a time to prevent triggering system three-finger gestures)")
                });
            self.render_options_opened = render_options_opened;
        }

        {
            let mut about_opened = self.about_opened;
            egui::Window::new("Portal Explorer")
                .open(&mut about_opened)
                .vscroll(true)
                .show(ctx, |ui| {
                    let text = self.about.text(ui);
                    egui_demo_lib::easy_mark::easy_mark(ui, text);
                    ui.separator();

                    let mut checked = ui.memory_mut(|memory| {
                        *memory
                            .data
                            .get_persisted_mut_or_default::<ShowHiddenScenes>(egui::Id::new(
                                "ShowHiddenScenes",
                            ))
                    });
                    ui.checkbox(&mut checked.0, "Show hidden scenes :P");
                    ui.memory_mut(|memory| {
                        memory
                            .data
                            .insert_persisted(egui::Id::new("ShowHiddenScenes"), checked)
                    });
                });
            self.about_opened = about_opened;
        }

        let mouse_over_canvas = !ctx.wants_pointer_input() && !ctx.is_pointer_over_area();

        if changed.uniform || self.scene.use_time {
            self.scene.set_uniforms(&mut self.material, &mut self.data);
            self.set_uniforms();

            let current_cam = ctx.memory_mut(|memory| {
                memory
                    .data
                    .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                    .0
            });
            if self.cam.from != current_cam {
                let calculated_cam = if let Some(id) = current_cam {
                    // set getted camera

                    if self.cam.from.is_none() {
                        let calculated_cam = self.cam.get_calculated_cam();
                        ctx.memory_mut(|memory| {
                            memory.data.insert_persisted(
                                egui::Id::new("OriginalCam"),
                                OriginalCam(calculated_cam),
                            )
                        });
                    }

                    with_swapped!(x => (self.scene.uniforms, self.data.formulas_cache);
                        self.scene.cameras.get_original(id).unwrap().get(&self.scene.matrices, &x).unwrap())
                } else {
                    // set original camera
                    ctx.memory_mut(|memory| {
                        memory
                            .data
                            .get_persisted_mut_or_default::<OriginalCam>(egui::Id::new(
                                "OriginalCam",
                            ))
                            .0
                    })
                };

                self.cam.from = current_cam;
                self.cam.alpha = calculated_cam.alpha;
                self.cam.beta = calculated_cam.beta;
                self.cam.r = calculated_cam.r;
                self.cam.look_at = calculated_cam.look_at;
            } else if let Some(id) = self.cam.from {
                let calculated_cam = with_swapped!(x => (self.scene.uniforms, self.data.formulas_cache);
                    self.scene.cameras.get_original(id).unwrap().get(&self.scene.matrices, &x).unwrap());
                self.cam.look_at = calculated_cam.look_at;
            }

            is_something_changed = true;
        }

        self.cam.current_dpi = ctx.pixels_per_point() as f64;
        self.cam.current_render_scale = self.render_scale;
        self.cam.is_something_changed = false;
        self.cam.mouse_over_canvas_right_now = mouse_over_canvas;
        macroquad::input::utils::repeat_all_miniquad_input(self, self.input_subscriber_id);
        if let Some(new_dpi) = self.cam.new_dpi {
            ctx.set_pixels_per_point(new_dpi as f32);
            self.cam.new_dpi = None;
            is_something_changed = true;
        }
        if let Some(new_render_scale) = self.cam.new_render_scale {
            self.render_scale = new_render_scale;
            self.cam.new_render_scale = None;
            is_something_changed = true;
        }
        ctx.memory_mut(|memory| {
            is_something_changed |= self.cam.process_mouse_and_keys(mouse_over_canvas, memory);
        });

        is_something_changed
    }

    fn set_uniforms(&mut self) {
        self.cam.get_cam(&mut self.scene.cam);
        self.scene.cam.offset_after_material = self.offset_after_material;
        self.material.set_uniform(
            "_resolution",
            (
                screen_width() * self.render_scale,
                screen_height() * self.render_scale,
            ),
        );
        self.material
            .set_uniform("_camera", self.cam.get_matrix().as_f32());
        self.material
            .set_uniform("_view_angle", self.cam.view_angle as f32);
        self.material
            .set_uniform("_panini_param", self.cam.panini_param as f32);
        self.material.set_uniform(
            "_use_panini_projection",
            self.cam.use_panini_projection as i32,
        );
        self.material
            .set_uniform("_ray_tracing_depth", self.render_depth);
        self.material.set_uniform("_aa_count", self.aa_count);
        self.material
            .set_uniform("_offset_after_material", self.offset_after_material as f32);
        self.material
            .set_uniform("_angle_color_disable", self.angle_color_disable as i32);
        self.material
            .set_uniform("_grid_disable", self.grid_disable as i32);
        self.material
            .set_uniform("_black_border_disable", self.black_border_disable as i32);
    }

    fn draw(&mut self) {
        self.set_uniforms();

        if self.render_scale == 1.0 {
            gl_use_material(&self.material);
            draw_rectangle(0., 0., screen_width(), screen_height(), WHITE);
            gl_use_default_material();
        } else {
            macroquad::prelude::set_camera(&macroquad::prelude::Camera2D {
                zoom: macroquad::prelude::vec2(
                    1. / (self.render_target.texture.size().x / 2.),
                    1. / (self.render_target.texture.size().y / 2.),
                ),
                offset: macroquad::prelude::vec2(-1., -1.),
                render_target: Some(self.render_target.clone()),
                ..Default::default()
            });
            gl_use_material(&self.material);
            draw_rectangle(
                0.,
                0.,
                screen_width() * self.render_scale,
                screen_height() * self.render_scale,
                WHITE,
            );
            gl_use_default_material();

            set_default_camera();

            draw_texture_ex(
                &self.render_target.texture,
                0.,
                0.,
                WHITE,
                DrawTextureParams {
                    source: Some(macroquad::math::Rect::new(
                        0.,
                        0.,
                        screen_width() * self.render_scale,
                        screen_height() * self.render_scale,
                    )),
                    dest_size: Some(macroquad::prelude::vec2(screen_width(), screen_height())),
                    ..Default::default()
                },
            );
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Portal Explorer".to_owned(),
        high_dpi: true,
        window_resizable: true,
        // fullscreen: true,
        window_width: 1920,
        window_height: 1080,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    color_backtrace::install();

    let mut window = Window::new().await;

    let mut texture = Texture2D::from_image(&get_screen_data());
    let mut w = screen_width();
    let mut h = screen_height();
    let mut image_size_changed = true;

    let mut ui_changed_image = true;

    macroquad::input::simulate_mouse_with_touch(false);

    loop {
        clear_background(BLACK);

        if (screen_width() - w).abs() > 0.5 {
            w = screen_width();
            image_size_changed = true;
        }
        if (screen_height() - h).abs() > 0.5 {
            h = screen_height();
            image_size_changed = true;
        }
        if image_size_changed {
            texture = Texture2D::from_image(&get_screen_data());
        }

        if image_size_changed || ui_changed_image {
            window.draw();
            set_default_camera();
            texture.grab_screen();
            image_size_changed = false;
            ui_changed_image = false;
        } else {
            draw_texture_ex(
                &texture,
                0.,
                0.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(macroquad::prelude::Vec2::new(
                        screen_width(),
                        screen_height(),
                    )),
                    flip_y: true,
                    ..Default::default()
                },
            );
        }

        egui_macroquad::ui(|ctx| {
            ui_changed_image = window.process_mouse_and_keys(ctx);
        });
        egui_macroquad::draw();

        window.reload_textures().await;

        next_frame().await;
    }
}
