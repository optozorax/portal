use crate::megaui::Ui;
use crate::megaui::Vector2;
use macroquad::prelude::*;
use macroquad_profiler as profiler;
use std::f32::consts::PI;

mod gui;
use crate::gui::*;

use megaui_macroquad::{
    draw_megaui, draw_window,
    megaui::{self, hash},
    WindowParams,
};

pub trait UniformStruct {
    fn uniforms(&self) -> Vec<(String, UniformType)>;
    fn set_uniforms(&self, material: Material);
}

pub struct MatWithInversion {
    matrix: Mat4,
    matrix_inverse: Mat4,

    name_matrix: String,
    name_matrix_inverse: String,
}

impl MatWithInversion {
    pub fn new(matrix: Mat4, name: &str) -> Self {
        Self {
            matrix_inverse: matrix.inverse(),
            matrix,

            name_matrix: name.to_owned(),
            name_matrix_inverse: format!("{}_inv", name),
        }
    }

    pub fn get(&self) -> &Mat4 {
        &self.matrix
    }

    pub fn get_inverse(&self) -> &Mat4 {
        &self.matrix_inverse
    }

    pub fn set(&mut self, matrix: Mat4) {
        self.matrix_inverse = matrix.inverse();
        self.matrix = matrix;
    }
}

impl UniformStruct for MatWithInversion {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        vec![
            (self.name_matrix.clone(), UniformType::Mat4),
            (self.name_matrix_inverse.clone(), UniformType::Mat4),
        ]
    }

    fn set_uniforms(&self, material: Material) {
        material.set_uniform(&self.name_matrix, self.matrix);
        material.set_uniform(&self.name_matrix_inverse, self.matrix_inverse);
    }
}

pub struct MatPortal {
    first: Mat4,
    first_inverse: Mat4,
    first_teleport: Mat4,
    second: Mat4,
    second_inverse: Mat4,
    second_teleport: Mat4,

    name_first: String,
    name_first_inverse: String,
    name_first_teleport: String,
    name_second: String,
    name_second_inverse: String,
    name_second_teleport: String,
}

impl MatPortal {
    pub fn new(first: Mat4, second: Mat4, name: &str) -> Self {
        let mut result = Self {
            first: Mat4::default(),
            first_inverse: Mat4::default(),
            first_teleport: Mat4::default(),
            second: Mat4::default(),
            second_inverse: Mat4::default(),
            second_teleport: Mat4::default(),

            name_first: format!("{}_first", name),
            name_first_inverse: format!("{}_first_inverse", name),
            name_first_teleport: format!("{}_first_teleport", name),
            name_second: format!("{}_second", name),
            name_second_inverse: format!("{}_second_inverse", name),
            name_second_teleport: format!("{}_second_teleport", name),
        };
        result.set(Some(first), Some(second));
        result
    }

    pub fn set(&mut self, first: Option<Mat4>, second: Option<Mat4>) {
        if let Some(new_first) = first {
            self.first = new_first;
        }
        if let Some(new_second) = second {
            self.second = new_second;
        }

        self.first_inverse = self.first.inverse();
        self.second_inverse = self.second.inverse();

        self.first_teleport = self.second * self.first.inverse();
        self.second_teleport = self.first * self.second.inverse();
    }
}

impl UniformStruct for MatPortal {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        vec![
            (self.name_first.clone(), UniformType::Mat4),
            (self.name_first_inverse.clone(), UniformType::Mat4),
            (self.name_first_teleport.clone(), UniformType::Mat4),
            (self.name_second.clone(), UniformType::Mat4),
            (self.name_second_inverse.clone(), UniformType::Mat4),
            (self.name_second_teleport.clone(), UniformType::Mat4),
        ]
    }

    fn set_uniforms(&self, material: Material) {
        material.set_uniform(&self.name_first, self.first);
        material.set_uniform(&self.name_first_inverse, self.first_inverse);
        material.set_uniform(&self.name_first_teleport, self.first_teleport);
        material.set_uniform(&self.name_second, self.second);
        material.set_uniform(&self.name_second_inverse, self.second_inverse);
        material.set_uniform(&self.name_second_teleport, self.second_teleport);
    }
}

