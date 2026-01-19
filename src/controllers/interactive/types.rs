//! Application-layer types for render requests.
//!
//! These types form the API contract between the GUI input adapter
//! and the interactive controller.

use crate::core::data::fractal::Fractal;
use crate::core::data::fractal_params::FractalParams;
use crate::core::data::pixel_rect::PixelRect;

/// A render request representing a snapshot of parameters for a single render job.
///
/// Immutable by design - represents the exact parameters for one render operation.
/// `PartialEq` enables change detection to skip redundant renders.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderRequest {
    /// Target render dimensions in pixels.
    pub pixel_rect: PixelRect,
    /// Which fractal algorithm to use.
    pub fractal: Fractal,
    /// Algorithm-specific parameters.
    pub params: FractalParams,
    /// Colour mapping to apply to iteration counts.
    pub colour_scheme: ColourSchemeKind,
}

/// Selects which colour mapping to apply to iteration counts.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ColourSchemeKind {
    BlueWhiteGradient,
    // Future: Grayscale, Rainbow, Custom, etc.
}
