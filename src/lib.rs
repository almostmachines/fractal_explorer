mod presenters;
mod controllers;
mod core;
#[cfg(feature = "gui")]
mod input;

pub use controllers::cli::test::cli_test::CliTestController;
pub use presenters::file::ppm::PpmFilePresenter;
#[cfg(feature = "gui")]
pub use input::gui::commands::run_gui::RunGuiCommand;
#[cfg(feature = "gui")]
pub use presenters::pixels::factory::PixelsPresenterFactory;
