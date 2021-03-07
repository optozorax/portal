#version 100
precision highp float;

// ---------------------------------------------------------------------------
// Utils ---------------------------------------------------------------------
// ---------------------------------------------------------------------------

#define PI acos(-1.)
#define PI2 (acos(-1.) / 2.0)

struct Ray
{
    vec3 o; // origin
    vec3 d; // direction
};

const Ray no_ray = Ray(vec3(0.), vec3(0.));

mat3 transpose(mat3 matrix) {
    vec3 row0 = matrix[0];
    vec3 row1 = matrix[1];
    vec3 row2 = matrix[2];
    mat3 result = mat3(
        vec3(row0.x, row1.x, row2.x),
        vec3(row0.y, row1.y, row2.y),
        vec3(row0.z, row1.z, row2.z)
    );
    return result;
}

float det(mat2 matrix) {
    return matrix[0].x * matrix[1].y - matrix[0].y * matrix[1].x;
}

mat3 inverse(mat3 matrix) {
    vec3 row0 = matrix[0];
    vec3 row1 = matrix[1];
    vec3 row2 = matrix[2];

    vec3 minors0 = vec3(
        det(mat2(row1.y, row1.z, row2.y, row2.z)),
        det(mat2(row1.z, row1.x, row2.z, row2.x)),
        det(mat2(row1.x, row1.y, row2.x, row2.y))
    );
    vec3 minors1 = vec3(
        det(mat2(row2.y, row2.z, row0.y, row0.z)),
        det(mat2(row2.z, row2.x, row0.z, row0.x)),
        det(mat2(row2.x, row2.y, row0.x, row0.y))
    );
    vec3 minors2 = vec3(
        det(mat2(row0.y, row0.z, row1.y, row1.z)),
        det(mat2(row0.z, row0.x, row1.z, row1.x)),
        det(mat2(row0.x, row0.y, row1.x, row1.y))
    );

    mat3 adj = transpose(mat3(minors0, minors1, minors2));

    return (1.0 / dot(row0, minors0)) * adj;
}

vec3 mul_dir(mat4 matrix, vec3 vec) {
    return (matrix * vec4(vec, 0.)).xyz;
}

vec3 mul_pos(mat4 matrix, vec3 vec) {
    return (matrix * vec4(vec, 1.)).xyz;
}

float project(vec3 a, vec3 to) {
    return dot(a, to) / dot(to, to);
}

vec3 projection(vec3 a, vec3 to) {
    return to * project(a, to);
}

vec2 two_lines_nearest_points(Ray a, Ray b) {
    vec3 n = cross(a.d, b.d);
    vec3 n1 = cross(a.d, n);
    vec3 n2 = cross(b.d, n);
    return vec2(
        dot(b.o-a.o, n2)/dot(a.d, n2),
        dot(a.o-b.o, n1)/dot(b.d, n1)
    );
}

float clamp_mod(float a, float max) {
    a = max + mod(a, max);
    if (a < 0.) {
        a += max;
    }
    if (a > max) {
        a -= max;
    }
    return a;
}

float clamp_angle(float a) {
    return clamp_mod(a, 2. * PI);
}

vec3 normalize_normal(vec3 normal, Ray r) {
    normal = normalize(normal);
    if (dot(normal, r.d) > 0.) {
        normal *= -1.;
    }
    return normal;
}

vec3 color(float r, float g, float b) {
    return vec3(r*r, g*g, b*b);
}

float deg2rad(float deg) {
    return deg/180. * PI;
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
    vec3 ro = (plane * vec4(r.o, 1.)).xyz;
    vec3 rd = (plane * vec4(r.d, 0.)).xyz;

    float t = -ro.z/rd.z;
    if (t < 0.) {
        return no_intersect;
    } else {
        vec3 pos = ro + rd * t; 
        return SurfaceIntersect(true, t, pos.x, pos.y, normalize_normal(normal, r));
    }
}

