use macroquad::prelude::*;
use macroquad_profiler as profiler;

#[macroquad::main("MACROQUAD")]
async fn main() {
    let lens_material = load_material(
        LENS_VERTEX_SHADER,
        LENS_FRAGMENT_SHADER,
        MaterialParams {
            uniforms: vec![
                ("angles".to_owned(), UniformType::Float3),
                ("resolution".to_owned(), UniformType::Float2),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    let mouse_sensitivity = 100.;

    let mut angles: (f32, f32, f32) = (20., 30., 3.);
    let angles_min: (f32, f32, f32) = (-f32::INFINITY, -89., 0.);
    let angles_max: (f32, f32, f32) = (f32::INFINITY, 89., 100.);

    let mut previous_mouse = Vec2::default();

    const SCALE_FACTOR: f32 = 1.1;

    loop {
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

        gl_use_material(lens_material);
        draw_rectangle(0., 0., screen_width(), screen_height(), WHITE);
        gl_use_default_material();

        previous_mouse = mouse_pos;

        set_default_camera();

        profiler::profiler(profiler::ProfilerParams {
            fps_counter_pos: vec2(10.0, 10.0),
        });

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
