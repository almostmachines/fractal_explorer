fn main() {
    let presenter_factory = fractal_explorer::PixelsPresenterFactory::new();
    let command = fractal_explorer::RunGuiCommand::new(presenter_factory);

    command.execute();
}
