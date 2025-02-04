//%predefined_library//%

// ---------------------------------------------------------------------------
// User library --------------------------------------------------------------
// ---------------------------------------------------------------------------

//%uniforms//%

//%textures//%

//%materials_defines//%

//%library//%

//%intersection_functions//%

//%intersection_material_functions//%

SceneIntersection scene_intersect(Ray r) {
    SceneIntersection i = SceneIntersection(0, intersection_none);
    SceneIntersection ihit = SceneIntersection(0, intersection_none);
    SurfaceIntersection hit = intersection_none;
    vec3 normal = vec3(0.);
    int inside = NOT_INSIDE;
    float len = 1.;
    Ray transformed_ray = ray_none;

//%intersections//%

    return i;
}

MaterialProcessing material_process(Ray r, SceneIntersection i) {
    SurfaceIntersection hit = i.hit;
    if (i.material == 0) {
    } else if (i.material == DEBUG_RED) {
        return material_simple(hit, r, color(0.9, 0.2, 0.2), 0.5, false, 1., 0.);
    } else if (i.material == DEBUG_GREEN) {
        return material_simple(hit, r, color(0.2, 0.9, 0.2), 0.5, false, 1., 0.);
    } else if (i.material == DEBUG_BLUE) {
        return material_simple(hit, r, color(0.2, 0.2, 0.9), 0.5, false, 1., 0.);

//%material_processing//%

    }

    // If there is no material with this number.
    return material_final(vec3(0.));
}

SceneIntersectionWithMaterial scene_intersect_material_process(Ray r) {
    SceneIntersectionWithMaterial result = SceneIntersectionWithMaterial(scene_intersection_none, material_empty());
    SceneIntersectionWithMaterial hit = SceneIntersectionWithMaterial(scene_intersection_none, material_empty());

//%intersection_material_processing//%
    
    return result;
}

// ---------------------------------------------------------------------------
// Ray tracing ---------------------------------------------------------------
// ---------------------------------------------------------------------------

uniform int _ray_tracing_depth;
uniform float _t_end;
uniform float _t_start;

vec3 ray_tracing(Ray r) {
    vec3 current_color = vec3(1.);
    float all_t = 0.;
    for (int j = 0; j < 10000; j++) {
        if (j >= _ray_tracing_depth) break;
        SceneIntersection i = scene_intersect(r);
        SceneIntersectionWithMaterial i2 = scene_intersect_material_process(r);
        
        MaterialProcessing m;
        if (nearer(i.hit, i2.scene.hit)) {
            r.o += r.d * i2.scene.hit.t;
            all_t += i2.scene.hit.t * r.tmul;
            if (i2.scene.material == CUSTOM_MATERIAL) {
                m = i2.material;
            } else {
                m = material_process(r, i2.scene);
            }
        } else if (i.hit.hit) {
            r.o += r.d * i.hit.t;
            all_t += i.hit.t * r.tmul;
            m = material_process(r, i);
        }

        if (all_t > _t_end) return color(0., 0., 0.);

        // Offset ray
        if (i.hit.hit || i2.scene.hit.hit) {
            current_color *= m.mul_to_color;
            if (m.is_final) {
                if (all_t > _t_start) {
                    float gray_t = (all_t - _t_start) / (_t_end - _t_start);
                    return color(0., 0., 0.) * sqr(sqr(gray_t)) + current_color * sqr(sqr(1.0 - gray_t));
                } else {
                    return current_color;
                }
            } else {
                r = m.new_ray;
            }
        } else {
            return current_color * color(0.6, 0.6, 0.6);
        }
    }
    return color(0., 0., 0.);
}

// ---------------------------------------------------------------------------
// Camera teleportation ------------------------------------------------------
// ---------------------------------------------------------------------------

