use gesture_recognizer::*;
use glam::Vec4Swizzles;
use glam::{DMat4, DVec2, DVec3, DVec4};
use macroquad::prelude::is_key_down;
use macroquad::prelude::is_key_pressed;
use macroquad::prelude::Image;
use once_cell::sync::Lazy;
use portal::gui::camera::CalculatedCam;
use portal::gui::camera::CameraId;
use portal::gui::camera::CurrentCam;
use portal::gui::camera::OriginalCam;
use portal::gui::eng_rus::EngRusSettings;
use portal::gui::eng_rus::EngRusText;
use portal::gui::scenes::ShowHiddenScenes;
use portal::gui::uniform::AnyUniform;
use portal::gui::uniform::ClampedValue;
use portal::with_swapped;
use std::f64::consts::PI;

use macroquad::prelude::{
    clamp, clear_background, draw_rectangle, draw_texture_ex, get_screen_data,
    gl_use_default_material, gl_use_material, is_mouse_button_down, mouse_position_local,
    mouse_wheel, next_frame, screen_height, screen_width, set_default_camera, Conf,
    DrawTextureParams, MouseButton, Texture2D, BLACK, WHITE,
};
use portal::gui::scene_serialized::SerializedScene;
use portal::gui::scenes::Scenes;
use portal::gui::{common::*, scene::*, texture::*};

use egui::{DragValue, Ui};

