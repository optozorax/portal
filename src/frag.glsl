#version 140
precision lowp float;

in vec2 uv;
in vec2 uv_screen;
uniform vec3 angles;

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
    return vec3(cos(u/2.)*cos(u), sin(u/2.), cos(u/2.)*sin(u))/2.;
    // return vec3(0.1, 1., 0.);
}

vec3 mobiusStep(in float u, in Ray r) {
    vec3 o = mobiusO(u);
    vec3 d = mobiusD(u);
    
    Ray l = Ray(o, d);
    vec2 ts = twoLinesNearestPoints(l, r);

    vec3 lnearest = l.o + l.d * ts.x;
    vec3 rnearest = r.o + r.d * ts.y;
    
    float distance = length(lnearest - rnearest);

    // distance *= 1. + exp(-ts.y);
    // distance *= 1. + exp(abs(ts.x) - 1.);

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
        float beta = 1.;
        while (mobiusStep(clampangle(u + beta*du), r).x > step.x && beta > eps_newton) {
            beta *= 0.5;
        }
        if (beta < eps_newton) {
            break;
        }
        u = clampangle(u + beta*du);
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

vec3 normalizeNormal(in vec3 normal, in Ray r) {
    normal = normalize(normal);
    if (dot(normal, r.d) > 0.) {
        normal *= -1.;
    }
    return normal;
}

MobiusIntersect intersectMobius2(in Ray r) {
    SearchResult best = SearchResult(-1., 0., 0.);

    float count = 20.;
    for (float i = 0.; i <= count; i += 1.) {
        float u = float(i)/count * 2. * PI;
        best = updateBestApprox(best, findBestApprox(u, r, 10, 0.0001, best));
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
    const float fr = 3.14159*8.0;
    vec3 col = start;
    col += 0.4*smoothstep(-0.01,0.01,cos(uv.x*fr*0.5)*cos(uv.y*fr*0.5)); 
    float wi = smoothstep(-1.0,-0.98,cos(uv.x*fr))*smoothstep(-1.0,-0.98,cos(uv.y*fr));
    col *= wi;
    
    return col;
}

vec3 intersectScene(Ray r) {
    Plane p = Plane(crdDefault);
    float size = 3.;

    PlaneIntersect hitp;
    
    for (int i = 0; i <= 3; ++i) {
        float current_t = 1e10;
        vec3 current_color = vec3(0.4, 0.4, 0.4);

        p.repr.pos.z = size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(vec3(0.6, 0.2, 0.2), vec2(hitp.u, hitp.v)), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.z = -size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(vec3(0.6, 0.2, 0.2), vec2(hitp.u, hitp.v)), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.z = 0.;
        p.repr.i = crdDefault.k;
        p.repr.k = crdDefault.i;

        p.repr.pos.x = size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(vec3(0.2, 0.6, 0.2), vec2(hitp.u, hitp.v)), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.x = -size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(vec3(0.2, 0.6, 0.2), vec2(hitp.u, hitp.v)), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.x = 0.;
        p.repr.i = crdDefault.i;
        p.repr.j = crdDefault.k;
        p.repr.k = crdDefault.j;

        p.repr.pos.y = size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(vec3(0.6), vec2(hitp.u, hitp.v)), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos.y = -size;
        hitp = intersectPlane(r, p);
        if (hitp.hit && abs(hitp.u) < size && abs(hitp.v) < size && hitp.t < current_t) {
            current_color = addNormalToColor(gridColor(vec3(0.2, 0.2, 0.6), vec2(hitp.u, hitp.v)), hitp.n, r.d);
            current_t = hitp.t;
        }

        p.repr.pos = vec3(0., 0., 0.);
        p.repr.j = crdDefault.j;
        p.repr.k = crdDefault.k;

        MobiusIntersect hit = intersectMobius2(r);
        if (hit.hit && hit.t < current_t) {
            // float coef = length(mobius_d1(0., hit.u)) / length(mobius_d1(hit.v, hit.u));

            if (abs(hit.v) > 0.90) {
                return addNormalToColor(gridColor(vec3(0.6, 0.6, 0.6), vec2(hit.u, hit.v)), hit.n, r.d);
                // return vec3(0., 0., 1.);
            } else {
                vec3 i = normalize(mobiusD(hit.u));
                vec3 j = normalize(vec3(-sin(hit.u), 0., cos(hit.u)));
                vec3 k = cross(i, j);
                vec3 pos = mobiusO(hit.u);

                crd3 crd = crd3(i, j, k, pos);

                vec3 intersect_point = r.o + r.d * hit.t;

                vec3 before_teleport_o = projectCrd(crd, intersect_point);
                vec3 before_teleport_d = projectDir(crd, r.d);

                crd.i *= -1;

                vec3 after_teleport_o = unprojectCrd(crd, before_teleport_o);
                vec3 after_teleport_d = unprojectDir(crd, before_teleport_d);

                after_teleport_o += after_teleport_d * 0.01;
                r.o = after_teleport_o;
                r.d = after_teleport_d;
                
                /*
                vec3 newPos = mobiusO(hit.u + 2. * PI) + mobiusD(hit.u + 2. * PI) * (hit.v);
                newPos += r.d * 0.01;
                r.o = newPos;
                */
            }
        } else {
            return current_color;
        }
    }
    return vec3(0., 1., 1.);
}

void main() {
    float viewAngle = deg2rad(80);
    float h = tan(viewAngle / 2.);

    vec2 uv = uv_screen * h;

    /*
    if (abs(uv.y - 1.) < 0.01) {
        out_Color = vec4(0., 1., 0., 1.);
        return;
    }
    if (abs(uv.x - 1.) < 0.01) {
        out_Color = vec4(0., 0., 1., 1.);
        return;
    }
    if (abs(uv.y - 0.) < 0.01) {
        out_Color = vec4(0., 0.5, 0., 1.);
        return;
    }
    if (abs(uv.x - 0.) < 0.01) {
        out_Color = vec4(0., 0., 0.5, 1.);
        return;
    }
    if (abs(uv.y - (-1.)) < 0.01) {
        out_Color = vec4(0., 0.1, 0., 1.);
        return;
    }
    if (abs(uv.x - (-1.)) < 0.01) {
        out_Color = vec4(0., 0., 0.1, 1.);
        return;
    }
    */

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