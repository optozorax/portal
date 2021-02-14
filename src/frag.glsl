#version 140
precision lowp float;

in vec2 uv;
in vec2 uv_screen;
uniform vec3 angles;

uniform mat4 first;
uniform mat4 first_inv;
uniform mat4 second;
uniform mat4 second_inv;

uniform float add_gray_after_teleportation;

uniform sampler2D Texture;

out vec4 out_Color;

#define PI acos(-1.)

struct Ray
{
    vec3 o;     // origin
    vec3 d;     // direction
};

struct crd3 {
    vec3 i;
    vec3 j;
    vec3 k;
    vec3 pos;
};

const crd3 crdDefault = crd3(vec3(1., 0., 0.), vec3(0., 1., 0.), vec3(0., 0., 1.), vec3(0.));

float project(in vec3 a, in vec3 to) {
    return dot(a, to) / dot(to, to);
}

vec3 projection(in vec3 a, in vec3 to) {
    return to * project(a, to);
}

crd3 orthonormalize(crd3 a) {
    a.i = normalize(a.i);
    a.j = normalize(a.j);
    a.k = normalize(a.k);
    a.j = a.j - projection(a.j, a.i);
    a.k = a.k - projection(a.k, a.i) - projection(a.k, a.j);
    return a;
}

vec3 projectDir(in crd3 crd, in vec3 d) {
    // i*result.x + j*result.y + k*result.z = d
    return inverse(mat3(crd.i, crd.j, crd.k))*d;
}

vec3 projectCrd(in crd3 crd, in vec3 o) {
    // i*result.x + j*result.y + k*result.z + pos = o
    return projectDir(crd, o-crd.pos);
}

vec3 unprojectDir(in crd3 crd, in vec3 d) {
    return crd.i * d.x + crd.j * d.y + crd.k * d.z;
}

vec3 unprojectCrd(in crd3 crd, in vec3 d) {
    return crd.i * d.x + crd.j * d.y + crd.k * d.z + crd.pos;
}

vec2 twoLinesNearestPoints(in Ray a, in Ray b) {
    crd3 crd = crd3(a.d, b.d, cross(a.d, b.d), a.o);
    vec3 pos = projectCrd(crd, b.o);
    return vec2(pos.x, -pos.y);
}

vec3 mobiusO(in float u) {
    return vec3(cos(u), 0, sin(u));
}

vec3 mobiusD(in float u) {
    return vec3(cos(u/2.)*cos(u), sin(u/2.), cos(u/2.)*sin(u))/2.; // mobius
}

