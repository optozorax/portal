use super::animation::*;
use super::camera::*;
use super::glsl::*;
use super::intersection_material::IntersectionMaterialId;
use super::material::*;
use super::matrix::*;
use super::object::*;
use super::scene::Scene;
use super::storage2::*;
use super::texture::*;
use super::uniform::*;
use super::unique_id::UniqueId;
use super::video::*;

use std::collections::BTreeMap;

#[derive(Default, Clone)]
pub struct IdMaps {
    pub uniforms: BTreeMap<UniqueId, UniqueId>,
    pub matrices: BTreeMap<UniqueId, UniqueId>,
    pub objects: BTreeMap<UniqueId, UniqueId>,
    pub cameras: BTreeMap<UniqueId, UniqueId>,
    pub textures: BTreeMap<UniqueId, UniqueId>,
    pub videos: BTreeMap<UniqueId, UniqueId>,
    pub materials: BTreeMap<UniqueId, UniqueId>,
    pub intersections: BTreeMap<UniqueId, UniqueId>,
    pub library: BTreeMap<UniqueId, UniqueId>,
    pub anim_stages: BTreeMap<UniqueId, UniqueId>,
    pub real_anims: BTreeMap<UniqueId, UniqueId>,
}

impl IdMaps {
    fn map<T: Wrapper>(map: &BTreeMap<UniqueId, UniqueId>, id: T) -> T {
        T::wrap(*map.get(&id.un_wrap()).unwrap_or(&id.un_wrap()))
    }
    fn map_opt<T: Wrapper>(map: &BTreeMap<UniqueId, UniqueId>, id: Option<T>) -> Option<T> {
        id.map(|id| Self::map(map, id))
    }

    pub fn map_uniform(&self, id: UniformId) -> UniformId {
        Self::map(&self.uniforms, id)
    }
    pub fn map_opt_uniform(&self, id: Option<UniformId>) -> Option<UniformId> {
        Self::map_opt(&self.uniforms, id)
    }
    pub fn map_matrix(&self, id: MatrixId) -> MatrixId {
        Self::map(&self.matrices, id)
    }
    pub fn map_opt_matrix(&self, id: Option<MatrixId>) -> Option<MatrixId> {
        Self::map_opt(&self.matrices, id)
    }
    pub fn map_object(&self, id: ObjectId) -> ObjectId {
        Self::map(&self.objects, id)
    }
    pub fn map_camera(&self, id: CameraId) -> CameraId {
        Self::map(&self.cameras, id)
    }
    pub fn map_opt_camera(&self, id: Option<CameraId>) -> Option<CameraId> {
        Self::map_opt(&self.cameras, id)
    }
    pub fn map_texture(&self, id: TextureId) -> TextureId {
        Self::map(&self.textures, id)
    }
    pub fn map_material(&self, id: MaterialId) -> MaterialId {
        Self::map(&self.materials, id)
    }
    pub fn map_intersection(&self, id: IntersectionMaterialId) -> IntersectionMaterialId {
        Self::map(&self.intersections, id)
    }
    pub fn map_library(&self, id: LibraryId) -> LibraryId {
        Self::map(&self.library, id)
    }
    pub fn map_anim_stage(&self, id: AnimationId) -> AnimationId {
        Self::map(&self.anim_stages, id)
    }
    pub fn map_real_anim(&self, id: RealAnimationId) -> RealAnimationId {
        Self::map(&self.real_anims, id)
    }
}

// ---------------- Value remapping helpers ----------------

fn remap_param(p: &ParametrizeOrNot, maps: &IdMaps) -> ParametrizeOrNot {
    match p {
        ParametrizeOrNot::No(f) => ParametrizeOrNot::No(*f),
        ParametrizeOrNot::Yes(opt) => ParametrizeOrNot::Yes(maps.map_opt_uniform(*opt)),
    }
}

