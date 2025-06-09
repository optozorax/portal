use crate::gui::combo_box::ComboBoxChoosable;
use serde::Deserialize;
use serde::Serialize;
use std::f64::consts::PI;

pub fn easing_linear(t: f64) -> f64 {
    t
}

pub fn easing_in(t: f64) -> f64 {
    1.0 - (t * PI * 0.5).cos()
}

pub fn easing_out(t: f64) -> f64 {
    1.0 - easing_in(1.0 - t)
}

pub fn easing_in_out(t: f64) -> f64 {
    (1.0 - (t * PI).cos()) * 0.5
}

pub fn easing_in_out_fast(t: f64) -> f64 {
    easing_in_out(easing_in_out(t))
}

pub fn easing_plus_minus(mut t: f64) -> f64 {
    // https://www.desmos.com/calculator/1ti1uakaov
    t *= 2. * PI;
    let t2 = 2. * t;
    t.sin() * (3. - t.cos() - t2.cos() - t.cos() * t2.cos()) / 4.
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum Easing {
    #[default]
    Linear,
    In,
    Out,
    InOut,
    InOutFast,
}

impl ComboBoxChoosable for Easing {
    fn variants() -> &'static [&'static str] {
        &["Linear", "In", "Out", "InOut", "InOutFast"]
    }

    fn get_number(&self) -> usize {
        use Easing::*;
        match self {
            Linear => 0,
            In => 1,
            Out => 2,
            InOut => 3,
            InOutFast => 4,
        }
    }

    fn set_number(&mut self, number: usize) {
        use Easing::*;
        *self = match number {
            0 => Linear,
            1 => In,
            2 => Out,
            3 => InOut,
            4 => InOutFast,
            _ => unreachable!(),
        };
    }
}

impl Easing {
    pub fn ease(&self, t: f64) -> f64 {
        use Easing::*;
        match self {
            Linear => easing_linear(t),
            In => easing_in(t),
            Out => easing_out(t),
            InOut => easing_in_out(t),
            InOutFast => easing_in_out_fast(t),
        }
    }
}
