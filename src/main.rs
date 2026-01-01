fn main() -> Result<(), Box<dyn std::error::Error>> {
    fractal_explorer::mandelbrot_controller()
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