fn remap_tvec3(v: &TVec3, maps: &IdMaps) -> TVec3 {
    TVec3 {
        x: remap_param(&v.x, maps),
        y: remap_param(&v.y, maps),
        z: remap_param(&v.z, maps),
    }
}

fn remap_tvec4(v: &TVec4, maps: &IdMaps) -> TVec4 {
    TVec4 {
        x: remap_param(&v.x, maps),
        y: remap_param(&v.y, maps),
        z: remap_param(&v.z, maps),
        w: remap_param(&v.w, maps),
    }
}

fn remap_matrix_value(m: &Matrix, maps: &IdMaps) -> Matrix {
    use Matrix::*;
    match m {
        Mul { to, what } => Mul {
            to: maps.map_opt_matrix(*to),
            what: maps.map_opt_matrix(*what),
        },
        Teleport {
            first_portal,
            second_portal,
            what,
        } => Teleport {
            first_portal: maps.map_opt_matrix(*first_portal),
            second_portal: maps.map_opt_matrix(*second_portal),
            what: maps.map_opt_matrix(*what),
        },
        Simple {
            offset,
            scale,
            rotate,
            mirror,
        } => Simple {
            offset: *offset,
            scale: *scale,
            rotate: *rotate,
            mirror: *mirror,
        },
        Parametrized {
            offset,
            rotate,
            mirror,
            scale,
        } => Parametrized {
            offset: remap_tvec3(offset, maps),
            rotate: remap_tvec3(rotate, maps),
            mirror: remap_tvec3(mirror, maps),
            scale: remap_param(scale, maps),
        },
        Exact { i, j, k, pos } => Exact {
            i: remap_tvec3(i, maps),
            j: remap_tvec3(j, maps),
            k: remap_tvec3(k, maps),
            pos: remap_tvec3(pos, maps),
        },
        ExactFull { c0, c1, c2, c3 } => ExactFull {
            c0: remap_tvec4(c0, maps),
            c1: remap_tvec4(c1, maps),
            c2: remap_tvec4(c2, maps),
            c3: remap_tvec4(c3, maps),
        },
        If {
            condition,
            then,
            otherwise,
        } => If {
            condition: remap_param(condition, maps),
            then: maps.map_opt_matrix(*then),
            otherwise: maps.map_opt_matrix(*otherwise),
        },
        Sqrt(a) => Sqrt(maps.map_opt_matrix(*a)),
        Lerp { t, first, second } => Lerp {
            t: remap_param(t, maps),
            first: maps.map_opt_matrix(*first),
            second: maps.map_opt_matrix(*second),
        },
        Camera => Camera,
        Inv(a) => Inv(maps.map_opt_matrix(*a)),
    }
}

fn remap_cam_value(c: &Cam, maps: &IdMaps) -> Cam {
    Cam {
        look_at: match c.look_at {
            CamLookAt::Coordinate(v) => CamLookAt::Coordinate(v),
            CamLookAt::MatrixCenter(id) => CamLookAt::MatrixCenter(maps.map_opt_matrix(id)),
        },
        alpha: c.alpha,
        beta: c.beta,
        r: c.r,
        in_subspace: c.in_subspace,
        free_movement: c.free_movement,
        matrix: c.matrix,
    }
}

fn remap_video_value(v: &Video, maps: &IdMaps) -> Video {
    let mut v2 = v.clone();
    v2.uniform = maps.map_opt_uniform(v.uniform);
    v2
}

