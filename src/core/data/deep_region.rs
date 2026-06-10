use crate::core::data::complex::Complex;
use crate::core::data::complex_rect::ComplexRect;
use crate::core::data::deep_complex::DeepComplex;
use std::error::Error;
use std::fmt;

/// Guard bits added on top of the zoom depth when sizing centre precision.
const PRECISION_GUARD_BITS: usize = 64;

/// Precision is rounded up to this block size so that smoothly zooming does
/// not change the centre representation every frame.
const PRECISION_BLOCK_BITS: usize = 64;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeepRegionError {
    InvalidSize { width: f64, height: f64 },
}

impl fmt::Display for DeepRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { width, height } => {
                write!(
                    f,
                    "deep region size must be positive and finite: {}x{}",
                    width, height
                )
            }
        }
    }
}

impl Error for DeepRegionError {}

/// A view rectangle in the complex plane held as an arbitrary-precision
/// centre plus f64 extents.
///
/// f64 comfortably represents extents down to ~1e-300, while the centre
/// coordinates need ever more precision as the zoom deepens — so the centre
/// is the only part that uses big floats.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepRegion {
    centre: DeepComplex,
    width: f64,
    height: f64,
}

impl DeepRegion {
    pub fn new(centre: DeepComplex, width: f64, height: f64) -> Result<Self, DeepRegionError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(DeepRegionError::InvalidSize { width, height });
        }

        Ok(Self {
            centre,
            width,
            height,
        })
    }

    /// Builds a deep region from an f64 rect, e.g. the default home view.
    #[must_use]
    pub fn from_complex_rect(rect: &ComplexRect) -> Self {
        let top_left = rect.top_left();
        let bottom_right = rect.bottom_right();
        let centre = DeepComplex::from_f64(
            (top_left.real + bottom_right.real) * 0.5,
            (top_left.imag + bottom_right.imag) * 0.5,
        )
        .expect("finite rect has a finite centre");

        Self {
            centre,
            width: rect.width(),
            height: rect.height(),
        }
        .normalised()
    }

    /// Approximates this region as an f64 rect. Returns `None` when the
    /// extents are so small that the bounds collapse in f64 — callers must
    /// switch to perturbation rendering long before that point.
    #[must_use]
    pub fn to_complex_rect(&self) -> Option<ComplexRect> {
        let (centre_re, centre_im) = self.centre.to_f64();
        let half_width = self.width * 0.5;
        let half_height = self.height * 0.5;

        ComplexRect::new(
            Complex {
                real: centre_re - half_width,
                imag: centre_im - half_height,
            },
            Complex {
                real: centre_re + half_width,
                imag: centre_im + half_height,
            },
        )
        .ok()
    }

    #[must_use]
    pub fn centre(&self) -> &DeepComplex {
        &self.centre
    }

    #[must_use]
    pub fn width(&self) -> f64 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> f64 {
        self.height
    }

    #[must_use]
    pub fn min_extent(&self) -> f64 {
        self.width.min(self.height)
    }

    #[must_use]
    pub fn max_extent(&self) -> f64 {
        self.width.max(self.height)
    }

    /// Returns the region translated by `(dre, dim)`. Returns `None` if the
    /// offsets are not finite.
    #[must_use]
    pub fn panned_by(&self, dre: f64, dim: f64) -> Option<Self> {
        Some(Self {
            centre: self.centre.add_f64(dre, dim)?,
            width: self.width,
            height: self.height,
        })
    }

    /// Returns the region with new extents about the same centre.
    pub fn with_extent(&self, width: f64, height: f64) -> Result<Self, DeepRegionError> {
        Self::new(self.centre.clone(), width, height)
    }

    /// Returns the region with the centre replaced.
    #[must_use]
    pub fn with_centre(&self, centre: DeepComplex) -> Self {
        Self {
            centre,
            width: self.width,
            height: self.height,
        }
    }

    /// Binary precision the centre needs at this zoom depth.
    #[must_use]
    pub fn required_precision_bits(&self) -> usize {
        precision_bits_for_extent(self.min_extent())
    }

    /// Grows the centre precision to what the current extent requires.
    /// Precision never shrinks, so zooming out and back in is stable.
    #[must_use]
    pub fn normalised(&self) -> Self {
        let required = self.required_precision_bits();

        if self.centre.precision_bits() >= required {
            return self.clone();
        }

        Self {
            centre: self.centre.with_precision(required),
            width: self.width,
            height: self.height,
        }
    }
}