// thanks https://stackoverflow.com/questions/17981163/webgl-read-pixels-from-floating-point-render-target/20859830#20859830
float shift_right(float v, float amt) { // !RETAIN_FLOAT!
    v = floor(v) + 0.5; // !RETAIN_FLOAT!
    return floor(v / exp2(amt)); // !RETAIN_FLOAT!
}
float shift_left(float v, float amt) { // !RETAIN_FLOAT!
    return floor(v * exp2(amt) + 0.5); // !RETAIN_FLOAT!
}
float mask_last(float v, float bits) { // !RETAIN_FLOAT!
    return mod(v, shift_left(1.0, bits)); // !RETAIN_FLOAT!
}
float extract_bits(float num, float from, float to) { // !RETAIN_FLOAT!
    from = floor(from + 0.5); // !RETAIN_FLOAT!
    to = floor(to + 0.5); // !RETAIN_FLOAT!
    return mask_last(shift_right(num, from), to - from); // !RETAIN_FLOAT!
}
vec4 encode_float(float val) { // !RETAIN_FLOAT!
    if(val == 0.0) // !RETAIN_FLOAT!
        return vec4(0, 0, 0, 0); // !RETAIN_FLOAT!
    float sign = val > 0.0 ? 0.0 : 1.0; // !RETAIN_FLOAT!
    val = abs(val); // !RETAIN_FLOAT!
    float exponent = floor(log2(val)); // !RETAIN_FLOAT!
    float biased_exponent = exponent + 127.0; // !RETAIN_FLOAT!
    float fraction = ((val / exp2(exponent)) - 1.0) * 8388608.0; // !RETAIN_FLOAT!
    float t = biased_exponent / 2.0; // !RETAIN_FLOAT!
    float last_bit_of_biased_exponent = fract(t) * 2.0; // !RETAIN_FLOAT!
    float remaining_bits_of_biased_exponent = floor(t); // !RETAIN_FLOAT!
    float byte4 = extract_bits(fraction, 0.0, 8.0) / 255.0; // !RETAIN_FLOAT!
    float byte3 = extract_bits(fraction, 8.0, 16.0) / 255.0; // !RETAIN_FLOAT!
    float byte2 = (last_bit_of_biased_exponent * 128.0 + extract_bits(fraction, 16.0, 23.0)) / 255.0; // !RETAIN_FLOAT!
    float byte1 = (sign * 128.0 + remaining_bits_of_biased_exponent) / 255.0; // !RETAIN_FLOAT!
    return vec4(byte4, byte3, byte2, byte1); // !RETAIN_FLOAT!
}

vec4 teleport_external_ray(Ray r) {
    r = normalize_ray(r);
    bool have_result = false;
    bool stop_at_object = false;
    float all_t = 0.;
    for (int j = 0; j < 10; j++) {
        SceneIntersection i = scene_intersect(r);
        SceneIntersectionWithMaterial i2 = scene_intersect_material_process(r);
        
        bool continue_intersect = false;
        MaterialProcessing m;
        if (nearer(i.hit, i2.scene.hit)) {
            if (i2.scene.hit.t * r.tmul + all_t < 1.0) {
                r.o += r.d * i2.scene.hit.t;
                all_t += i2.scene.hit.t * r.tmul;
                if (i2.scene.material == CUSTOM_MATERIAL) {
                    m = i2.material;
                } else {
                    m = material_process(r, i2.scene);
                }
                continue_intersect = !m.is_final;
                stop_at_object = stop_at_object || m.is_final;
            }
        } else if (i.hit.hit) {
            if (i.hit.t * r.tmul + all_t < 1.0) {
                r.o += r.d * i.hit.t;
                all_t += i.hit.t * r.tmul;
                m = material_process(r, i);
                continue_intersect = !m.is_final;
                stop_at_object = stop_at_object || m.is_final;
            }
        }

        if (continue_intersect) {
            r = m.new_ray;
            have_result = true;
        } else {
            break;
        }
    }
    float last_value = 1.;
    if (stop_at_object) last_value = -1.;
    if (have_result) {
        r.o += r.d * (1.0 - all_t) / r.tmul;
        return vec4(r.o.xyz, last_value);
    } else {
        return vec4(vec3(0.), last_value);
    }
}

// ---------------------------------------------------------------------------
// Draw image ----------------------------------------------------------------
// ---------------------------------------------------------------------------

uniform dmat4 _camera; // !RETAIN_FLOAT!
uniform float _view_angle;
uniform int _use_panini_projection;
uniform float _panini_param;
uniform int _aa_count;
in vec2 uv; // absolute coordinates, integer values, from 0 // !RETAIN_FLOAT!
in vec2 uv_screen; // !RETAIN_FLOAT!
in float pixel_size; // !RETAIN_FLOAT!
out vec4 fragColor; // !RETAIN_FLOAT!

uniform int _teleport_external_ray;
uniform vec3 _external_ray_a;
uniform vec3 _external_ray_b;

const float Pi = 3.14159265359;
const float Pi2 = Pi * 2.0;
const float Pi05 = Pi * 0.5;