#[derive(Clone)]
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

    use_360_camera: bool,

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

    prev_cam_pos: DVec3,
    teleport_matrix: DMat4,
    allow_teleport: bool,
    stop_at_objects: bool,

    in_subspace: bool,

    free_movement: bool,

    prev_view_angle: f64,
    zoom_mode: bool,

    do_not_teleport_one_frame: bool,
    send_camera_object_matrix: bool,

    left_eye_matrix: DMat4,
    right_eye_matrix: DMat4,
    left_eye_in_subspace: bool,
    right_eye_in_subspace: bool,
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

            mouse_sensitivity: 2.0,
            scale_factor: 1.1,
            view_angle: deg2rad(90.),

            use_panini_projection: false,
            panini_param: 1.0,

            use_360_camera: false,

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

            teleport_matrix: DMat4::IDENTITY,
            allow_teleport: true,
            stop_at_objects: false,
            prev_cam_pos: Default::default(),

            in_subspace: false,

            free_movement: false,

            prev_view_angle: deg2rad(90.),
            zoom_mode: false,

            do_not_teleport_one_frame: false,
            send_camera_object_matrix: true,

            left_eye_matrix: DMat4::IDENTITY,
            right_eye_matrix: DMat4::IDENTITY,
            left_eye_in_subspace: false,
            right_eye_in_subspace: false,
        }
    }

    fn get_calculated_cam(&self) -> CalculatedCam {
        CalculatedCam {
            look_at: self.look_at,
            alpha: self.alpha,
            beta: self.beta,
            r: self.r,
            in_subspace: self.in_subspace,
            free_movement: self.free_movement,
            matrix: self.teleport_matrix,
            override_matrix: true,
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
        self.beta = clamp(self.beta + dbeta, Self::BETA_MIN, Self::BETA_MAX);
    }

    fn process_mouse_and_keys(
        &mut self,
        mouse_over_canvas: bool,
        egui_using_keyboard: bool,
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

        if !egui_using_keyboard && is_key_pressed(macroquad::input::KeyCode::Q) {
            self.change_free_movement();
            is_something_changed = true;
        }

        if !egui_using_keyboard && self.free_movement {
            let move_speed = 0.03;

            let dir = self.get_pos_vec();

            let i = dir.normalize().cross(DVec3::new(0., 1., 0.)).normalize();

            if is_key_down(macroquad::input::KeyCode::W) {
                self.look_at += -dir * move_speed;
                is_something_changed = true;
            }
            if is_key_down(macroquad::input::KeyCode::S) {
                self.look_at += dir * move_speed;
                is_something_changed = true;
            }

            if is_key_down(macroquad::input::KeyCode::Space) {
                self.look_at.y += self.r * move_speed;
                is_something_changed = true;
            }
            if is_key_down(macroquad::input::KeyCode::LeftShift)
                || is_key_down(macroquad::input::KeyCode::RightShift)
            {
                self.look_at.y -= self.r * move_speed;
                is_something_changed = true;
            }

            if is_key_down(macroquad::input::KeyCode::A) {
                self.look_at += i * self.r * move_speed;
                is_something_changed = true;
            }
            if is_key_down(macroquad::input::KeyCode::D) {
                self.look_at -= i * self.r * move_speed;
                is_something_changed = true;
            }

            if is_key_down(macroquad::input::KeyCode::LeftControl)
                || is_key_down(macroquad::input::KeyCode::RightControl)
            {
                if !self.zoom_mode {
                    self.zoom_mode = true;
                    self.prev_view_angle = self.view_angle;
                    self.view_angle = deg2rad(10.);
                    is_something_changed = true;
                }
            } else {
                if self.zoom_mode {
                    self.zoom_mode = false;
                    self.view_angle = self.prev_view_angle;
                    is_something_changed = true;
                }
            }
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

        self.previous_mouse = mouse_pos;

        is_something_changed
    }

    fn get_pos_vec(&self) -> DVec3 {
        DVec3::new(
            self.beta.sin() * self.alpha.cos(),
            self.beta.cos(),
            self.beta.sin() * self.alpha.sin(),
        ) * self.r
    }

    fn get_matrix(&self) -> DMat4 {
        let pos = self.get_pos_vec() + self.look_at;

        let k = (self.look_at - pos).normalize();
        let i = k.cross(DVec3::new(0., 1., 0.)).normalize();
        let j = k.cross(i).normalize();

        self.teleport_matrix
            * DMat4::from_cols(
                DVec4::new(i.x, i.y, i.z, 0.),
                DVec4::new(j.x, j.y, j.z, 0.),
                DVec4::new(k.x, k.y, k.z, 0.),
                if self.free_movement {
                    DVec4::new(self.look_at.x, self.look_at.y, self.look_at.z, 1.)
                } else {
                    DVec4::new(pos.x, pos.y, pos.z, 1.)
                },
            )
    }

    fn change_free_movement(&mut self) {
        if !self.free_movement {
            self.look_at = self.get_pos_vec() + self.look_at;
            self.free_movement = true;
        } else {
            self.look_at = self.look_at - self.get_pos_vec();
            self.free_movement = false;
        }
    }

    fn get_cam_pos(&self) -> DVec3 {
        (self.get_matrix() * DVec4::new(0., 0., 0., 1.)).truncate()
    }

    fn set_cam(&mut self, s: &CamSettings) {
        self.look_at = DVec3::new(s.look_at.x, s.look_at.y, s.look_at.z);
        self.alpha = s.alpha;
        self.beta = s.beta;
        self.r = s.r;

        if self.free_movement {
            self.look_at = self.get_pos_vec() + self.look_at;
        }

        self.teleport_matrix = DMat4::IDENTITY;
        self.in_subspace = false;
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
            .process(&mut self.renderer.cam, phase, id, x, y);
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
            ui.label("Enable camera free movement:");
            changed |= check_changed(&mut self.free_movement, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
            });
        });
        ui.label("Toggled by Q. Controls: WASD + Space (up) + Shift (down). Use wheel to control movement speed. Hold Ctrl to zoom. ");
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Stop camera at objects:");
            changed |= check_changed(&mut self.stop_at_objects, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
            });
        });
        ui.horizontal(|ui| {
            ui.label("Allow camera teleportation:");
            changed |= check_changed(&mut self.allow_teleport, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
            });
        });
        ui.horizontal(|ui| {
            ui.label("Inside subspace:");
            changed |= check_changed(&mut self.in_subspace, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
            });
        });
        ui.horizontal(|ui| {
            ui.label("Send matrix for camera object:");
            changed |= check_changed(&mut self.send_camera_object_matrix, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
            });
        });
        ui.horizontal(|ui| {
            ui.label("Teleportation matrix: ");
            if ui.button("Reset").clicked() {
                self.teleport_matrix = DMat4::IDENTITY;
                self.prev_cam_pos = self.get_cam_pos();
                changed = true;
            }
        });
        let visualize_matrix = |ui: &mut Ui, matrix: DMat4| {
            for row in matrix.to_cols_array_2d() {
                ui.horizontal(|ui| {
                    for mut value in row {
                        ui.add_enabled(
                            false,
                            DragValue::new(&mut value)
                                .speed(0.01)
                                .min_decimals(0)
                                .max_decimals(2),
                        );
                    }
                });
            }
        };
        visualize_matrix(ui, self.teleport_matrix);
        ui.separator();
        egui::CollapsingHeader::new("Eye matrices")
            .id_salt(0)
            .show(ui, |ui| {
                ui.label(format!(
                    "Left eye: {}",
                    if self.left_eye_in_subspace {
                        "(in subspace)"
                    } else {
                        "(not in subspace)"
                    }
                ));
                visualize_matrix(ui, self.left_eye_matrix);
                ui.separator();
                ui.label(format!(
                    "Right eye: {}",
                    if self.right_eye_in_subspace {
                        "(in subspace)"
                    } else {
                        "(not in subspace)"
                    }
                ));
                visualize_matrix(ui, self.right_eye_matrix);
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
                        .range(rad2deg(Self::BETA_MIN)..=rad2deg(Self::BETA_MAX))
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
                        .range(0.01..=1000.0)
                        .min_decimals(0)
                        .max_decimals(2),
                );
            });
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("360 camera:");
            changed |= check_changed(&mut self.use_360_camera, |is_use| {
                ui.add(egui::Checkbox::new(is_use, ""));
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
                ui.add_enabled_ui(is_use, |ui| {
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
                    .clamping(egui::widgets::SliderClamping::Always),
            );
            *m = deg2rad(current);
        });

        ui.separator();

        changed |= check_changed(&mut self.mouse_sensitivity, |m| {
            ui.add(egui::Slider::new(m, 0.0..=4.0).text("Mouse sensivity"));
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

/// ----------------------------------------------------------
///  2¹ᐟγ  with γ = 2  ⇒  linear   = (srgb/255)²
///  Our integer  "linear units"  are in   0 … 65 025 (=255²)
///
///  We therefore need two tables:
///    • L_TO_S[c]   : 0 … 65 025  → 0 … 255   (√ + rounding)
///    • S_TO_L[c]   : 0 … 255     → 0 … 65 025 (square)
/// -----------------------------------------------------------

/// 0 … 65 025  →  0 … 255  (rounded √)
static L_TO_S: Lazy<[u8; 65_026]> = Lazy::new(|| {
    let mut t = [0u8; 65_026];
    for (i, v) in t.iter_mut().enumerate() {
        *v = ((i as f32).sqrt() + 0.5) as u8;
    }
    t
});

/// 0 … 255  →  0 … 65 025  (square) – still fine as a genuine `const`
const S_TO_L: [u16; 256] = {
    let mut t = [0u16; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = (i * i) as u16;
        i += 1;
    }
    t
};

pub fn average_images(mut images: Vec<Image>) -> Image {
    use std::num::NonZeroUsize;

    assert!(!images.is_empty(), "empty slice");

    if images.len() == 1 {
        return images.pop().unwrap();
    }

    let w = images[0].width as usize;
    let h = images[0].height as usize;
    let px = NonZeroUsize::new(w * h).expect("zero-sized image");

    // ---- Fast dimension check ------------------------------------------------
    if !images
        .iter()
        .all(|im| im.width as usize == w && im.height as usize == h)
    {
        panic!("all images must have identical dimensions");
    }

    // ---- Accumulators (u32 → cannot overflow: 65 025 * N_images < 4 G) -------
    let mut sum_r = vec![0u32; px.get()];
    let mut sum_g = vec![0u32; px.get()];
    let mut sum_b = vec![0u32; px.get()];

    // ---- Hot loop ------------------------------------------------------------
    for img in &images {
        for (idx, px_rgba) in img.bytes.chunks_exact(4).enumerate() {
            // Unsafe unchecked table read is another ~5 % but keep it safe for now
            sum_r[idx] += S_TO_L[px_rgba[0] as usize] as u32;
            sum_g[idx] += S_TO_L[px_rgba[1] as usize] as u32;
            sum_b[idx] += S_TO_L[px_rgba[2] as usize] as u32;
        }
    }

    let n = images.len() as u32;

    // ---- Produce output ------------------------------------------------------
    let mut out = vec![0u8; px.get() * 4];

    out.chunks_exact_mut(4).enumerate().for_each(|(idx, dst)| {
        // integer average in linear space
        let lr = (sum_r[idx] / n) as usize;
        let lg = (sum_g[idx] / n) as usize;
        let lb = (sum_b[idx] / n) as usize;

        dst[0] = L_TO_S[lr]; // back to sRGB (γ=2 ⇒ √)
        dst[1] = L_TO_S[lg];
        dst[2] = L_TO_S[lb];
        dst[3] = 255; // opaque
    });

    Image {
        bytes: out,
        width: w as u16,
        height: h as u16,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ViewportRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

struct SceneRenderer {
    scene: Scene,
    cam: RotateAroundCam,
    prev_cam: RotateAroundCam,
    material: macroquad::material::Material,
    data: Data,
    offset_after_material: f64,
    gray_t_start: f64,
    gray_t_size: f64,
    render_depth: i32,
    aa_count: i32,
    aa_start: i32,
    draw_side_by_side: bool,
    eye_distance: f64,
    swap_eyes: bool,
    draw_anaglyph: bool,
    anaglyph_mode: bool,
    angle_color_disable: bool,
    grid_disable: bool,
    black_border_disable: bool,
    darken_by_distance: bool,
    render_target: macroquad::prelude::RenderTarget,
    external_ray_render_target: macroquad::prelude::RenderTarget,
    width: u32,
    height: u32,
    scene_name: String,
    current_fps: usize,
    current_motion_blur_frames: usize,
    texture_storage: Vec<Texture2D>,
    #[cfg(not(target_arch = "wasm32"))]
    video_runtimes: Vec<VideoRuntime>,
}

#[cfg(not(target_arch = "wasm32"))]
struct VideoRuntime {
    /// Name of the video entry in Scene.videos; must match texture name.
    video_name: String,
    /// Id of the video entry in Scene.videos.
    video_id: portal::gui::video::VideoId,
    /// Cached uniform id controlling the frame position.
    uniform_id: Option<portal::gui::uniform::UniformId>,
    /// Sorted list of PNG frame file paths.
    frame_files: Vec<String>,
    /// Index of the last frame uploaded to GPU.
    last_frame_index: Option<usize>,
    /// Index in SceneRenderer.texture_storage where current texture is kept.
    texture_storage_index: Option<usize>,
}

#[cfg(not(target_arch = "wasm32"))]
fn video_frames_dir(path: &str) -> Option<String> {
    use std::path::Path;
    let p = Path::new(path);
    let stem = p.file_stem()?.to_string_lossy();
    Some(format!("video_png/{}", stem))
}

#[cfg(not(target_arch = "wasm32"))]
fn video_collect_frame_files(path: &str) -> Option<Vec<String>> {
    use std::fs;
    let dir = video_frames_dir(path)?;
    let mut files: Vec<String> = fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("png") {
                Some(p.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    files.sort();
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl VideoRuntime {
    fn from_scene(scene: &Scene) -> Vec<Self> {
        let mut result = Vec::new();

        for (vid_id, name) in scene.videos.visible_elements() {
            if let Some(video) = scene.videos.get_original(vid_id) {
                if video.path.is_empty() {
                    continue;
                }
                let uniform_id = video.uniform;
                if uniform_id.is_none() {
                    continue;
                }
                let frame_files = match video_collect_frame_files(&video.path) {
                    Some(v) => v,
                    None => continue,
                };

                result.push(Self {
                    video_name: name.to_owned(),
                    video_id: vid_id,
                    uniform_id,
                    frame_files,
                    last_frame_index: None,
                    texture_storage_index: None,
                });
            }
        }

        result
    }

    fn update(&mut self, renderer: &mut SceneRenderer) {
        use portal::gui::texture::TextureName;

        if self.frame_files.is_empty() {
            return;
        }

        // Ensure we have a valid uniform id (re-resolve from scene if needed).
        let uniform_id = match self.uniform_id.or_else(|| {
            renderer
                .scene
                .videos
                .get_original(self.video_id)
                .and_then(|v| v.uniform)
        }) {
            Some(id) => {
                self.uniform_id = Some(id);
                id
            }
            None => return,
        };

        // Read current uniform value and clamp to [0, 1].
        let value = match renderer
            .scene
            .uniforms
            .get(uniform_id, &renderer.data.formulas_cache)
        {
            Some(v) => {
                let f: f64 = v.into();
                f.clamp(0.0, 1.0)
            }
            None => return,
        };

        let frame_count = self.frame_files.len();
        if frame_count == 0 {
            return;
        }

        let target_index = ((frame_count - 1) as f64 * value)
            .round()
            .clamp(0.0, (frame_count - 1) as f64) as usize;

        // Avoid reload if frame didn't change.
        if Some(target_index) == self.last_frame_index {
            return;
        }

        let path = &self.frame_files[target_index];
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => {
                // Ignore IO errors for now; texture simply won't update.
                return;
            }
        };

        let texture = Texture2D::from_file_with_format(&bytes[..], None);

        // Update material sampler (sampler name derived from texture name).
        renderer
            .material
            .set_texture(&TextureName::name(&self.video_name), texture.clone());

        // Keep texture alive in texture_storage, reusing the same slot to avoid leaks.
        if let Some(index) = self.texture_storage_index {
            if index < renderer.texture_storage.len() {
                renderer.texture_storage[index] = texture;
            } else {
                renderer.texture_storage.push(texture);
                self.texture_storage_index = Some(renderer.texture_storage.len() - 1);
            }
        } else {
            renderer.texture_storage.push(texture);
            self.texture_storage_index = Some(renderer.texture_storage.len() - 1);
        }

        self.last_frame_index = Some(target_index);
    }
}

impl SceneRenderer {
    async fn new(mut scene: Scene, max_width: u32, max_height: u32, scene_name: &str) -> Self {
        let mut data: Data = Data {
            reload_textures: true,
            for_prefer_variable: cfg!(not(target_arch = "wasm32")),
            use_300_version: cfg!(not(target_arch = "wasm32")),
            disable_anaglyph: true,
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

                // use this material so that users can change some settings so that their code compiles
                let dummy_material = macroquad::prelude::load_material(
                    macroquad::prelude::ShaderSource::Glsl {
                        vertex: "#version 100
                                attribute vec3 position;
                                attribute vec2 texcoord;

                                varying lowp vec2 center;
                                varying lowp vec2 uv;
                                varying lowp vec2 uv_screen;

                                uniform mat4 Model;
                                uniform mat4 Projection;

                                uniform vec2 Center;

                                void main() {
                                    vec4 res = Projection * Model * vec4(position, 1);
                                    vec4 c = Projection * Model * vec4(Center, 0, 1);

                                    uv_screen = res.xy / 2.0 + vec2(0.5, 0.5);
                                    center = c.xy / 2.0 + vec2(0.5, 0.5);
                                    uv = texcoord;

                                    gl_Position = res;
                                }
                                ",
                        fragment: r#"#version 100
                                precision lowp float;

                                varying vec2 uv;
                                varying vec2 uv_screen;
                                varying vec2 center;

                                uniform sampler2D _ScreenTexture;

                                void main() {
                                    float gradient = length(uv);
                                    vec2 uv_zoom = (uv_screen - center) * gradient + center;

                                    gl_FragColor = texture2D(_ScreenTexture, uv_zoom);
                                }
                                "#,
                    },
                    macroquad::prelude::MaterialParams {
                        uniforms: vec![macroquad::prelude::UniformDesc::new(
                            "Center",
                            macroquad::prelude::UniformType::Float2,
                        )],
                        textures: scene.textures(),
                        ..Default::default()
                    },
                )
                .unwrap();

                dummy_material
            });

        scene.set_uniforms(&mut material, &mut data);

        let mut result = Self {
            scene,
            cam: RotateAroundCam::new(),
            prev_cam: RotateAroundCam::new(),
            material,
            data,

            offset_after_material: 0.005,
            gray_t_start: 10.,
            gray_t_size: 200.,
            render_depth: 100,
            aa_count: 1,
            aa_start: 0,
            draw_side_by_side: false,
            eye_distance: 0.0999,
            swap_eyes: false,
            draw_anaglyph: true,
            anaglyph_mode: false,
            angle_color_disable: false,
            grid_disable: false,
            black_border_disable: false,
            darken_by_distance: true,
            render_target: macroquad::prelude::render_target(max_width, max_height),
            external_ray_render_target: macroquad::prelude::render_target(2, 3),
            width: max_width,
            height: max_height,
            scene_name: scene_name.to_owned(),
            current_fps: 60,
            current_motion_blur_frames: 1,
            texture_storage: vec![],
            #[cfg(not(target_arch = "wasm32"))]
            video_runtimes: Vec::new(),
        };

        result
            .render_target
            .texture
            .set_filter(macroquad::prelude::FilterMode::Nearest);
        result.cam.set_cam(&result.scene.cam);
        result.cam.prev_cam_pos = result.cam.get_cam_pos();
        result.offset_after_material = result.scene.cam.offset_after_material;
        result.reload_textures().await;
        result.scene.run_animations = false;
        result.teleport_eye_matrices();
        result
    }

    async fn reload_textures(&mut self) {
        if self.data.reload_textures {
            self.texture_storage.clear();
            self.data.reload_textures = false;
            self.data.texture_errors.0.clear();
            for (id, name) in self.scene.textures.visible_elements() {
                let path = self.scene.textures.get_original(id).unwrap();
                match macroquad::file::load_file(&path.0).await {
                    Ok(bytes) => {
                        let texture = Texture2D::from_file_with_format(&bytes[..], None);

                        self.material
                            .set_texture(&TextureName::name(name), texture.clone());

                        self.texture_storage.push(texture);
                    }
                    Err(file) => {
                        self.data.texture_errors.0.insert(name.to_string(), file);
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Rebuild video runtimes and immediately upload current frames
                // so that videos are visible right after recompilation / scene load.
                let mut runtimes = VideoRuntime::from_scene(&self.scene);
                for v in &mut runtimes {
                    v.update(self);
                }
                self.video_runtimes = runtimes;
            }
        }
    }

    fn load_from_scene(
        &mut self,
        scene: Scene,
        memory: &mut egui::Memory,
    ) -> Option<Result<(), (String, String, ShaderErrors)>> {
        self.scene = scene;
        self.scene.init(&mut self.data, memory);
        self.material = match self.scene.get_new_material(&self.data)? {
            Ok(material) => material,
            Err(err) => return Some(Err(err)),
        };
        self.data.reload_textures = true;
        self.cam.set_cam(&self.scene.cam);
        memory.data.insert_persisted(
            egui::Id::new("OriginalCam"),
            OriginalCam(self.cam.get_calculated_cam()),
        );
        self.offset_after_material = self.scene.cam.offset_after_material;
        Some(Ok(()))
    }

    fn teleport_eye_matrices(&mut self) {
        if !((self.draw_anaglyph || self.draw_side_by_side) && self.cam.allow_teleport) {
            return;
        }

        let eye_distance = if self.swap_eyes {
            -self.eye_distance
        } else {
            self.eye_distance
        };

        let mut process_one_eye = |eye_vector: DVec4| -> (DMat4, bool) {
            let start_pos = self.cam.get_cam_pos();
            let direction_pos = (self.cam.get_matrix() * eye_vector).xyz();
            let mut result = (
                DMat4::from_translation(direction_pos - start_pos) * self.cam.get_matrix(),
                self.cam.in_subspace,
            );

            let (teleported, _, change_subspace) =
                self.teleport_external_ray(start_pos, direction_pos);
            if let Some(new_pos) = teleported {
                let mut res = |cam_teleport_dx: f64| -> Option<()> {
                    result.0 = self.teleport_matrix(
                        result.0,
                        start_pos,
                        direction_pos,
                        new_pos,
                        cam_teleport_dx,
                    )?;
                    if change_subspace {
                        result.1 = !self.cam.in_subspace;
                    }
                    Some(())
                };
                if res(0.001) == None
                    && res(0.0001) == None
                    && res(0.00001) == None
                    && res(0.000001) == None
                {
                    // ok
                }
            }

            result
        };

        let res1 = process_one_eye(DVec4::new(-eye_distance, 0., 0., 1.));
        let res2 = process_one_eye(DVec4::new(eye_distance, 0., 0., 1.));
        (self.cam.left_eye_matrix, self.cam.left_eye_in_subspace) = res1;
        (self.cam.right_eye_matrix, self.cam.right_eye_in_subspace) = res2;
    }

    fn teleport_matrix(
        &mut self,
        matrix: DMat4,
        start_pos: DVec3,
        direction_pos: DVec3,
        actual_teleported_pos: DVec3,
        dx: f64,
    ) -> Option<DMat4> {
        let i = (matrix * DVec4::new(1., 0., 0., 0.) * dx).truncate();
        let i = self
            .teleport_external_ray(start_pos + i, direction_pos + i)
            .0?
            - actual_teleported_pos;
        let i = DVec4::from((i, 0.)) / dx;

        let j = (matrix * DVec4::new(0., 1., 0., 0.) * dx).truncate();
        let j = self
            .teleport_external_ray(start_pos + j, direction_pos + j)
            .0?
            - actual_teleported_pos;
        let j = DVec4::from((j, 0.)) / dx;

        let k = (matrix * DVec4::new(0., 0., 1., 0.) * dx).truncate();
        let k = self
            .teleport_external_ray(start_pos + k, direction_pos + k)
            .0?
            - actual_teleported_pos;
        let k = DVec4::from((k, 0.)) / dx;

        let pos = DVec4::new(0., 0., 0., 1.);

        let new_mat = DMat4::from_cols(i, j, k, pos);

        let pos = actual_teleported_pos
            - (new_mat
                * matrix.inverse()
                * DVec4::new(direction_pos.x, direction_pos.y, direction_pos.z, 1.))
            .truncate();
        let pos = DVec4::from((pos, 1.));

        Some(DMat4::from_cols(i, j, k, pos))
    }

    fn teleport_camera(&mut self, prev_cam: RotateAroundCam) {
        if self.cam.do_not_teleport_one_frame {
            self.cam.do_not_teleport_one_frame = false;
            self.cam.prev_cam_pos = self.cam.get_cam_pos();
            return;
        }

        if !(self.cam.allow_teleport || self.cam.stop_at_objects) {
            return;
        }
        let cam_pos = self.cam.get_cam_pos();
        let (teleported, encounter_object, change_subspace) =
            self.teleport_external_ray(self.cam.prev_cam_pos, cam_pos);
        if self.cam.stop_at_objects && encounter_object {
            self.cam = prev_cam.clone();
            return;
        }
        if let Some(new_pos) = teleported {
            if !self.cam.allow_teleport {
                return;
            }
            let mut res = |cam_teleport_dx: f64| -> Option<()> {
                self.cam.teleport_matrix = self.teleport_matrix(
                    self.cam.teleport_matrix,
                    self.cam.prev_cam_pos,
                    cam_pos,
                    new_pos,
                    cam_teleport_dx,
                )?;

                if change_subspace {
                    self.cam.in_subspace = !self.cam.in_subspace;
                }

                self.cam.prev_cam_pos = self.cam.get_cam_pos();
                Some(())
            };
            if res(0.001) == None
                && res(0.0001) == None
                && res(0.00001) == None
                && res(0.000001) == None
            {
                self.cam = prev_cam;
            }
        } else {
            self.cam.prev_cam_pos = cam_pos;
        }
    }

    fn set_uniforms(&mut self, width: f32, height: f32) {
        self.cam.get_cam(&mut self.scene.cam);
        self.scene.cam.offset_after_material = self.offset_after_material;
        self.material.set_uniform("_resolution", (width, height));
        self.material
            .set_uniform("_camera", self.cam.get_matrix().as_f32());
        self.material
            .set_uniform("_camera_left_eye", self.cam.left_eye_matrix.as_f32());
        self.material
            .set_uniform("_camera_right_eye", self.cam.right_eye_matrix.as_f32());
        self.material.set_uniform(
            "_left_eye_in_subspace",
            self.cam.left_eye_in_subspace as i32,
        );
        self.material.set_uniform(
            "_right_eye_in_subspace",
            self.cam.right_eye_in_subspace as i32,
        );
        self.material.set_uniform(
            "_camera_mul_inv",
            self.cam.teleport_matrix.inverse().as_f32(),
        );
        self.material
            .set_uniform("_camera_in_subspace", self.cam.in_subspace as i32);
        self.material
            .set_uniform("_view_angle", self.cam.view_angle as f32);
        self.material
            .set_uniform("_panini_param", self.cam.panini_param as f32);
        self.material.set_uniform(
            "_use_panini_projection",
            self.cam.use_panini_projection as i32,
        );
        self.material
            .set_uniform("_use_360_camera", self.cam.use_360_camera as i32);
        self.material
            .set_uniform("_ray_tracing_depth", self.render_depth);
        self.material.set_uniform("_aa_count", self.aa_count);
        self.material.set_uniform("_aa_start", self.aa_start);
        self.material
            .set_uniform("_draw_side_by_side", self.draw_side_by_side as i32);
        self.material
            .set_uniform("_draw_anaglyph", self.draw_anaglyph as i32);
        self.material
            .set_uniform("_anaglyph_mode", self.anaglyph_mode as i32);
        self.material
            .set_uniform("_offset_after_material", self.offset_after_material as f32);

        let calc_scale = |matrix: &DMat4| -> f64 {
            matrix
                .to_cols_array_2d()
                .into_iter()
                .take(3)
                .map(|x| DVec4::from(x).length())
                .sum::<f64>()
                / 3.0
        };

        self.material
            .set_uniform("_t_start", self.gray_t_start as f32);
        self.material
            .set_uniform("_t_end", (self.gray_t_start + self.gray_t_size) as f32);
        self.material
            .set_uniform("_camera_scale", calc_scale(&self.cam.get_matrix()) as f32);
        self.material.set_uniform(
            "_left_eye_scale",
            calc_scale(&self.cam.left_eye_matrix) as f32,
        );
        self.material.set_uniform(
            "_right_eye_scale",
            calc_scale(&self.cam.right_eye_matrix) as f32,
        );

        self.material
            .set_uniform("_angle_color_disable", self.angle_color_disable as i32);
        self.material
            .set_uniform("_grid_disable", self.grid_disable as i32);
        self.material
            .set_uniform("_black_border_disable", self.black_border_disable as i32);
        self.material
            .set_uniform("_darken_by_distance", self.darken_by_distance as i32);
        self.material.set_uniform("_teleport_external_ray", 0);
    }

    fn teleport_external_ray(&mut self, a: DVec3, b: DVec3) -> (Option<glam::DVec3>, bool, bool) {
        self.set_uniforms(0., 0.);
        self.material
            .set_uniform("_teleport_external_ray", 1 as i32);
        self.material.set_uniform("_external_ray_a", a.as_f32());
        self.material.set_uniform("_external_ray_b", b.as_f32());
        self.material.set_uniform("teleport_light_u", 1 as i32);

        macroquad::prelude::set_camera(&macroquad::prelude::Camera2D {
            zoom: macroquad::prelude::vec2(
                1. / (self.external_ray_render_target.texture.size().x / 2.),
                1. / (self.external_ray_render_target.texture.size().y / 2.),
            ),
            offset: macroquad::prelude::vec2(-1., -1.),
            render_target: Some(self.external_ray_render_target.clone()),
            ..Default::default()
        });
        gl_use_material(&self.material);
        draw_rectangle(
            0.,
            0.,
            self.external_ray_render_target.texture.size().x,
            self.external_ray_render_target.texture.size().y,
            WHITE,
        );
        gl_use_default_material();
        set_default_camera();

        let arr = self
            .external_ray_render_target
            .texture
            .get_texture_data()
            .bytes;

        let encounter_object = (arr[0 + 5] == 255) || (arr[8 + 5] == 255) || (arr[16 + 5] == 255);
        let change_subspace = (arr[0 + 6] == 255) || (arr[8 + 6] == 255) || (arr[16 + 6] == 255);
        let x = f32::from_le_bytes([arr[0 + 0], arr[0 + 1], arr[0 + 2], arr[0 + 4]]);
        let y = f32::from_le_bytes([arr[8 + 0], arr[8 + 1], arr[8 + 2], arr[8 + 4]]);
        let z = f32::from_le_bytes([arr[16 + 0], arr[16 + 1], arr[16 + 2], arr[16 + 4]]);
        if !(x == 0.0 && y == 0.0 && z == 0.0) {
            return (
                Some(DVec3::new(x.into(), y.into(), z.into())),
                encounter_object,
                change_subspace,
            );
        } else {
            return (None, encounter_object, change_subspace);
        }
    }

    fn draw_texture(&mut self, width: f32, height: f32, flip_y: bool) {
        let flip_y = if flip_y { -1. } else { 1. };
        self.scene.set_uniforms(&mut self.material, &mut self.data);
        self.set_uniforms(width, height);
        macroquad::prelude::set_camera(&macroquad::prelude::Camera2D {
            zoom: macroquad::prelude::vec2(
                1. / (self.render_target.texture.size().x / 2.),
                1. / (self.render_target.texture.size().y / 2.) * flip_y,
            ),
            offset: macroquad::prelude::vec2(-1., -1. * flip_y),
            render_target: Some(self.render_target.clone()),
            ..Default::default()
        });
        gl_use_material(&self.material);
        draw_rectangle(0., 0., width, height, WHITE);
        gl_use_default_material();
        set_default_camera();
    }

    fn update(&mut self, memory: &mut egui::Memory, time: f64) {
        self.scene.update(memory, &mut self.data, time);
        if self.cam.send_camera_object_matrix {
            self.data
                .formulas_cache
                .set_camera_matrix(self.cam.get_matrix());
        }

        let current_cam = memory
            .data
            .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
            .0;
        if self.cam.from != current_cam {
            let calculated_cam = if let Some(id) = current_cam {
                // set getted camera

                if self.cam.from.is_none() {
                    let calculated_cam = self.cam.get_calculated_cam();
                    memory.data.insert_persisted(
                        egui::Id::new("OriginalCam"),
                        OriginalCam(calculated_cam),
                    );
                }

                with_swapped!(x => (self.scene.uniforms, self.data.formulas_cache);
                    self.scene.cameras.get_original(id).unwrap().get(&self.scene.matrices, &x).unwrap())
            } else {
                // set original camera
                memory
                    .data
                    .get_persisted_mut_or_default::<OriginalCam>(egui::Id::new("OriginalCam"))
                    .0
            };

            self.cam.from = current_cam;
            self.cam.alpha = calculated_cam.alpha;
            self.cam.beta = calculated_cam.beta;
            self.cam.r = calculated_cam.r;
            self.cam.look_at = calculated_cam.look_at;
            self.cam.teleport_matrix = calculated_cam.matrix;
            self.cam.in_subspace = calculated_cam.in_subspace;
            self.cam.free_movement = calculated_cam.free_movement;

            if self.cam.free_movement {
                self.cam.look_at = self.cam.get_pos_vec() + self.cam.look_at;
            }

            self.cam.do_not_teleport_one_frame = true;
        } else if let Some(id) = self.cam.from {
            let calculated_cam = with_swapped!(x => (self.scene.uniforms, self.data.formulas_cache);
                self.scene.cameras.get_original(id).unwrap().get(&self.scene.matrices, &x).unwrap());

            if !self.cam.free_movement {
                self.cam.look_at = calculated_cam.look_at;
            }
        }

        if memory
            .data
            .get_persisted::<bool>(egui::Id::new("do_not_teleport_one_frame"))
            .unwrap_or(false)
        {
            self.cam.do_not_teleport_one_frame = true;
            memory
                .data
                .remove::<bool>(egui::Id::new("do_not_teleport_one_frame"));
        }

        if let Some(override_cam) = memory
            .data
            .get_persisted::<CalculatedCam>(egui::Id::new("OverrideCam"))
        {
            self.cam.alpha = override_cam.alpha;
            self.cam.beta = override_cam.beta;
            self.cam.r = override_cam.r;
            self.cam.look_at = override_cam.look_at;
            self.cam.free_movement = override_cam.free_movement;

            if override_cam.override_matrix {
                self.cam.teleport_matrix = override_cam.matrix;
                self.cam.in_subspace = override_cam.in_subspace;
                self.cam.do_not_teleport_one_frame = true;
            }

            memory
                .data
                .remove::<CalculatedCam>(egui::Id::new("OverrideCam"));
        }

        if self.cam.get_matrix() != self.prev_cam.get_matrix() {
            self.teleport_camera(self.prev_cam.clone());
        }
        self.teleport_eye_matrices();
        self.prev_cam = self.cam.clone();

        memory.data.insert_persisted(
            egui::Id::new("CalculatedCam"),
            self.cam.get_calculated_cam(),
        );

        if self.cam.send_camera_object_matrix {
            self.data
                .formulas_cache
                .set_camera_matrix(self.cam.get_matrix());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut runtimes = Vec::new();
            std::mem::swap(&mut runtimes, &mut self.video_runtimes);
            for mut video in runtimes {
                video.update(self);
                self.video_runtimes.push(video);
            }
        }
    }

    fn egui_rendering_settings(&mut self, ui: &mut Ui) -> WhatChanged {
        let mut changed = WhatChanged::default();
        ui.label("3D rendering options:");
        if self.data.disable_anaglyph {
            ui.label("Anaglyph is disabled, can't control it. Enable it lower in the settings and recompile/reload the scene.");
        } else {
            changed.uniform |= ui
                .checkbox(&mut self.draw_anaglyph, "Draw anaglyph")
                .changed();
            ui.label("By default anaglyph is grayscale for better visuals. Colorful anaglyph uses Dubois matrices for better color fidelity.");
            changed.uniform |= ui
                .checkbox(&mut self.anaglyph_mode, "Colorful anaglyph")
                .changed();
        }
        changed.uniform |= ui
            .checkbox(&mut self.draw_side_by_side, "Draw side-by-side")
            .changed();
        ui.label("Disable \"Swap eyes\" when you render video. Enable it when you want to look at 3D image with your eyes crossed.");
        changed.uniform |= ui.checkbox(&mut self.swap_eyes, "Swap eyes").changed();
        ui.horizontal(|ui| {
            ui.label("Eye distance:");
            changed.uniform |= egui_f64_positive(ui, &mut self.eye_distance);
        });
        ui.separator();
        ui.label("Render depth:");
        changed.uniform |= check_changed(&mut self.render_depth, |depth| {
            ui.add(
                egui::Slider::new(depth, 0..=1000).clamping(egui::widgets::SliderClamping::Always),
            );
        });
        ui.label("(Max count of ray bounce after portal, reflect, refract)");
        ui.separator();
        if self.data.disable_antialiasing {
            ui.label("Antialiasing is disabled, can't control it. Enable it lower in the settings and recompile/reload the scene.");
        } else {
            ui.label("Antialiasing count:");
            changed.uniform |= check_changed(&mut self.aa_count, |count| {
                ui.add(
                    egui::Slider::new(count, 1..=16)
                        .clamping(egui::widgets::SliderClamping::Always),
                );
            });
        }
        ui.separator();
        ui.label("Offset after material:");
        changed.uniform |= check_changed(&mut self.offset_after_material, |offset| {
            const MIN: f64 = 0.0000001;
            const MAX: f64 = 0.1;
            ui.add(
                egui::Slider::new(offset, MIN..=MAX)
                    .logarithmic(true)
                    .clamping(egui::widgets::SliderClamping::Always)
                    .largest_finite(MAX)
                    .smallest_positive(MIN),
            );
        });
        ui.label("(Ofsetting after ray being teleported, reflected, refracted)");
        ui.separator();
        ui.label("Darkening by distance:");
        changed.uniform |= egui_bool(ui, &mut self.darken_by_distance);
        ui.horizontal(|ui| {
            ui.label("Darkening after distance:");
            changed.uniform |= egui_f64_positive(ui, &mut self.gray_t_start);
        });
        ui.horizontal(|ui| {
            ui.label("Darkening size:");
            changed.uniform |= egui_f64_positive(ui, &mut self.gray_t_size);
        });
        ui.separator();
        ui.label("Disable darkening by angle with normal:");
        changed.uniform |= egui_bool(ui, &mut self.angle_color_disable);
        ui.label("(This increases resulting gif size if you capturing screen)");
        ui.separator();
        ui.label("Disable grid:");
        changed.uniform |= egui_bool(ui, &mut self.grid_disable);
        ui.label("(If you want extreme small gif)");
        ui.separator();
        ui.label("Disable small black border:");
        changed.uniform |= egui_bool(ui, &mut self.black_border_disable);
        ui.separator();
        ui.label("Improve compilation speed time 1: (uses for loops on numbers instead of variables, may not compile)");
        changed.shader |= egui_bool(ui, &mut self.data.for_prefer_variable);
        ui.separator();
        ui.label("Improve compilation speed time 2: (disables antialiasing code)");
        changed.shader |= egui_bool(ui, &mut self.data.disable_antialiasing);
        ui.separator();
        ui.label("Improve compilation speed time 3: (disables camera teleportation code)");
        changed.shader |= egui_bool(ui, &mut self.data.disable_camera_teleportation);
        ui.separator();
        ui.label("Improve compilation speed time 4: (uses 300 glsl version instead of 100)");
        changed.shader |= egui_bool(ui, &mut self.data.use_300_version);
        ui.separator();
        ui.label("Improve compilation speed time 5: (disable anaglyph)");
        changed.shader |= egui_bool(ui, &mut self.data.disable_anaglyph);
        changed
    }

    fn update_inner_variables(&mut self, animation_name: &str) -> Option<()> {
        if [
            "v2.face.2",
            "v2.face.3",
            "v2.face.4",
            "v2.face.5",
            "v2.inside.1",
            "v2.inside.3",
            "v2.intro.1",
            "v2.normal.2",
            "v2.normal.3",
            "v2.rod.2",
            "v2.rod.3",
            "v2.spiral.3",
            "v2.spiral.4",
            "v2.spiral.5",
            "v2.spiral.6",
            "v2.spiral.7",
            "v2.spiral.9",
            "v2.spaaaace.0",
        ]
        .contains(&animation_name)
        {
            let id = self.scene.uniforms.find_id("subspace_degree")?;
            *self.scene.uniforms.get_original_mut(id).unwrap() =
                AnyUniform::Int(ClampedValue::new(500));
        }
        if ["v2.spiral.4", "v2.spiral.5", "v2.spiral.6"].contains(&animation_name) {
            let id = self.scene.uniforms.find_id("subspace_degree")?;
            *self.scene.uniforms.get_original_mut(id).unwrap() =
                AnyUniform::Int(ClampedValue::new(1000));
        }
        if [
            // cone
            "v2.rotated.0",
            "v2.spiral.0",
            // teleportation_degrees
            "v2.screenshot.5",
            "v2.screenshot.6",
        ]
        .contains(&animation_name)
        {
            self.render_depth = 100;
        }
        if [
            // plus_ultra
            "v2.screenshot.3",
        ]
        .contains(&animation_name)
        {
            self.current_fps = 600;
        }
        // if self.scene_name == "inverted_surface" {
        //     self.offset_after_material = 0.00015;
        // }
        Some(())
    }

    fn use_animation_stage(&mut self, name: &str, memory: &mut egui::Memory) {
        self.scene.init_animation_by_name(name, memory).unwrap();
        self.update_inner_variables(name);
        self.update(memory, 0.);
    }

    fn render_animation(
        &mut self,
        duration_seconds: f32,
        fps: usize,
        motion_blur_frames: usize,
        output_name: &str,
        memory: &mut egui::Memory,
        width: u32,
        height: u32,
        skip_existing: bool,
    ) {
        if matches!(
            std::fs::exists(&format!("video/{}.mov", output_name)),
            Ok(true)
        ) {
            if skip_existing {
                println!("Skip `{output_name}`, because it's already exists");
                return;
            }
        }

        drop(std::fs::create_dir("anim"));
        let count = ((duration_seconds * fps as f32) as usize).max(1);
        let exposure = 0.5; // number from 0 to 1
        for i in 0..count {
            let name = format!("anim/frame_{i}.png");
            if std::fs::exists(&name).unwrap() {
                continue;
            }

            let mut images = vec![];
            for j in 0..motion_blur_frames {
                let t = (i as f64 / count as f64)
                    + j as f64 / motion_blur_frames as f64 / count as f64 * exposure;
                self.aa_start = j as i32;
                self.update(memory, t * duration_seconds as f64);
                self.draw_texture(width as f32, height as f32, true);

                if i == 0 && j == 0 {
                    self.render_target
                        .texture
                        .get_texture_data()
                        .export_png(&format!("video/{output_name}.start.png"));
                }

                if i == count - 1 && j == motion_blur_frames - 1 {
                    self.render_target
                        .texture
                        .get_texture_data()
                        .export_png(&format!("video/{output_name}.end.png"));
                }

                images.push(self.render_target.texture.get_texture_data());
            }
            let result = std::hint::black_box(average_images(images));

            result.export_png(&name);

            print!("\r{i}/{count} done      ");

            use std::io::Write;
            std::io::stdout().lock().flush().unwrap();
        }
        println!();

        println!("Start ffmpeg to render video");
        drop(std::fs::create_dir("video"));
        let command = std::process::Command::new("ffmpeg")
        .arg("-framerate").arg(fps.to_string())
        .arg("-i").arg("anim/frame_%d.png")

        // Keep full-range; tag/convert explicitly to sRGB in YUV420 10-bit
        .arg("-vf").arg("zscale=primariesin=bt709:transferin=iec61966-2-1:matrixin=bt709:rangein=full:primaries=bt709:transfer=iec61966-2-1:matrix=bt709:range=full,format=yuv420p10le")

        .arg("-c:v").arg("libx265")
        .arg("-pix_fmt").arg("yuv420p10le")
        .arg("-crf").arg("15")
        .arg("-preset").arg("slow")

        // Bitstream + container tags (sRGB transfer)
        .arg("-x265-params").arg("colorprim=bt709:transfer=iec61966-2-1:colormatrix=bt709:range=full")
        .arg("-colorspace").arg("bt709")
        .arg("-color_primaries").arg("bt709")
        .arg("-color_trc").arg("iec61966-2-1")
        .arg("-color_range").arg("pc")

        // MOV tends to keep colr atom; hvc1 tag improves compatibility
        .arg("-movflags").arg("+write_colr+faststart")
        .arg("-tag:v").arg("hvc1")

        .arg("-y")
        .arg(format!("video/{output_name}.mov"))
        .output()
        .expect("failed to execute ffmpeg");
        std::fs::remove_dir_all("anim").unwrap();
        if command.status.code() != Some(0) {
            println!(
                "ffmpeg error:\n{}\n\n{}",
                std::str::from_utf8(&command.stdout).unwrap(),
                std::str::from_utf8(&command.stderr).unwrap()
            );
        }
        println!("ffmpeg status: {}", command.status);
    }

    fn render_all_animations(
        &mut self,
        fps: usize,
        motion_blur_frames: usize,
        skip_existing: bool,
        starts_with: Option<&str>,
    ) {
        let mut memory = egui::Memory::default();

        drop(std::fs::create_dir("video"));
        drop(std::fs::create_dir(format!("video/{}", self.scene_name)));

        let len = self.scene.animations_len();
        for i in 0..len {
            self.scene
                .init_animation_by_position(i, &mut memory)
                .unwrap();
            self.update(&mut memory, 0.);
            let duration = self.scene.get_current_animation_duration().unwrap();

            let name = self.scene.get_current_animation_name().unwrap().to_owned();
            self.current_fps = fps;
            self.current_motion_blur_frames = motion_blur_frames;
            self.update_inner_variables(&name);

            if let Some(starts_with) = starts_with {
                if !name.starts_with(starts_with) {
                    continue;
                }
            }
            println!("Rendering animation {name}, {i}/{len}");

            self.render_animation(
                duration as f32,
                self.current_fps,
                self.current_motion_blur_frames,
                &format!("{}/{}", self.scene_name, name),
                &mut memory,
                self.width,
                self.height,
                skip_existing,
            );
        }
    }
}

struct Window {
    renderer: SceneRenderer,

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

    available_scenes: Scenes,

    about: EngRusText,
    welcome: EngRusText,

    scene_initted: bool,

    scene_name: &'static str,

    gesture_recognizer: GestureRecognizer,
    input_subscriber_id: usize,

    render_scale: f32,

    edit_scene_side_panel: bool,
    scene_viewport: Option<ViewportRect>,
}

impl Window {
    fn edit_scene_ui_contents(&mut self, ui: &mut egui::Ui, changed: &mut WhatChanged) {
        let (changed1, material) =
            self.renderer
                .scene
                .egui(ui, &mut self.renderer.data, &mut self.should_recompile);

        *changed |= changed1;

        if changed.shader {
            self.should_recompile = true;
        }

        if let Some(material) = material {
            match material {
                Ok(material) => {
                    self.renderer.material = material;
                    self.error_message = None;
                }
                Err(err) => {
                    self.error_message = Some((err.0, err.1));
                    self.renderer.data.errors = err.2;
                }
            }
        }
    }

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

        // Prefer new serialized format, fallback to legacy
        let scene = match ron::from_str::<SerializedScene>(scene_content) {
            Ok(ser) => {
                // Build scene from new serialized format
                Scene::from_serialized(ser)
            }
            Err(_) => ron::from_str::<Scene>(scene_content).unwrap(),
        };

        Window {
            renderer: SceneRenderer::new(scene, 4000, 4000, scene_name).await,
            render_scale: 0.5,
            edit_scene_side_panel: true,
            scene_viewport: None,
            should_recompile: false,
            dpi_set: false,

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
        }
    }

    fn process_mouse_and_keys(&mut self, ctx: &egui::Context) -> bool {
        let mut is_something_changed = false;

        if !self.dpi_set {
            #[cfg(target_arch = "wasm32")]
            ctx.set_pixels_per_point(1.7);

            #[cfg(not(target_arch = "wasm32"))]
            ctx.set_pixels_per_point(1.3);
            self.dpi_set = true;
        }

        if !self.scene_initted {
            self.scene_initted = true;
            ctx.memory_mut(|memory| {
                self.renderer.scene.init(&mut self.renderer.data, memory);
                memory.data.insert_persisted(
                    egui::Id::new("OriginalCam"),
                    OriginalCam(self.renderer.cam.get_calculated_cam()),
                );
            });
        }

        let mut changed = WhatChanged::default();

        let mut menu_height_points = 0.0f32;

        if is_key_pressed(macroquad::input::KeyCode::Escape) {
            self.draw_menu = !self.draw_menu;
        }
        if self.draw_menu {
            let before_top = ctx.available_rect();
            egui::containers::panel::TopBottomPanel::top("my top").show(ctx, |ui| {
                use egui::menu;

                menu::bar(ui, |ui| {
                    ui.menu_button("🗋 Load", |ui| {
                        if let Some((content, link, name)) = self.available_scenes.egui(ui) {
                            if self.scene_name == "Room" {
                                self.control_scene_opened = true;
                            }
                            ctx.memory_mut(|memory| {
                                changed.uniform = true;
                                let scene = match ron::from_str::<SerializedScene>(content) {
                                    Ok(ser) => Scene::from_serialized(ser),
                                    Err(_) => ron::from_str::<Scene>(content).unwrap(),
                                };
                                self.renderer
                                    .load_from_scene(scene, memory)
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
                            });
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
            let after_top = ctx.available_rect();
            menu_height_points = after_top.top() - before_top.top();
        }
        let errors_count = self.renderer.scene.errors_count(0, &mut self.renderer.data);
        let edit_scene_title = if errors_count > 0 {
            format!("Edit scene ({} err)", errors_count)
        } else {
            "Edit scene".to_owned()
        };

        let mut edit_scene_side_panel = self.edit_scene_side_panel;
        let mut side_panel_width_points = 0.0f32;

        if self.edit_scene_opened {
            if edit_scene_side_panel {
                let before_side = ctx.available_rect();
                egui::SidePanel::left("Edit scene side panel").show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(edit_scene_title.clone());
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut edit_scene_side_panel, "Dock as side panel");
                            if ui.button("Close").clicked() {
                                self.edit_scene_opened = false;
                            }
                        });
                        ui.separator();

                        self.edit_scene_ui_contents(ui, &mut changed);
                    });
                });
                let after_side = ctx.available_rect();
                side_panel_width_points = after_side.left() - before_side.left();
            } else {
                let mut open = self.edit_scene_opened;
                egui::Window::new(edit_scene_title)
                    .id(egui::Id::new("Edit scene"))
                    .open(&mut open)
                    .vscroll(true)
                    .hscroll(true)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut edit_scene_side_panel, "Dock as side panel");
                        });
                        ui.separator();

                        self.edit_scene_ui_contents(ui, &mut changed);
                    });
                self.edit_scene_opened = open;
            }
        }
        if let Some((code, message)) = self.error_message.as_ref() {
            if self.renderer.data.show_error_window {
                egui::Window::new("Error message")
                    .vscroll(true)
                    .default_width(700.)
                    .show(ctx, |ui| {
                        egui::CollapsingHeader::new("code")
                            .id_salt(0)
                            .show(ui, |ui| {
                                ui.monospace(add_line_numbers(code));
                            });
                        egui::CollapsingHeader::new("message")
                            .id_salt(1)
                            .show(ui, |ui| {
                                ui.monospace(message);
                            });
                        egui::CollapsingHeader::new("message to copy")
                            .id_salt(2)
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
            let show_compiled_code = &mut self.renderer.data.show_compiled_code;
            let generated_code_show_text = &mut self.renderer.data.generated_code_show_text;
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
                self.renderer.data.show_compiled_code = None;
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
            if let Some(_to_export) = self.renderer.data.to_export.as_ref() {
                egui::Window::new("Export scene")
                    .open(&mut not_remove_export)
                    .vscroll(true)
                    .show(ctx, |ui| {
                        // UI state in memory
                        let (mut use_old, mut compact) = ui.memory_mut(|mem| {
                            let use_old = *mem.data.get_persisted_mut_or_default::<bool>(
                                egui::Id::new("export_use_old"),
                            );
                            let compact = *mem.data.get_persisted_mut_or_default::<bool>(
                                egui::Id::new("export_compact"),
                            );
                            (use_old, compact)
                        });

                        ui.checkbox(&mut use_old, "Use old format");
                        ui.checkbox(&mut compact, "Compact output");
                        ui.separator();

                        ui.memory_mut(|mem| {
                            mem.data
                                .insert_persisted(egui::Id::new("export_use_old"), use_old);
                            mem.data
                                .insert_persisted(egui::Id::new("export_compact"), compact);
                        });

                        // Build content
                        let content = {
                            if !use_old {
                                let ser = self.renderer.scene.to_serialized();
                                if !compact {
                                    ron::ser::to_string_pretty(
                                        &ser,
                                        ron::ser::PrettyConfig::default(),
                                    )
                                    .unwrap_or_else(|_| "<serialize error>".into())
                                } else {
                                    ron::to_string(&ser)
                                        .unwrap_or_else(|_| "<serialize error>".into())
                                }
                            } else {
                                if !compact {
                                    ron::ser::to_string_pretty(
                                        &self.renderer.scene,
                                        ron::ser::PrettyConfig::default(),
                                    )
                                    .unwrap_or_else(|_| "<serialize error>".into())
                                } else {
                                    ron::to_string(&self.renderer.scene)
                                        .unwrap_or_else(|_| "<serialize error>".into())
                                }
                            }
                        };
                        let mut content_mut = content.clone();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut content_mut)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor()
                                    .desired_rows(10)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY),
                            )
                        });
                    });
            }
            if !not_remove_export {
                self.renderer.data.to_export = None;
            }
        }

        {
            let mut opened = self.import_window.is_some();
            let mut import_window = self.import_window.clone();
            if let Some(content) = &mut import_window {
                egui::Window::new("Import scene")
                    .open(&mut opened)
                    .vscroll(true)
                    .show(ctx, |ui| {
                        let mut use_old = ui.memory_mut(|mem| {
                            *mem.data.get_persisted_mut_or_default::<bool>(egui::Id::new("import_use_old"))
                        });
                        ui.checkbox(&mut use_old, "Use old format");
                        ui.memory_mut(|mem| mem.data.insert_persisted(egui::Id::new("import_use_old"), use_old));
                        if ui.button("Recompile").clicked() {
                            if !use_old {
                                match ron::from_str::<SerializedScene>(content) {
                                    Ok(ser) => {
                                        let scene: Scene = Scene::from_serialized(ser);
                                        ui.memory_mut(|memory| {
                                            match self.renderer.load_from_scene(scene, memory) {
                                                Some(Ok(())) => {},
                                                Some(Err(_)) | None => {
                                                    self.should_recompile = true;
                                                    self.import_window_errors = Some("Errors in shaders, look into `Edit scene` window after pressing `Recompile`.".to_owned());
                                                },
                                            }
                                        });
                                    },
                                    Err(err) => {
                                        self.import_window_errors = Some(err.to_string());
                                    }
                                }
                            } else {
                                match ron::from_str::<Scene>(content) {
                                    Ok(scene) => {
                                        ui.memory_mut(|memory| {
                                            match self.renderer.load_from_scene(scene, memory) {
                                                Some(Ok(())) => {},
                                                Some(Err(_)) | None => {
                                                    self.should_recompile = true;
                                                    self.import_window_errors = Some("Errors in shaders, look into `Edit scene` window after pressing `Recompile`.".to_owned());
                                                },
                                            }
                                        });
                                    },
                                    Err(err) => {
                                        self.import_window_errors = Some(err.to_string());
                                    }
                                }
                            }
                        }

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(content)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor()
                                    .desired_rows(10)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY),
                            )
                        });

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
                        let text = self.renderer.scene.desc.text(ui);
                        egui_demo_lib::easy_mark::easy_mark(ui, text);
                    });
                    changed |= self
                        .renderer
                        .scene
                        .control_egui(ui, &mut self.renderer.data);
                });
            self.control_scene_opened = control_scene_opened;
        }

        {
            egui::Window::new("GLSL library")
                .open(&mut self.renderer.data.show_glsl_library)
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
                    changed |= self.renderer.cam.egui(ui);
                });
            self.camera_settings_opened = camera_settings_opened;
        }

        {
            let mut render_options_opened = self.render_options_opened;
            egui::Window::new("Render options")
                .open(&mut render_options_opened)
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label("Lower rendering resolution ratio (can significantly increase FPS):");
                    changed.uniform |= check_changed(&mut self.render_scale, |scale| {
                        ui.add(egui::Slider::new(scale, 0.01..=1.0).clamping(egui::widgets::SliderClamping::Always));
                    });
                    ui.label("(Render scale can be changed using four fingers, add one finger at a time to prevent triggering system four-finger gestures)");
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
                    ui.label("(DPI can be changed using three fingers, add one finger at a time to prevent triggering system three-finger gestures)");
                    ui.separator();
                    changed |= self.renderer.egui_rendering_settings(ui);
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
        let egui_using_keyboard = ctx.wants_keyboard_input();

        self.renderer.cam.current_dpi = ctx.pixels_per_point() as f64;
        self.renderer.cam.current_render_scale = self.render_scale;
        self.renderer.cam.is_something_changed = false;
        self.renderer.cam.mouse_over_canvas_right_now = mouse_over_canvas;
        macroquad::input::utils::repeat_all_miniquad_input(self, self.input_subscriber_id);
        if let Some(new_dpi) = self.renderer.cam.new_dpi {
            ctx.set_pixels_per_point(new_dpi as f32);
            self.renderer.cam.new_dpi = None;
            is_something_changed = true;
        }
        if let Some(new_render_scale) = self.renderer.cam.new_render_scale {
            self.render_scale = new_render_scale;
            self.renderer.cam.new_render_scale = None;
            is_something_changed = true;
        }

        let cam_changed = self
            .renderer
            .cam
            .process_mouse_and_keys(mouse_over_canvas, egui_using_keyboard);
        if changed.uniform
            || self.renderer.scene.use_time
            || cam_changed
            || self.renderer.scene.is_current_stage_real_animation()
            || self.renderer.scene.animation_stage_edit_state
        {
            ctx.memory_mut(|memory| {
                self.renderer
                    .update(memory, macroquad::miniquad::date::now());
            });

            is_something_changed = true;
        }

        if changed.shader {
            self.should_recompile = true;
        }

        self.edit_scene_side_panel = edit_scene_side_panel;

        let egui_screen = ctx.screen_rect();
        let scale_x = screen_width() / egui_screen.width();
        let scale_y = screen_height() / egui_screen.height();

        let menu_height = menu_height_points * scale_y;
        let side_panel_width = side_panel_width_points * scale_x;

        let viewport_x = side_panel_width;
        let viewport_y = menu_height;
        let viewport_width = screen_width() - side_panel_width;
        let viewport_height = screen_height() - menu_height;

        let new_viewport = ViewportRect {
            x: viewport_x,
            y: viewport_y,
            width: viewport_width,
            height: viewport_height,
        };

        if self
            .scene_viewport
            .map(|v| v != new_viewport)
            .unwrap_or(true)
        {
            is_something_changed = true;
        }

        self.scene_viewport = Some(new_viewport);

        is_something_changed
    }

    fn draw(&mut self) {
        let viewport = self.scene_viewport.unwrap_or(ViewportRect {
            x: 0.0,
            y: 0.0,
            width: screen_width(),
            height: screen_height(),
        });

        let render_width = (viewport.width * self.render_scale).max(1.0);
        let render_height = (viewport.height * self.render_scale).max(1.0);

        self.renderer
            .draw_texture(render_width, render_height, false);

        draw_texture_ex(
            &self.renderer.render_target.texture,
            viewport.x,
            viewport.y,
            WHITE,
            DrawTextureParams {
                source: Some(macroquad::math::Rect::new(
                    0.0,
                    0.0,
                    render_width,
                    render_height,
                )),
                dest_size: Some(macroquad::prelude::vec2(viewport.width, viewport.height)),
                ..Default::default()
            },
        );
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

async fn render() {
    // let (width, height) = (1920, 1080);
    let (width, height) = (3840, 2160);
    // let (width, height) = (3840, 3840);
    // let (width, height) = (854, 480);

    // render all scenes as a pictures
    /*
    let (width, height) = (4000, 2000);
    for scene_name in Scenes::default().get_all_scenes_links() {
        if scene_name != "room" {
            continue;
        }
        println!("Rendering scene {scene_name}");
        let scene_content = Scenes::default().get_by_link(&scene_name).unwrap().0;
        let scene = ron::from_str(scene_content).unwrap();
        let mut renderer = SceneRenderer::new(scene, width, height, &scene_name).await;
        renderer.cam.use_360_camera = true;
        renderer.aa_count = 256;
        renderer.cam.r = 0.;
        renderer.draw_texture(width as f32, height as f32, true);
        renderer.render_target.texture.get_texture_data().export_png(&format!("anim/{scene_name}.png"));
    }
    return;
    */

    #[allow(clippy::single_element_loop)]
    for scene_name in [
        // "triple_tiling",
        "boot.dev",
        // "plus_ultra",

        // "sphere_to_sphere",
        // "half_spheres",
        // "recursive_space",
        // "sphere_intersection",
        // "spheres_anim",
        // "teleportation_degrees",
        // "triple_portal_ish",
        // "monoportal_rotating",
    ] {
        println!("Rendering scene {scene_name}");

        let fps = 60;
        let motion_blur_frames = 10;
        let skip_existing = true;
        let starts_with = Some("anim");

        let scene_content = Scenes::default().get_by_link(scene_name).unwrap().0;
        let scene: SerializedScene = ron::from_str(scene_content).unwrap();
        let mut renderer =
            SceneRenderer::new(Scene::from_serialized(scene), width, height, scene_name).await;
        renderer.aa_count = 4;
        renderer.render_depth = 100;
        // renderer.cam.view_angle *= 1.5;

        if true {
            // if false {
            renderer.render_all_animations(fps, motion_blur_frames, skip_existing, starts_with);
        }

        // if true {
        if false {
            #[allow(clippy::single_element_loop)]
            for animation_stage in ["v2.screenshot.5"] {
                drop(std::fs::create_dir("video"));
                drop(std::fs::create_dir(format!(
                    "video/{}",
                    renderer.scene_name
                )));

                let mut memory = egui::Memory::default();
                renderer.current_fps = fps;
                renderer.current_motion_blur_frames = motion_blur_frames;
                renderer.use_animation_stage(animation_stage, &mut memory);
                renderer.render_animation(
                    renderer.scene.get_current_animation_duration().unwrap() as f32,
                    renderer.current_fps,
                    renderer.current_motion_blur_frames,
                    &format!("{scene_name}/{animation_stage}"),
                    &mut memory,
                    width,
                    height,
                    false,
                );
            }
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    color_backtrace::install();

    // if true {
    if false {
        render().await;
        return;
    }

    let mut window = Window::new().await;

    let mut texture = Texture2D::from_image(&get_screen_data());
    let mut w = screen_width();
    let mut h = screen_height();
    let mut image_size_changed = true;

    let mut ui_changed_image = true;

    macroquad::input::simulate_mouse_with_touch(false);

    // Always show hidden scenes on native (mostly for me to code)
    #[cfg(not(target_arch = "wasm32"))]
    egui_macroquad::ui(|ctx| {
        ctx.memory_mut(|memory| {
            memory
                .data
                .insert_persisted(egui::Id::new("ShowHiddenScenes"), ShowHiddenScenes(true))
        });
    });

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

        window.renderer.reload_textures().await;

        next_frame().await;
    }
}
