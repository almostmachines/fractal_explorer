//! Application-layer types for render requests.
//!
//! These types form the API contract between the GUI input adapter
//! and the interactive controller.

use crate::core::data::complex_rect::ComplexRect;
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
    pub fractal: FractalKind,
    /// Algorithm-specific parameters.
    pub params: FractalParams,
    /// Colour mapping to apply to iteration counts.
    pub colour_scheme: ColourSchemeKind,
}

/// Selects which fractal algorithm to render.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FractalKind {
    Mandelbrot,
    // Future: Julia, BurningShip, etc.
}

/// Algorithm-specific parameters for fractal computation.
///
/// Each variant contains the fields needed by that specific algorithm.
#[derive(Debug, Clone, PartialEq)]
pub enum FractalParams {
    Mandelbrot {
        /// View region in the complex plane.
        region: ComplexRect,
        /// Maximum iterations before considering a point bounded.
        max_iterations: u32,
    },
    // Future variants for other fractal types...
}

/// Selects which colour mapping to apply to iteration counts.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ColourSchemeKind {
    BlueWhiteGradient,
    // Future: Grayscale, Rainbow, Custom, etc.
}
