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
                    // Scene {
                    //     name: "Misc",
                    //     link: "misc",
                    //     content: include_str!("../../scenes/misc.json"),
                    //     hidden: true,
                    // },
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
                name: "Triple portal",
                hidden: false,
                scenes: vec![Scene {
                    name: "Triple portal",
                    link: "triple_portal",
                    content: include_str!("../../scenes/triple_portal.ron"),
                    hidden: false,
                }],
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
                name: "Links",
                hidden: false,
                scenes: vec![Scene {
                    name: "Hopf Link portal",
                    link: "hopf_link",
                    content: include_str!("../../scenes/hopf_link.ron"),
                    hidden: false,
                }],
            },
            // SceneSection {
            //     name: "Trefoil",
            //     hidden: true,
            //     scenes: vec![
            //         Scene {
            //             name: "Self-hiding order 1",
            //             link: "trefoil_knot",
            //             content: include_str!(
            //                 "../../scenes/trefoil_knot_monoportal_self_hiding.json"
            //             ),
            //             hidden: false,
            //         },
            //         Scene {
            //             name: "Order 1",
            //             link: "trefoil_knot",
            //             content: include_str!("../../scenes/trefoil_knot_monoportal.json"),
            //             hidden: false,
            //         },
            //         Scene {
            //             name: "Order 2",
            //             link: "trefoil_knot",
            //             content: include_str!("../../scenes/trefoil_knot.json"),
            //             hidden: false,
            //         },
            //         Scene {
            //             name: "Order 3",
            //             link: "trefoil_knot_3",
            //             content: include_str!("../../scenes/trefoil_knot_3.json"),
            //             hidden: false,
            //         },
            //     ],
            // },
            SceneSection {
                name: "Portal in portal",
                hidden: false,
                scenes: vec![
                    Scene {
                        name: "Portal in portal: two pairs",
                        link: "portal_in_portal_two_pairs",
                        content: include_str!("../../scenes/portal_in_portal_two_pairs.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Ellipse portal in portal",
                        link: "portal_in_portal",
                        content: include_str!("../../scenes/portal_in_portal.ron"),
                        hidden: false,
                    },
                    Scene {
                        name: "Monoportal in monoportal",
                        link: "monoportal_in_monoportal",
                        content: include_str!("../../scenes/monoportal_in_monoportal.ron"),
                        hidden: true,
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
        } in self.0.iter().map(|x| x.scenes.iter()).flatten()
        {
            if *link == need_link {
                return Some((content, name));
            }
        }
        None
    }

    pub fn egui(&self, ui: &mut Ui) -> Option<(&'static str, &'static str, &'static str)> {
        let show_hidden = ui.memory().data.get_or_default::<ShowHiddenScenes>().0;
        ui.set_width(170.);
        let mut result = None;
        for (
            pos,
            SceneSection {
                name,
                hidden,
                scenes: inner,
            },
        ) in self.0.iter().enumerate()
        {
            if show_hidden || !hidden {
                if pos != 0 {
                    ui.separator();
                }
                ui.add(egui::Label::new(*name).strong().underline().monospace());
                for Scene {
                    name,
                    content,
                    hidden,
                    link,
                } in inner
                {
                    if (show_hidden || !hidden) && ui.button(*name).clicked() {
                        result = Some((*content, *link, *name))
                    }
                }
            }
        }
        result
    }
}
