use crate::gui::common::*;
use crate::gui::storage::*;
use crate::gui::uniform::*;

use egui::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCode(pub GlslCode);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlslCode(pub String);

impl StorageElem for LibraryCode {
    type GetType = LibraryCode;
    type Input = ShaderErrors;

    fn get<F: FnMut(&str) -> GetEnum<Self::GetType>>(
        &self,
        _: F,
        _: &StorageWithNames<AnyUniformComboBox>,
        _: &FormulasCache,
    ) -> GetEnum<Self::GetType> {
        GetEnum::Ok(self.clone())
    }

    fn egui(
        &mut self,
        ui: &mut Ui,
        pos: usize,
        input: &mut Self::Input,
        _: &[String],
    ) -> WhatChanged {
        let mut changed = WhatChanged::default();
        egui_with_red_field(ui, input.get_errors(self, pos).is_some(), |ui| {
            changed = WhatChanged::from_shader(
                ui.add(TextEdit::multiline(&mut self.0.0).text_style(TextStyle::Monospace))
                    .changed(),
            );
            if let Some(local_errors) = input.get_errors(self, pos) {
                egui_errors(ui, local_errors);
            }
        });
        changed
    }

    fn errors_count(&self, pos: usize, input: &Self::Input, _: &[String]) -> usize {
        if let Some(local_errors) = input.get_errors(self, pos) {
            local_errors.len()
        } else {
            0
        }
    }
}

impl Default for MaterialCode {
    fn default() -> Self {
        MaterialCode(GlslCode(
            "return material_simple(hit, r, vec3(9.21e-2, 7.28e-1, 6.81e-2), 5e-1, true, 4e0, 3e-1);".to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryCode(pub GlslCode);

impl GlslCode {
    pub fn egui(&mut self, ui: &mut Ui) -> WhatChanged {
        WhatChanged::from_shader(
            ui.add(TextEdit::multiline(&mut self.0).text_style(TextStyle::Monospace))
                .changed(),
        )
    }
}

// Code must return integer - material. NOT_INSIDE if not inside. TELEPORT is should be teleported by current matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsInsideCode(pub GlslCode);

impl Default for IsInsideCode {
    fn default() -> Self {
        IsInsideCode(GlslCode(
            "if (x*x + y*y < 1.) {\n  return black_M;\n} else {\n  return NOT_INSIDE;\n}"
                .to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntersectCode(pub GlslCode);

impl Default for IntersectCode {
    fn default() -> Self {
        Self(GlslCode(
            r#"vec3 op = -r.o.xyz;
float b = dot(op, r.d.xyz);
float det = b*b - dot(op, op) + 1.0;
if (det < 0.) return scene_intersection_none;

det = sqrt(det);
float t = b - det;
if (t < 0.) t = b + det;
if (t < 0.) return scene_intersection_none;

vec4 pos = r.o + r.d * t;
vec3 n = normalize(pos.xyz);

float u = atan(pos.z, pos.x);
float v = atan(sqrt(pos.x * pos.x + pos.z * pos.z), pos.y);

return SceneIntersection(black_M, SurfaceIntersection(true, t, u, v, n));"#
                .to_owned(),
        ))
    }
}
