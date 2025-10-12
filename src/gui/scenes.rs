use egui::Ui;

struct Scene {
    name: &'static str,
    link: &'static str,
    content: &'static str,
    hidden: bool,
}

pub struct Scenes(Vec<SceneSection>);

struct SceneSection {
    name: &'static str,
    hidden: bool,
    scenes: Vec<Scene>,
}

#[derive(Clone, Copy, Default)]
pub struct ShowHiddenScenes(pub bool);

impl Default for Scenes {
    fn default() -> Self {
        Self(vec![
            SceneSection {
                name: "Technical",
                hidden: true,
                scenes: vec![
                    Scene {
                        name: "Empty",
                        link: "empty",
                        content: include_str!("../../scenes/empty.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Room",
                        link: "room",
                        content: include_str!("../../scenes/room.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "White room",
                        link: "white_room",
                        content: include_str!("../../scenes/white_room.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Analytical approach (for video)",
                        link: "analytical_approach",
                        content: include_str!("../../scenes/analytical_approach.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Digits texture",
                        link: "digits_debug",
                        content: include_str!("../../scenes/digits_debug.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Wheatley texture",
                        link: "wheatley_texture",
                        content: include_str!("../../scenes/wheatley_texture.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Companion cube texture",
                        link: "companion_cube_texture",
                        content: include_str!("../../scenes/companion_cube_texture.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Teleportation degrees (for video)",
                        link: "teleportation_degrees",
                        content: include_str!("../../scenes/teleportation_degrees.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Spheres (for video)",
                        link: "spheres_anim",
                        content: include_str!("../../scenes/spheres_anim.ron"),
                        hidden: true,
                    },
                ],
            },
            SceneSection {
                name: "Basics",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Basics",
                        link: "basics",
                        content: include_str!("../../scenes/basics.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Same shape",
                        link: "same_shape",
                        content: include_str!("../../scenes/same_shape.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Cutting prism",
                        link: "cut_prism",
                        content: include_str!("../../scenes/cut_prism.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Cutting plane",
                        link: "cut_plane",
                        content: include_str!("../../scenes/cut_plane.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Linear transformations",
                        link: "linear_transformations",
                        content: include_str!("../../scenes/linear_transformations.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Surface portal",
                        link: "surface_portal",
                        content: include_str!("../../scenes/surface_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Surface portal???",
                        link: "surface_portal2",
                        content: include_str!("../../scenes/surface_portal2.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Support portals",
                        link: "support_portals",
                        content: include_str!("../../scenes/support_portals.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Inverted surface",
                        link: "inverted_surface",
                        content: include_str!("../../scenes/inverted_surface.ron"),
                        hidden: false,
                    },
                ],
            },
            SceneSection {
                name: "Physics",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Moving doorway",
                        link: "moving_doorway",
                        content: include_str!("../../scenes/moving_doorway.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Speed model",
                        link: "speed_model",
                        content: include_str!("../../scenes/speed_model.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Non linear movement",
                        link: "non_linear",
                        content: include_str!("../../scenes/non_linear.ron"),
                        hidden: false,
                    },
                ],
            },
            SceneSection {
                name: "Triple portal",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Triple portal",
                        link: "triple_portal",
                        content: include_str!("../../scenes/triple_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Triple portal fully in itself",
                        link: "triple_portal2",
                        content: include_str!("../../scenes/triple_portal2.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "\"Triple portal\"",
                        link: "triple_portal_ish",
                        content: include_str!("../../scenes/triple_portal_ish.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "N-tuple portal",
                        link: "n_tuple_portal",
                        content: include_str!("../../scenes/n_tuple_portal.ron"),
                        hidden: false,
                    },
                ],
            },
            SceneSection {
                name: "Advanced",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Cylinder",
                        link: "cylinder",
                        content: include_str!("../../scenes/cylinder.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Cylinder spherical",
                        link: "cylinder_spherical",
                        content: include_str!("../../scenes/cylinder_spherical.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Trefoil knot",
                        link: "trefoil",
                        content: include_str!("../../scenes/trefoil.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Hopf Link portal",
                        link: "hopf_link",
                        content: include_str!("../../scenes/hopf_link.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Spherical geometry model",
                        link: "spherical_geometry",
                        content: include_str!("../../scenes/spherical_geometry.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Sphere to plane mapping portal",
                        link: "sphere_to_plane",
                        content: include_str!("../../scenes/sphere_to_plane.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Sphere to sphere mapping portal",
                        link: "sphere_to_sphere",
                        content: include_str!("../../scenes/sphere_to_sphere.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Sphere intersection",
                        link: "sphere_intersection",
                        content: include_str!("../../scenes/sphere_intersection.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Recursive space",
                        link: "recursive_space",
                        content: include_str!("../../scenes/recursive_space.ron"),
                        hidden: true,
                    },
                ],
            },
            SceneSection {
                name: "Monoportals",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Monoportal",
                        link: "monoportal",
                        content: include_str!("../../scenes/monoportal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Rotating monoportal",
                        link: "monoportal_rotating",
                        content: include_str!("../../scenes/monoportal_rotating.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "N-monoportal",
                        link: "monoportal_n",
                        content: include_str!("../../scenes/monoportal_n.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Offsetting monoportal",
                        link: "monoportal_offset",
                        content: include_str!("../../scenes/monoportal_offset.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Offsetting monoportal 2",
                        link: "monoportal_offset2",
                        content: include_str!("../../scenes/monoportal_offset2.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Scaling monoportal",
                        link: "monoportal_scale",
                        content: include_str!("../../scenes/monoportal_scale.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Scaling monoportal 2",
                        link: "monoportal_scale2",
                        content: include_str!("../../scenes/monoportal_scale2.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Logarithmic spiral monoportal",
                        link: "monoportal_log",
                        content: include_str!("../../scenes/monoportal_log.ron"),
                        hidden: false,
                    },
                ],
            },
            SceneSection {
                name: "Tilings",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Triangle tiling",
                        link: "triangle_tiling",
                        content: include_str!("../../scenes/triangle_tiling.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Square tiling",
                        link: "square_tiling",
                        content: include_str!("../../scenes/square_tiling.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Hexagonal tiling",
                        link: "hexagonal_tiling",
                        content: include_str!("../../scenes/hexagonal_tiling.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Triple tiling",
                        link: "triple_tiling",
                        content: include_str!("../../scenes/triple_tiling.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Recursive room",
                        link: "recursive_room",
                        content: include_str!("../../scenes/recursive_room.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Double pentagon",
                        link: "double_pentagon",
                        content: include_str!("../../scenes/double_pentagon.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Cone point",
                        link: "cone",
                        content: include_str!("../../scenes/cone.ron"),
                        hidden: true,
                    },
                ],
            },
            SceneSection {
                name: "Mobius band",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Mobius Portal",
                        link: "mobius",
                        content: include_str!("../../scenes/mobius.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Mobius Monoportal",
                        link: "mobius_monoportal",
                        content: include_str!("../../scenes/mobius_monoportal.ron"),
                        hidden: false,
                    },
                ],
            },
            SceneSection {
                name: "Portal in portal",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Two pairs",
                        link: "portal_in_portal_two_pairs",
                        content: include_str!("../../scenes/portal_in_portal_two_pairs.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Ellipse",
                        link: "portal_in_portal",
                        content: include_str!("../../scenes/portal_in_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Fully inside attempt with support portal",
                        link: "portal_in_portal_1x_attempt",
                        content: include_str!("../../scenes/portal_in_portal_1x_attempt.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Plus ultra",
                        link: "plus_ultra",
                        content: include_str!("../../scenes/portal_in_portal_plus_ultra.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Infinite planes to emulate plus ultra",
                        link: "portal_in_portal_cone",
                        content: include_str!("../../scenes/portal_in_portal_cone.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Half-spheres",
                        link: "half_spheres",
                        content: include_str!("../../scenes/half_spheres.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Monoportal",
                        link: "monoportal_in_monoportal",
                        content: include_str!("../../scenes/monoportal_in_monoportal.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Self-intersect",
                        link: "self_intersect",
                        content: include_str!("../../scenes/self_intersect.ron"),
                        hidden: true,
                    },
                    Scene {
                        name: "Matryoshka",
                        link: "matryoshka",
                        content: include_str!("../../scenes/matryoshka.ron"),
                        hidden: false,
                    },
                ],
            },
            SceneSection {
                name: "Community scenes",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Borromean rings (by Frisk256)",
                        link: "borromean_rings",
                        content: include_str!("../../scenes/borromean_rings.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Portal leaving Pocket Dimension using Supportal (by AiQube)",
                        link: "leave_dimension",
                        content: include_str!("../../scenes/leave_dimension.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Flat triple portal (by AiQube)",
                        link: "flat_triple_portal",
                        content: include_str!("../../scenes/flat_triple_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Triangular Prism Portal (by AiQube)",
                        link: "triangular_prism_portal",
                        content: include_str!("../../scenes/triangular_prism_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Zeno Portal (by Frisk256)",
                        link: "zeno_portal",
                        content: include_str!("../../scenes/zeno_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Octahedral Portal (by cuongvd)",
                        link: "octahedral_portal",
                        content: include_str!("../../scenes/octahedral_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Cubic Portal (by cuongvd)",
                        link: "cubic_portal",
                        content: include_str!("../../scenes/cubic_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Tetrahedral Portal (by cuongvd)",
                        link: "tetrahedral_portal",
                        content: include_str!("../../scenes/tetrahedral_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Pocket dimension in pocket dimension (by cuongvd)",
                        link: "pocket_dimension_in_pocket_dimension",
                        content: include_str!("../../scenes/pocket_dimension_in_pocket_dimension.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Borromean rings pocket dimension (by cuongvd)",
                        link: "borromean_rings_pocket_dimension",
                        content: include_str!("../../scenes/borromean_rings_pocket_dimension.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "2n Rotating Monoportal (by cuongvd)",
                        link: "2n_rotating_monoportal",
                        content: include_str!("../../scenes/2n_rotating_monoportal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "k-offsetting n-monoportal (by cuongvd)",
                        link: "k_offsetting_n_monoportal",
                        content: include_str!("../../scenes/k_offsetting_n_monoportal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "All Pocket Dimenions with Negatie Monopotals (by AiQube)",
                        link: "all_dimension",
                        content: include_str!("../../scenes/all_dimension.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Flower Monoportal made from Triple Portal (by AiQube)",
                        link: "flower_monoportal",
                        content: include_str!("../../scenes/flower_monoportal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Time Portal (by AiQube)",
                        link: "time_portal",
                        content: include_str!("../../scenes/time_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Wormholes using 1 way portal (by cuongvd)",
                        link: "wormholes_using_1_way_portal",
                        content: include_str!("../../scenes/wormholes_using_1_way_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Time Portal Plus Ultra (by cuongvd and AiQube)",
                        link: "time_portal_plus_ultra",
                        content: include_str!("../../scenes/time_portal_plus_ultra.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Infinitely nested pocket dimension (by cuongvd)",
                        link: "nested_infinite_pocket_dimension",
                        content: include_str!("../../scenes/nested_infinite_pocket_dimension.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Pocket dimension using spherical portals (by cuongvd)",
                        link: "spherical_pocket_dimension",
                        content: include_str!("../../scenes/spherical_pocket_dimension.ron"),
                        hidden: false,
                    },
                ],
            },
        ])
    }
}

impl Scenes {
    pub fn get_by_link(&self, need_link: &str) -> Option<(&'static str, &'static str)> {
        for Scene {
            content,
            link,
            name,
            ..
        } in self.0.iter().flat_map(|x| x.scenes.iter())
        {
            if *link == need_link {
                return Some((content, name));
            }
        }
        None
    }

    pub fn get_all_scenes_links(&self) -> Vec<String> {
        self.0
            .iter()
            .flat_map(|x| x.scenes.iter())
            .map(|x| x.link.to_owned())
            .collect()
    }

    pub fn egui(&self, ui: &mut Ui) -> Option<(&'static str, &'static str, &'static str)> {
        let show_hidden = ui.memory_mut(|memory| {
            memory
                .data
                .get_persisted_mut_or_default::<ShowHiddenScenes>(egui::Id::new("ShowHiddenScenes"))
                .0
        });

        ui.set_width(170.);
        let mut result = None;
        for SceneSection {
            name,
            hidden,
            scenes: inner,
        } in self.0.iter()
        {
            if show_hidden || !hidden {
                let name2 = if *hidden {
                    format!("* {}", *name).to_string()
                } else {
                    name.to_string()
                };
                ui.menu_button(&name2, |ui| {
                    for Scene {
                        name,
                        content,
                        hidden,
                        link,
                    } in inner
                    {
                        let name2 = if *hidden {
                            format!("* {}", *name).to_string()
                        } else {
                            name.to_string()
                        };
                        if (show_hidden || !hidden) && ui.button(&name2).clicked() {
                            result = Some((*content, *link, *name))
                        }
                    }
                });
            }
        }
        result
    }
}
