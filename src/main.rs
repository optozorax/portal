use egui::{DragValue, Ui};
use egui_macroquad::Egui;
use macroquad::prelude::*;
use std::f32::consts::PI;

mod gui;
use crate::gui::*;

struct RotateAroundCam {
    alpha: f32,
    beta: f32,
    r: f32,
    previous_mouse: Vec2,

    mouse_sensitivity: f32,
    scale_factor: f32,
    view_angle: f32,
}

impl RotateAroundCam {
    const BETA_MIN: f32 = 0.01;
    const BETA_MAX: f32 = PI - 0.01;

    fn new() -> Self {
        Self {
            alpha: 0.,
            beta: 5. * PI / 7.,
            r: 3.5,
            previous_mouse: Vec2::default(),

            mouse_sensitivity: 1.4,
            scale_factor: 1.1,
            view_angle: deg2rad(80.),
        }
    }

    fn process_mouse_and_keys(&mut self, mouse_over_canvas: bool) -> bool {
        let mut is_something_changed = false;

        let mouse_pos: Vec2 = mouse_position_local();

        if is_mouse_button_down(MouseButton::Left) && mouse_over_canvas {
            let size = mymax(screen_width(), screen_height());
            let dalpha =
                (mouse_pos.x - self.previous_mouse.x) * self.mouse_sensitivity * size / 800.;
            let dbeta =
                (mouse_pos.y - self.previous_mouse.y) * self.mouse_sensitivity * size / 800.;

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
            -self.beta.sin() * self.alpha.cos(),
            self.beta.cos(),
            -self.beta.sin() * self.alpha.sin(),
        ) * self.r;
        let look_at = Vec3::new(0., 0., 0.);

        let h = (self.view_angle / 2.).tan();

        let k = (look_at - pos).normalize();
        let i = k.cross(Vec3::new(0., 1., 0.)).normalize() * h;
        let j = k.cross(i).normalize() * h;

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

        // TODO add inverse by X axis and inverse by Y axis

        changed |= check_changed(&mut self.mouse_sensitivity, |m| {
            ui.add(egui::Slider::f32(m, 0.0..=3.0).text("Mouse sensivity"));
        });
        changed |= check_changed(&mut self.scale_factor, |m| {
            ui.add(egui::Slider::f32(m, 1.0..=2.0).text("Wheel R multiplier"));
        });
        changed |= check_changed(&mut self.view_angle, |m| {
            let mut current = rad2deg(*m);
            ui.add(
                egui::Slider::f32(&mut current, 20.0..=130.)
                    .text("View angle")
                    .suffix("°"),
            );
            *m = deg2rad(current);
        });

        WhatChanged::from_uniform(changed)
    }
}

struct Window {
    scene: Scene,
    cam: RotateAroundCam,
    material: macroquad::material::Material,
    should_recompile: bool,

    edit_scene_opened: bool,
    camera_settings_opened: bool,

    error_message: Option<(String, String)>,

    data: Data,
}

impl Window {
    async fn new() -> Self {
        let scene = Scene::new();
        let material = scene.get_new_material().unwrap_or_else(|err| {
            println!("code:\n{}\n\nmessage:\n{}", add_line_numbers(&err.0), err.1);
            dbg!(&err);
            crate::miniquad::error!("code:\n{}\n\nmessage:\n{}", add_line_numbers(&err.0), err.1);
            std::process::exit(1)
        });
        scene.set_uniforms(material);
        Window {
            should_recompile: false,
            scene,
            cam: RotateAroundCam::new(),

            material,

            edit_scene_opened: true,
            camera_settings_opened: false,

            error_message: None,

            data: Default::default(),
        }
    }

    fn process_mouse_and_keys(&mut self, ctx: &egui::CtxRef) -> bool {
        let mut is_something_changed = false;

        let mut changed = WhatChanged::default();

        egui::TopPanel::top("my top").show(ctx, |ui| {
            use egui::menu;
            menu::bar(ui, |ui| {
                ui.button("Control scene").clicked();
                if ui.button("Edit scene").clicked() {
                    self.edit_scene_opened = true;
                }
                if ui.button("Camera settings").clicked() {
                    self.camera_settings_opened = true;
                }
                ui.button("Rendering options").clicked();
            });
        });
        let mut edit_scene_opened = self.edit_scene_opened;
        egui::Window::new("Edit scene")
            .open(&mut edit_scene_opened)
            .scroll(true)
            .show(ctx, |ui| {
                let (changed1, material) =
                    self.scene
                        .egui(ui, &mut self.data, &mut self.should_recompile);

                changed = changed1;

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
                            self.data.errors = Some(err.2);
                        }
                    }
                }
            });
        if let Some((code, message)) = self.error_message.as_ref() {
            if self.data.show_error_window {
                egui::Window::new("Error message")
                    .scroll(true)
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
        let mut camera_settings_opened = self.camera_settings_opened;
        egui::Window::new("Camera settings")
            .open(&mut camera_settings_opened)
            .show(ctx, |ui| {
                changed |= self.cam.egui(ui);
            });
        self.camera_settings_opened = camera_settings_opened;
        let mouse_over_canvas = !ctx.wants_pointer_input() && !ctx.is_pointer_over_area();

        if changed.uniform {
            self.scene.set_uniforms(self.material);
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
        self.material.set_uniform("_ray_tracing_depth", 100);
        self.material
            .set_uniform("_offset_after_material", 0.001f32);
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
        window_title: "Portal visualization".to_owned(),
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

        next_frame().await;
    }
}
