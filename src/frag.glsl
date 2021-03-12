#version 100
precision highp float;

// ---------------------------------------------------------------------------
// Utils ---------------------------------------------------------------------
// ---------------------------------------------------------------------------

#define PI acos(-1.)
#define PI2 (acos(-1.) / 2.0)

struct Ray
{
    vec4 o; // origin
    vec4 d; // direction
};

const Ray no_ray = Ray(vec4(0.), vec4(0.));

vec3 normalize_normal(vec3 normal, Ray r) {
    normal = normalize(normal);
    if (dot(normal, r.d.xyz) > 0.) {
        normal *= -1.;
    }
    return normal;
}

vec3 color(float r, float g, float b) {
    return vec3(r*r, g*g, b*b);
}

bool is_collinear(vec3 a, vec3 b) {
    return abs(dot(a, b) / (length(a) * length(b)) - 1.) < 0.01;
}

bool between(float a, float x, float b) {
    return a <= x && x <= b;
}

float sqr(float a) {
    return a*a;
}

// ---------------------------------------------------------------------------
// Ray tracing of other objects ----------------------------------------------
// ---------------------------------------------------------------------------

struct SurfaceIntersect {
    bool hit;
    float t;
    float u;
    float v;
    vec3 n;
};

const SurfaceIntersect no_intersect = SurfaceIntersect(false, 1e10, 0., 0., vec3(0.));

SurfaceIntersect plane_intersect(Ray r, mat4 plane, vec3 normal) {
    vec3 ro = (plane * r.o).xyz;
    vec3 rd = (plane * r.d).xyz;

    float t = -ro.z/rd.z;
    if (t < 0.) {
        return no_intersect;
    } else {
        vec3 pos = ro + rd * t; 
        return SurfaceIntersect(true, t, pos.x, pos.y, normalize_normal(normal, r));
    }
}

// ---------------------------------------------------------------------------
// Material utils ------------------------------------------------------------
// ---------------------------------------------------------------------------

vec3 add_normal_to_color(vec3 color, vec3 normal, vec4 direction) {
    const float not_dark_count = 0.4;
    color *= (abs(dot(normalize(direction.xyz), normalize(normal))) + not_dark_count) / (1. + not_dark_count);
    return color;
}

vec3 grid_color(vec3 start, vec2 uv) {
    uv = uv - vec2(0.125, 0.125);
    const float fr = 3.14159*8.0;
    vec3 col = start;
    col += 0.4*smoothstep(-0.01,0.01,cos(uv.x*fr*0.5)*cos(uv.y*fr*0.5)); 
    float wi = smoothstep(-1.0,-0.98,cos(uv.x*fr))*smoothstep(-1.0,-0.98,cos(uv.y*fr));
    col *= wi;
    
    return col;
}

// ---------------------------------------------------------------------------
// Scene intersection --------------------------------------------------------
// ---------------------------------------------------------------------------

struct SceneIntersection {
    int material;
    SurfaceIntersect hit;
};

bool nearer(SurfaceIntersect hit, SceneIntersection i) {
    return hit.hit && hit.t < i.hit.t;
}

%%uniforms%%

#define NOT_INSIDE 0
#define TELEPORT 1
%%materials_defines%%

%%intersection_functions%%

SceneIntersection process_plane_intersection(SceneIntersection i, SurfaceIntersect hit, int inside) {
    if (hit.hit && hit.t < i.hit.t) {
        if (inside == NOT_INSIDE) {
            // Not inside, do nothing
        } else if (inside == TELEPORT) {
            // This is wrong code, do nothing
        } else {
            i.hit = hit;
            i.material = inside;
        }
    }
    return i;
}

SceneIntersection process_portal_intersection(SceneIntersection i, SurfaceIntersect hit, int inside, int material) {
    if (hit.hit && hit.t < i.hit.t) {
        if (inside == NOT_INSIDE) {
            // Not inside, do nothing
        } else if (inside == TELEPORT) {
            i.hit = hit;
            i.material = material;
        } else {
            i.hit = hit;
            i.material = inside;
        }
    }
    return i;
}

SceneIntersection scene_intersect(Ray r) {
    SceneIntersection i = SceneIntersection(0, no_intersect);
    SurfaceIntersect hit = no_intersect;
    int inside = NOT_INSIDE;

%%intersections%%

    return i;
}

// ---------------------------------------------------------------------------
// Scene materials processing ------------------------------------------------
// ---------------------------------------------------------------------------

struct MaterialProcessing {
    bool is_final;
    vec3 mul_to_color;
    Ray new_ray;
};

MaterialProcessing material_final(vec3 color) {
    return MaterialProcessing(true, color, no_ray);    
}

MaterialProcessing material_next(vec3 mul_color, Ray new_ray) {
    return MaterialProcessing(false, mul_color, new_ray);
}

MaterialProcessing plane_process_material(SurfaceIntersect hit, Ray r, vec3 clr) {
    const float plane_size = 4.5;
    const float plane_scale = 1./(plane_size * 2.);
    vec3 new_clr = grid_color(clr, vec2(hit.u, hit.v) * plane_scale);
    clr = (clr*2. + new_clr)/3.;
    return MaterialProcessing(true, add_normal_to_color(clr, hit.n, r.d), no_ray);
}

vec3 reflect(vec3 dir, vec3 normal) {
     return dir - normal * dot(dir, normal) / dot(normal, normal) * 2.;
}

vec3 refract(vec3 dir, vec3 normal, float refractive_index) {
    float ri = refractive_index;
    bool from_outside = dot(normal, dir) > 0.;
    if (!from_outside) {
        ri = 1. / ri;
    } else {
        normal = -normal;
    }

    dir = normalize(dir);
    float c = -dot(normal, dir);
    float d = 1.0 - ri * ri * (1.0 - c*c);
    if (d > 0.) {
        return dir * ri + normal * (ri * c - sqrt(d));
    } else {
        return reflect(dir, normal);
    }
}

MaterialProcessing teleport(mat4 matrix, Ray r) {
    r.o += r.d * 0.0001;

    r.o = matrix * r.o;
    r.d = matrix * r.d;

    return MaterialProcessing(false, vec3(1.0), r);
}

MaterialProcessing material_process(Ray r, SceneIntersection i) {
    SurfaceIntersect hit = i.hit;
    if (i.material == -1) {

%%material_processing%%

    }
    return material_final(vec3(0.));
}

// ---------------------------------------------------------------------------
// Ray tracing ---------------------------------------------------------------
// ---------------------------------------------------------------------------

vec3 ray_tracing(Ray r) {
    vec3 current_color = vec3(1.);
    for (int j = 0; j < 100; j++) {
        SceneIntersection i = scene_intersect(r);
        r.o += r.d * i.hit.t;
        if (i.hit.hit) {
            MaterialProcessing m = material_process(r, i);
            current_color *= m.mul_to_color;
            if (m.is_final) {
                return current_color;
            } else {
                r = m.new_ray;
            }
        } else {
            return current_color * vec3(0.6, 0.6, 0.6);
        }
    }
    return current_color;
}

// ---------------------------------------------------------------------------
// Draw image ----------------------------------------------------------------
// ---------------------------------------------------------------------------

uniform mat4 _camera;
varying vec2 uv;
varying vec2 uv_screen;

void main() {
    vec4 o = _camera * vec4(0., 0., 0., 1.);
    vec4 d = normalize(_camera * vec4(uv_screen.x, uv_screen.y, 1., 0.));
    Ray r = Ray(o, d);
    gl_FragColor = vec4(sqrt(ray_tracing(r)), 1.);
}