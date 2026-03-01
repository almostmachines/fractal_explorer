#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FractalKinds {
    Mandelbrot,
    #[default]
    Julia,
}

impl FractalKinds {
    pub const ALL: &'static [Self] = &[Self::Julia, Self::Mandelbrot];

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Mandelbrot => "Mandelbrot",
            Self::Julia => "Julia",
        }
    }
}
