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

pub fn easing_elastic_out(x: f64) -> f64 {
    let c4 = (2.0 * std::f64::consts::PI) / 3.0;

    if x == 0.0 {
        0.0
    } else if x == 1.0 {
        1.0
    } else {
        (2.0_f64).powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum Easing {
    #[default]
    Linear,
    In,
    Out,
    InOut,
    InOutFast,
    ElasticOut,
}

impl ComboBoxChoosable for Easing {
    fn variants() -> &'static [&'static str] {
        &["Linear", "In", "Out", "InOut", "InOutFast", "ElasticOut"]
    }

    fn get_number(&self) -> usize {
        use Easing::*;
        match self {
            Linear => 0,
            In => 1,
            Out => 2,
            InOut => 3,
            InOutFast => 4,
            ElasticOut => 5,
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
            5 => ElasticOut,
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
            ElasticOut => easing_elastic_out(t),
        }
    }
}