fn remap_object_value(o: &Object, maps: &IdMaps) -> Object {
    use Object::*;
    match o {
        DebugMatrix(id) => DebugMatrix(maps.map_opt_matrix(*id)),
        Flat {
            kind,
            is_inside,
            in_subspace,
        } => Flat {
            kind: match kind {
                ObjectType::Simple(a) => ObjectType::Simple(maps.map_opt_matrix(*a)),
                ObjectType::Portal(a, b) => {
                    ObjectType::Portal(maps.map_opt_matrix(*a), maps.map_opt_matrix(*b))
                }
            },
            is_inside: is_inside.clone(),
            in_subspace: in_subspace.clone(),
        },
        Complex {
            kind,
            intersect,
            in_subspace,
        } => Complex {
            kind: match kind {
                ObjectType::Simple(a) => ObjectType::Simple(maps.map_opt_matrix(*a)),
                ObjectType::Portal(a, b) => {
                    ObjectType::Portal(maps.map_opt_matrix(*a), maps.map_opt_matrix(*b))
                }
            },
            intersect: intersect.clone(),
            in_subspace: in_subspace.clone(),
        },
    }
}

fn remap_animation_stage_value(st: &AnimationStage, maps: &IdMaps) -> AnimationStage {
    let mut st2 = st.clone();
    // uniforms map
    st2.uniforms = StageChanging(
        st.uniforms
            .0
            .iter()
            .map(|(k, v)| {
                let nv = match v {
                    Animation::ProvidedToUser => Animation::ProvidedToUser,
                    Animation::FromDev => Animation::FromDev,
                    Animation::Changed(opt) => Animation::Changed(maps.map_opt_uniform(*opt)),
                    Animation::ChangedAndToUser(opt) => {
                        Animation::ChangedAndToUser(maps.map_opt_uniform(*opt))
                    }
                };
                (maps.map_uniform(*k), nv)
            })
            .collect(),
    );
    // matrices map
    st2.matrices = StageChanging(
        st.matrices
            .0
            .iter()
            .map(|(k, v)| {
                let nv = match v {
                    Animation::ProvidedToUser => Animation::ProvidedToUser,
                    Animation::FromDev => Animation::FromDev,
                    Animation::Changed(opt) => Animation::Changed(maps.map_opt_matrix(*opt)),
                    Animation::ChangedAndToUser(opt) => {
                        Animation::ChangedAndToUser(maps.map_opt_matrix(*opt))
                    }
                };
                (maps.map_matrix(*k), nv)
            })
            .collect(),
    );
    // cameras map
    st2.cams = st
        .cams
        .iter()
        .map(|(k, v)| (maps.map_camera(*k), *v))
        .collect();
    // set_cam
    st2.set_cam = match st.set_cam {
        None => None,
        Some(None) => Some(None),
        Some(Some(id)) => Some(Some(maps.map_camera(id))),
    };
    st2
}

fn remap_real_animation_value(ra: &RealAnimation, maps: &IdMaps) -> RealAnimation {
    let mut r = ra.clone();
    // stage
    r.animation_stage = match ra.animation_stage {
        super::scene::CurrentStage::Dev => super::scene::CurrentStage::Dev,
        super::scene::CurrentStage::Animation(id) => {
            super::scene::CurrentStage::Animation(maps.map_anim_stage(id))
        }
        super::scene::CurrentStage::RealAnimation(id) => {
            super::scene::CurrentStage::RealAnimation(maps.map_real_anim(id))
        }
    };
    // uniforms
    r.uniforms = RealAnimationStageChanging(
        ra.uniforms
            .0
            .iter()
            .map(|(k, v)| {
                let nv = match v {
                    RealAnimationPart::CopyPrev => RealAnimationPart::CopyPrev,
                    RealAnimationPart::Changed(opt) => {
                        RealAnimationPart::Changed(maps.map_opt_uniform(*opt))
                    }
                };
                (maps.map_uniform(*k), nv)
            })
            .collect(),
    );
    // matrices
    r.matrices = RealAnimationStageChanging(
        ra.matrices
            .0
            .iter()
            .map(|(k, v)| {
                let nv = match v {
                    RealAnimationPart::CopyPrev => RealAnimationPart::CopyPrev,
                    RealAnimationPart::Changed(opt) => {
                        RealAnimationPart::Changed(maps.map_opt_matrix(*opt))
                    }
                };
                (maps.map_matrix(*k), nv)
            })
            .collect(),
    );
    // cameras
    r.cam_start = maps.map_opt_camera(ra.cam_start);
    r.cam_end = maps.map_opt_camera(ra.cam_end);
    r.cam_any_start = ra.cam_any_start.map(|id| maps.map_real_anim(id));
    r.cam_any_end = ra.cam_any_end.map(|id| maps.map_real_anim(id));
    r.cam_easing_uniform = ra.cam_easing_uniform.map(|opt| maps.map_opt_uniform(opt));
    r
}

