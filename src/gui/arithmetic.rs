use crate::gui::combo_box::*;
use crate::gui::common::*;
use crate::gui::storage2::*;
use crate::gui::unique_id::*;
use egui::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Arithmetic {
    Float(f32),
    Sum(Option<ArithmeticId>, Option<ArithmeticId>),
    Mul(Option<ArithmeticId>, Option<ArithmeticId>),
}

impl ComboBoxChoosable for Arithmetic {
    fn variants() -> &'static [&'static str] {
        &["Float", "Sum", "Mul"]
    }
    fn get_number(&self) -> usize {
        use Arithmetic::*;
        match self {
            Float { .. } => 0,
            Sum { .. } => 1,
            Mul { .. } => 2,
        }
    }
    fn set_number(&mut self, number: usize) {
        use Arithmetic::*;
        *self = match number {
            0 => Float(0.0),
            1 => Sum(None, None),
            2 => Mul(None, None),
            _ => unreachable!(),
        };
    }
}

impl Default for Arithmetic {
    fn default() -> Self {
        Arithmetic::Float(0.0)
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ArithmeticId(UniqueId);

impl Wrapper<UniqueId> for ArithmeticId {
    fn wrap(id: UniqueId) -> Self {
        ArithmeticId(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for Arithmetic {
    type IdWrapper = ArithmeticId;
    type GetType = f32;

    const SAFE_TO_RENAME: bool = true;

    type Input = ();

    fn egui(
        &mut self,
        ui: &mut Ui,
        (): &mut Self::Input,
        self_storage: &mut Storage2<Self>,
        data_id: egui::Id,
    ) -> WhatChanged {
        use Arithmetic::*;

        egui_combo_label(ui, "Type:", 45., self);

        match self {
            Float(f) => WhatChanged::from_uniform(egui_f32(ui, f)),
            Sum(a, b) => {
                let mut result = WhatChanged::default();

                ui.label("Sum first argument:");
                result |= self_storage.inline(&mut *a, ui, &mut (), data_id.with(0));

                ui.label("Sum second argument:");
                result |= self_storage.inline(&mut *b, ui, &mut (), data_id.with(1));

                result
            }
            Mul(a, b) => {
                let mut result = WhatChanged::default();

                ui.label("Mul first argument:");
                result |= self_storage.inline(&mut *a, ui, &mut (), data_id.with(0));

                ui.label("Mul second argument:");
                result |= self_storage.inline(&mut *b, ui, &mut (), data_id.with(1));

                result
            }
        }
    }

    fn get<F: FnMut(Self::IdWrapper) -> Option<Self::GetType>>(
        &self,
        mut f: F,
        (): &Self::Input,
    ) -> Option<Self::GetType> {
        use Arithmetic::*;
        match self {
            Float(f) => Some(*f),
            Sum(a, b) => Some(f((*a)?)? + f((*b)?)?),
            Mul(a, b) => Some(f((*a)?)? * f((*b)?)?),
        }
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(
        &self,
        mut f: F,
        input: &mut Self::Input,
    ) {
        use Arithmetic::*;
        match self {
            Float(_) => {}
            Sum(a, b) => {
                if let Some(a) = a {
                    f(*a, input);
                }
                if let Some(b) = b {
                    f(*b, input);
                }
            }
            Mul(a, b) => {
                if let Some(a) = a {
                    f(*a, input);
                }
                if let Some(b) = b {
                    f(*b, input);
                }
            }
        }
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(
        &self,
        mut f: F,
        (): &Self::Input,
    ) -> usize {
        use Arithmetic::*;
        match self {
            Float(_) => 0,
            Sum(a, b) => a.map(|a| f(a)).unwrap_or(1) + b.map(|b| f(b)).unwrap_or(1),
            Mul(a, b) => a.map(|a| f(a)).unwrap_or(1) + b.map(|b| f(b)).unwrap_or(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MoreArithmetic {
    Sin(Option<ArithmeticId>),
    Cos(Option<ArithmeticId>),
}

impl ComboBoxChoosable for MoreArithmetic {
    fn variants() -> &'static [&'static str] {
        &["Sin", "Cos"]
    }
    fn get_number(&self) -> usize {
        use MoreArithmetic::*;
        match self {
            Sin { .. } => 0,
            Cos { .. } => 1,
        }
    }
    fn set_number(&mut self, number: usize) {
        use MoreArithmetic::*;
        *self = match number {
            0 => Sin(None),
            1 => Cos(None),
            _ => unreachable!(),
        };
    }
}

impl Default for MoreArithmetic {
    fn default() -> Self {
        MoreArithmetic::Sin(None)
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MoreArithmeticId(UniqueId);

impl Wrapper<UniqueId> for MoreArithmeticId {
    fn wrap(id: UniqueId) -> Self {
        Self(id)
    }
    fn un_wrap(self) -> UniqueId {
        self.0
    }
}

impl StorageElem2 for MoreArithmetic {
    type IdWrapper = MoreArithmeticId;
    type GetType = f32;

    const SAFE_TO_RENAME: bool = true;

    type Input = Storage2<Arithmetic>;

    fn egui(
        &mut self,
        ui: &mut Ui,
        storage: &mut Self::Input,
        _: &mut Storage2<Self>,
        data_id: egui::Id,
    ) -> WhatChanged {
        use MoreArithmetic::*;

        egui_combo_label(ui, "Type:", 45., self);

        match self {
            Sin(a) => {
                let mut result = WhatChanged::default();

                ui.label("Arguments:");
                result |= storage.inline(&mut *a, ui, &mut (), data_id.with(0));

                result
            }
            Cos(a) => {
                let mut result = WhatChanged::default();

                ui.label("Arguments:");
                result |= storage.inline(&mut *a, ui, &mut (), data_id.with(0));

                result
            }
        }
    }

    fn get<F: FnMut(Self::IdWrapper) -> Option<Self::GetType>>(
        &self,
        _: F,
        storage: &Self::Input,
    ) -> Option<Self::GetType> {
        use MoreArithmetic::*;
        match self {
            Sin(a) => Some((storage.get((*a)?, &())?).sin()),
            Cos(a) => Some((storage.get((*a)?, &())?).cos()),
        }
    }

    fn remove<F: FnMut(Self::IdWrapper, &mut Self::Input)>(&self, _: F, input: &mut Self::Input) {
        use MoreArithmetic::*;
        match self {
            Sin(a) => {
                if let Some(a) = a {
                    input.remove_as_field(*a, &mut ());
                }
            }
            Cos(a) => {
                if let Some(a) = a {
                    input.remove_as_field(*a, &mut ());
                }
            }
        }
    }

    fn errors_count<F: FnMut(Self::IdWrapper) -> usize>(&self, _: F, _: &Self::Input) -> usize {
        use MoreArithmetic::*;
        match self {
            Sin(a) => a.is_none() as usize,
            Cos(a) => a.is_none() as usize,
        }
    }
}
