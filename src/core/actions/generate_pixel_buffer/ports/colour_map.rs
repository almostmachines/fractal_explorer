use crate::core::data::colour::Colour;

/// Error type for colour map operations. Uses `Send + Sync` bounds so that
/// colour-map failures can safely propagate from rayon worker threads.
pub type ColourMapError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub trait ColourMap<T>: Send + Sync {
    fn map(&self, value: T) -> Result<Colour, ColourMapError>;
    #[allow(dead_code)]
    fn display_name(&self) -> &str;
}
