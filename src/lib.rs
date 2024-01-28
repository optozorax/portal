#[macro_use]
pub mod with_swapped;

pub mod gui;

pub mod code_generation;

pub mod shader_error_parser;

#[macro_export]
macro_rules! error {
	(format, $format_string:literal, $($args:expr),*) => {
		macroquad::prelude::error!("Error on {}:{}. {}", file!(), line!(), format!($format_string, $($args),*))
	};

	(debug, $context:expr) => {
		macroquad::prelude::error!("Error on {}:{}, debug context: {:?}", file!(), line!(), $context)
	};

	() => {
		macroquad::prelude::error!("Error on {}:{}", file!(), line!())
	};
}
