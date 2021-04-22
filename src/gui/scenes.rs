use egui::Ui;

struct Scene {
    name: &'static str,
    link: &'static str,
    content: &'static str,
    hidden: bool,
}

pub struct Scenes(Vec<(&'static str, Vec<Scene>)>);

impl Default for Scenes {
    fn default() -> Self {
        Self(vec![
            (
                "Technical",
                vec![
                    Scene {
                        name: "Empty",
                        link: "empty",
                        content: include_str!("../../scenes/empty.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Room",
                        link: "room",
                        content: include_str!("../../scenes/room.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Misc",
                        link: "misc",
                        content: include_str!("../../scenes/misc.json"),
                        hidden: false,
                    },
                ],
            ),
            (
                "Not 2 parts",
                vec![
                    Scene {
                        name: "Monoportal",
                        link: "monoportal",
                        content: include_str!("../../scenes/monoportal.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Monoportal offset",
                        link: "monoportal_offset",
                        content: include_str!("../../scenes/monoportal_offset.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Triple portal",
                        link: "triple_portal",
                        content: include_str!("../../scenes/triple_portal.json"),
                        hidden: false,
                    },
                ],
            ),
            (
                "Mobius",
                vec![
                    Scene {
                        name: "Portal",
                        link: "mobius",
                        content: include_str!("../../scenes/mobius.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Monoportal",
                        link: "mobius_monoportal",
                        content: include_str!("../../scenes/mobius_monoportal.json"),
                        hidden: false,
                    },
                ],
            ),
            (
                "Links",
                vec![Scene {
                    name: "Hopf Link portal",
                    link: "hopf_link",
                    content: include_str!("../../scenes/hopf_link.json"),
                    hidden: false,
                }],
            ),
            (
                "Trefoil",
                vec![
                    Scene {
                        name: "Self-hiding order 1",
                        link: "trefoil_knot",
                        content: include_str!(
                            "../../scenes/trefoil_knot_monoportal_self_hiding.json"
                        ),
                        hidden: false,
                    },
                    Scene {
                        name: "Order 1",
                        link: "trefoil_knot",
                        content: include_str!("../../scenes/trefoil_knot_monoportal.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Order 2",
                        link: "trefoil_knot",
                        content: include_str!("../../scenes/trefoil_knot.json"),
                        hidden: false,
                    },
                    Scene {
                        name: "Order 3",
                        link: "trefoil_knot_3",
                        content: include_str!("../../scenes/trefoil_knot_3.json"),
                        hidden: false,
                    },
                ],
            ),
            (
                "Portal in portal",
                vec![Scene {
                    name: "Ellipse portals",
                    link: "portal_in_portal",
                    content: include_str!("../../scenes/portal_in_portal.json"),
                    hidden: false,
                }],
            ),
        ])
    }
}

impl Scenes {
    pub fn get_by_link(&self, need_link: &str) -> Option<&'static str> {
        for Scene { content, link, .. } in self.0.iter().map(|x| x.1.iter()).flatten() {
            if *link == need_link {
                return Some(content);
            }
        }
        None
    }

    pub fn egui(&self, ui: &mut Ui) -> Option<(&'static str, &'static str)> {
        let mut result = None;
        for (pos, (name, inner)) in self.0.iter().enumerate() {
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
                if !hidden && ui.button(*name).clicked() {
                    result = Some((*content, *link))
                }
            }
        }
        result
    }
}
