#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MandelbrotColourMapKinds {
    BlueWhiteGradient,
    FireGradient,
}

impl MandelbrotColourMapKinds {
    pub const ALL: &'static [Self] = &[Self::FireGradient, Self::BlueWhiteGradient];

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::FireGradient => "Fire gradient",
            Self::BlueWhiteGradient => "Blue-white gradient",
        }
    }
}

impl Default for MandelbrotColourMapKinds {
    fn default() -> Self {
        Self::FireGradient
    }
}

impl std::fmt::Display for MandelbrotColourMapKinds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).display_name())
    }
}
