fn main() -> Result<(), Box<dyn std::error::Error>> {
    let presenter = fractal_explorer::PpmFilePresenter::new();
    let mut controller = fractal_explorer::CliTestController::new(presenter);

    controller.generate()?;
    controller.write("output/mandelbrot.ppm")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_returns_ok() {
        let result = main();

        assert!(result.is_ok());
    }
}
