use egui::{DragValue, Ui};
use egui_macroquad::Egui;
use macroquad::prelude::*;
use miniquad_parameters::PROGRAM_PARAMETERS;
use std::f32::consts::PI;

mod gui;
use crate::gui::*;

struct RotateAroundCam {
    look_at: Vec3,
    alpha: f32,
    beta: f32,
    r: f32,
    previous_mouse: Vec2,

    mouse_sensitivity: f32,
    scale_factor: f32,
    view_angle: f32,
    use_panini_projection: bool,
    panini_param: f32,

    inverse_x: bool,
    inverse_y: bool,
}

impl RotateAroundCam {
    const BETA_MIN: f32 = 0.01;
    const BETA_MAX: f32 = PI - 0.01;

    fn new() -> Self {
        Self {
            look_at: Vec3::new(0., 0., 0.),
            alpha: deg2rad(81.),
            beta: deg2rad(64.),
            r: 3.5,
            previous_mouse: Vec2::default(),

            mouse_sensitivity: 1.4,
            scale_factor: 1.1,
            view_angle: deg2rad(90.),

            use_panini_projection: false,
            panini_param: 1.0,

            inverse_x: false,
            inverse_y: false,
        }
    }

    fn process_mouse_and_keys(&mut self, mouse_over_canvas: bool) -> bool {
        let mut is_something_changed = false;

        let mouse_pos: Vec2 = mouse_position_local();

        if is_mouse_button_down(MouseButton::Left) && mouse_over_canvas {
            let size = mymax(screen_width(), screen_height());
            let mut dalpha =
                (mouse_pos.x - self.previous_mouse.x) * self.mouse_sensitivity * size / 800.;
            let mut dbeta =
                -(mouse_pos.y - self.previous_mouse.y) * self.mouse_sensitivity * size / 800.;

            if self.inverse_x {
                dalpha *= -1.0;
            }
            if self.inverse_y {
                dbeta *= -1.0;
            }

            self.alpha += dalpha;
            self.beta = clamp(self.beta + dbeta, Self::BETA_MIN, Self::BETA_MAX);

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
        }

        self.previous_mouse = mouse_pos;

        return is_something_changed;
    }

    fn get_matrix(&self) -> Mat4 {
        let pos = Vec3::new(
            self.beta.sin() * self.alpha.cos(),
            self.beta.cos(),
            self.beta.sin() * self.alpha.sin(),
        ) * self.r
            + self.look_at;

        let k = (self.look_at - pos).normalize();
        let i = k.cross(Vec3::new(0., 1., 0.)).normalize();
        let j = k.cross(i).normalize();

        Mat4::from_cols(
            Vec4::new(i.x, i.y, i.z, 0.),
            Vec4::new(j.x, j.y, j.z, 0.),
            Vec4::new(k.x, k.y, k.z, 0.),
            Vec4::new(pos.x, pos.y, pos.z, 1.),
        )
    }
}