// ---------------- Scene-wide mapping ----------------

pub fn compute_maps(scene: &Scene) -> IdMaps {
    let mut maps = IdMaps::default();
    maps.uniforms = scene.uniforms.hash_id_map();
    maps.matrices = scene.matrices.hash_id_map();
    maps.objects = scene.objects.hash_id_map();
    maps.cameras = scene.cameras.hash_id_map();
    maps.textures = scene.textures.hash_id_map();
    maps.videos = scene.videos.hash_id_map();
    maps.materials = scene.materials.hash_id_map();
    maps.intersections = scene.intersection_materials.hash_id_map();
    maps.library = scene.library.hash_id_map();
    maps.anim_stages = scene.animation_stages.hash_id_map();
    maps.real_anims = scene.animations.hash_id_map();
    maps
}

pub fn remap_scene(scene: &Scene, maps: &IdMaps) -> Scene {
    let mut s = scene.clone();

    // storages
    s.uniforms = s
        .uniforms
        .remap_ids_and_values(&|id| *maps.uniforms.get(&id).unwrap_or(&id), &|v| v.clone());

    s.matrices = s
        .matrices
        .remap_ids_and_values(&|id| *maps.matrices.get(&id).unwrap_or(&id), &|v| {
            remap_matrix_value(v, maps)
        });

    s.objects = s
        .objects
        .remap_ids_and_values(&|id| *maps.objects.get(&id).unwrap_or(&id), &|v| {
            remap_object_value(v, maps)
        });

    s.cameras = s
        .cameras
        .remap_ids_and_values(&|id| *maps.cameras.get(&id).unwrap_or(&id), &|v| {
            remap_cam_value(v, maps)
        });

    s.textures = s
        .textures
        .remap_ids_and_values(&|id| *maps.textures.get(&id).unwrap_or(&id), &|v| v.clone());
    s.videos = s
        .videos
        .remap_ids_and_values(&|id| *maps.videos.get(&id).unwrap_or(&id), &|v| {
            remap_video_value(v, maps)
        });
    s.materials = s
        .materials
        .remap_ids_and_values(&|id| *maps.materials.get(&id).unwrap_or(&id), &|v| {
            v.clone()
        });
    s.intersection_materials = s
        .intersection_materials
        .remap_ids_and_values(&|id| *maps.intersections.get(&id).unwrap_or(&id), &|v| {
            v.clone()
        });
    s.library = s
        .library
        .remap_ids_and_values(&|id| *maps.library.get(&id).unwrap_or(&id), &|v| v.clone());

    s.animation_stages = s
        .animation_stages
        .remap_ids_and_values(&|id| *maps.anim_stages.get(&id).unwrap_or(&id), &|v| {
            remap_animation_stage_value(v, maps)
        });
    s.animations = s
        .animations
        .remap_ids_and_values(&|id| *maps.real_anims.get(&id).unwrap_or(&id), &|v| {
            remap_real_animation_value(v, maps)
        });

    // current stage
    s.current_stage = match s.current_stage {
        super::scene::CurrentStage::Dev => super::scene::CurrentStage::Dev,
        super::scene::CurrentStage::Animation(id) => {
            super::scene::CurrentStage::Animation(maps.map_anim_stage(id))
        }
        super::scene::CurrentStage::RealAnimation(id) => {
            super::scene::CurrentStage::RealAnimation(maps.map_real_anim(id))
        }
    };

    // Animation filters
    s.animations_filters.uniforms = AnimationFilter(
        s.animations_filters
            .uniforms
            .0
            .iter()
            .map(|(k, v)| (maps.map_uniform(*k), *v))
            .collect(),
    );
    s.animations_filters.matrices = AnimationFilter(
        s.animations_filters
            .matrices
            .0
            .iter()
            .map(|(k, v)| (maps.map_matrix(*k), *v))
            .collect(),
    );
    s.animations_filters.cameras = AnimationFilter(
        s.animations_filters
            .cameras
            .0
            .iter()
            .map(|(k, v)| (maps.map_camera(*k), *v))
            .collect(),
    );

    // Elements descriptions
    s.elements_descriptions.uniforms = ElementsDescription(
        s.elements_descriptions
            .uniforms
            .0
            .iter()
            .map(|(k, v)| (maps.map_uniform(*k), v.clone()))
            .collect(),
    );
    s.elements_descriptions.matrices = ElementsDescription(
        s.elements_descriptions
            .matrices
            .0
            .iter()
            .map(|(k, v)| (maps.map_matrix(*k), v.clone()))
            .collect(),
    );
    s.elements_descriptions.cameras = ElementsDescription(
        s.elements_descriptions
            .cameras
            .0
            .iter()
            .map(|(k, v)| (maps.map_camera(*k), v.clone()))
            .collect(),
    );

    // Global user uniforms
    s.user_uniforms.uniforms = GlobalStage(
        s.user_uniforms
            .uniforms
            .0
            .iter()
            .map(|(k, v)| (maps.map_uniform(*k), *v))
            .collect(),
    );
    s.user_uniforms.matrices = GlobalStage(
        s.user_uniforms
            .matrices
            .0
            .iter()
            .map(|(k, v)| (maps.map_matrix(*k), *v))
            .collect(),
    );

    // Dev stage
    s.dev_stage.uniforms = DevStageChanging(
        s.dev_stage
            .uniforms
            .0
            .iter()
            .map(|(k, v)| (maps.map_uniform(*k), v.clone()))
            .collect(),
    );
    s.dev_stage.matrices = DevStageChanging(
        s.dev_stage
            .matrices
            .0
            .iter()
            .map(|(k, v)| (maps.map_matrix(*k), remap_matrix_value(v, maps)))
            .collect(),
    );

    s
}

