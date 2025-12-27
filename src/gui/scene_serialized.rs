use crate::gui::scene::Scene;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::camera::{Cam as OldCam, CamLookAt, CameraId};
use super::eng_rus::EngRusText;
use super::glsl::LibraryCode;
use super::intersection_material::IntersectionMaterial as OldIntersectionMaterial;
use super::material::Material as OldMaterial;
use super::matrix::{Matrix as OldMatrix, MatrixId};
use super::object::{Object as OldObject, ObjectType as OldObjectType};
use super::scene::CamSettings;
use super::storage2::Storage2;
use super::storage2::Wrapper;
use super::texture::TextureName;
use super::uniform::{
    AnyUniform as OldAnyUniform, ParametrizeOrNot as OldParamOrNot, TVec3 as OldTVec3,
    TVec4 as OldTVec4, UniformId,
};

// Serialization-only, tree-like types

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Named<T> {
    name: String,
    data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SerStorage<T>(Vec<Named<T>>);

// Uniforms
type AnyUniform = OldAnyUniform;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum UniformRef {
    Named(String),
    Inline(Box<AnyUniform>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum ParametrizeOrNot {
    Value(f64),
    Uniform(Option<UniformRef>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TVec3 {
    x: ParametrizeOrNot,
    y: ParametrizeOrNot,
    z: ParametrizeOrNot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TVec4 {
    x: ParametrizeOrNot,
    y: ParametrizeOrNot,
    z: ParametrizeOrNot,
    w: ParametrizeOrNot,
}

// Matrices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum MatrixRef {
    Named(String),
    Inline(Box<Matrix>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::large_enum_variant)]
enum Matrix {
    Mul {
        to: Option<MatrixRef>,
        what: Option<MatrixRef>,
    },
    Teleport {
        first_portal: Option<MatrixRef>,
        second_portal: Option<MatrixRef>,
        what: Option<MatrixRef>,
    },
    Simple {
        offset: glam::DVec3,
        scale: f64,
        rotate: glam::DVec3,
        mirror: (bool, bool, bool),
    },
    Parametrized {
        offset: TVec3,
        rotate: TVec3,
        mirror: TVec3,
        scale: ParametrizeOrNot,
    },
    Exact {
        i: TVec3,
        j: TVec3,
        k: TVec3,
        pos: TVec3,
    },
    ExactFull {
        c0: TVec4,
        c1: TVec4,
        c2: TVec4,
        c3: TVec4,
    },
    If {
        condition: ParametrizeOrNot,
        then: Option<MatrixRef>,
        otherwise: Option<MatrixRef>,
    },
    Sqrt(Option<MatrixRef>),
    Lerp {
        t: ParametrizeOrNot,
        first: Option<MatrixRef>,
        second: Option<MatrixRef>,
    },
    Camera,
    Inv(Option<MatrixRef>),
}

// Cameras
#[derive(Debug, Clone, Serialize, Deserialize)]
enum CamLookAtSer {
    MatrixCenter(Option<MatrixRef>),
    Coordinate(glam::DVec3),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cam {
    look_at: CamLookAtSer,
    alpha: f64,
    beta: f64,
    r: f64,
    #[serde(default)]
    in_subspace: bool,
    #[serde(default)]
    free_movement: bool,
    #[serde(default)]
    matrix: glam::DMat4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CamRef {
    Named(String),
    Inline(Box<Cam>),
}

// Objects
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ObjectType {
    Simple(Option<MatrixRef>),
    Portal(Option<MatrixRef>, Option<MatrixRef>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Object {
    DebugMatrix(Option<MatrixRef>),
    Flat {
        kind: ObjectType,
        is_inside: super::glsl::IsInsideCode,
        #[serde(default)]
        in_subspace: super::object::SubspaceType,
    },
    Complex {
        kind: ObjectType,
        intersect: super::glsl::IntersectCode,
        #[serde(default)]
        in_subspace: super::object::SubspaceType,
    },
}

// Materials, intersections, library are reused
type Material = OldMaterial;
type IntersectionMaterial = OldIntersectionMaterial;

// ---------- Current stage helpers ----------
fn current_stage_to_ser(scene: &Scene) -> CurrentStageSer {
    match scene.current_stage {
        super::scene::CurrentStage::Dev => CurrentStageSer::Dev,
        super::scene::CurrentStage::Animation(id) => {
            let n = scene
                .animation_stages
                .get_name(id)
                .and_then(|x| x)
                .unwrap_or("?");
            CurrentStageSer::Animation(n.to_owned())
        }
        super::scene::CurrentStage::RealAnimation(id) => {
            let n = scene.animations.get_name(id).and_then(|x| x).unwrap_or("?");
            CurrentStageSer::RealAnimation(n.to_owned())
        }
    }
}

fn set_current_stage_from_ser(scene: &mut Scene, s: CurrentStageSer) {
    use super::scene::CurrentStage;
    match s {
        CurrentStageSer::Dev => scene.current_stage = CurrentStage::Dev,
        CurrentStageSer::Animation(name) => {
            if let Some(id) = scene.animation_stages.find_id(&name) {
                scene.current_stage = CurrentStage::Animation(id);
            } else {
                scene.current_stage = CurrentStage::Dev;
            }
        }
        CurrentStageSer::RealAnimation(name) => {
            if let Some(id) = scene.animations.find_id(&name) {
                scene.current_stage = CurrentStage::RealAnimation(id);
            } else {
                scene.current_stage = CurrentStage::Dev;
            }
        }
    }
}

// ---------- Top-level helpers for (de)serializing matrices/uniforms ----------
fn param_to_ser(p: &OldParamOrNot, uniforms: &Storage2<OldAnyUniform>) -> ParametrizeOrNot {
    match p {
        OldParamOrNot::No(f) => ParametrizeOrNot::Value(*f),
        OldParamOrNot::Yes(opt) => match opt {
            None => ParametrizeOrNot::Uniform(None),
            Some(uid) => match uniforms.get_name(*uid) {
                Some(Some(name)) => {
                    ParametrizeOrNot::Uniform(Some(UniformRef::Named(name.to_owned())))
                }
                Some(None) => {
                    let u = uniforms.get_original(*uid).unwrap().clone();
                    ParametrizeOrNot::Uniform(Some(UniformRef::Inline(Box::new(u))))
                }
                None => ParametrizeOrNot::Uniform(None),
            },
        },
    }
}

fn tvec3_to_ser(v: &OldTVec3, uniforms: &Storage2<OldAnyUniform>) -> TVec3 {
    TVec3 {
        x: param_to_ser(&v.x, uniforms),
        y: param_to_ser(&v.y, uniforms),
        z: param_to_ser(&v.z, uniforms),
    }
}

fn tvec4_to_ser(v: &OldTVec4, uniforms: &Storage2<OldAnyUniform>) -> TVec4 {
    TVec4 {
        x: param_to_ser(&v.x, uniforms),
        y: param_to_ser(&v.y, uniforms),
        z: param_to_ser(&v.z, uniforms),
        w: param_to_ser(&v.w, uniforms),
    }
}

fn mat_id_to_ref(
    id: Option<MatrixId>,
    mats: &Storage2<OldMatrix>,
    visited_inline: &mut std::collections::BTreeSet<usize>,
    uniforms: &Storage2<OldAnyUniform>,
) -> Option<MatrixRef> {
    let id = id?;
    match mats.get_name(id) {
        Some(Some(name)) => Some(MatrixRef::Named(name.to_owned())),
        Some(None) => {
            let uid = id.un_wrap().to_string().parse::<usize>().unwrap_or(0);
            if visited_inline.contains(&uid) {
                return None;
            }
            visited_inline.insert(uid);
            let val = mats.get_original(id)?.clone();
            let inner = matrix_to_ser(&val, mats, uniforms, visited_inline);
            visited_inline.remove(&uid);
            Some(MatrixRef::Inline(Box::new(inner)))
        }
        None => None,
    }
}

fn matrix_to_ser(
    m: &OldMatrix,
    mats: &Storage2<OldMatrix>,
    uniforms: &Storage2<OldAnyUniform>,
    visited_inline: &mut std::collections::BTreeSet<usize>,
) -> Matrix {
    use OldMatrix as OM;
    match m {
        OM::Mul { to, what } => Matrix::Mul {
            to: mat_id_to_ref(*to, mats, visited_inline, uniforms),
            what: mat_id_to_ref(*what, mats, visited_inline, uniforms),
        },
        OM::Teleport {
            first_portal,
            second_portal,
            what,
        } => Matrix::Teleport {
            first_portal: mat_id_to_ref(*first_portal, mats, visited_inline, uniforms),
            second_portal: mat_id_to_ref(*second_portal, mats, visited_inline, uniforms),
            what: mat_id_to_ref(*what, mats, visited_inline, uniforms),
        },
        OM::Simple {
            offset,
            scale,
            rotate,
            mirror,
        } => Matrix::Simple {
            offset: *offset,
            scale: *scale,
            rotate: *rotate,
            mirror: *mirror,
        },
        OM::Parametrized {
            offset,
            rotate,
            mirror,
            scale,
        } => Matrix::Parametrized {
            offset: tvec3_to_ser(offset, uniforms),
            rotate: tvec3_to_ser(rotate, uniforms),
            mirror: tvec3_to_ser(mirror, uniforms),
            scale: param_to_ser(scale, uniforms),
        },
        OM::Exact { i, j, k, pos } => Matrix::Exact {
            i: tvec3_to_ser(i, uniforms),
            j: tvec3_to_ser(j, uniforms),
            k: tvec3_to_ser(k, uniforms),
            pos: tvec3_to_ser(pos, uniforms),
        },
        OM::ExactFull { c0, c1, c2, c3 } => Matrix::ExactFull {
            c0: tvec4_to_ser(c0, uniforms),
            c1: tvec4_to_ser(c1, uniforms),
            c2: tvec4_to_ser(c2, uniforms),
            c3: tvec4_to_ser(c3, uniforms),
        },
        OM::If {
            condition,
            then,
            otherwise,
        } => Matrix::If {
            condition: param_to_ser(condition, uniforms),
            then: mat_id_to_ref(*then, mats, visited_inline, uniforms),
            otherwise: mat_id_to_ref(*otherwise, mats, visited_inline, uniforms),
        },
        OM::Sqrt(a) => Matrix::Sqrt(mat_id_to_ref(*a, mats, visited_inline, uniforms)),
        OM::Lerp { t, first, second } => Matrix::Lerp {
            t: param_to_ser(t, uniforms),
            first: mat_id_to_ref(*first, mats, visited_inline, uniforms),
            second: mat_id_to_ref(*second, mats, visited_inline, uniforms),
        },
        OM::Camera => Matrix::Camera,
        OM::Inv(a) => Matrix::Inv(mat_id_to_ref(*a, mats, visited_inline, uniforms)),
    }
}

// ---------- Object helpers ----------
fn obj_type_to_ser(
    t: &OldObjectType,
    mats: &Storage2<OldMatrix>,
    uniforms: &Storage2<OldAnyUniform>,
) -> ObjectType {
    let mut visited_inline = Default::default();
    match t {
        OldObjectType::Simple(a) => {
            ObjectType::Simple(mat_id_to_ref(*a, mats, &mut visited_inline, uniforms))
        }
        OldObjectType::Portal(a, b) => ObjectType::Portal(
            mat_id_to_ref(*a, mats, &mut visited_inline, uniforms),
            mat_id_to_ref(*b, mats, &mut visited_inline, uniforms),
        ),
    }
}

fn obj_type_from_ser(
    t: ObjectType,
    mats: &mut Storage2<OldMatrix>,
    uniforms: &mut Storage2<OldAnyUniform>,
    names: &BTreeMap<String, MatrixId>,
) -> OldObjectType {
    match t {
        ObjectType::Simple(a) => {
            OldObjectType::Simple(a.and_then(|r| matrix_ref_to_id(r, mats, uniforms, names)))
        }
        ObjectType::Portal(a, b) => OldObjectType::Portal(
            a.and_then(|r| matrix_ref_to_id(r, mats, uniforms, names)),
            b.and_then(|r| matrix_ref_to_id(r, mats, uniforms, names)),
        ),
    }
}

// ---------- Camera helpers ----------
fn cam_to_ser(
    c: &OldCam,
    matrices_s: &Storage2<OldMatrix>,
    uniforms_s: &Storage2<OldAnyUniform>,
) -> Cam {
    let look_at = match c.look_at {
        CamLookAt::MatrixCenter(mid) => {
            let mut visited_inline = Default::default();
            CamLookAtSer::MatrixCenter(mat_id_to_ref(
                mid,
                matrices_s,
                &mut visited_inline,
                uniforms_s,
            ))
        }
        CamLookAt::Coordinate(v) => CamLookAtSer::Coordinate(v),
    };
    Cam {
        look_at,
        alpha: c.alpha,
        beta: c.beta,
        r: c.r,
        in_subspace: c.in_subspace,
        free_movement: c.free_movement,
        matrix: c.matrix,
    }
}

fn cam_id_to_ref(
    cid: Option<CameraId>,
    cameras_s: &Storage2<OldCam>,
    matrices_s: &Storage2<OldMatrix>,
    uniforms_s: &Storage2<OldAnyUniform>,
) -> Option<CamRef> {
    let cid = cid?;
    match cameras_s.get_name(cid) {
        Some(Some(n)) => Some(CamRef::Named(n.to_owned())),
        Some(None) => {
            let c = cameras_s.get_original(cid)?;
            Some(CamRef::Inline(Box::new(cam_to_ser(
                c, matrices_s, uniforms_s,
            ))))
        }
        None => None,
    }
}

fn param_from_ser(p: ParametrizeOrNot, uniforms: &mut Storage2<OldAnyUniform>) -> OldParamOrNot {
    match p {
        ParametrizeOrNot::Value(f) => OldParamOrNot::No(f),
        ParametrizeOrNot::Uniform(opt) => {
            OldParamOrNot::Yes(opt.and_then(|u| uniform_ref_to_id(u, uniforms)))
        }
    }
}

fn tvec3_from_ser(v: TVec3, uniforms: &mut Storage2<OldAnyUniform>) -> OldTVec3 {
    OldTVec3 {
        x: param_from_ser(v.x, uniforms),
        y: param_from_ser(v.y, uniforms),
        z: param_from_ser(v.z, uniforms),
    }
}

fn tvec4_from_ser(v: TVec4, uniforms: &mut Storage2<OldAnyUniform>) -> OldTVec4 {
    OldTVec4 {
        x: param_from_ser(v.x, uniforms),
        y: param_from_ser(v.y, uniforms),
        z: param_from_ser(v.z, uniforms),
        w: param_from_ser(v.w, uniforms),
    }
}

fn cam_from_ser(
    data: &Cam,
    matrices: &mut Storage2<OldMatrix>,
    uniforms: &mut Storage2<OldAnyUniform>,
    mat_name_to_id: &BTreeMap<String, MatrixId>,
) -> OldCam {
    let mut c = OldCam::default();
    c.look_at = match data.look_at.clone() {
        CamLookAtSer::Coordinate(v) => CamLookAt::Coordinate(v),
        CamLookAtSer::MatrixCenter(opt) => CamLookAt::MatrixCenter(
            opt.and_then(|r| matrix_ref_to_id(r, matrices, uniforms, mat_name_to_id)),
        ),
    };
    c.alpha = data.alpha;
    c.beta = data.beta;
    c.r = data.r;
    c.in_subspace = data.in_subspace;
    c.free_movement = data.free_movement;
    c.matrix = data.matrix;
    c
}

// Animations/Stages/Filters/Descriptions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
enum CurrentStageSer {
    #[default]
    Dev,
    Animation(String),
    RealAnimation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StageAnimSer<T> {
    ProvidedToUser,
    FromDev,
    Changed(Option<T>),
    ChangedAndToUser(Option<T>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AnimationFiltersSer {
    uniforms: BTreeMap<String, bool>,
    matrices: BTreeMap<String, bool>,
    cameras: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ElementsDescriptionsSer {
    uniforms: BTreeMap<String, super::animation::ValueToUser>,
    matrices: BTreeMap<String, super::animation::ValueToUser>,
    cameras: BTreeMap<String, super::animation::ValueToUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DevStageSer {
    uniforms: BTreeMap<String, AnyUniform>,
    matrices: BTreeMap<String, Matrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnimationStageSer {
    name: EngRusText,
    description: Option<EngRusText>,
    uniforms: BTreeMap<String, StageAnimSer<UniformRef>>,
    matrices: BTreeMap<String, StageAnimSer<MatrixRef>>,
    set_cam: Option<Option<CamRef>>,
    cams: BTreeMap<String, bool>,
    #[serde(default)]
    hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum RealAnimPartSer<T> {
    CopyPrev,
    Changed(Option<T>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RealStageChangingSer<T>(BTreeMap<String, RealAnimPartSer<T>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RealAnimationSer {
    #[serde(default)]
    duration: f64,
    animation_stage: CurrentStageSer,
    uniforms: RealStageChangingSer<UniformRef>,
    matrices: RealStageChangingSer<MatrixRef>,
    #[serde(default)]
    use_prev_cam: bool,
    #[serde(default)]
    use_start_cam_as_end: bool,
    cam_start: Option<CamRef>,
    cam_end: Option<CamRef>,
    #[serde(default)]
    use_any_cam_as_start: Option<bool>,
    #[serde(default)]
    use_any_cam_as_end: Option<bool>,
    #[serde(default)]
    cam_any_start: Option<String>,
    #[serde(default)]
    cam_any_end: Option<String>,
    #[serde(default)]
    cam_easing: super::easing::Easing,
    #[serde(default)]
    cam_easing_uniform: Option<UniformRef>,
}

impl Default for RealAnimationSer {
    fn default() -> Self {
        Self {
            duration: 0.0,
            animation_stage: CurrentStageSer::Dev,
            uniforms: RealStageChangingSer(BTreeMap::new()),
            matrices: RealStageChangingSer(BTreeMap::new()),
            use_prev_cam: false,
            use_start_cam_as_end: false,
            cam_start: None,
            cam_end: None,
            use_any_cam_as_start: None,
            use_any_cam_as_end: None,
            cam_any_start: None,
            cam_any_end: None,
            cam_easing: super::easing::Easing::Linear,
            cam_easing_uniform: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GlobalUserUniformsSer {
    uniforms: BTreeMap<String, bool>,
    matrices: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedScene {
    desc: EngRusText,
    cam: CamSettings,

    uniforms: SerStorage<AnyUniform>,
    matrices: SerStorage<Matrix>,
    objects: SerStorage<Object>,
    cameras: SerStorage<Cam>,
    textures: SerStorage<TextureName>,
    materials: SerStorage<Material>,
    #[serde(default)]
    intersection_materials: SerStorage<IntersectionMaterial>,
    library: SerStorage<LibraryCode>,

    #[serde(default)]
    animations_filters: AnimationFiltersSer,
    #[serde(default)]
    elements_descriptions: ElementsDescriptionsSer,
    #[serde(default)]
    user_uniforms: GlobalUserUniformsSer,
    animation_stages: SerStorage<AnimationStageSer>,
    #[serde(default)]
    current_stage: CurrentStageSer,
    #[serde(default)]
    dev_stage: DevStageSer,
    #[serde(default)]
    animations: SerStorage<RealAnimationSer>,

    #[serde(default)]
    use_time: bool,
    #[serde(default)]
    skybox: Option<String>,
}

// Helpers: name resolvers for storages
// (no IdMaps; resolve names inline)

// Conversions Scene -> SerializedScene are inlined into
// serialize_scene_new_format to avoid duplication.

pub(crate) fn serialize_scene_new_format(scene: &super::scene::Scene) -> SerializedScene {
    // Inline former serialize_parts + serialize_all
    let desc = &scene.desc;
    let cam = &scene.cam;
    let uniforms_s = &scene.uniforms;
    let matrices_s = &scene.matrices;
    let objects_s = &scene.objects;
    let cameras_s = &scene.cameras;
    let textures_s = &scene.textures;
    let materials_s = &scene.materials;
    let intersection_materials_s = &scene.intersection_materials;
    let library_s = &scene.library;
    let use_time = &scene.use_time;
    let skybox = &scene.skybox;

    // uniforms (named only)
    let uniforms = SerStorage(
        uniforms_s
            .visible_elements()
            .map(|(id, name)| Named {
                name: name.to_owned(),
                data: uniforms_s.get_original(id).unwrap().clone(),
            })
            .collect(),
    );

    // matrices
    let mut matrices_vec = Vec::new();
    for (id, name) in matrices_s.visible_elements() {
        let mut visited_inline = Default::default();
        let val = matrices_s.get_original(id).unwrap();
        matrices_vec.push(Named {
            name: name.to_owned(),
            data: matrix_to_ser(val, matrices_s, uniforms_s, &mut visited_inline),
        });
    }
    let matrices = SerStorage(matrices_vec);

    // objects
    let objects = SerStorage(
        objects_s
            .visible_elements()
            .map(|(id, name)| {
                let o = objects_s.get_original(id).unwrap();
                let data = match o {
                    OldObject::DebugMatrix(a) => {
                        let mut visited_inline = Default::default();
                        Object::DebugMatrix(mat_id_to_ref(
                            *a,
                            matrices_s,
                            &mut visited_inline,
                            uniforms_s,
                        ))
                    }
                    OldObject::Flat {
                        kind,
                        is_inside,
                        in_subspace,
                    } => Object::Flat {
                        kind: obj_type_to_ser(kind, matrices_s, uniforms_s),
                        is_inside: is_inside.clone(),
                        in_subspace: in_subspace.clone(),
                    },
                    OldObject::Complex {
                        kind,
                        intersect,
                        in_subspace,
                    } => Object::Complex {
                        kind: obj_type_to_ser(kind, matrices_s, uniforms_s),
                        intersect: intersect.clone(),
                        in_subspace: in_subspace.clone(),
                    },
                };
                Named {
                    name: name.to_owned(),
                    data,
                }
            })
            .collect(),
    );

    // cameras
    let cameras = SerStorage(
        cameras_s
            .visible_elements()
            .map(|(id, name)| {
                let c = cameras_s.get_original(id).unwrap();
                let data = cam_to_ser(c, matrices_s, uniforms_s);
                Named {
                    name: name.to_owned(),
                    data,
                }
            })
            .collect(),
    );

    // textures/materials/intersections/library
    let textures = SerStorage(
        textures_s
            .visible_elements()
            .map(|(id, name)| Named {
                name: name.to_owned(),
                data: textures_s.get_original(id).unwrap().clone(),
            })
            .collect(),
    );
    let materials = SerStorage(
        materials_s
            .visible_elements()
            .map(|(id, name)| Named {
                name: name.to_owned(),
                data: materials_s.get_original(id).unwrap().clone(),
            })
            .collect(),
    );
    let intersection_materials = SerStorage(
        intersection_materials_s
            .visible_elements()
            .map(|(id, name)| Named {
                name: name.to_owned(),
                data: intersection_materials_s.get_original(id).unwrap().clone(),
            })
            .collect(),
    );
    let library = SerStorage(
        library_s
            .visible_elements()
            .map(|(id, name)| Named {
                name: name.to_owned(),
                data: library_s.get_original(id).unwrap().clone(),
            })
            .collect(),
    );

    let mut base = SerializedScene {
        desc: desc.clone(),
        cam: cam.clone(),
        uniforms,
        matrices,
        objects,
        cameras,
        textures,
        materials,
        intersection_materials,
        library,
        animations_filters: AnimationFiltersSer::default(),
        elements_descriptions: ElementsDescriptionsSer::default(),
        user_uniforms: GlobalUserUniformsSer::default(),
        animation_stages: SerStorage(Vec::new()),
        current_stage: CurrentStageSer::Dev,
        dev_stage: DevStageSer::default(),
        animations: SerStorage(Vec::new()),
        use_time: *use_time,
        skybox: skybox.clone(),
    };

    // Append the rest of data formerly added in serialize_all
    let uniforms_s = &scene.uniforms;
    let matrices_s = &scene.matrices;
    let cameras_s = &scene.cameras;
    let animations_filters = &scene.animations_filters;
    let elements_descriptions = &scene.elements_descriptions;
    let user_uniforms = &scene.user_uniforms;
    let animation_stages_s = &scene.animation_stages;
    let dev_stage = &scene.dev_stage;
    let animations_s = &scene.animations;

    // animations filters
    let mut af = AnimationFiltersSer::default();
    for (id, name) in uniforms_s.visible_elements() {
        if let Some(v) = animations_filters.uniforms.0.get(&id).copied() {
            af.uniforms.insert(name.to_owned(), v);
        }
    }
    for (id, name) in matrices_s.visible_elements() {
        if let Some(v) = animations_filters.matrices.0.get(&id).copied() {
            af.matrices.insert(name.to_owned(), v);
        }
    }
    for (id, name) in cameras_s.visible_elements() {
        if let Some(v) = animations_filters.cameras.0.get(&id).copied() {
            af.cameras.insert(name.to_owned(), v);
        }
    }
    base.animations_filters = af;

    // elements descriptions
    let mut ed = ElementsDescriptionsSer::default();
    for (id, v) in &elements_descriptions.uniforms.0 {
        if let Some(Some(name)) = uniforms_s.get_name(*id) {
            ed.uniforms.insert(name.to_owned(), v.clone());
        }
    }
    for (id, v) in &elements_descriptions.matrices.0 {
        if let Some(Some(name)) = matrices_s.get_name(*id) {
            ed.matrices.insert(name.to_owned(), v.clone());
        }
    }
    for (id, v) in &elements_descriptions.cameras.0 {
        if let Some(Some(name)) = cameras_s.get_name(*id) {
            ed.cameras.insert(name.to_owned(), v.clone());
        }
    }
    base.elements_descriptions = ed;

    // user uniforms (global stage)
    let mut uu = GlobalUserUniformsSer::default();
    for (id, flag) in &user_uniforms.uniforms.0 {
        if let Some(Some(name)) = uniforms_s.get_name(*id) {
            uu.uniforms.insert(name.to_owned(), *flag);
        }
    }
    for (id, flag) in &user_uniforms.matrices.0 {
        if let Some(Some(name)) = matrices_s.get_name(*id) {
            uu.matrices.insert(name.to_owned(), *flag);
        }
    }
    base.user_uniforms = uu;

    // animation stages
    let mut stages_vec = Vec::new();
    for (sid, name) in animation_stages_s.visible_elements() {
        let st = animation_stages_s.get_original(sid).unwrap();
        // uniforms
        let mut u = BTreeMap::new();
        for (uid, a) in &st.uniforms.0 {
            let k = uniforms_s
                .get_name(*uid)
                .and_then(|x| x)
                .unwrap_or("?")
                .to_owned();
            let v = match a {
                super::animation::Animation::ProvidedToUser => StageAnimSer::ProvidedToUser,
                super::animation::Animation::FromDev => StageAnimSer::FromDev,
                super::animation::Animation::Changed(opt) => {
                    StageAnimSer::Changed(opt.and_then(|id| name_or_uniform_ref(uniforms_s, id)))
                }
                super::animation::Animation::ChangedAndToUser(opt) => {
                    StageAnimSer::ChangedAndToUser(
                        opt.and_then(|id| name_or_uniform_ref(uniforms_s, id)),
                    )
                }
            };
            u.insert(k, v);
        }
        // matrices
        let mut m = BTreeMap::new();
        for (mid, a) in &st.matrices.0 {
            let k = matrices_s
                .get_name(*mid)
                .and_then(|x| x)
                .unwrap_or("?")
                .to_owned();
            let v = match a {
                super::animation::Animation::ProvidedToUser => StageAnimSer::ProvidedToUser,
                super::animation::Animation::FromDev => StageAnimSer::FromDev,
                super::animation::Animation::Changed(opt) => {
                    StageAnimSer::Changed(opt.and_then(|id| {
                        let mut visited_inline = Default::default();
                        mat_id_to_ref(Some(id), matrices_s, &mut visited_inline, uniforms_s)
                    }))
                }
                super::animation::Animation::ChangedAndToUser(opt) => {
                    StageAnimSer::ChangedAndToUser(opt.and_then(|id| {
                        let mut visited_inline = Default::default();
                        mat_id_to_ref(Some(id), matrices_s, &mut visited_inline, uniforms_s)
                    }))
                }
            };
            m.insert(k, v);
        }
        // cams
        let mut cams = BTreeMap::new();
        for (cid, v) in &st.cams {
            if let Some(Some(n)) = cameras_s.get_name(*cid) {
                cams.insert(n.to_owned(), *v);
            }
        }
        let set_cam = match st.set_cam {
            None => None,
            Some(None) => Some(None),
            Some(Some(cid)) => Some(cam_id_to_ref(Some(cid), cameras_s, matrices_s, uniforms_s)),
        };
        stages_vec.push(Named {
            name: name.to_owned(),
            data: AnimationStageSer {
                name: st.name.clone(),
                description: st.description.clone(),
                uniforms: u,
                matrices: m,
                set_cam,
                cams,
                hidden: st.hidden,
            },
        });
    }
    base.animation_stages = SerStorage(stages_vec);

    // current stage
    base.current_stage = current_stage_to_ser(scene);

    // dev stage
    let mut ds = DevStageSer::default();
    for (uid, val) in dev_stage.uniforms.0.iter() {
        if let Some(Some(n)) = uniforms_s.get_name(*uid) {
            ds.uniforms.insert(n.to_owned(), val.clone());
        }
    }
    for (mid, val) in dev_stage.matrices.0.iter() {
        if let Some(Some(n)) = matrices_s.get_name(*mid) {
            let mut visited_inline = Default::default();
            ds.matrices.insert(
                n.to_owned(),
                matrix_to_ser(val, matrices_s, uniforms_s, &mut visited_inline),
            );
        }
    }
    base.dev_stage = ds;

    // real animations
    let mut ras = Vec::new();
    let real_names = animations_s
        .visible_elements()
        .map(|(id, name)| (id, name.to_owned()))
        .collect::<Vec<_>>();
    let stage_names = animation_stages_s
        .visible_elements()
        .map(|(id, name)| (id, name.to_owned()))
        .collect::<Vec<_>>();
    for (rid, name) in real_names.iter() {
        let a = animations_s.get_original(*rid).unwrap();
        // stage name
        let animation_stage = match a.animation_stage {
            super::scene::CurrentStage::Dev => CurrentStageSer::Dev,
            super::scene::CurrentStage::Animation(id) => {
                let n = stage_names
                    .iter()
                    .find(|(id2, _)| *id2 == id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| "?".into());
                CurrentStageSer::Animation(n)
            }
            super::scene::CurrentStage::RealAnimation(id) => {
                let n = real_names
                    .iter()
                    .find(|(id2, _)| *id2 == id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| "?".into());
                CurrentStageSer::RealAnimation(n)
            }
        };
        // uniforms
        let mut u = BTreeMap::new();
        for (uid, part) in a.uniforms.0.iter() {
            let k = uniforms_s
                .get_name(*uid)
                .and_then(|x| x)
                .unwrap_or("?")
                .to_owned();
            let v = match part {
                super::animation::RealAnimationPart::CopyPrev => RealAnimPartSer::CopyPrev,
                super::animation::RealAnimationPart::Changed(opt) => {
                    RealAnimPartSer::Changed(opt.and_then(|id| name_or_uniform_ref(uniforms_s, id)))
                }
            };
            u.insert(k, v);
        }
        // matrices
        let mut m = BTreeMap::new();
        for (mid, part) in a.matrices.0.iter() {
            let k = matrices_s
                .get_name(*mid)
                .and_then(|x| x)
                .unwrap_or("?")
                .to_owned();
            let v = match part {
                super::animation::RealAnimationPart::CopyPrev => RealAnimPartSer::CopyPrev,
                super::animation::RealAnimationPart::Changed(opt) => {
                    RealAnimPartSer::Changed(opt.and_then(|id| {
                        let mut visited_inline = Default::default();
                        mat_id_to_ref(Some(id), matrices_s, &mut visited_inline, uniforms_s)
                    }))
                }
            };
            m.insert(k, v);
        }
        // cam refs
        let cam_start_ref = cam_id_to_ref(a.cam_start, cameras_s, matrices_s, uniforms_s);
        let cam_end_ref = cam_id_to_ref(a.cam_end, cameras_s, matrices_s, uniforms_s);
        let cam_easing_uniform = a
            .cam_easing_uniform
            .and_then(|opt| opt.and_then(|id| name_or_uniform_ref(uniforms_s, id)));

        ras.push(Named {
            name: name.clone(),
            data: RealAnimationSer {
                duration: a.duration,
                animation_stage,
                uniforms: RealStageChangingSer(u),
                matrices: RealStageChangingSer(m),
                use_prev_cam: a.use_prev_cam,
                use_start_cam_as_end: a.use_start_cam_as_end,
                cam_start: cam_start_ref,
                cam_end: cam_end_ref,
                use_any_cam_as_start: a.use_any_cam_as_start,
                use_any_cam_as_end: a.use_any_cam_as_end,
                cam_any_start: a
                    .cam_any_start
                    .and_then(|id| animations_s.get_name(id))
                    .and_then(|x| x)
                    .map(|s| s.to_owned()),
                cam_any_end: a
                    .cam_any_end
                    .and_then(|id| animations_s.get_name(id))
                    .and_then(|x| x)
                    .map(|s| s.to_owned()),
                cam_easing: a.cam_easing.clone(),
                cam_easing_uniform,
            },
        });
    }
    base.animations = SerStorage(ras);

    base
}

pub(crate) fn deserialize_scene_new_format(ser: SerializedScene, scene: &mut super::scene::Scene) {
    // Inline former deserialize_into_parts
    scene.desc = ser.desc.clone();
    scene.cam = ser.cam.clone();
    scene.use_time = ser.use_time;
    scene.skybox = ser.skybox.clone();

    // textures
    for Named { name, data } in ser.textures.0.clone().into_iter() {
        scene.textures.insert_named_with_order(name, data);
    }

    // uniforms
    for Named { name, data } in ser.uniforms.0.clone().into_iter() {
        scene.uniforms.insert_named_with_order(name, data);
    }

    // matrices: two-pass
    let mut mat_name_to_id = BTreeMap::new();
    for Named { name, .. } in ser.matrices.0.iter() {
        let id = scene
            .matrices
            .insert_named_with_order(name.clone(), Default::default());
        mat_name_to_id.insert(name.clone(), id);
    }
    for Named { name, data } in ser.matrices.0.clone().into_iter() {
        let id = *mat_name_to_id.get(&name).unwrap();
        let value = matrix_from_ser(
            data,
            &mut scene.matrices,
            &mut scene.uniforms,
            &mat_name_to_id,
        );
        scene.matrices.set(id, value);
    }

    // objects
    for Named { name, data } in ser.objects.0.clone().into_iter() {
        let old = match data {
            Object::DebugMatrix(a) => OldObject::DebugMatrix(a.and_then(|r| {
                matrix_ref_to_id(r, &mut scene.matrices, &mut scene.uniforms, &mat_name_to_id)
            })),
            Object::Flat {
                kind,
                is_inside,
                in_subspace,
            } => OldObject::Flat {
                kind: obj_type_from_ser(
                    kind,
                    &mut scene.matrices,
                    &mut scene.uniforms,
                    &mat_name_to_id,
                ),
                is_inside,
                in_subspace,
            },
            Object::Complex {
                kind,
                intersect,
                in_subspace,
            } => OldObject::Complex {
                kind: obj_type_from_ser(
                    kind,
                    &mut scene.matrices,
                    &mut scene.uniforms,
                    &mat_name_to_id,
                ),
                intersect,
                in_subspace,
            },
        };
        scene.objects.insert_named_with_order(name, old);
    }

    // cameras
    for Named { name, data } in ser.cameras.0.clone().into_iter() {
        let c = cam_from_ser(
            &data,
            &mut scene.matrices,
            &mut scene.uniforms,
            &mat_name_to_id,
        );
        scene.cameras.insert_named_with_order(name, c);
    }

    // materials/intersections/library
    for Named { name, data } in ser.materials.0.clone().into_iter() {
        scene.materials.insert_named_with_order(name, data);
    }
    for Named { name, data } in ser.intersection_materials.0.clone().into_iter() {
        scene
            .intersection_materials
            .insert_named_with_order(name, data);
    }
    for Named { name, data } in ser.library.0.clone().into_iter() {
        scene.library.insert_named_with_order(name, data);
    }

    // Build name->id maps for follow-up structures
    let uni_by_name: BTreeMap<String, UniformId> = scene
        .uniforms
        .visible_elements()
        .map(|(id, name)| (name.to_owned(), id))
        .collect();
    let mat_by_name: BTreeMap<String, MatrixId> = scene
        .matrices
        .visible_elements()
        .map(|(id, name)| (name.to_owned(), id))
        .collect();
    let cam_by_name: BTreeMap<String, CameraId> = scene
        .cameras
        .visible_elements()
        .map(|(id, name)| (name.to_owned(), id))
        .collect();

    // Animations filters
    for (name, v) in ser.animations_filters.uniforms.into_iter() {
        if let Some(id) = uni_by_name.get(&name).copied() {
            scene.animations_filters.uniforms.0.insert(id, v);
        }
    }
    for (name, v) in ser.animations_filters.matrices.into_iter() {
        if let Some(id) = mat_by_name.get(&name).copied() {
            scene.animations_filters.matrices.0.insert(id, v);
        }
    }
    for (name, v) in ser.animations_filters.cameras.into_iter() {
        if let Some(id) = cam_by_name.get(&name).copied() {
            scene.animations_filters.cameras.0.insert(id, v);
        }
    }

    // Elements descriptions
    for (name, v) in ser.elements_descriptions.uniforms.into_iter() {
        if let Some(id) = uni_by_name.get(&name).copied() {
            scene.elements_descriptions.uniforms.0.insert(id, v);
        }
    }
    for (name, v) in ser.elements_descriptions.matrices.into_iter() {
        if let Some(id) = mat_by_name.get(&name).copied() {
            scene.elements_descriptions.matrices.0.insert(id, v);
        }
    }
    for (name, v) in ser.elements_descriptions.cameras.into_iter() {
        if let Some(id) = cam_by_name.get(&name).copied() {
            scene.elements_descriptions.cameras.0.insert(id, v);
        }
    }

    // Global user uniforms
    for (name, flag) in ser.user_uniforms.uniforms.into_iter() {
        if let Some(id) = uni_by_name.get(&name).copied() {
            scene.user_uniforms.uniforms.0.insert(id, flag);
        }
    }
    for (name, flag) in ser.user_uniforms.matrices.into_iter() {
        if let Some(id) = mat_by_name.get(&name).copied() {
            scene.user_uniforms.matrices.0.insert(id, flag);
        }
    }

    // Animation stages two-pass
    let mut stage_name_to_id = BTreeMap::new();
    for Named { name, .. } in ser.animation_stages.0.iter() {
        let id = scene
            .animation_stages
            .insert_named_with_order(name.clone(), Default::default());
        stage_name_to_id.insert(name.clone(), id);
    }
    for Named { name, data } in ser.animation_stages.0.into_iter() {
        let id = *stage_name_to_id.get(&name).unwrap();
        let mut st = super::animation::AnimationStage::default();
        st.name = data.name;
        st.description = data.description;
        // uniforms
        for (uname, anim) in data.uniforms.into_iter() {
            if let Some(uid) = uni_by_name.get(&uname).copied() {
                let old = match anim {
                    StageAnimSer::ProvidedToUser => super::animation::Animation::ProvidedToUser,
                    StageAnimSer::FromDev => super::animation::Animation::FromDev,
                    StageAnimSer::Changed(opt) => super::animation::Animation::Changed(
                        opt.and_then(|u| uniform_ref_to_id(u, &mut scene.uniforms)),
                    ),
                    StageAnimSer::ChangedAndToUser(opt) => {
                        super::animation::Animation::ChangedAndToUser(
                            opt.and_then(|u| uniform_ref_to_id(u, &mut scene.uniforms)),
                        )
                    }
                };
                st.uniforms.0.insert(uid, old);
            }
        }
        // matrices
        for (mname, anim) in data.matrices.into_iter() {
            if let Some(mid) = mat_by_name.get(&mname).copied() {
                let old = match anim {
                    StageAnimSer::ProvidedToUser => super::animation::Animation::ProvidedToUser,
                    StageAnimSer::FromDev => super::animation::Animation::FromDev,
                    StageAnimSer::Changed(opt) => {
                        super::animation::Animation::Changed(opt.and_then(|r| {
                            matrix_ref_to_id(
                                r,
                                &mut scene.matrices,
                                &mut scene.uniforms,
                                &mat_by_name,
                            )
                        }))
                    }
                    StageAnimSer::ChangedAndToUser(opt) => {
                        super::animation::Animation::ChangedAndToUser(opt.and_then(|r| {
                            matrix_ref_to_id(
                                r,
                                &mut scene.matrices,
                                &mut scene.uniforms,
                                &mat_by_name,
                            )
                        }))
                    }
                };
                st.matrices.0.insert(mid, old);
            }
        }
        // set_cam
        st.set_cam = match data.set_cam {
            None => None,
            Some(None) => Some(None),
            Some(Some(CamRef::Named(name))) => Some(cam_by_name.get(&name).copied()),
            Some(Some(CamRef::Inline(boxed))) => {
                // insert inline camera and return its id
                let c = cam_from_ser(
                    &boxed,
                    &mut scene.matrices,
                    &mut scene.uniforms,
                    &mat_by_name,
                );
                Some(Some(scene.cameras.insert_inline(c)))
            }
        };
        for (cname, v) in data.cams.into_iter() {
            if let Some(cid) = cam_by_name.get(&cname).copied() {
                st.cams.insert(cid, v);
            }
        }
        st.hidden = data.hidden;
        scene.animation_stages.set(id, st);
    }

    // Dev stage
    for (uname, val) in ser.dev_stage.uniforms.into_iter() {
        if let Some(uid) = uni_by_name.get(&uname).copied() {
            scene.dev_stage.uniforms.0.insert(uid, val);
        }
    }
    for (mname, val) in ser.dev_stage.matrices.into_iter() {
        if let Some(mid) = mat_by_name.get(&mname).copied() {
            let old = matrix_from_ser(val, &mut scene.matrices, &mut scene.uniforms, &mat_by_name);
            scene.dev_stage.matrices.0.insert(mid, old);
        }
    }

    // Real animations two-pass
    let mut real_name_to_id = BTreeMap::new();
    for Named { name, .. } in ser.animations.0.iter() {
        let id = scene
            .animations
            .insert_named_with_order(name.clone(), Default::default());
        real_name_to_id.insert(name.clone(), id);
    }
    for Named { name, data } in ser.animations.0.into_iter() {
        let id = *real_name_to_id.get(&name).unwrap();
        let mut a = super::animation::RealAnimation::default();
        a.duration = data.duration;
        a.animation_stage = match data.animation_stage {
            CurrentStageSer::Dev => super::scene::CurrentStage::Dev,
            CurrentStageSer::Animation(n) => stage_name_to_id
                .get(&n)
                .map(|&x| super::scene::CurrentStage::Animation(x))
                .unwrap_or(super::scene::CurrentStage::Dev),
            CurrentStageSer::RealAnimation(n) => real_name_to_id
                .get(&n)
                .map(|&x| super::scene::CurrentStage::RealAnimation(x))
                .unwrap_or(super::scene::CurrentStage::Dev),
        };
        // uniforms
        for (uname, part) in data.uniforms.0.into_iter() {
            if let Some(uid) = uni_by_name.get(&uname).copied() {
                let p = match part {
                    RealAnimPartSer::CopyPrev => super::animation::RealAnimationPart::CopyPrev,
                    RealAnimPartSer::Changed(opt) => super::animation::RealAnimationPart::Changed(
                        opt.and_then(|u| uniform_ref_to_id(u, &mut scene.uniforms)),
                    ),
                };
                a.uniforms.0.insert(uid, p);
            }
        }
        // matrices
        for (mname, part) in data.matrices.0.into_iter() {
            if let Some(mid) = mat_by_name.get(&mname).copied() {
                let p = match part {
                    RealAnimPartSer::CopyPrev => super::animation::RealAnimationPart::CopyPrev,
                    RealAnimPartSer::Changed(opt) => {
                        super::animation::RealAnimationPart::Changed(opt.and_then(|r| {
                            matrix_ref_to_id(
                                r,
                                &mut scene.matrices,
                                &mut scene.uniforms,
                                &mat_by_name,
                            )
                        }))
                    }
                };
                a.matrices.0.insert(mid, p);
            }
        }
        a.use_prev_cam = data.use_prev_cam;
        a.use_start_cam_as_end = data.use_start_cam_as_end;
        // cam_start
        a.cam_start = match data.cam_start {
            None => None,
            Some(CamRef::Named(n)) => cam_by_name.get(&n).copied(),
            Some(CamRef::Inline(boxed)) => {
                // insert inline camera and return id
                let c = cam_from_ser(
                    &boxed,
                    &mut scene.matrices,
                    &mut scene.uniforms,
                    &mat_by_name,
                );
                Some(scene.cameras.insert_inline(c))
            }
        };
        // cam_end
        a.cam_end = match data.cam_end {
            None => None,
            Some(CamRef::Named(n)) => cam_by_name.get(&n).copied(),
            Some(CamRef::Inline(boxed)) => {
                let c = cam_from_ser(
                    &boxed,
                    &mut scene.matrices,
                    &mut scene.uniforms,
                    &mat_by_name,
                );
                Some(scene.cameras.insert_inline(c))
            }
        };
        a.use_any_cam_as_start = data.use_any_cam_as_start;
        a.use_any_cam_as_end = data.use_any_cam_as_end;
        a.cam_any_start = data
            .cam_any_start
            .and_then(|n| real_name_to_id.get(&n).copied());
        a.cam_any_end = data
            .cam_any_end
            .and_then(|n| real_name_to_id.get(&n).copied());
        a.cam_easing = data.cam_easing;
        a.cam_easing_uniform = data
            .cam_easing_uniform
            .and_then(|u| uniform_ref_to_id(u, &mut scene.uniforms))
            .map(|id| Some(id));
        scene.animations.set(id, a);
    }

    // Set current stage from serialized value
    set_current_stage_from_ser(scene, ser.current_stage);
}

fn name_or_uniform_ref(uniforms: &Storage2<OldAnyUniform>, id: UniformId) -> Option<UniformRef> {
    match uniforms.get_name(id) {
        Some(Some(name)) => Some(UniformRef::Named(name.to_owned())),
        Some(None) => Some(UniformRef::Inline(Box::new(
            uniforms.get_original(id).unwrap().clone(),
        ))),
        None => None,
    }
}

fn uniform_ref_to_id(u: UniformRef, uniforms: &mut Storage2<OldAnyUniform>) -> Option<UniformId> {
    match u {
        UniformRef::Named(n) => uniforms.find_id(&n),
        UniformRef::Inline(bx) => Some(uniforms.insert_inline(*bx)),
    }
}

fn matrix_ref_to_id(
    r: MatrixRef,
    mats: &mut Storage2<OldMatrix>,
    uniforms: &mut Storage2<OldAnyUniform>,
    mat_name_to_id: &BTreeMap<String, MatrixId>,
) -> Option<MatrixId> {
    match r {
        MatrixRef::Named(n) => mat_name_to_id.get(&n).copied(),
        MatrixRef::Inline(bx) => {
            let val = matrix_from_ser(*bx, mats, uniforms, mat_name_to_id);
            Some(mats.insert_inline(val))
        }
    }
}

fn matrix_from_ser(
    m: Matrix,
    mats: &mut Storage2<OldMatrix>,
    uniforms: &mut Storage2<OldAnyUniform>,
    mat_name_to_id: &BTreeMap<String, MatrixId>,
) -> OldMatrix {
    use Matrix as M;
    match m {
        M::Mul { to, what } => OldMatrix::Mul {
            to: to.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
            what: what.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
        },
        M::Teleport {
            first_portal,
            second_portal,
            what,
        } => OldMatrix::Teleport {
            first_portal: first_portal
                .and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
            second_portal: second_portal
                .and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
            what: what.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
        },
        M::Simple {
            offset,
            scale,
            rotate,
            mirror,
        } => OldMatrix::Simple {
            offset,
            scale,
            rotate,
            mirror,
        },
        M::Parametrized {
            offset,
            rotate,
            mirror,
            scale,
        } => OldMatrix::Parametrized {
            offset: tvec3_from_ser(offset, uniforms),
            rotate: tvec3_from_ser(rotate, uniforms),
            mirror: tvec3_from_ser(mirror, uniforms),
            scale: param_from_ser(scale, uniforms),
        },
        M::Exact { i, j, k, pos } => OldMatrix::Exact {
            i: tvec3_from_ser(i, uniforms),
            j: tvec3_from_ser(j, uniforms),
            k: tvec3_from_ser(k, uniforms),
            pos: tvec3_from_ser(pos, uniforms),
        },
        M::ExactFull { c0, c1, c2, c3 } => OldMatrix::ExactFull {
            c0: tvec4_from_ser(c0, uniforms),
            c1: tvec4_from_ser(c1, uniforms),
            c2: tvec4_from_ser(c2, uniforms),
            c3: tvec4_from_ser(c3, uniforms),
        },
        M::If {
            condition,
            then,
            otherwise,
        } => OldMatrix::If {
            condition: param_from_ser(condition, uniforms),
            then: then.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
            otherwise: otherwise.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
        },
        M::Sqrt(a) => {
            OldMatrix::Sqrt(a.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)))
        }
        M::Lerp { t, first, second } => OldMatrix::Lerp {
            t: param_from_ser(t, uniforms),
            first: first.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
            second: second.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)),
        },
        M::Camera => OldMatrix::Camera,
        M::Inv(a) => {
            OldMatrix::Inv(a.and_then(|x| matrix_ref_to_id(x, mats, uniforms, mat_name_to_id)))
        }
    }
}