impl RotateAroundCam {
    fn egui(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Look at: ");
            ui.label("X");
            changed |= egui_f32(ui, &mut self.look_at.x);
            ui.separator();
            ui.label("Y");
            changed |= egui_f32(ui, &mut self.look_at.y);
            ui.separator();
            ui.label("Z");
            changed |= egui_f32(ui, &mut self.look_at.z);
            ui.separator();
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("α");
            changed |= check_changed(&mut self.alpha, |alpha| {
                let mut current = rad2deg(*alpha);
                ui.add(
                    DragValue::f32(&mut current)
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
                    DragValue::f32(&mut current)
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
                    DragValue::f32(r)
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
                    ui.add(egui::Slider::f32(param, 0.0..=1.0));
                });
            });
        });

        let is_use = self.use_panini_projection;
        changed |= check_changed(&mut self.view_angle, |m| {
            let mut current = rad2deg(*m);
            ui.add(
                egui::Slider::f32(
                    &mut current,
                    if is_use { 20.0..=250.0 } else { 20.0..=140.0 },
                )
                .text("View angle")
                .suffix("°")
                .clamp_to_range(true),
            );
            *m = deg2rad(current);
        });

        ui.separator();

        changed |= check_changed(&mut self.mouse_sensitivity, |m| {
            ui.add(egui::Slider::f32(m, 0.0..=3.0).text("Mouse sensivity"));
        });
        changed |= check_changed(&mut self.scale_factor, |m| {
            ui.add(egui::Slider::f32(m, 1.0..=2.0).text("Wheel R multiplier"));
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

    control_scene_opened: bool,
    edit_scene_opened: bool,
    camera_settings_opened: bool,
    render_options_opened: bool,
    about_opened: bool,

    error_message: Option<(String, String)>,

    data: Data,

    offset_after_material: f32,
    render_depth: i32,

    available_scenes: Vec<(String, String, String)>,
}

impl Window {
    async fn new() -> Self {
        let available_scenes: Vec<(String, String, String)> = vec![
            (
                "Monoportal",
                "monoportal_offset",
                include_str!("../scenes/monoportal_offset.json"),
            ),
            ("Misc", "misc", include_str!("../scenes/misc.json")),
        ]
        .into_iter()
        .map(|(a, b, c)| (a.to_owned(), b.to_owned(), c.to_owned()))
        .collect();

        let start1 = "--scene=";
        let start2 = "-s=";

        let default_scene = PROGRAM_PARAMETERS
            .iter()
            .find(|s| s.starts_with(start1) || s.starts_with(start2))
            .and_then(|s| {
                let mut s = &s[..];
                if s.starts_with(start1) {
                    s = &s[start1.len()..];
                } else if s.starts_with(start2) {
                    s = &s[start2.len()..];
                }

                available_scenes.iter().position(|(_, path, _)| path == s)
            })
            .unwrap_or(0);

        let mut data = Default::default();

        let mut scene: Scene = serde_json::from_str(&available_scenes[default_scene].2).unwrap();
        // let mut scene: Scene = serde_json::from_str::<OldScene>(&available_scenes[default_scene].2) .unwrap() .into();
        scene.init(&mut data);

        data.reload_textures = true;

        let material = scene.get_new_material().unwrap_or_else(|err| {
            println!("code:\n{}\n\nmessage:\n{}", add_line_numbers(&err.0), err.1);
            dbg!(&err);
            crate::miniquad::error!("code:\n{}\n\nmessage:\n{}", add_line_numbers(&err.0), err.1);
            std::process::exit(1)
        });
        scene.set_uniforms(material, &mut data, &scene.uniforms);
        let mut result = Window {
            should_recompile: false,
            scene,
            cam: RotateAroundCam::new(),

            material,

            control_scene_opened: true,
            edit_scene_opened: false,
            camera_settings_opened: false,
            render_options_opened: false,
            about_opened: false,

            error_message: None,

            data,

            offset_after_material: 0.0005,
            render_depth: 100,

            available_scenes,
        };
        result.reload_textures().await;
        result
    }

    async fn reload_textures(&mut self) {
        if self.data.reload_textures {
            self.data.reload_textures = false;
            self.data.texture_errors.clear();
            for (name, path) in self.scene.textures.iter() {
                match macroquad::file::load_file(&path.0).await {
                    Ok(bytes) => {
                        let context = unsafe { get_internal_gl().quad_context };

                        let texture = Texture2D::from_file_with_format(context, &bytes[..], None);

                        self.material.set_texture(&TextureName::name(name), texture);
                    }
                    Err(file) => {
                        self.data.texture_errors.insert(name.to_string(), file);
                    }
                }
            }
        }
    }

    fn process_mouse_and_keys(&mut self, ctx: &egui::CtxRef) -> bool {
        let mut is_something_changed = false;

        let mut changed = WhatChanged::default();

        egui::TopPanel::top("my top").show(ctx, |ui| {
            use egui::menu;
            menu::bar(ui, |ui| {
                menu::menu(ui, "🗋 Load", |ui| {
                    for (name, _, text) in &self.available_scenes {
                        if ui.button(name).clicked() {
                            let s = text;
                            // let old: OldScene = serde_json::from_str(&s).unwrap();
                            // *self = old.into();
                            self.scene = serde_json::from_str(&s).unwrap();
                            self.scene.init(&mut self.data);
                            self.material = self.scene.get_new_material().unwrap();
                            changed.uniform = true;
                        }
                    }
                });
                if ui.button("☑ Control scene").clicked() {
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
            });
        });
        let mut edit_scene_opened = self.edit_scene_opened;

        self.data.names = self.scene.matrices.names_iter().cloned().collect();
        self.data.formulas_names = self.scene.uniforms.names_iter().cloned().collect();
        let errors_count = self.scene.errors_count(0, &mut self.data);
        egui::Window::new(if errors_count > 0 {
            format!("Edit scene ({} err)", errors_count)
        } else {
            "Edit scene".to_owned()
        })
        .id(egui::Id::new("Edit scene"))
        .open(&mut edit_scene_opened)
        .scroll(true)
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
                    .scroll(true)
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
                                        .text_style(egui::TextStyle::Monospace),
                                );
                            });
                    });
            }
        }

        {
            let mut not_close = true;
            if let Some(code) = &self.data.show_compiled_code {
                egui::Window::new("Generated GLSL code")
                    .scroll(true)
                    .open(&mut not_close)
                    .default_width(700.)
                    .show(ctx, |ui| {
                        egui::experimental::easy_mark(ui, r#"# What is this

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
                        ui.monospace(code);
                    });
            }
            if !not_close {
                self.data.show_compiled_code = None;
            }
        }

        {
            let mut not_remove_export = true;
            if let Some(to_export) = self.data.to_export.as_ref() {
                egui::Window::new("Export scene")
                    .open(&mut not_remove_export)
                    .scroll(true)
                    .show(ctx, |ui| {
                        let mut clone = to_export.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut clone)
                                .text_style(egui::TextStyle::Monospace),
                        );
                    });
            }
            if !not_remove_export {
                self.data.to_export = None;
            }
            self.edit_scene_opened = edit_scene_opened;
        }

        {
            let mut control_scene_opened = self.control_scene_opened;
            egui::Window::new("Control scene")
                .open(&mut control_scene_opened)
                .show(ctx, |ui| {
                    changed |= self.scene.control_egui(ui, &mut self.data);
                });
            self.control_scene_opened = control_scene_opened;
        }

        {
            egui::Window::new("GLSL library")
                .open(&mut self.data.show_glsl_library)
                .scroll(true)
                .default_width(600.)
                .show(ctx, |ui| {
                    egui::experimental::easy_mark(ui, "# What is this\n\nThis is predefined library GLSL code, it included before user code, so you can use functions and etc. from this.\n\n---\n\n# Code\n\n");
                    ui.separator();
                    ui.monospace(gui::LIBRARY);
                });
        }

        {
            let mut camera_settings_opened = self.camera_settings_opened;
            egui::Window::new("Camera settings")
                .open(&mut camera_settings_opened)
                .show(ctx, |ui| {
                    changed |= self.cam.egui(ui);
                });
            self.camera_settings_opened = camera_settings_opened;
        }

        {
            let mut render_options_opened = self.render_options_opened;
            egui::Window::new("Render options")
                .open(&mut render_options_opened)
                .show(ctx, |ui| {
                    ui.label("Offset after material:");
                    changed.uniform |= check_changed(&mut self.offset_after_material, |offset| {
                        const MIN: f32 = 0.0000001;
                        const MAX: f32 = 0.1;
                        ui.add(
                            egui::Slider::f32(offset, MIN..=MAX)
                                .logarithmic(true)
                                .clamp_to_range(true)
                                .largest_finite(MAX.into())
                                .smallest_positive(MIN.into()),
                        );
                    });
                    ui.label("(Ofsetting after ray being teleported, reflected, refracted)");
                    ui.separator();
                    ui.label("Render depth:");
                    changed.uniform |= check_changed(&mut self.render_depth, |depth| {
                        ui.add(egui::Slider::i32(depth, 0..=100).clamp_to_range(true));
                    });
                    ui.label("(Max count of ray bounce after portal, reflect, refract)");
                });
            self.render_options_opened = render_options_opened;
        }

        {
            let mut about_opened = self.about_opened;
            egui::Window::new("Portal Explorer")
                .open(&mut about_opened)
                .show(ctx, |ui| {
                    egui::experimental::easy_mark(ui, include_str!("description.easymarkup"));
                });
            self.about_opened = about_opened;
        }

        let mouse_over_canvas = !ctx.wants_pointer_input() && !ctx.is_pointer_over_area();

        if changed.uniform {
            self.scene
                .set_uniforms(self.material, &mut self.data, &self.scene.uniforms);
            self.set_uniforms();
            is_something_changed = true;
        }

        is_something_changed |= self.cam.process_mouse_and_keys(mouse_over_canvas);

        return is_something_changed;
    }

    fn set_uniforms(&self) {
        self.material
            .set_uniform("_resolution", (screen_width(), screen_height()));
        self.material.set_uniform("_camera", self.cam.get_matrix());
        self.material
            .set_uniform("_view_angle", self.cam.view_angle);
        self.material
            .set_uniform("_panini_param", self.cam.panini_param);
        self.material.set_uniform(
            "_use_panini_projection",
            self.cam.use_panini_projection as i32,
        );
        self.material
            .set_uniform("_ray_tracing_depth", self.render_depth);
        self.material
            .set_uniform("_offset_after_material", self.offset_after_material);
    }

    fn draw(&self) {
        self.set_uniforms();

        gl_use_material(self.material);
        draw_rectangle(0., 0., screen_width(), screen_height(), WHITE);
        gl_use_default_material();
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Portal Explorer".to_owned(),
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut window = Window::new().await;

    let mut texture = load_texture_from_image(&get_screen_data());
    let mut w = screen_width();
    let mut h = screen_height();
    let mut image_size_changed = true;

    let mut egui = Egui::new();

    let mut ui_changed_image = true;

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
            texture = load_texture_from_image(&get_screen_data());
        }

        if image_size_changed || ui_changed_image {
            window.draw();
            set_default_camera();
            texture.grab_screen();
            image_size_changed = false;
            ui_changed_image = false;
        } else {
            draw_texture_ex(
                texture,
                0.,
                0.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(screen_width(), screen_height())),
                    flip_y: true,
                    ..Default::default()
                },
            );
        }

        egui.ui(|ctx| {
            ui_changed_image = window.process_mouse_and_keys(ctx);
        });

        window.reload_textures().await;

        next_frame().await;
    }
}