/// Centre precision needed to resolve positions inside a view of the given
/// extent, with guard bits for the reference orbit computation.
#[must_use]
pub fn precision_bits_for_extent(extent: f64) -> usize {
    let zoom_bits = if extent > 0.0 && extent.is_finite() {
        (-extent.log2()).ceil().max(0.0) as usize
    } else {
        0
    };

    (zoom_bits + PRECISION_GUARD_BITS).div_ceil(PRECISION_BLOCK_BITS) * PRECISION_BLOCK_BITS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home_rect() -> ComplexRect {
        ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .unwrap()
    }

    #[test]
    fn from_complex_rect_preserves_centre_and_extent() {
        let region = DeepRegion::from_complex_rect(&home_rect());
        let (re, im) = region.centre().to_f64();

        assert_eq!(re, -0.75);
        assert_eq!(im, 0.0);
        assert_eq!(region.width(), 3.5);
        assert_eq!(region.height(), 2.0);
    }

    #[test]
    fn round_trips_through_complex_rect_at_shallow_zoom() {
        let region = DeepRegion::from_complex_rect(&home_rect());
        let rect = region.to_complex_rect().unwrap();

        assert!((rect.width() - 3.5).abs() < 1e-12);
        assert!((rect.height() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_non_positive_or_non_finite_extents() {
        let centre = DeepComplex::zero();

        assert!(DeepRegion::new(centre.clone(), 0.0, 1.0).is_err());
        assert!(DeepRegion::new(centre.clone(), 1.0, -1.0).is_err());
        assert!(DeepRegion::new(centre.clone(), f64::NAN, 1.0).is_err());
        assert!(DeepRegion::new(centre, 1.0, f64::INFINITY).is_err());
    }

    #[test]
    fn precision_grows_with_zoom_depth() {
        let shallow = precision_bits_for_extent(1.0);
        let deep = precision_bits_for_extent(1e-50);
        let deeper = precision_bits_for_extent(1e-200);

        assert!(shallow >= 64);
        assert!(deep > shallow);
        assert!(deeper > deep);
        // ~166 zoom bits + guard for 1e-50.
        assert!(deep >= 166 + 64);
    }

    #[test]
    fn precision_is_block_aligned_for_stability() {
        for extent in [1.0, 1e-10, 1e-100, 1e-250] {
            assert_eq!(precision_bits_for_extent(extent) % PRECISION_BLOCK_BITS, 0);
        }
    }

    #[test]
    fn normalised_grows_but_never_shrinks_precision() {
        let region = DeepRegion::from_complex_rect(&home_rect());
        let deep = region.with_extent(1e-60, 1e-60).unwrap().normalised();
        let required = deep.required_precision_bits();

        assert!(deep.centre().precision_bits() >= required);

        // Zooming back out keeps the higher precision.
        let shallow_again = deep.with_extent(1.0, 1.0).unwrap().normalised();
        assert!(shallow_again.centre().precision_bits() >= required);
    }

    #[test]
    fn panned_by_moves_centre_below_f64_resolution() {
        let region = DeepRegion::from_complex_rect(&home_rect())
            .with_extent(1e-40, 1e-40)
            .unwrap()
            .normalised();

        let panned = region.panned_by(1e-41, 0.0).unwrap();

        assert_ne!(panned.centre(), region.centre());
        let (dre, dim) = panned.centre().sub_to_f64(region.centre());
        assert!((dre - 1e-41).abs() < 1e-50);
        assert_eq!(dim, 0.0);
    }

    #[test]
    fn to_complex_rect_fails_when_bounds_collapse() {
        let region = DeepRegion::from_complex_rect(&home_rect())
            .with_extent(1e-300, 1e-300)
            .unwrap();

        // Centre is -0.75; +/- 5e-301 collapses to the same f64.
        assert!(region.to_complex_rect().is_none());
    }
}