struct Scene {
    portals: Vec<MatPortal>,
    planes: Vec<MatWithInversion>,
    images: Vec<Texture2D>,

    portal_rotation: f32,
    portal_offset: f32,

    triangle_offset: Vec3,
    triangle_size: f32,
    portal_color_blend: f32,
    side_border_progress: f32,
    teleportation_enabled: bool,
    second_portal_disabled: bool,

    teleport_light: bool,
    add_gray_after_teleportation: f32,
}

enum SceneStage {
    DoorwayToPortal { progress: f32 },
    RotatePortal { progress: f32 },
    DisableBorders { progress: f32 },
    ShowTriangle { progress: f32 },
    Final { triangle_offset: Vec3 },
}

fn progress_on_segment<F: Fn(f32) -> f32>(
    segment: std::ops::Range<f32>,
    value: f32,
    outside: f32,
    f: F,
) -> f32 {
    if segment.start <= value && value <= segment.end {
        f((value - segment.start) / (segment.end - segment.start))
    } else {
        outside
    }
}

impl Scene {
    fn textures(&self) -> Vec<String> {
        vec!["watermark".to_owned()]
    }

    fn first_portal(change: f32, offset: f32) -> Mat4 {
        Mat4::from_rotation_y(PI / 2. - PI * change)
            * Mat4::from_scale(Vec3::new(2., 2., 2.))
            * Mat4::from_rotation_z(0.)
            * Mat4::from_translation(Vec3::new(0., 0., offset))
    }

    async fn new() -> Self {
        let portal_rotation = 1.;
        let portal_offset = 0.;

        let triangle_offset = Vec3::new(-1.2, 0.5, 0.);

        let portals = vec![
            MatPortal::new(
                Self::first_portal(portal_rotation, portal_offset),
                Mat4::from_rotation_y(PI / 2.)
                    * Mat4::from_scale(Vec3::new(-2., 2., 2.))
                    * Mat4::from_rotation_z(1. * PI),
                "portal_mat",
            ),
            MatPortal::new(
                Mat4::from_rotation_y(PI / 2. - PI)
                    * Mat4::from_scale(Vec3::new(2., 2., 2.))
                    * Mat4::from_rotation_z(1.5 * PI),
                Mat4::from_rotation_y(PI / 2.)
                    * Mat4::from_scale(Vec3::new(-2., 2., 2.))
                    * Mat4::from_rotation_z(0.5 * PI),
                "portal1_mat",
            ),
        ];

        let planes = vec![
            MatWithInversion::new(Mat4::from_translation(Vec3::new(0., 0., 4.5)), "plane1"),
            MatWithInversion::new(Mat4::from_translation(Vec3::new(0., 0., -4.5)), "plane2"),
            MatWithInversion::new(
                Mat4::from_rotation_x(PI / 2.) * Mat4::from_translation(Vec3::new(0., 0., 4.5)),
                "plane3",
            ),
            MatWithInversion::new(
                Mat4::from_rotation_x(PI / 2.) * Mat4::from_translation(Vec3::new(0., 0., -4.5)),
                "plane4",
            ),
            MatWithInversion::new(
                Mat4::from_rotation_y(PI / 2.) * Mat4::from_translation(Vec3::new(0., 0., 4.5)),
                "plane5",
            ),
            MatWithInversion::new(
                Mat4::from_rotation_y(PI / 2.) * Mat4::from_translation(Vec3::new(0., 0., -4.5)),
                "plane6",
            ),
            MatWithInversion::new(
                Mat4::from_rotation_x(PI / 2.) * Mat4::from_translation(triangle_offset),
                "triangle",
            ),
        ];

        let images = vec![load_texture("monoportal.png").await];

        Self {
            portals,
            planes,
            images,

            portal_rotation,
            portal_offset,
            triangle_size: 1.4,
            portal_color_blend: 1.,
            triangle_offset,
            add_gray_after_teleportation: 1.0,
            teleport_light: true,
            teleportation_enabled: true,
            second_portal_disabled: false,
            side_border_progress: 1.0,
        }
    }

    fn process_mouse_and_keys(&mut self) -> bool {
        let mut is_something_changed = false;

        return is_something_changed;
    }

