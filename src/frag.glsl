//%predefined_library//%

// ---------------------------------------------------------------------------
// User library --------------------------------------------------------------
// ---------------------------------------------------------------------------

//%uniforms//%

//%materials_defines//%

//%library//%

//%intersection_functions//%

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

// ---------------------------------------------------------------------------
// Ray tracing ---------------------------------------------------------------
// ---------------------------------------------------------------------------

uniform int _ray_tracing_depth;

vec3 ray_tracing(Ray r) {
    vec3 current_color = vec3(1.);
    for (int j = 0; j < 10000; j++) {
        if (j > _ray_tracing_depth) {
            return current_color;
        }
        SceneIntersection i = scene_intersect(r);

        // Offset ray
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
            return current_color * color(0.6, 0.6, 0.6);
        }
    }
    return current_color;
}

// ---------------------------------------------------------------------------
// Draw image ----------------------------------------------------------------
// ---------------------------------------------------------------------------

uniform mat4 _camera;
uniform float _view_angle;
uniform int _use_panini_projection;
uniform float _panini_param;
varying vec2 uv;
varying vec2 uv_screen;

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
        float f = cos(fo)/sin(fo) /* * 2.0 */;
        float f2 = f*f;

        float b = (sqrt(max(0.0, Pow2(d+d2)*(f2+f2*f2))) - (d*f+f)) / (d2+d2*f2-1.0);

        tc *= b;
    }
    
    /* http://tksharpless.net/vedutismo/Pannini/panini.pdf */
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

void main() {
    vec4 o = _camera * vec4(0., 0., 0., 1.);
    vec4 d;
    if (_use_panini_projection == 1) {
        d = normalize(_camera * vec4(PaniniProjection(vec2(uv_screen.x, uv_screen.y), _view_angle, _panini_param), 0.));
    } else {
        float h = tan(_view_angle / 2.);
        d = normalize(_camera * vec4(uv_screen.x * h, uv_screen.y * h, 1.0, 0.));
    }
     
    Ray r = Ray(o, d);
    gl_FragColor = vec4(sqrt(ray_tracing(r)), 1.);
}
