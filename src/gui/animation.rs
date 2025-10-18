use crate::gui::camera::Cam;
use crate::gui::camera::CameraId;
use crate::gui::camera::CurrentCam;
use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::easing::Easing;
use crate::gui::eng_rus::EngRusText;
use crate::gui::matrix::Matrix;
use crate::gui::scene::CurrentStage;
use crate::gui::storage2::GetHelper;
use crate::gui::storage2::*;
use crate::gui::uniform::*;
use crate::gui::unique_id::UniqueId;
use egui::*;
use glam::*;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::hlist;

const ANIMATION_STAGE_NAME_SIZE: f64 = 100.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValueToUser {
    help_description: Option<EngRusText>,
    pub overrided_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Animation<T: StorageElem2> {
    ProvidedToUser,
    FromDev,
    Changed(Option<T::IdWrapper>),
    ChangedAndToUser(Option<T::IdWrapper>),
}

impl<T: StorageElem2> Animation<T> {
    pub fn get_t(&self) -> Option<&T::IdWrapper> {
        if let Animation::Changed(Some(t)) | Animation::ChangedAndToUser(Some(t)) = self {
            Some(t)
        } else {
            None
        }
    }
}

#[allow(clippy::derivable_impls)]
impl<T: StorageElem2> Default for Animation<T> {
    fn default() -> Self {
        Animation::FromDev
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageChanging<T: StorageElem2>(BTreeMap<T::IdWrapper, Animation<T>>);

impl<T: StorageElem2> Default for StageChanging<T> {
    fn default() -> Self {
        StageChanging(BTreeMap::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevStageChanging<T: StorageElem2>(BTreeMap<T::IdWrapper, T>);

impl<T: StorageElem2> Default for DevStageChanging<T> {
    fn default() -> Self {
        DevStageChanging(BTreeMap::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationFilter<T: StorageElem2>(BTreeMap<T::IdWrapper, bool>);

impl<T: StorageElem2> Default for AnimationFilter<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl ValueToUser {
    pub fn egui(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                egui_label(ui, "Name:", 45.);
                ui.text_edit_singleline(&mut self.overrided_name);
            });
            egui_option(
                ui,
                &mut self.help_description,
                "Has description",
                Default::default,
                |ui, help| {
                    ui.horizontal(|ui| {
                        egui_label(ui, "Desc:", 45.);
                        help.egui_multiline(ui);
                    });
                    false
                },
            );
        });
    }

    pub fn user_egui(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let previous = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = 0.;
            ui.label(self.overrided_name.to_string());
            if let Some(help) = &self.help_description {
                let response = ui.add(egui::Label::new(egui::RichText::new("(?)").small_raised()));
                response.on_hover_text(help.text(ui));
            }
            ui.label(": ");
            ui.spacing_mut().item_spacing.x = previous;
        });
    }

    pub fn description(&self, ui: &mut Ui) {
        let previous = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = 0.;
        if let Some(help) = &self.help_description {
            let response = ui.add(egui::Label::new("(?)"));
            response.on_hover_text(help.text(ui));
        }
        ui.spacing_mut().item_spacing.x = previous;
    }
}

impl<T: StorageElem2 + std::fmt::Debug> StageChanging<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        global: &mut GlobalStage<T>,
        filter: &mut AnimationFilter<T>,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        let visible_elements = storage
            .visible_elements()
            .map(|(id, name)| (id, name.to_owned()))
            .collect::<Vec<_>>();
        for (id, name) in visible_elements {
            let global = *global.0.entry(id).or_default();
            let enabled = *filter.0.entry(id).or_default();
            let anim = self.0.entry(id).or_default();
            if !global && enabled {
                changed |= anim.egui(ui, storage, input, &name, data_id.with(id));
            }
        }
        changed
    }

    pub fn remove(&self, storage: &mut Storage2<T>, input: &mut T::Input) {
        for id in self.0.values().filter_map(|x| x.get_t()) {
            storage.remove_as_field(*id, input);
        }
    }

    pub fn errors_count(&self, storage: &Storage2<T>, input: &T::Input) -> usize {
        self.0
            .values()
            .map(|x| {
                x.get_t()
                    .map(|id| storage.errors_inline(*id, input))
                    .unwrap_or(0)
            })
            .sum::<usize>()
    }

    pub fn init_stage(&self, storage: &mut Storage2<T>, dev_stage: &DevStageChanging<T>) {
        for (id, uniform) in self.0.iter() {
            if let Some(new_id) = uniform.get_t() {
                storage.set_id(*id, *new_id);
            } else if let Animation::FromDev | Animation::ProvidedToUser = uniform {
                if let Some(value) = dev_stage.0.get(id).cloned() {
                    storage.set(*id, value);
                } else {
                    crate::error!(debug, (id, uniform));
                }
            }
        }
    }

    pub fn user_egui(
        &self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        names: &mut ElementsDescription<T>,
        user_egui: impl Fn(&mut T, &mut Ui) -> WhatChanged,
        vertical: bool,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        for (i, id) in storage
            .visible_elements()
            .map(|(id, _)| id)
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
        {
            if let Some(element) = self.0.get(&id) {
                ui.memory_mut(|memory| {
                    memory
                        .data
                        .insert_persisted::<usize>(egui::Id::new("AnimationStage i"), i)
                });
                changed |= element.user_egui(ui, storage, &user_egui, names, id, vertical);
            } else {
                crate::error!();
            }
        }
        changed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevStage {
    pub uniforms: DevStageChanging<AnyUniform>,
    pub matrices: DevStageChanging<Matrix>,
}

impl<T: StorageElem2> DevStageChanging<T> {
    pub fn init_stage(&self, storage: &mut Storage2<T>) {
        for (id, value) in self.0.iter() {
            storage.set(*id, value.clone());
        }
    }

    pub fn copy(&mut self, storage: &Storage2<T>) {
        self.0.clear();
        for (id, _) in storage.visible_elements() {
            let value = storage.get_original(id).unwrap().clone();
            self.0.insert(id, value);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStage<T: StorageElem2>(BTreeMap<T::IdWrapper, bool>);

impl<T: StorageElem2> Default for GlobalStage<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: StorageElem2> GlobalStage<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        filter: &mut AnimationFilter<T>,
    ) -> bool {
        let mut changed = false;
        for (id, name) in storage.visible_elements() {
            let enabled = *filter.0.entry(id).or_default();
            if enabled {
                let enabled = self.0.entry(id).or_default();
                changed |= check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
            }
        }
        changed
    }

    pub fn user_egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        names: &mut ElementsDescription<T>,
        user_egui: impl Fn(&mut T, &mut Ui) -> WhatChanged,
        vertical: bool,
    ) -> WhatChanged {
        let mut result = WhatChanged::default();
        if self.0.iter().any(|x| *x.1) {
            for id in storage
                .visible_elements()
                .map(|(id, _)| id)
                .collect::<Vec<_>>()
            {
                if let Some(has) = self.0.get(&id) {
                    if *has {
                        let name = names.0.entry(id).or_default().clone();
                        if let Some(element) = storage.get_original_mut(id) {
                            if vertical {
                                ui.vertical(|ui| {
                                    name.user_egui(ui);
                                    result |= user_egui(element, ui);
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    name.user_egui(ui);
                                    result |= user_egui(element, ui);
                                });
                            }
                        } else {
                            self.0.remove(&id);
                        }
                    }
                } else {
                    // crate::error!();
                }
            }
            ui.separator();
        }
        result
    }
}

impl<T: StorageElem2> AnimationFilter<T> {
    pub fn egui(&mut self, ui: &mut Ui, storage: &Storage2<T>) {
        for (id, name) in storage.visible_elements() {
            let enabled = self.0.entry(id).or_default();
            ui.checkbox(enabled, name);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementsDescription<T: StorageElem2>(BTreeMap<T::IdWrapper, ValueToUser>);

impl<T: StorageElem2> Default for ElementsDescription<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<T: StorageElem2> ElementsDescription<T> {
    pub fn get(&mut self, id: T::IdWrapper) -> &ValueToUser {
        self.0.entry(id).or_default()
    }

    pub fn egui(&mut self, ui: &mut Ui, storage: &Storage2<T>, filter: &mut AnimationFilter<T>) {
        for (id, name) in storage.visible_elements() {
            let enabled = *filter.0.entry(id).or_default();
            if enabled {
                ui.group(|ui| {
                    ui.label(name);
                    self.0.entry(id).or_default().egui(ui);
                });
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElementsDescriptions {
    uniforms: ElementsDescription<AnyUniform>,
    matrices: ElementsDescription<Matrix>,
    cameras: ElementsDescription<Cam>,
}

impl ElementsDescriptions {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        uniforms: &Storage2<AnyUniform>,
        matrices: &Storage2<Matrix>,
        cameras: &Storage2<Cam>,
        filter: &mut AnimationFilters,
    ) {
        self.uniforms.egui(ui, uniforms, &mut filter.uniforms);
        ui.separator();
        self.matrices.egui(ui, matrices, &mut filter.matrices);
        ui.separator();
        self.cameras.egui(ui, cameras, &mut filter.cameras);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationFilters {
    uniforms: AnimationFilter<AnyUniform>,
    matrices: AnimationFilter<Matrix>,
    cameras: AnimationFilter<Cam>,
}

impl AnimationFilters {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        uniforms: &Storage2<AnyUniform>,
        matrices: &Storage2<Matrix>,
        cameras: &Storage2<Cam>,
    ) {
        self.uniforms.egui(ui, uniforms);
        ui.separator();
        self.matrices.egui(ui, matrices);
        ui.separator();
        self.cameras.egui(ui, cameras);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimationStage {
    pub name: EngRusText,
    pub description: Option<EngRusText>,
    pub uniforms: StageChanging<AnyUniform>,
    pub matrices: StageChanging<Matrix>,

    original_cam_button: bool,
    pub set_cam: Option<Option<CameraId>>,
    cams: BTreeMap<CameraId, bool>,

    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalUserUniforms {
    pub uniforms: GlobalStage<AnyUniform>,
    pub matrices: GlobalStage<Matrix>,
}

impl<T: StorageElem2> ComboBoxChoosable for Animation<T> {
    fn variants() -> &'static [&'static str] {
        &["To user", "From dev", "Changed", "Changed + To user"]
    }

    fn get_number(&self) -> usize {
        use Animation::*;
        match self {
            ProvidedToUser { .. } => 0,
            FromDev => 1,
            Changed { .. } => 2,
            ChangedAndToUser { .. } => 3,
        }
    }

    fn set_number(&mut self, number: usize) {
        use Animation::*;
        *self = match number {
            0 => ProvidedToUser,
            1 => FromDev,
            2 => Changed(None),
            3 => ChangedAndToUser(None),
            _ => unreachable!(),
        };
    }
}

impl GlobalUserUniforms {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        uniforms: &mut Storage2<AnyUniform>,
        matrices: &mut Storage2<Matrix>,
        animation_filters: &mut AnimationFilters,
    ) -> WhatChanged {
        let mut changed = false;
        changed |= self
            .uniforms
            .egui(ui, uniforms, &mut animation_filters.uniforms);
        ui.separator();
        changed |= self
            .matrices
            .egui(ui, matrices, &mut animation_filters.matrices);
        WhatChanged::from_uniform(changed)
    }

    pub fn user_egui(
        &mut self,
        ui: &mut Ui,
        uniforms: &mut Storage2<AnyUniform>,
        matrices: &mut Storage2<Matrix>,
        elements_descriptions: &mut ElementsDescriptions,
        id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        changed |= self.uniforms.user_egui(
            ui,
            uniforms,
            &mut elements_descriptions.uniforms,
            |elem, ui| elem.user_egui(ui, id.with("1")),
            false,
        );
        changed |= self.matrices.user_egui(
            ui,
            matrices,
            &mut elements_descriptions.matrices,
            |elem, ui| elem.user_egui(ui, id.with("2")),
            true,
        );
        changed
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AnimationId(UniqueId);

impl Wrapper for AnimationId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl<T: StorageElem2> Animation<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        name: &str,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        ui.horizontal(|ui| {
            changed.uniform |= egui_combo_box(
                ui,
                name,
                ANIMATION_STAGE_NAME_SIZE,
                self,
                data_id.with("combo"),
            );
            if let Animation::Changed(x) | Animation::ChangedAndToUser(x) = self {
                changed |= storage.inline("", 0.0, x, ui, input, data_id);
            }
        });

        changed
    }

    pub fn user_egui(
        &self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        user_egui: impl Fn(&mut T, &mut Ui) -> WhatChanged,
        names: &mut ElementsDescription<T>,
        id: T::IdWrapper,
        vertical: bool,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        use Animation::*;
        match self {
            ProvidedToUser | ChangedAndToUser(_) => drop(ui.horizontal(|ui| {
                let element = storage.get_original_mut(id).unwrap();
                let name = names.0.entry(id).or_default();
                if vertical {
                    ui.vertical(|ui| {
                        name.user_egui(ui);
                        changed |= user_egui(element, ui);
                    });
                } else {
                    ui.horizontal(|ui| {
                        name.user_egui(ui);
                        changed |= user_egui(element, ui);
                    });
                }
            })),
            FromDev => {}
            Changed(_) => {}
        }
        changed
    }
}

impl AnimationStage {
    pub fn user_egui(
        &self,
        ui: &mut Ui,
        input: &mut hlist![Storage2<AnyUniform>, FormulasCache],
        matrices: &mut Storage2<Matrix>,
        cameras: &mut Storage2<Cam>,
        elements_descriptions: &mut ElementsDescriptions,
        id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        if let Some(description) = &self.description {
            let text = description.text(ui);
            egui_demo_lib::easy_mark::easy_mark(ui, text);
            ui.separator();
        }

        if self.original_cam_button {
            let id = ui.memory_mut(|memory| {
                memory
                    .data
                    .get_persisted_mut_or_default::<CurrentCam>(egui::Id::new("CurrentCam"))
                    .0
            });
            let selected = id.is_none();
            if ui.radio(selected, "Original camera").clicked() && !selected {
                Cam::set_original_cam(ui);
                changed.uniform = true;
            }
        }

        for id in cameras
            .visible_elements()
            .filter_map(|(id, _)| Some((id, self.cams.get(&id)?)))
            .filter(|(_, enabled)| **enabled)
            .map(|(id, _)| id)
            .collect::<Vec<_>>()
        {
            if let Some(element) = cameras.get_original_mut(id) {
                ui.horizontal(|ui| {
                    changed |= element.user_egui(ui, &mut elements_descriptions.cameras, id);
                });
            }
        }

        ui.separator();

        changed |= self.uniforms.user_egui(
            ui,
            &mut input.0,
            &mut elements_descriptions.uniforms,
            |elem, ui| elem.user_egui(ui, id.with("1")),
            false,
        );
        ui.separator();
        changed |= self.matrices.user_egui(
            ui,
            matrices,
            &mut elements_descriptions.matrices,
            |elem, ui| {
                let i = ui.memory_mut(|memory| {
                    memory
                        .data
                        .get_persisted::<usize>(egui::Id::new("AnimationStage i"))
                });
                elem.user_egui(ui, id.with(i))
            },
            true,
        );
        changed
    }
}

impl StorageElem2 for AnimationStage {
    type IdWrapper = AnimationId;
    type GetType = AnimationStage;

    const SAFE_TO_RENAME: bool = true;

    type Input = hlist![
        Storage2<Cam>,
        AnimationFilters,
        GlobalUserUniforms,
        Storage2<Matrix>,
        Storage2<AnyUniform>,
        FormulasCache
    ];
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        (cams, (filters, (global, (matrices, input)))): &mut Self::Input,
        _: &mut InlineHelper<Self>,
        mut data_id: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        self.name.egui_singleline(ui);
        ui.separator();

        egui_bool_named(ui, &mut self.hidden, "Hidden");

        ui.separator();

        ui.checkbox(&mut self.original_cam_button, "Original cam button");

        egui_option(
            ui,
            &mut self.set_cam,
            "Set cam",
            || None,
            |ui, cam| {
                changed |= cams.inline("", 0.0, cam, ui, matrices, data_id.with("cam"));
                false
            },
        );

        for (id, name) in cams.visible_elements() {
            let enabled = *filters.cameras.0.entry(id).or_default();
            if enabled {
                let enabled = self.cams.entry(id).or_default();
                changed.uniform |=
                    check_changed(enabled, |enabled| drop(ui.checkbox(enabled, name)));
            }
        }

        ui.separator();

        egui_option(
            ui,
            &mut self.description,
            "Has description",
            Default::default,
            |ui, desc| {
                ui.horizontal(|ui| {
                    egui_label(ui, "Desc.:", 45.);
                    desc.egui_multiline(ui);
                });
                false
            },
        );
        ui.separator();
        changed |= self.matrices.egui(
            ui,
            matrices,
            input,
            &mut global.matrices,
            &mut filters.matrices,
            data_id.with("uniforms"),
        );
        data_id = data_id.with("matrices");
        ui.separator();
        let hpat![uniforms, formulas_cache] = input;
        changed |= self.uniforms.egui(
            ui,
            uniforms,
            formulas_cache,
            &mut global.uniforms,
            &mut filters.uniforms,
            data_id.with("uniforms"),
        );
        changed
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(self.clone())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(
        &self,
        _: F,
        (_, (_, (_, (matrices, input)))): &mut Self::Input,
    ) {
        self.matrices.remove(matrices, input);
        let hpat![uniforms, formulas_cache] = input;
        self.uniforms.remove(uniforms, formulas_cache);
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        (_, (_, (_, (matrices, input)))): &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        self.matrices.errors_count(matrices, input) + {
            let hpat![uniforms, formulas_cache] = input;
            self.uniforms.errors_count(uniforms, formulas_cache)
        }
    }

    fn duplicate_inline<F>(
        &self,
        _map_self: &mut F,
        (_cams, (_filters, (_global, (matrices, input)))): &mut Self::Input,
    ) -> Self
    where
        F: FnMut(Self::IdWrapper, &mut Self::Input) -> Self::IdWrapper,
    {
        let mut new = self.clone();
        use crate::gui::unique_id::UniqueId;
        use std::collections::BTreeMap;

        // Duplicate inline matrices used in stage changes
        {
            let mut m_visited: BTreeMap<UniqueId, UniqueId> = BTreeMap::new();
            for part in new.matrices.0.values_mut() {
                use crate::gui::animation::Animation::*;
                match part {
                    Changed(ref mut opt) | ChangedAndToUser(ref mut opt) => {
                        if let Some(id) = *opt {
                            let nid =
                                matrices.duplicate_as_field_with_visited(id, input, &mut m_visited);
                            *opt = Some(nid);
                        }
                    }
                    ProvidedToUser | FromDev => {}
                }
            }
        }

        // Duplicate inline uniforms used in stage changes
        {
            let hpat![uniforms, formulas_cache] = input;
            let mut u_visited: BTreeMap<UniqueId, UniqueId> = BTreeMap::new();
            for part in new.uniforms.0.values_mut() {
                use crate::gui::animation::Animation::*;
                match part {
                    Changed(ref mut opt) | ChangedAndToUser(ref mut opt) => {
                        if let Some(id) = *opt {
                            let nid = uniforms.duplicate_as_field_with_visited(
                                id,
                                formulas_cache,
                                &mut u_visited,
                            );
                            *opt = Some(nid);
                        }
                    }
                    ProvidedToUser | FromDev => {}
                }
            }
        }

        new
    }
}

impl AnyUniform {
    pub fn user_egui(&mut self, ui: &mut Ui, data_id: egui::Id) -> WhatChanged {
        use AnyUniform::*;
        let mut result = WhatChanged::default();
        match self {
            Bool(x) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_bool(ui, x))),
            Int(value) => {
                ui.centered_and_justified(|ui| {
                    result |= value.user_egui(ui, 1.0, 0..=0);
                });
            }
            Angle(a) => {
                drop(ui.centered_and_justified(|ui| result.uniform |= egui_angle_f64(ui, a)))
            }
            Progress(a) => drop(ui.centered_and_justified(|ui| result.uniform |= egui_0_1(ui, a))),
            Float(value) => {
                ui.centered_and_justified(|ui| {
                    result |= value.user_egui(ui, 0.01, 0..=2);
                });
            }
            Formula(_) | FormulaInt(_) => {
                drop(ui.label("Internal error, formulas are not allowed to be accessed by user."))
            }
            TrefoilSpecial(arr) => result |= arr.egui(ui, data_id),
        }
        result
    }
}

//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RealAnimationPart<T: StorageElem2> {
    CopyPrev,
    Changed(Option<T::IdWrapper>),
}

impl<T: StorageElem2> RealAnimationPart<T> {
    pub fn get_t(&self) -> Option<&T::IdWrapper> {
        if let RealAnimationPart::Changed(Some(t)) = self {
            Some(t)
        } else {
            None
        }
    }
}

#[allow(clippy::derivable_impls)]
impl<T: StorageElem2> Default for RealAnimationPart<T> {
    fn default() -> Self {
        RealAnimationPart::CopyPrev
    }
}

impl<T: StorageElem2> ComboBoxChoosable for RealAnimationPart<T> {
    fn variants() -> &'static [&'static str] {
        &["Copy prev", "Changed"]
    }

    fn get_number(&self) -> usize {
        use RealAnimationPart::*;
        match self {
            CopyPrev => 0,
            Changed { .. } => 1,
        }
    }

    fn set_number(&mut self, number: usize) {
        use RealAnimationPart::*;
        *self = match number {
            0 => CopyPrev,
            1 => Changed(None),
            _ => unreachable!(),
        };
    }
}

impl<T: StorageElem2> RealAnimationPart<T> {
    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        name: &str,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        ui.horizontal(|ui| {
            changed.uniform |= egui_combo_box(
                ui,
                name,
                ANIMATION_STAGE_NAME_SIZE,
                self,
                data_id.with("combo"),
            );
            if let RealAnimationPart::Changed(x) = self {
                changed |= storage.inline("", 0.0, x, ui, input, data_id);
            }
        });

        changed
    }
}

//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealAnimationStageChanging<T: StorageElem2>(
    BTreeMap<T::IdWrapper, RealAnimationPart<T>>,
);

impl<T: StorageElem2> Default for RealAnimationStageChanging<T> {
    fn default() -> Self {
        RealAnimationStageChanging(BTreeMap::new())
    }
}

impl<T: StorageElem2 + std::fmt::Debug> RealAnimationStageChanging<T> {
    pub fn remove(&self, storage: &mut Storage2<T>, input: &mut T::Input) {
        for id in self.0.values().filter_map(|x| x.get_t()) {
            storage.remove_as_field(*id, input);
        }
    }

    pub fn errors_count(&self, storage: &Storage2<T>, input: &T::Input) -> usize {
        self.0
            .values()
            .map(|x| {
                x.get_t()
                    .map(|id| storage.errors_inline(*id, input))
                    .unwrap_or(0)
            })
            .sum::<usize>()
    }

    pub fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Storage2<T>,
        input: &mut T::Input,
        global: &mut GlobalStage<T>,
        filter: &mut AnimationFilter<T>,
        data_id: egui::Id,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();

        let visible_elements = storage
            .visible_elements()
            .map(|(id, name)| (id, name.to_owned()))
            .collect::<Vec<_>>();

        ui.label("Local:");
        for (id, name) in &visible_elements {
            let global = *global.0.entry(*id).or_default();
            let enabled = *filter.0.entry(*id).or_default();
            let anim = self.0.entry(*id).or_default();
            if !global && enabled {
                changed |= anim.egui(ui, storage, input, &name, data_id.with(id));
            }
        }

        ui.separator();

        ui.label("Global:");
        for (id, name) in &visible_elements {
            let global = *global.0.entry(*id).or_default();
            let enabled = *filter.0.entry(*id).or_default();
            let anim = self.0.entry(*id).or_default();
            if global && enabled {
                changed |= anim.egui(ui, storage, input, &name, data_id.with(id));
            }
        }

        changed
    }

    pub fn init_stage(&self, storage: &mut Storage2<T>) {
        for (id, uniform) in self.0.iter() {
            if let Some(new_id) = uniform.get_t() {
                storage.set_id(*id, *new_id);
            }
        }
    }
}

//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RealAnimationId(UniqueId);

impl Wrapper for RealAnimationId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealAnimation {
    #[serde(default)]
    pub duration: f64,

    pub animation_stage: CurrentStage,

    pub uniforms: RealAnimationStageChanging<AnyUniform>,
    pub matrices: RealAnimationStageChanging<Matrix>,

    #[serde(default)]
    pub use_prev_cam: bool,
    #[serde(default)]
    pub use_start_cam_as_end: bool,
    pub cam_start: Option<CameraId>,
    pub cam_end: Option<CameraId>,

    #[serde(default)]
    pub use_any_cam_as_start: Option<bool>,
    #[serde(default)]
    pub use_any_cam_as_end: Option<bool>,
    #[serde(default)]
    pub cam_any_start: Option<RealAnimationId>,
    #[serde(default)]
    pub cam_any_end: Option<RealAnimationId>,

    #[serde(default)]
    pub cam_easing: Easing,
}

impl Default for RealAnimation {
    fn default() -> Self {
        Self {
            duration: 1.0,
            animation_stage: CurrentStage::Dev,
            uniforms: Default::default(),
            matrices: Default::default(),
            use_prev_cam: false,
            use_start_cam_as_end: false,
            cam_start: None,
            cam_end: None,
            cam_easing: Easing::Linear,
            use_any_cam_as_start: None,
            use_any_cam_as_end: None,
            cam_any_start: None,
            cam_any_end: None,
        }
    }
}

fn get_real_animation_name(
    id: Option<RealAnimationId>,
    real_animations: &Vec<(RealAnimationId, String)>,
) -> String {
    if let Some(id) = id {
        for (id2, name) in real_animations {
            if id == *id2 {
                return name.clone();
            }
        }
        "???".to_owned()
    } else {
        "None".to_owned()
    }
}

fn get_stage_name(
    current_stage: CurrentStage,
    animation_stages: &Vec<(AnimationId, String)>,
    real_animations: &Vec<(RealAnimationId, String)>,
) -> String {
    match current_stage {
        CurrentStage::Dev => "dev".to_owned(),
        CurrentStage::Animation(id) => {
            for (id2, name) in animation_stages {
                if id == *id2 {
                    return name.clone();
                }
            }
            "??? animation".to_owned()
        }
        CurrentStage::RealAnimation(id) => {
            for (id2, name) in real_animations {
                if id == *id2 {
                    return name.clone();
                }
            }
            "??? real animation".to_owned()
        }
    }
}

impl StorageElem2 for RealAnimation {
    type IdWrapper = RealAnimationId;
    type GetType = RealAnimation;

    const SAFE_TO_RENAME: bool = true;

    type Input = hlist![
        Vec<(AnimationId, String)>,
        Vec<(RealAnimationId, String)>,
        Storage2<Cam>,
        AnimationFilters,
        GlobalUserUniforms,
        Storage2<Matrix>,
        Storage2<AnyUniform>,
        FormulasCache
    ];
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        (animation_stages, (real_animations, (cams, (filters, (global, (matrices, input)))))): &mut Self::Input,
        _: &mut InlineHelper<Self>,
        mut data_id: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();

        ui.horizontal(|ui| {
            ui.label("Duration: ");
            changed.uniform |= egui_f64_positive(ui, &mut self.duration);
        });

        changed.uniform |= egui_combo_box(
            ui,
            "Camera easing:",
            100.,
            &mut self.cam_easing,
            data_id.with("cam_easing"),
        );

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("From stage: ");
            egui::ComboBox::new(data_id.with("from stage"), "")
                .selected_text(get_stage_name(
                    self.animation_stage,
                    &*animation_stages,
                    &*real_animations,
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.animation_stage, CurrentStage::Dev, "dev");
                    ui.separator();
                    for (id, name) in &*animation_stages {
                        changed.uniform |= check_changed(&mut self.animation_stage, |enabled| {
                            ui.selectable_value(
                                enabled,
                                CurrentStage::Animation(*id),
                                name.clone(),
                            );
                        });
                    }
                    ui.separator();
                    for (id, name) in &*real_animations {
                        changed.uniform |= check_changed(&mut self.animation_stage, |enabled| {
                            ui.selectable_value(
                                enabled,
                                CurrentStage::RealAnimation(*id),
                                name.clone(),
                            );
                        });
                    }
                });
        });

        ui.separator();

        if self.use_any_cam_as_start.is_some() {
            self.use_prev_cam = false;
        }
        ui.add_enabled_ui(!self.use_any_cam_as_start.is_some(), |ui| {
            changed.uniform |= egui_bool_named(ui, &mut self.use_prev_cam, "Use prev end cam");
        });

        if self.use_any_cam_as_end.is_some() {
            self.use_start_cam_as_end = false;
        }
        ui.add_enabled_ui(!self.use_any_cam_as_end.is_some(), |ui| {
            changed.uniform |= egui_bool_named(
                ui,
                &mut self.use_start_cam_as_end,
                "Use start cam as end cam",
            );
        });

        if self.use_prev_cam {
            self.use_any_cam_as_start = None;
        }
        ui.add_enabled_ui(!self.use_prev_cam, |ui| {
            ui.horizontal(|ui| {
                changed.uniform |= egui_option(
                    ui,
                    &mut self.use_any_cam_as_start,
                    "Use any cam as start",
                    || false,
                    |ui, t| {
                        ui.separator();
                        let mut changed = false;
                        changed |= ui.selectable_value(t, false, "Start").changed();
                        changed |= ui.selectable_value(t, true, "End").changed();
                        changed
                    },
                );
                if self.use_any_cam_as_start.is_some() {
                    ui.separator();
                    egui::ComboBox::new(data_id.with("combo1"), "")
                        .selected_text(get_real_animation_name(
                            self.cam_any_start,
                            &*real_animations,
                        ))
                        .show_ui(ui, |ui| {
                            for (id, name) in &*real_animations {
                                changed.uniform |=
                                    check_changed(&mut self.cam_any_start, |value| {
                                        ui.selectable_value(value, Some(*id), name.clone());
                                    });
                            }
                        });
                } else {
                    self.cam_any_start = None;
                }
            });
        });

        if self.use_start_cam_as_end {
            self.use_any_cam_as_end = None;
        }
        ui.add_enabled_ui(!self.use_start_cam_as_end, |ui| {
            ui.horizontal(|ui| {
                changed.uniform |= egui_option(
                    ui,
                    &mut self.use_any_cam_as_end,
                    "Use any cam as end",
                    || false,
                    |ui, t| {
                        ui.separator();
                        let mut changed = false;
                        changed |= ui.selectable_value(t, false, "Start").changed();
                        changed |= ui.selectable_value(t, true, "End").changed();
                        changed
                    },
                );
                if self.use_any_cam_as_end.is_some() {
                    ui.separator();
                    egui::ComboBox::new(data_id.with("combo2"), "")
                        .selected_text(get_real_animation_name(self.cam_any_end, &*real_animations))
                        .show_ui(ui, |ui| {
                            for (id, name) in &*real_animations {
                                changed.uniform |= check_changed(&mut self.cam_any_end, |value| {
                                    ui.selectable_value(value, Some(*id), name.clone());
                                });
                            }
                        });
                } else {
                    self.cam_any_end = None;
                }
            });
        });
        if !self.use_prev_cam && !self.use_any_cam_as_start.is_some() {
            changed |= cams.inline(
                "Start cam:",
                65.0,
                &mut self.cam_start,
                ui,
                matrices,
                data_id.with("cam_start"),
            );
        } else {
            self.cam_start = None;
        }
        if !self.use_start_cam_as_end && !self.use_any_cam_as_end.is_some() {
            changed |= cams.inline(
                "End cam:",
                65.0,
                &mut self.cam_end,
                ui,
                matrices,
                data_id.with("cam_end"),
            );
        } else {
            self.cam_end = None;
        }

        ui.separator();

        ui.separator();
        changed |= self.matrices.egui(
            ui,
            matrices,
            input,
            &mut global.matrices,
            &mut filters.matrices,
            data_id.with("uniforms"),
        );
        data_id = data_id.with("matrices");
        ui.separator();
        let hpat![uniforms, formulas_cache] = input;
        changed |= self.uniforms.egui(
            ui,
            uniforms,
            formulas_cache,
            &mut global.uniforms,
            &mut filters.uniforms,
            data_id.with("uniforms"),
        );

        changed
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(self.clone())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(
        &self,
        _: F,
        (_, (_, (_, (_, (_, (matrices, input)))))): &mut Self::Input,
    ) {
        self.matrices.remove(matrices, input);
        let hpat![uniforms, formulas_cache] = input;
        self.uniforms.remove(uniforms, formulas_cache);
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        (_, (_, (_, (_, (_, (matrices, input)))))): &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        self.matrices.errors_count(matrices, input) + {
            let hpat![uniforms, formulas_cache] = input;
            self.uniforms.errors_count(uniforms, formulas_cache)
        }
    }

    fn duplicate_inline<F>(
        &self,
        _map_self: &mut F,
        (_animation_stages, (_real_animations, (_cams, (_filters, (_global, (matrices, input)))))): &mut Self::Input,
    ) -> Self
    where
        F: FnMut(Self::IdWrapper, &mut Self::Input) -> Self::IdWrapper,
    {
        let mut new = self.clone();
        use crate::gui::unique_id::UniqueId;
        use std::collections::BTreeMap;

        // Duplicate inline matrices used in real animation changes
        {
            let mut m_visited: BTreeMap<UniqueId, UniqueId> = BTreeMap::new();
            for part in new.matrices.0.values_mut() {
                use crate::gui::animation::RealAnimationPart::*;
                match part {
                    Changed(ref mut opt) => {
                        if let Some(id) = *opt {
                            let nid =
                                matrices.duplicate_as_field_with_visited(id, input, &mut m_visited);
                            *opt = Some(nid);
                        }
                    }
                    CopyPrev => {}
                }
            }
        }

        // Duplicate inline uniforms used in real animation changes
        {
            let hpat![uniforms, formulas_cache] = input;
            let mut u_visited: BTreeMap<UniqueId, UniqueId> = BTreeMap::new();
            for part in new.uniforms.0.values_mut() {
                use crate::gui::animation::RealAnimationPart::*;
                match part {
                    Changed(ref mut opt) => {
                        if let Some(id) = *opt {
                            let nid = uniforms.duplicate_as_field_with_visited(
                                id,
                                formulas_cache,
                                &mut u_visited,
                            );
                            *opt = Some(nid);
                        }
                    }
                    CopyPrev => {}
                }
            }
        }

        new
    }
}