    fn process_stage(&mut self, stage: SceneStage) {
        use SceneStage::*;
        match stage {
            DoorwayToPortal { progress } => {
                self.portal_rotation = 0.;
                self.portal_offset = progress * 0.3;

                self.triangle_offset = Vec3::default();
                self.triangle_size = 0.;
                self.portal_color_blend = progress;
                self.side_border_progress = 1.;
                if progress == 0. {
                    self.teleportation_enabled = false;
                    self.second_portal_disabled = true;
                } else {
                    self.teleportation_enabled = true;
                    self.second_portal_disabled = false;
                }
            }
            RotatePortal { progress } => {
                self.portal_rotation = progress;
                self.portal_offset =
                    progress_on_segment(0.0..0.2, progress, 0., |p| (1. - p) * 0.3);

                self.triangle_offset = Vec3::default();
                self.triangle_size = 0.;
                self.portal_color_blend = 1.;
                self.side_border_progress = 1.;
                self.teleportation_enabled = true;
                self.second_portal_disabled = false;
            }
            DisableBorders { progress } => {
                self.portal_rotation = 1.;
                self.portal_offset = 0.;

                self.triangle_offset = Vec3::default();
                self.triangle_size = 0.;
                self.portal_color_blend = 1. - progress;
                self.side_border_progress = 1. - progress;
                self.teleportation_enabled = true;
                self.second_portal_disabled = false;
            }
            ShowTriangle { progress } => {
                self.portal_rotation = 1.;
                self.portal_offset = 0.;

                self.triangle_offset = Vec3::new(-1.2, 0.5, 0.);
                self.triangle_size = progress * 1.2;
                self.portal_color_blend = 0.;
                self.side_border_progress = 0.;
                self.teleportation_enabled = true;
                self.second_portal_disabled = false;
            }
            Final { triangle_offset } => {
                self.portal_rotation = 1.;
                self.portal_offset = 0.;

                self.triangle_offset = triangle_offset;
                self.triangle_size = 1.2;
                self.portal_color_blend = 0.;
                self.side_border_progress = 0.;
                self.teleportation_enabled = true;
                self.second_portal_disabled = false;
            }
        }
        self.planes[6]
            .set(Mat4::from_rotation_x(PI / 2.) * Mat4::from_translation(self.triangle_offset));
        self.portals[0].set(
            Some(Self::first_portal(self.portal_rotation, self.portal_offset)),
            None,
        );
    }
}

impl UniformStruct for Scene {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        let mut result = vec![
            ("triangle_size".to_owned(), UniformType::Float1),
            ("portal_color_blend".to_owned(), UniformType::Float1),
            ("teleportation_enabled".to_owned(), UniformType::Int1),
            ("second_portal_disabled".to_owned(), UniformType::Int1),
            ("side_border_progress".to_owned(), UniformType::Float1),
            (
                "add_gray_after_teleportation".to_owned(),
                UniformType::Float1,
            ),
            ("teleport_light".to_owned(), UniformType::Int1),
        ];

        for i in &self.portals {
            result.extend(i.uniforms());
        }
        for i in &self.planes {
            result.extend(i.uniforms());
        }

        result
    }

    fn set_uniforms(&self, material: Material) {
        material.set_uniform("triangle_size", self.triangle_size);
        material.set_uniform("portal_color_blend", self.portal_color_blend);
        material.set_uniform(
            "add_gray_after_teleportation",
            self.add_gray_after_teleportation,
        );
        material.set_uniform("teleport_light", self.teleport_light as i32);
        material.set_uniform("teleportation_enabled", self.teleportation_enabled as i32);
        material.set_uniform("second_portal_disabled", self.second_portal_disabled as i32);
        material.set_uniform("side_border_progress", self.side_border_progress);

        for i in &self.portals {
            i.set_uniforms(material);
        }
        for i in &self.planes {
            i.set_uniforms(material);
        }

        material.set_texture("watermark", self.images[0]);
    }
}

struct RotateAroundCam {
    alpha: f32,
    beta: f32,
    r: f32,
    previous_mouse: Vec2,
}

