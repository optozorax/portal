use std::f32::consts::PI;
use macroquad::prelude::*;
use macroquad_profiler as profiler;
use glam::Mat4;

fn draw_multiline_text(text: &str, x: f32, y: f32, font_size: f32, color: Color) {
    for (pos, text) in text.split('\n').enumerate() {
        draw_text(text, x, y + (pos as f32) * font_size, font_size, color);
    }
}

#[macroquad::main("Mobius portal")]
async fn main() {
    let texture: Texture2D = load_texture("watermark.png").await;

    let lens_material = load_material(
        LENS_VERTEX_SHADER,
        LENS_FRAGMENT_SHADER,
        MaterialParams {
            uniforms: vec![
                ("angles".to_owned(), UniformType::Float3),
                ("resolution".to_owned(), UniformType::Float2),
                ("add_gray_after_teleportation".to_owned(), UniformType::Float1),
                ("first".to_owned(), UniformType::Mat4),
                ("first_inv".to_owned(), UniformType::Mat4),
                ("second".to_owned(), UniformType::Mat4),
                ("second_inv".to_owned(), UniformType::Mat4),
                ("teleport_light".to_owned(), UniformType::Int1),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    let mouse_sensitivity = 100.;

    let mut rotation_angle = 0.0f32;

    let mut angles: (f32, f32, f32) = (180., 0., 3.5);
    let angles_min: (f32, f32, f32) = (-f32::INFINITY, -89., 0.);
    let angles_max: (f32, f32, f32) = (f32::INFINITY, 89., 100.);

    let mut previous_mouse = Vec2::default();

    let mut add_gray_after_teleportation = 1.0f32;

    let mut teleport_light = false;

    const SCALE_FACTOR: f32 = 1.1;

    let mut first = Mat4::from_rotation_x(PI/2.) * Mat4::from_translation(-Vec3::new(0., 0., -2.));
    let second = Mat4::from_rotation_x(PI/2.) * Mat4::from_translation(-Vec3::new(0., 0., 2.));

    let mut first_inv = first.inverse() * second;
    let mut second_inv = second.inverse() * first;

    let mut show_help = true;
    let mut show_profiler = false;

    loop {
        if is_key_pressed(KeyCode::H) {
            show_help = !show_help;
        }
        if is_key_pressed(KeyCode::T) {
            teleport_light = !teleport_light;
        }
        if is_key_pressed(KeyCode::P) {
            show_profiler = !show_profiler;
        }
        if is_key_down(KeyCode::A) {
            rotation_angle = clamp(rotation_angle + 1./180.*PI, 0., PI);
            first = Mat4::from_rotation_y(rotation_angle) * Mat4::from_rotation_x(PI/2.) * Mat4::from_translation(-Vec3::new(0., 0., -2.));
            first_inv = first.inverse() * second;
            second_inv = second.inverse() * first;
        }
        if is_key_down(KeyCode::B) {
            rotation_angle = clamp(rotation_angle - 1./180.*PI, 0., PI);
            first = Mat4::from_rotation_y(rotation_angle) * Mat4::from_rotation_x(PI/2.) * Mat4::from_translation(-Vec3::new(0., 0., -2.));
            first_inv = first.inverse() * second;
            second_inv = second.inverse() * first;
        }
        if is_key_down(KeyCode::X) {
            add_gray_after_teleportation = clamp(add_gray_after_teleportation - 0.01, 0., 1.);
        }
        if is_key_down(KeyCode::Y) {
            add_gray_after_teleportation = clamp(add_gray_after_teleportation + 0.01, 0., 1.);
        }

        let mouse_pos: Vec2 = mouse_position_local();

        if is_mouse_button_down(MouseButton::Left) {
            let dalpha = -(mouse_pos.x - previous_mouse.x) * mouse_sensitivity;
            let dbeta = (mouse_pos.y - previous_mouse.y) * mouse_sensitivity;

            angles.0 = clamp(angles.0 + dalpha, angles_min.0, angles_max.0);
            angles.1 = clamp(angles.1 + dbeta, angles_min.1, angles_max.1);
        }

        let wheel_value = mouse_wheel().1;
        if wheel_value > 0. {
            angles.2 *= 1.0 / SCALE_FACTOR;
        } else if wheel_value < 0. {
            angles.2 *= SCALE_FACTOR;
        }

        lens_material.set_uniform("angles", angles);
        lens_material.set_uniform("resolution", (screen_width(), screen_height()));
        lens_material.set_uniform("add_gray_after_teleportation", add_gray_after_teleportation);
        lens_material.set_uniform("first", first);
        lens_material.set_uniform("second", second);
        lens_material.set_uniform("first_inv", first_inv);
        lens_material.set_uniform("second_inv", second_inv);
        lens_material.set_uniform("teleport_light", teleport_light as i32);

        gl_use_material(lens_material);
        draw_texture_ex(
            texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
        gl_use_default_material();

        if show_help {
            draw_multiline_text(
                "h - hide this message\nt - enable texture on Mobius strip\na/b - rotate blue portal\nx/y - make teleported rays darker\np - enable profiler",
                5.0,
                15.0,
                20.0,
                BLACK,
            );
        }

        previous_mouse = mouse_pos;

        if show_profiler {
            set_default_camera();
            profiler::profiler(profiler::ProfilerParams {
                fps_counter_pos: vec2(10.0, 10.0),
            });
        }

        next_frame().await
    }
}

const LENS_FRAGMENT_SHADER: &'static str = include_str!("frag.glsl");

const LENS_VERTEX_SHADER: &'static str = "#version 100
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
