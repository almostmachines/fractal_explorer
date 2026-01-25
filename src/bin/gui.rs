fn main() {
    let presenter_factory = fractal_explorer::factory::PixelsPresenterFactory {};
    let command = fractal_explorer::gui_command::GuiCommand::new(presenter_factory);

    command.run();
}
