mod presenters;
mod controllers;
mod core;
#[cfg(feature = "gui")]
mod input;
mod storage;

pub use controllers::mandelbrot::mandelbrot_controller;

#[cfg(feature = "gui")]
pub use input::gui::command::gui_command;
pub use presenters::pixels::factory;