// ---------------------------------------------------------------------------
// Mobius strip calculation --------------------------------------------------
// ---------------------------------------------------------------------------

vec3 mobius_o(float u) {
    return vec3(cos(u), 0, sin(u));
}

vec3 mobius_d(float u) {
    return vec3(cos(u/2.)*cos(u), sin(u/2.), cos(u/2.)*sin(u))/2.; // mobius
}

vec3 mobius_step(float u, Ray r) {
    Ray l = Ray(mobius_o(u), mobius_d(u));
    vec2 ts = two_lines_nearest_points(l, r);

    vec3 lnearest = l.o + l.d * ts.x;
    vec3 rnearest = r.o + r.d * ts.y;
    
    float distance = length(lnearest - rnearest);

    if (abs(ts.x) > 1.) {
        distance *= 2. * abs(ts.x);
    }

    if (ts.y < 0.) {
        distance *= 4. * abs(ts.y);
    }

    return vec3(distance, ts.x, ts.y); // distance, v, t
}

vec3 mobius_d1(float v, float u) {
    float a = sin(u/2.);
    float b = cos(u/2.);
    float c = sin(u);
    float d = cos(u);
    return vec3(
        b*d/2., 
        b*c/2., 
        a/2.
    );
}

vec3 mobius_d2(float v, float u) {
    float a = sin(u/2.);
    float b = cos(u/2.);
    float c = sin(u);
    float d = cos(u);
    return vec3(
        -(0.25*v*a*d+0.5*v*c*b+c), 
        -(0.25*(v*a*c-2.*d*(v*b+2.))), 
        0.25*v*b
    );
}

struct SearchResult {
    float t;
    float u;
    float v;
};

SearchResult mobius_best_approx(float u, Ray r, float eps_newton, SearchResult best) {
    float eps_der = 0.0001;

    vec3 step = mobius_step(u, r);
    for (int k = 0; k < 10; k++) {
        if (step.x < eps_newton) {
            break;
        }
        float du = -step.x/(mobius_step(u + eps_der, r).x - step.x)*eps_der;
        u = clamp_angle(u + du);
        step = mobius_step(u, r);
        if (best.t > 0. && abs(u-best.u) < 0.01) {
            return SearchResult(-1., 0., 0.);
        }
    }

    if (step.x < eps_newton) {
        return SearchResult(step.z, u, step.y);    
    } else {
        return SearchResult(-1., 0., 0.);
    }
}

SearchResult update_best_approx(SearchResult best, SearchResult current) {
    if (current.t > 0. && (current.v > -1. && current.v < 1.)) {
        if (best.t < 0.) {
            best = current;
        } else {
            if (current.t < best.t) {
                best = current;
            }
        }
    }
    return best;
}

SearchResult mobius_find_best(Ray r) {
    SearchResult best = SearchResult(-1., 0., 0.);
    best = update_best_approx(best, mobius_best_approx(0., r, 0.0001, best));
    best = update_best_approx(best, mobius_best_approx(PI, r, 0.0001, best));
    for (int i = 0; i < 2; i++) {
        float u = float(i*2 + 1)/4. * 2. * PI;
        best = update_best_approx(best, mobius_best_approx(u, r, 0.0001, best));
    }
    for (int i = 0; i < 4; i++) {
        float u = float(i*2 + 1)/8. * 2. * PI;
        best = update_best_approx(best, mobius_best_approx(u, r, 0.0001, best));
    }
    if (best.t < 0.) {
        return best;
    }
    best = update_best_approx(best, mobius_best_approx(float(8 - 1)/16. * 2. * PI, r, 0.0001, best));
    best = update_best_approx(best, mobius_best_approx(float(8 + 1)/16. * 2. * PI, r, 0.0001, best));
    return best;
}

bool intersect_mobius_sphere(Ray r) {
    vec3 op = -r.o;
    float b = dot(op, r.d);
    float det = b * b - dot(op, op) + 2.4055; // 1.55²
    return det >= 0.;
}

