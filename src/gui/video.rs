use crate::gui::common::*;
use crate::gui::storage2::*;
use crate::gui::uniform::*;
use crate::gui::unique_id::UniqueId;

use egui::*;
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    /// Path to the source MP4 file.
    pub path: String,

    /// Uniform that controls frame position in [0, 1].
    #[serde(default)]
    pub uniform: Option<UniformId>,
}

impl Default for Video {
    fn default() -> Self {
        Self {
            path: String::new(),
            uniform: None,
        }
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct VideoId(UniqueId);

impl Wrapper for VideoId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl Video {
    #[cfg(not(target_arch = "wasm32"))]
    fn frames_dir(&self) -> Option<String> {
        let path = Path::new(&self.path);
        let stem = path.file_stem()?.to_string_lossy();
        Some(format!("video_png/{}", stem))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn frames_dir_exists(&self) -> bool {
        if let Some(dir) = self.frames_dir() {
            if let Ok(meta) = std::fs::metadata(dir) {
                meta.is_dir()
            } else {
                false
            }
        } else {
            false
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn generate_frames(&self) -> Result<(), String> {
        use std::process::Command;

        let path = self.path.clone();
        if path.is_empty() {
            return Err("Video path is empty".to_owned());
        }

        let frames_dir = self
            .frames_dir()
            .ok_or_else(|| "Cannot deduce frames directory from path".to_owned())?;

        if let Err(err) = std::fs::create_dir_all(&frames_dir) {
            return Err(format!(
                "Failed to create frames directory {}: {err}",
                frames_dir
            ));
        }

        let output_pattern = format!("{}/%05d.png", frames_dir);

        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(&path)
            .arg(&output_pattern)
            .status()
            .map_err(|e| format!("Failed to run ffmpeg: {e}"))?;

        if !status.success() {
            return Err(format!(
                "ffmpeg exited with status code {:?}",
                status.code()
            ));
        }

        Ok(())
    }
}

impl StorageElem2 for Video {
    type IdWrapper = VideoId;
    type GetType = ();

    const SAFE_TO_RENAME: bool = true;

    type Input = hlist![VideoErrors, Storage2<AnyUniform>, FormulasCache];
    type GetInput = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        input: &mut Self::Input,
        _: &mut InlineHelper<Self>,
        data_id: egui::Id,
        _: Self::IdWrapper,
    ) -> WhatChanged {
        let hpat!(video_errors, uniforms, formulas_cache) = input;

        let mut changed = WhatChanged::default();

        // Path field with optional error tooltip.
        let error_opt = video_errors.0.get(&self.path).cloned();
        changed.shader |= check_changed(&mut self.path, |text| {
            let has_error = error_opt.is_some();
            let response = egui_with_red_field(ui, has_error, |ui| ui.text_edit_singleline(text));
            if let Some(err) = &error_opt {
                response.on_hover_text(err);
            }
        });

        // Uniform chooser.
        ui.horizontal(|ui| {
            ui.label("Uniform:");
            let res = uniforms.inline(
                "",
                0.0,
                &mut self.uniform,
                ui,
                formulas_cache,
                data_id.with("uniform"),
            );
            changed |= res;
        });

        // Info about frame directory and ffmpeg helper.
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.path.is_empty() {
                ui.label("Set video path to enable frame generation.");
            } else if self.frames_dir_exists() {
                if let Some(dir) = self.frames_dir() {
                    ui.label(format!("Frames directory exists: {}", dir));
                }
                // Clear any previous ffmpeg error for this path.
                video_errors.0.remove(&self.path);
            } else {
                let mut button = ui.button("Generate frames with ffmpeg into video_png");
                if let Some(err) = video_errors.0.get(&self.path) {
                    button = button.on_hover_text(err);
                }
                if button.clicked() {
                    match self.generate_frames() {
                        Ok(()) => {
                            video_errors.0.remove(&self.path);
                        }
                        Err(e) => {
                            video_errors.0.insert(self.path.clone(), e);
                        }
                    }
                }
            }
        }

        changed
    }

    fn get(&self, _: &GetHelper<Self>, _: &Self::GetInput) -> Option<Self::GetType> {
        Some(())
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, _: F, _: &mut Self::Input) {}

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        _: F,
        video_errors: &Self::Input,
        _: Self::IdWrapper,
    ) -> usize {
        let hpat!(errors, _, _) = video_errors;
        errors.0.contains_key(&self.path) as usize
    }

    fn duplicate_inline<F>(&self, _map_self: &mut F, _input: &mut Self::Input) -> Self
    where
        F: FnMut(Self::IdWrapper, &mut Self::Input) -> Self::IdWrapper,
    {
        self.clone()
    }
}
