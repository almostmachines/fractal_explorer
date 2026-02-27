#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum JuliaColourMapKinds {
    BlueWhiteGradient,
    #[default]
    FireGradient,
}

impl JuliaColourMapKinds {
    pub const ALL: &'static [Self] = &[Self::FireGradient, Self::BlueWhiteGradient];

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::FireGradient => "Fire gradient",
            Self::BlueWhiteGradient => "Blue-white gradient",
        }
    }
}

impl std::fmt::Display for JuliaColourMapKinds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).display_name())
    }
}