SurfaceIntersect mobius_intersect(Ray r) {
    if (intersect_mobius_sphere(r)) {
        SearchResult best = mobius_find_best(r);
        if (best.t >= 0.) {
            vec3 normal = normalize_normal(cross(mobius_d1(best.v, best.u), mobius_d2(best.v, best.u)), r);
            return SurfaceIntersect(true, best.t, best.u, best.v, normal);
        }
    }

    return no_intersect;
}

// ---------------------------------------------------------------------------
// Material utils ------------------------------------------------------------
// ---------------------------------------------------------------------------

vec3 add_normal_to_color(vec3 color, vec3 normal, vec3 direction) {
    const float not_dark_count = 0.4;
    color *= (abs(dot(normalize(direction), normalize(normal))) + not_dark_count) / (1. + not_dark_count);
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

uniform mat4 portal_mat_first;
uniform mat4 portal_mat_first_inverse;
uniform mat4 portal_mat_first_teleport;
uniform mat4 portal_mat_second;
uniform mat4 portal_mat_second_inverse;
uniform mat4 portal_mat_second_teleport;

uniform mat4 plane1;
uniform mat4 plane1_inv;
uniform mat4 plane2;
uniform mat4 plane2_inv;
uniform mat4 plane3;
uniform mat4 plane3_inv;
uniform mat4 plane4;
uniform mat4 plane4_inv;
uniform mat4 plane5;
uniform mat4 plane5_inv;
uniform mat4 plane6;
uniform mat4 plane6_inv;
uniform mat4 triangle;
uniform mat4 triangle_inv;

uniform int teleport_light;
uniform int second_portal_disabled;
uniform int teleportation_enabled;

uniform float side_border_progress;
uniform float portal_color_blend;
uniform float triangle_size;

const float plane_size = 4.5;
const float plane_scale = 1./(plane_size * 2.);

bool nearer(SurfaceIntersect hit, SceneIntersection i) {
    return hit.hit && hit.t < i.hit.t;
}

bool on_plane(SurfaceIntersect hit) {
    return abs(hit.u) < plane_size && abs(hit.v) < plane_size;
}

SceneIntersection plane_process_intersection(SceneIntersection i, SurfaceIntersect hit, int material) {
    if (nearer(hit, i) && on_plane(hit)) {
        i.hit = hit;
        i.material = material;
    }
    return i;
}

SceneIntersection mobius_process_intersection(SceneIntersection i, SurfaceIntersect hit, int material) {
    if (nearer(hit, i)) {
        i.hit = hit;
        if (abs(hit.v) > 0.80) {
            i.material = material;
        } else {
            if (teleport_light == 0) {
                i.material = material + 1;
            } else {
                i.material = material + 2;
            }
        }        
    }
    return i;
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

// `r` is original ellipse radius, `b` is border radius. For better results `r` must be 1, and `b` must lie in [0; 0.1].
float ellipse_parallel_curve_approximation(float r, float b, float x) {
    return sqr(r + b + sqrt(abs(abs(x) - 1. - b)) * b);
}

SceneIntersection half_ellipse_process_intersection(SceneIntersection i, Ray r, int material, mat4 plane_matrix, vec3 normal) {
    float border = 0.05;
    float black_border = 0.02;
    float portal_size = 1.;
    float ellipse_portal = sqr(portal_size);
    float ellipse_border = sqr(portal_size + border);
    float ellipse_black_border = sqr(portal_size + border + black_border);

    SurfaceIntersect hit = plane_intersect(r, plane_matrix, normal);
    if (hit.hit && nearer(hit, i)) {
        float ellipse_pos = 4. * hit.u * hit.u + hit.v * hit.v;
        if (ellipse_pos < ellipse_black_border && hit.u > 0.) {
            bool is_on_portal = 
                between(0., ellipse_pos, ellipse_portal) && 
                (border + black_border) * side_border_progress < hit.u;
            bool is_on_portal_side = is_collinear(hit.n, normal);

            bool is_on_border = 
                between(black_border * side_border_progress, hit.u, (border + black_border) * side_border_progress) || 
                between(ellipse_portal, ellipse_pos, ellipse_border);

            bool is_on_black_side = 
                between(0., hit.u, black_border * side_border_progress) || 
                between(ellipse_border, ellipse_pos, ellipse_black_border);

            if (is_on_portal && is_on_portal_side) {
                if (teleport_light == 1 && teleportation_enabled == 1) {
                    i.hit = hit;
                    i.material = material;
                } else if (teleport_light == 0) {
                    i.hit = hit;
                    i.material = material + 2;
                }
            } else if (is_on_black_side) {
                i.material = 6;
                i.hit = hit;
            } else if (is_on_border || (is_on_portal && !is_on_portal_side && teleportation_enabled == 1)) {
                i.material = material + 1;
                i.hit = hit;
            }
        }
    }
    return i;
}

SceneIntersection scene_intersect_1(Ray r) {
    SceneIntersection i = SceneIntersection(0, no_intersect);
    SurfaceIntersect hit = no_intersect;

    // Cube room -------------------------------------------------------------
    i = plane_process_intersection(i, plane_intersect(r, plane1_inv, plane1[2].xyz), 0);
    i = plane_process_intersection(i, plane_intersect(r, plane2_inv, plane2[2].xyz), 1);
    i = plane_process_intersection(i, plane_intersect(r, plane3_inv, plane3[2].xyz), 2);
    i = plane_process_intersection(i, plane_intersect(r, plane4_inv, plane4[2].xyz), 3);
    i = plane_process_intersection(i, plane_intersect(r, plane5_inv, plane5[2].xyz), 4);
    i = plane_process_intersection(i, plane_intersect(r, plane6_inv, plane6[2].xyz), 5);

    // Triangle
    float triangle_border_size = 0.05;

    hit = plane_intersect(r, triangle_inv, triangle[2].xyz);
    if (nearer(hit, i) && hit.u > 0. && hit.u < -abs(3. * hit.v) + triangle_size) {
        vec3 pos = r.o + r.d * hit.t;
        if (pos.x < 0.) {
            i.hit = hit;
            if (hit.u < triangle_border_size/3. || hit.u > -abs(3. * hit.v) + triangle_size - triangle_border_size) {
                i.material = 6;
            } else {
                i.material = 7;
            }
        }
    }


    hit = plane_intersect(r, triangle_inv * portal_mat_first_teleport, triangle[2].xyz);
    if (nearer(hit, i) && hit.u > 0. && hit.u < -abs(3. * hit.v) + triangle_size) {
        vec3 pos = r.o + r.d * hit.t;
        if (pos.x < 0.) {
            i.hit = hit;
            if (hit.u < triangle_border_size/3. || hit.u > -abs(3. * hit.v) + triangle_size - triangle_border_size) {
                i.material = 6;
            } else {
                i.material = 7;
            }
        }
    }

    // Portals --------------------------------------------------------
    i = half_ellipse_process_intersection(i, r, 100, portal_mat_first_inverse, portal_mat_first[2].xyz);

    if (second_portal_disabled == 0) {
        i = half_ellipse_process_intersection(i, r, 103, portal_mat_second_inverse, -portal_mat_second[2].xyz);
    }

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

uniform float add_gray_after_teleportation;
uniform sampler2D watermark;

MaterialProcessing plane_process_material(SurfaceIntersect hit, Ray r, vec3 clr) {
    vec3 new_clr = grid_color(clr, vec2(hit.u, hit.v) * plane_scale);
    clr = (clr*2. + new_clr)/3.;
    return MaterialProcessing(true, add_normal_to_color(clr, hit.n, r.d), no_ray);
}

MaterialProcessing teleport(float t, mat4 matrix, Ray r) {
    r.o += r.d * t;
    r.o += r.d * 0.0001;

    r.o = mul_pos(matrix, r.o);
    r.d = mul_dir(matrix, r.d);

    return MaterialProcessing(false, vec3(add_gray_after_teleportation), r);
}

MaterialProcessing material_process_1(Ray r, SceneIntersection i) {
    if (i.material == -1) {


    // Box around
    } else if (i.material == 0) {
        return plane_process_material(i.hit, r, color(0.6, 0.2, 0.2));
    } else if (i.material == 1) {
        return plane_process_material(i.hit, r, color(0.6, 0.2, 0.6));
    } else if (i.material == 2) {
        return plane_process_material(i.hit, r, color(0.6, 0.6, 0.6));
    } else if (i.material == 3) {
        return plane_process_material(i.hit, r, color(0.2, 0.2, 0.6));
    } else if (i.material == 4) {
        return plane_process_material(i.hit, r, color(0.6, 0.6, 0.2));
    } else if (i.material == 5) {
        vec3 current_color = color(0.2, 0.6, 0.2);
        vec3 new_color = grid_color(current_color, vec2(i.hit.u, i.hit.v) * plane_scale);
        current_color = (current_color*2. + new_color)/3.;
        current_color = add_normal_to_color(current_color, i.hit.n, r.d);
        current_color *= texture2D(watermark, (vec2(i.hit.u, i.hit.v) + vec2(plane_size, plane_size))/(plane_size * 2.)).rgb;
        return MaterialProcessing(true, current_color, no_ray);


    // Triangle and borders
    } else if (i.material == 6) {
        return MaterialProcessing(true, add_normal_to_color(color(0., 0., 0.), i.hit.n, r.d), no_ray);
    } else if (i.material == 7) {
        return MaterialProcessing(true, add_normal_to_color(color(0.6, 0.6, 0.6), i.hit.n, r.d), no_ray);


    // Portals
    } else if (i.material == 100) {
        return teleport(i.hit.t, portal_mat_first_teleport, r);
    } else if (i.material == 101) {
        return MaterialProcessing(true, portal_color_blend * add_normal_to_color(color(0.1, 0.15, 1.), i.hit.n, r.d), no_ray);
    } else if (i.material == 102) {
        return MaterialProcessing(true, grid_color(color(0.1, 0.15, 1.), vec2(i.hit.u, i.hit.v)), no_ray);

    } else if (i.material == 103) {
        return teleport(i.hit.t, portal_mat_second_teleport, r);
    } else if (i.material == 104) {
        return MaterialProcessing(true, portal_color_blend * add_normal_to_color(color(1., 0.55, 0.15), i.hit.n, r.d), no_ray);
    } else if (i.material == 105) {
        return MaterialProcessing(true, grid_color(color(1., 0.55, 0.15), vec2(i.hit.u, i.hit.v)), no_ray);


    // End    
    }
    return MaterialProcessing(false, vec3(0.), Ray(vec3(0.), vec3(0.)));
}


// ---------------------------------------------------------------------------
// Ray tracing ---------------------------------------------------------------
// ---------------------------------------------------------------------------

vec3 ray_tracing(Ray r) {
    vec3 current_color = vec3(1.);
    for (int j = 0; j < 100; j++) {
        SceneIntersection i = scene_intersect_1(r);
        if (i.hit.hit) {
            MaterialProcessing m = material_process_1(r, i);
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

uniform mat4 camera;
varying vec2 uv;
varying vec2 uv_screen;

void main() {
    vec3 o = mul_pos(camera, vec3(0.));
    vec3 d = normalize(mul_dir(camera, vec3(uv_screen.x, uv_screen.y, 1.)));
    Ray r = Ray(o, d);
    gl_FragColor = vec4(sqrt(ray_tracing(r)), 1.);
}