vec3 mobiusStep(in float u, in Ray r) {
    vec3 o = mobiusO(u);
    vec3 d = mobiusD(u);
    
    Ray l = Ray(o, d);
    vec2 ts = twoLinesNearestPoints(l, r);

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

vec3 mobius_d1(in float v, in float u) {
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

vec3 mobius_d2(in float v, in float u) {
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

struct MobiusIntersect {
    bool hit;
    float t;
    float u;
    float v;
    vec3 n;
};

float clampmod(in float a, in float max) {
    a = max + mod(a, max);
    if (a < 0.) {
        a += max;
    }
    if (a > max) {
        a -= max;
    }
    return a;
}

float clampangle(in float a) {
    return clampmod(a, 2. * PI);
}

struct SearchResult {
    float t;
    float u;
    float v;
};

SearchResult findBestApprox(in float u, in Ray r, in int max, in float eps_newton, in SearchResult best) {
    float eps_der = 0.0001;

    int k = 0;
    vec3 step = mobiusStep(u, r);
    while (step.x > eps_newton && k < max) {
        float du = -step.x/(mobiusStep(u + eps_der, r).x - step.x)*eps_der;
        u = clampangle(u + du);
        k += 1;
        step = mobiusStep(u, r);
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

SearchResult updateBestApprox(in SearchResult best, in SearchResult current) {
    if (current.t > 0. && (current.v > -1 && current.v < 1)) {
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

vec3 normalizeNormal(in vec3 normal, in Ray r) {
    normal = normalize(normal);
    if (dot(normal, r.d) > 0.) {
        normal *= -1.;
    }
    return normal;
}

MobiusIntersect intersectMobius2(in Ray r) {
    SearchResult best = SearchResult(-1., 0., 0.);

    float count = 10.;
    const float attempts[32] = float[32](
            0., 
            1./2., 
            1./4., 3./4., 
            1./8., 3./8., 5./8., 7./8., 
            1./16., 3./16., 5./16., 7./16., 9./16., 11./16., 13./16., 15./16., 
            1./32., 3./32., 5./32., 7./32., 9./32., 11./32., 13./32., 15./32., 17./32., 19./32., 21./32., 23./32., 25./32., 27./32., 29./32., 31./32.
    );
    for (int i = 0; i < count; i++) {
        float u = attempts[i] * 2. * PI;    
        
        best = updateBestApprox(best, findBestApprox(u, r, 10, 0.0001, best));
        if (best.t < 0. && i > 8) {
            break;
        }
    }

    if (best.t >= 0.) {
        vec3 normal = normalizeNormal(cross(mobius_d1(best.v, best.u), mobius_d2(best.v, best.u)), r);
        return MobiusIntersect(true, best.t, best.u, best.v, normal);
    } else {
        return MobiusIntersect(false, 0., 0., 0., vec3(0.));
    }
}

struct Plane {
    crd3 repr;
};

struct PlaneIntersect {
    bool hit;
    float t;
    float u;
    float v;
    vec3 n;
};

vec3 color(float r, float g, float b) {
    return vec3(r*r, g*g, b*b);
}

PlaneIntersect intersectPlane(in Ray r, in Plane p) {
    vec3 ro = projectCrd(p.repr, r.o);
    vec3 rd = projectDir(p.repr, r.d);

    float t = -ro.z/rd.z;
    if (t < 0.) {
        return PlaneIntersect(false, 0., 0., 0., vec3(0.));
    } else {
        vec3 pos = ro + rd * t; 
        return PlaneIntersect(true, t, pos.x, pos.y, normalizeNormal(p.repr.k, r));
    }
}

float deg2rad(in float deg) {
    return deg/180. * PI;
}

vec3 addNormalToColor(in vec3 color, in vec3 normal, in vec3 direction) {
    const float not_dark_count = 0.4;
    color *= (abs(dot(normalize(direction), normalize(normal))) + not_dark_count) / (1. + not_dark_count);
    return color;
}

vec3 gridColor(in vec3 start, in vec2 uv) {
    uv = uv - vec2(0.125, 0.125);
    const float fr = 3.14159*8.0;
    vec3 col = start;
    col += 0.4*smoothstep(-0.01,0.01,cos(uv.x*fr*0.5)*cos(uv.y*fr*0.5)); 
    float wi = smoothstep(-1.0,-0.98,cos(uv.x*fr))*smoothstep(-1.0,-0.98,cos(uv.y*fr));
    col *= wi;
    
    return col;
}

vec3 mulDir(mat4 matrix, vec3 vec) {
    return (matrix * vec4(vec, 0.)).xyz;
}

vec3 mulCrd(mat4 matrix, vec3 vec) {
    return (matrix * vec4(vec, 1.)).xyz;
}

vec3 intersectScene(in Ray r) {
    Plane p = Plane(crdDefault);
    float size = 4.;
    float scale = 1./8.;

    PlaneIntersect hitp;

    float gray = 1.;
    
    for (int i = 0; i < 100; ++i) {
        float current_t = 1e10;
        vec3 current_color = color(0.6, 0.6, 0.6);

        p.repr.pos.z = size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(color(0.6, 0.2, 0.2), vec2(hitp.u, hitp.v) * scale), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.z = -size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(color(0.6, 0.2, 0.6), vec2(hitp.u, hitp.v) * scale), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.z = 0.;
        p.repr.i = crdDefault.k;
        p.repr.k = crdDefault.i;

        p.repr.pos.x = size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = color(0.2, 0.6, 0.2);
            vec3 new_color = gridColor(current_color, vec2(hitp.u, hitp.v) * scale);
            current_color = (current_color*2 + new_color)/3.;
            current_color = addNormalToColor(current_color, hitp.n, r.d);
            current_color *= texture2D(Texture, (vec2(-hitp.u, -hitp.v) + vec2(size, size))/(size * 2.)).rgb;
            current_t = hitp.t;
        }

        p.repr.pos.x = -size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(color(0.6, 0.6, 0.2), vec2(hitp.u, hitp.v) * scale), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.x = 0.;
        p.repr.i = crdDefault.i;
        p.repr.j = crdDefault.k;
        p.repr.k = crdDefault.j;

        p.repr.pos.y = size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(color(0.6, 0.6, 0.6), vec2(hitp.u, hitp.v) * scale), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.y = -size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(color(0.2, 0.2, 0.6), vec2(hitp.u, hitp.v) * scale), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos = vec3(0., 0., 0.);
        p.repr.j = crdDefault.j;
        p.repr.k = crdDefault.k;

        MobiusIntersect hit = intersectMobius2(Ray(mulCrd(first, r.o), mulDir(first, r.d)));
        MobiusIntersect hit2 = intersectMobius2(Ray(mulCrd(second, r.o), mulDir(second, r.d)));

        int portal_to_process = 0;

        if (hit.hit && hit2.hit && min(hit.t, hit2.t) < current_t) {
            if (hit.t < hit2.t) {
                portal_to_process = 1;
            } else {
                portal_to_process = 2;
            }
        } else if (hit.hit && hit.t < current_t) {
            portal_to_process = 1;
        } else if (hit2.hit && hit2.t < current_t) {
            portal_to_process = 2;
        } else {
            portal_to_process = 0;
        }

        if (portal_to_process == 1) {
            current_t = hit.t;
            if (abs(hit.v) > 0.90) {
                current_color = addNormalToColor(color(0.1, 0.15, 1.), hit.n, r.d);
            } else {
                r.o += r.d * hit.t;
                r.o += r.d * 0.01;

                r.o = mulCrd(second_inv, r.o);
                r.d = mulDir(second_inv, r.d);

                gray *= add_gray_after_teleportation;
                continue;
            }
        }

        if (portal_to_process == 2) {
            current_t = hit2.t;
            if (abs(hit2.v) > 0.90) {
                current_color = addNormalToColor(color(1., 0.55, 0.15), hit2.n, r.d);
            } else {
                r.o += r.d * hit2.t;
                r.o += r.d * 0.01;

                r.o = mulCrd(first_inv, r.o);
                r.d = mulDir(first_inv, r.d);

                gray *= add_gray_after_teleportation;
                continue;
            }
        }

        return current_color * gray;
    }
    return color(0., 1., 1.);
}

void main() {
    float viewAngle = deg2rad(80);
    float h = tan(viewAngle / 2.);

    vec2 uv = uv_screen * h;

    float alpha = deg2rad(angles.x);
    float beta = deg2rad(angles.y);
    float radius = angles.z;

    vec3 lookAt = vec3(0., 0., 0.);
    vec3 pos = vec3(sin(PI/2. - beta) * cos(alpha), cos(PI/2. - beta), sin(PI/2. - beta) * sin(alpha)) * radius + lookAt;

    vec3 k = normalize(lookAt - pos);
    vec3 i = normalize(cross(vec3(0., 1., 0.), k));
    vec3 j = normalize(cross(k, i));
    
    Ray r = Ray(pos, normalize(i * uv.x + j * uv.y + k));
    out_Color = vec4(sqrt(intersectScene(r)), 1.);
}