pub fn stabilize_ids(mut scene: Scene, max_iters: usize) -> Scene {
    let mut prev_maps = IdMaps::default();
    for _ in 0..max_iters {
        let maps = compute_maps(&scene);

        // Check stabilization: all maps are identity and equal to previous?
        let is_identity = |m: &BTreeMap<UniqueId, UniqueId>| m.iter().all(|(k, v)| k == v);
        if is_identity(&maps.uniforms)
            && is_identity(&maps.matrices)
            && is_identity(&maps.objects)
            && is_identity(&maps.cameras)
            && is_identity(&maps.textures)
            && is_identity(&maps.videos)
            && is_identity(&maps.materials)
            && is_identity(&maps.intersections)
            && is_identity(&maps.library)
            && is_identity(&maps.anim_stages)
            && is_identity(&maps.real_anims)
            && maps.uniforms == prev_maps.uniforms
            && maps.matrices == prev_maps.matrices
            && maps.objects == prev_maps.objects
            && maps.cameras == prev_maps.cameras
            && maps.textures == prev_maps.textures
            && maps.videos == prev_maps.videos
            && maps.materials == prev_maps.materials
            && maps.intersections == prev_maps.intersections
            && maps.library == prev_maps.library
            && maps.anim_stages == prev_maps.anim_stages
            && maps.real_anims == prev_maps.real_anims
        {
            break;
        }
        prev_maps = maps.clone();
        scene = remap_scene(&scene, &maps);
    }
    scene
}
