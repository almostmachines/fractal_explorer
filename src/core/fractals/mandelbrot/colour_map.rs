use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;
use crate::core::data::colour::Colour;
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MandelbrotColourMapKind {
    BlueWhiteGradient,
    FireGradient,
}

impl MandelbrotColourMapKind {
    /// All supported Mandelbrot colour maps, in deterministic order (default first).
    ///
    /// Invariant: this list must contain every enum variant exactly once.
    pub const ALL: &'static [Self] = &[Self::FireGradient, Self::BlueWhiteGradient];

    /// Canonical human-readable label (single source of truth for UI/logging).
    ///
    /// Concrete implementations should implement `ColourMap::display_name()` by delegating to
    /// `self.kind().display_name()` to prevent string drift.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::FireGradient => "Fire gradient",
            Self::BlueWhiteGradient => "Blue-white gradient",
        }
    }
}

impl Default for MandelbrotColourMapKind {
    fn default() -> Self {
        Self::FireGradient
    }
}

impl std::fmt::Display for MandelbrotColourMapKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).display_name())
    }
}

pub trait MandelbrotColourMap: ColourMap<u32> + Send + Sync {
    fn kind(&self) -> MandelbrotColourMapKind;
}

impl ColourMap<u32> for Box<dyn MandelbrotColourMap> {
    fn map(&self, value: u32) -> Result<Colour, Box<dyn Error>> {
        (**self).map(value)
    }

    fn display_name(&self) -> &str {
        (**self).display_name()
    }
}