impl RotateAroundCam {
    const BETA_MIN: f32 = 0.01;
    const BETA_MAX: f32 = PI - 0.01;
    const MOUSE_SENSITIVITY: f32 = 1.2;
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

impl UniformStruct for RotateAroundCam {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        vec![("camera".to_owned(), UniformType::Mat4)]
    }

    fn set_uniforms(&self, material: Material) {
        material.set_uniform("camera", self.get_matrix());
    }
}

struct Window {
    show_profiler: bool,

    scene: Scene,
    cam: RotateAroundCam,

    chosen: usize,
    progress1: f32,
    progress2: f32,
    progress3: f32,
    progress4: f32,
    progress5: f32,
    progress6: f32,
    progress7: f32,

    matrices: Matrices,
}

impl Window {
    async fn new() -> Self {
        Window {
            show_profiler: false,

            scene: {
                let mut result = Scene::new().await;
                result.process_stage(SceneStage::Final {
                    triangle_offset: Vec3::new(-1.2, 0.5, 0.),
                });
                result
            },
            cam: RotateAroundCam::new(),

            chosen: 4,
            progress1: 0.,
            progress2: 0.,
            progress3: 0.,
            progress4: 0.,
            progress5: 0.,
            progress6: 0.5,
            progress7: -1.2,

            matrices: Matrices::default(),
        }
    }

    fn process_mouse_and_keys(&mut self) -> bool {
        let mut is_something_changed = false;

        let mut mouse_over_canvas = true;

        let mut scene_changed = false;

        draw_window(
            hash!(),
            vec2(20., 400.),
            vec2(400., 150.),
            WindowParams {
                label: "Configure scene".to_owned(),
                close_button: false,
                ..Default::default()
            },
            |ui| {
                mouse_over_canvas &=
                    !ui.is_mouse_over(Vector2::new(mouse_position().0, mouse_position().1));

                if ui.button(None, "Teleport light") {
                    self.scene.teleport_light = !self.scene.teleport_light;
                    is_something_changed = true;
                }

                ui.same_line(0.);

                // if ui.button(None, "Show profiler") {
                //     self.show_profiler = !self.show_profiler;
                //     is_something_changed = true;
                // }

                ui.separator();

                let previous = self.chosen;
                let new = ui.combo_box(
                    hash!(),
                    "Step",
                    &[
                        "From doorway to portals",
                        "Rotate one of portal",
                        "Disable borders",
                        "Show triangle",
                        "Move triangle",
                    ],
                    Some(&mut self.chosen),
                );

                if previous != new {
                    scene_changed = true;
                }

                if self.chosen == 0 {
                    let previous = self.progress1;
                    ui.slider(hash!(), "Progress", 0.0..1.0, &mut self.progress1);
                    if previous != self.progress1 {
                        scene_changed = true;
                    }
                } else if self.chosen == 1 {
                    let previous = self.progress2;
                    ui.slider(hash!(), "Progress", 0.0..1.0, &mut self.progress2);
                    if previous != self.progress2 {
                        scene_changed = true;
                    }
                } else if self.chosen == 2 {
                    let previous = self.progress3;
                    ui.slider(hash!(), "Progress", 0.0..1.0, &mut self.progress3);
                    if previous != self.progress3 {
                        scene_changed = true;
                    }
                } else if self.chosen == 3 {
                    let previous = self.progress4;
                    ui.slider(hash!(), "Progress", 0.0..1.0, &mut self.progress4);
                    if previous != self.progress4 {
                        scene_changed = true;
                    }
                } else if self.chosen == 4 {
                    let previous = (self.progress5, self.progress6, self.progress7);
                    ui.slider(hash!(), "Vertical", -1.6..1.6, &mut self.progress5);
                    ui.slider(hash!(), "Horizontal", -1.6..1.6, &mut self.progress6);
                    ui.slider(hash!(), "Into portal", -1.4..1.4, &mut self.progress7);
                    if previous != (self.progress5, self.progress6, self.progress7) {
                        scene_changed = true;
                    }
                    if ui.button(None, "Clear") {
                        self.progress5 = 0.;
                        self.progress6 = 0.5;
                        self.progress7 = -1.4;
                        scene_changed = true;
                    }
                }
            },
        );

        draw_window(
            hash!(),
            vec2(300., 400.),
            vec2(400., 650.),
            WindowParams {
                label: "Configure matrix".to_owned(),
                close_button: false,
                ..Default::default()
            },
            |ui| {
                mouse_over_canvas &=
                    !ui.is_mouse_over(Vector2::new(mouse_position().0, mouse_position().1));

                if self.matrices.ui(ui, hash!()) {
                    is_something_changed = true;
                    if let Some(matrix) = self.matrices.get_matrix("portal1") {
                        self.scene.portals[0].set(Some(matrix), None);
                    }
                }
                if self.matrices.get_matrix("portal1").is_none() {
                    ui.label(None, "Can't get matrix, these is errror somewhere");
                }
                
            },
        );

        if scene_changed {
            if self.chosen == 0 {
                self.scene.process_stage(SceneStage::DoorwayToPortal {
                    progress: self.progress1,
                });
            } else if self.chosen == 1 {
                self.scene.process_stage(SceneStage::RotatePortal {
                    progress: self.progress2,
                });
            } else if self.chosen == 2 {
                self.scene.process_stage(SceneStage::DisableBorders {
                    progress: self.progress3,
                });
            } else if self.chosen == 3 {
                self.scene.process_stage(SceneStage::ShowTriangle {
                    progress: self.progress4,
                });
            } else if self.chosen == 4 {
                self.scene.process_stage(SceneStage::Final {
                    triangle_offset: Vec3::new(self.progress7, self.progress6, self.progress5),
                });
            }
            is_something_changed = true;
        }
        is_something_changed |= self.scene.process_mouse_and_keys();
        is_something_changed |= self.cam.process_mouse_and_keys(mouse_over_canvas);

        return is_something_changed;
    }

