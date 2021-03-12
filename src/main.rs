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
}

impl RotateAroundCam {
    const BETA_MIN: f32 = 0.01;
    const BETA_MAX: f32 = PI - 0.01;
    const MOUSE_SENSITIVITY: f32 = 1.4;
    const SCALE_FACTOR: f32 = 1.1;
    const VIEW_ANGLE: f32 = 80. / 180. * PI;

    fn new() -> Self {
        Self {
            alpha: 0.,
            beta: 5. * PI / 7.,
            r: 3.5,
            previous_mouse: Vec2::default(),
        }
    }

    fn process_mouse_and_keys(&mut self, mouse_over_canvas: bool) -> bool {
        let mut is_something_changed = false;

        let mouse_pos: Vec2 = mouse_position_local();

        if is_mouse_button_down(MouseButton::Left) && mouse_over_canvas {
            let dalpha = (mouse_pos.x - self.previous_mouse.x) * Self::MOUSE_SENSITIVITY;
            let dbeta = (mouse_pos.y - self.previous_mouse.y) * Self::MOUSE_SENSITIVITY;

            self.alpha += dalpha;
            self.beta = clamp(self.beta + dbeta, Self::BETA_MIN, Self::BETA_MAX);

            is_something_changed = true;
        }

        let wheel_value = mouse_wheel().1;
        if mouse_over_canvas {
            if wheel_value > 0. {
                self.r *= 1.0 / Self::SCALE_FACTOR;
                is_something_changed = true;
            } else if wheel_value < 0. {
                self.r *= Self::SCALE_FACTOR;
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

        let h = (Self::VIEW_ANGLE / 2.).tan();

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

struct Window {
    scene: Scene,
    cam: RotateAroundCam,
    material: Material,
    should_recompile: bool,

    edit_scene_opened: bool,
}

impl Window {
    async fn new() -> Self {
        let scene = Scene::new();
        let material = scene.get_new_material().unwrap_or_else(|err| {
            println!("code:\n{}\n\nmessage:\n{}", add_line_numbers(&err.0), err.1);
            std::process::exit(1)
        });
        scene.set_uniforms(material);
        Window {
            should_recompile: false,
            scene,
            cam: RotateAroundCam::new(),

            material,

            edit_scene_opened: true,
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
                ui.button("Camera settings").clicked();
                ui.button("Rendering options").clicked();
            });
        });
        let mut edit_scene_opened = self.edit_scene_opened;
        egui::Window::new("Edit scene")
            .open(&mut edit_scene_opened)
            .show(ctx, |ui| {
                let (changed1, material) = self.scene.egui(ui, &mut self.should_recompile);

                changed = changed1;

                if changed.shader {
                    self.should_recompile = true;
                }

                if let Some(material) = material {
                    self.material = material;
                }
            });
        self.edit_scene_opened = edit_scene_opened;
        let mouse_over_canvas = !ctx.is_using_pointer();

        if changed.uniform {
            self.scene.set_uniforms(self.material);
            self.material
                .set_uniform("_resolution", (screen_width(), screen_height()));
            self.material.set_uniform("_camera", self.cam.get_matrix());
            is_something_changed = true;
        }

        is_something_changed |= self.cam.process_mouse_and_keys(mouse_over_canvas);

        return is_something_changed;
    }

    fn draw(&self) {
        self.material
            .set_uniform("_resolution", (screen_width(), screen_height()));
        self.material.set_uniform("_camera", self.cam.get_matrix());

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