float Pow2(float x) {return x*x;}

// Thanks https://www.shadertoy.com/view/Wt3fzB
// tc ∈ [-1,1]² | fov ∈ [0, π) | d ∈ [0,1]
vec3 PaniniProjection(vec2 tc, float fov, float d)
{
    float d2 = d*d;

    {
        float fo = Pi05 - fov * 0.5;

        // There was * 2, because image should be 2×1 with coords [-1,1]², but in my version, image should be 1×1 with coords [-1,1]².
        float f = cos(fo)/sin(fo); // * 2.0
        float f2 = f*f;

        float b = (sqrt(max(0.0, Pow2(d+d2)*(f2+f2*f2))) - (d*f+f)) / (d2+d2*f2-1.0);

        tc *= b;
    }
    
    // http://tksharpless.net/vedutismo/Pannini/panini.pdf
    float h = tc.x;
    float v = tc.y;
    
    float h2 = h*h;
    
    float k = h2/Pow2(d+1.0);
    float k2 = k*k;
    
    float discr = max(0.0, k2*d2 - (k+1.0)*(k*d2-1.0));
    
    float cosPhi = (-k*d+sqrt(discr))/(k+1.0);
    float S = (d+1.0)/(d+cosPhi);
    float tanTheta = v/S;
    
    float sinPhi = sqrt(max(0.0, 1.0-Pow2(cosPhi)));
    if(tc.x < 0.0) sinPhi *= -1.0;
    
    float s = inversesqrt(1.0+Pow2(tanTheta));
    
    return vec3(sinPhi, tanTheta, cosPhi) * s;
}

vec3 get_color(vec2 image_position) {
    vec4 o = _camera * vec4(0., 0., 0., 1.);
    vec4 d;
    if (_use_panini_projection == 1) {
        d = normalize(_camera * vec4(PaniniProjection(vec2(image_position.x, image_position.y), _view_angle, _panini_param), 0.));
    } else {
        float h = tan(_view_angle / 2.);
        d = normalize(_camera * vec4(image_position.x * h, image_position.y * h, 1.0, 0.));
    }
     
    Ray r = Ray(o, d, 1.0);
    return ray_tracing(r);
}

// thanks https://habr.com/ru/post/440892/
vec2 quasi_random(int i) {
    float a1 = 0.7548776662466927600500267982588025643670318456949186300834636687;
    float a2 = 0.5698402909980532659121818632752155853637566123932930564053138358;
    return vec2(
        mod(0.5 + a1*float(i), 1.0),
        mod(0.5 + a2*float(i), 1.0)
    );
}

void main() {
    vec3 result = vec3(0.);

    if (_teleport_external_ray == 0) {
        for (int a = 0; a < 16; a++) {
            if (a >= _aa_count) break;
            vec2 offset = quasi_random(a);
            result += get_color(vec2(uv_screen.x, uv_screen.y) + offset * double(pixel_size) * 2.);
        }
        result = sqrt(result/float(_aa_count));
    } else {
        vec4 teleported = teleport_external_ray(Ray(vec4(_external_ray_a, 1.), vec4(_external_ray_b - _external_ray_a, 0.), 1.));

        float val = 0.;
        if (int(uv.y) == 0) { val = teleported.x; } else
        if (int(uv.y) == 1) { val = teleported.y; } else
        if (int(uv.y) == 2) { val = teleported.z; }

        vec4 encoded = encode_float(float(val)); // !RETAIN_FLOAT!
        if (int(uv.x) == 0) {
            result = vec3(encoded.x, encoded.y, encoded.z); 
        } else {
            result = vec3(encoded.w, encoded.w, encoded.w);
        }

        if (teleported.w < 0. && int(uv.x) == 1) {
            // we encountered object
            result.y = 1.;
            result.z = 0.;
        }
    }

    /*
    double val = (_camera * dvec4(1., 0., 0., 0.)).x; // !RETAIN_FLOAT!
    // double val = 420.420;
    vec4 encoded = encode_float(float(val)); // !RETAIN_FLOAT!
    if (int(uv.x) == 0) {
        result = vec3(encoded.x, encoded.y, encoded.z); 
    } else {
        result = vec3(encoded.w, encoded.w, encoded.w);
    }
    */

    fragColor = vec4(result.x, result.y, result.z, 1.); // !RETAIN_FLOAT!
}