    fn draw(&self, material: Material) {
        gl_use_material(material);
        draw_rectangle(0., 0., screen_width(), screen_height(), WHITE);
        gl_use_default_material();

        if self.show_profiler {
            set_default_camera();
            profiler::profiler(profiler::ProfilerParams {
                fps_counter_pos: vec2(10.0, 10.0),
            });
        }
    }
}

impl UniformStruct for Window {
    fn uniforms(&self) -> Vec<(String, UniformType)> {
        let mut result = vec![("resolution".to_owned(), UniformType::Float2)];
        result.extend(self.scene.uniforms());
        result.extend(self.cam.uniforms());
        result
    }

    fn set_uniforms(&self, material: Material) {
        material.set_uniform("resolution", (screen_width(), screen_height()));

        self.scene.set_uniforms(material);
        self.cam.set_uniforms(material);
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

    let lens_material = load_material(
        VERTEX_SHADER,
        FRAGMENT_SHADER,
        MaterialParams {
            uniforms: window.uniforms(),
            textures: window.scene.textures(),
            ..Default::default()
        },
    )
    .unwrap_or_else(|err| {
        if let miniquad::graphics::ShaderError::CompilationError { error_message, .. } = err {
            println!("Fragment shader compilation error:\n{}", error_message);
        } else {
            println!("Other material error:\n{:#?}", err);
        }
        std::process::exit(1)
    });

    let mut texture = load_texture_from_image(&get_screen_data());
    let mut w = screen_width();
    let mut h = screen_height();
    let mut image_size_changed = true;

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

        if window.process_mouse_and_keys() || image_size_changed {
            window.set_uniforms(lens_material);
            window.draw(lens_material);
            set_default_camera();
            draw_megaui();
            texture.grab_screen();
            image_size_changed = false;
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
            draw_megaui();
        }

        next_frame().await;
    }
}

const FRAGMENT_SHADER: &'static str = include_str!("frag.glsl");

const VERTEX_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;

varying lowp vec2 uv;
varying lowp vec2 uv_screen;

uniform mat4 Model;
uniform mat4 Projection;

uniform vec2 Center;
uniform vec2 resolution;

void main() {
    vec4 res = Projection * Model * vec4(position, 1);

    uv_screen = (position.xy - resolution/2.) / min(resolution.x, resolution.y) * 2.;
    uv_screen.y *= -1.;
    uv = texcoord;

    gl_Position = res;
}
";
