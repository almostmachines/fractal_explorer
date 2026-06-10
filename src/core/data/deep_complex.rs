use dashu_float::FBig;

/// A complex number with arbitrary-precision components.
///
/// Used to track the view centre at zoom depths beyond what f64 can
/// represent. Extents and per-pixel offsets remain f64; only the absolute
/// position needs deep precision.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepComplex {
    pub re: FBig,
    pub im: FBig,
}

impl DeepComplex {
    #[must_use]
    pub fn zero() -> Self {
        // FBig::ZERO carries "unlimited" precision (0), which some dashu
        // operations reject; give it a concrete one.
        let zero = FBig::ZERO.with_precision(64).value();

        Self {
            re: zero.clone(),
            im: zero,
        }
    }

    /// Converts from f64 components. Returns `None` if either component is
    /// not finite.
    #[must_use]
    pub fn from_f64(re: f64, im: f64) -> Option<Self> {
        if !re.is_finite() || !im.is_finite() {
            return None;
        }

        Some(Self {
            re: FBig::try_from(re).ok()?,
            im: FBig::try_from(im).ok()?,
        })
    }

    /// Rounds both components to f64. Lossy at deep zoom by design.
    #[must_use]
    pub fn to_f64(&self) -> (f64, f64) {
        (self.re.to_f64().value(), self.im.to_f64().value())
    }

    /// Returns a copy with both components rounded/extended to `bits` of
    /// binary precision.
    #[must_use]
    pub fn with_precision(&self, bits: usize) -> Self {
        Self {
            re: self.re.clone().with_precision(bits).value(),
            im: self.im.clone().with_precision(bits).value(),
        }
    }

    /// The larger of the two components' binary precision.
    #[must_use]
    pub fn precision_bits(&self) -> usize {
        self.re.precision().max(self.im.precision())
    }

    /// Returns `self + (dre, dim)`, preserving the precision of `self`
    /// (assuming it is at least f64 precision).
    #[must_use]
    pub fn add_f64(&self, dre: f64, dim: f64) -> Option<Self> {
        let delta = Self::from_f64(dre, dim)?;

        Some(Self {
            re: &self.re + delta.re,
            im: &self.im + delta.im,
        })
    }

    /// Returns a copy with the real component replaced by an f64 value,
    /// keeping the current precision. Returns `None` if `re` is not finite.
    #[must_use]
    pub fn with_re_f64(&self, re: f64) -> Option<Self> {
        if !re.is_finite() {
            return None;
        }

        Some(Self {
            re: FBig::try_from(re)
                .ok()?
                .with_precision(self.precision_bits())
                .value(),
            im: self.im.clone(),
        })
    }

    /// Returns a copy with the imaginary component replaced by an f64 value,
    /// keeping the current precision. Returns `None` if `im` is not finite.
    #[must_use]
    pub fn with_im_f64(&self, im: f64) -> Option<Self> {
        if !im.is_finite() {
            return None;
        }

        Some(Self {
            re: self.re.clone(),
            im: FBig::try_from(im)
                .ok()?
                .with_precision(self.precision_bits())
                .value(),
        })
    }

    /// Returns `self - other` rounded to f64 components. Only meaningful
    /// when the two points are close (e.g. view centre minus reference
    /// point); the difference is then well within f64 range.
    #[must_use]
    pub fn sub_to_f64(&self, other: &Self) -> (f64, f64) {
        (
            (&self.re - &other.re).to_f64().value(),
            (&self.im - &other.im).to_f64().value(),
        )
    }

    /// Formats both components as decimal strings with `sig_digits`
    /// significant digits.
    #[must_use]
    pub fn to_decimal_strings(&self, sig_digits: usize) -> (String, String) {
        (
            format_decimal(&self.re, sig_digits),
            format_decimal(&self.im, sig_digits),
        )
    }
}

fn format_decimal(value: &FBig, sig_digits: usize) -> String {
    let bounded = if value.precision() == 0 {
        value.clone().with_precision(64).value()
    } else {
        value.clone()
    };

    let decimal = bounded
        .with_base::<10>()
        .value()
        .with_precision(sig_digits.max(1))
        .value();

    decimal.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_f64_round_trips() {
        let c = DeepComplex::from_f64(-0.75, 0.1).unwrap();
        let (re, im) = c.to_f64();

        assert_eq!(re, -0.75);
        assert_eq!(im, 0.1);
    }

    #[test]
    fn from_f64_rejects_non_finite() {
        assert!(DeepComplex::from_f64(f64::NAN, 0.0).is_none());
        assert!(DeepComplex::from_f64(0.0, f64::INFINITY).is_none());
    }

    #[test]
    fn with_precision_extends_precision() {
        let c = DeepComplex::from_f64(-0.75, 0.1).unwrap().with_precision(256);

        assert!(c.precision_bits() >= 256);
    }

    #[test]
    fn add_f64_accumulates_below_f64_resolution() {
        // Adding a tiny pan offset to a high-precision centre must not be
        // swallowed by rounding, even when the offset is far below the
        // f64 resolution of the centre value.
        let base = DeepComplex::from_f64(-0.75, 0.1).unwrap().with_precision(256);
        let stepped = base.add_f64(1e-40, -1e-40).unwrap();

        assert_ne!(stepped, base);

        let (dre, dim) = stepped.sub_to_f64(&base);
        assert!((dre - 1e-40).abs() < 1e-50, "dre={dre}");
        assert!((dim + 1e-40).abs() < 1e-50, "dim={dim}");
    }

    #[test]
    fn sub_to_f64_returns_small_difference() {
        let a = DeepComplex::from_f64(-0.75, 0.1).unwrap().with_precision(128);
        let b = a.add_f64(3e-20, -2e-20).unwrap();

        let (dre, dim) = b.sub_to_f64(&a);

        assert!((dre - 3e-20).abs() < 1e-30);
        assert!((dim + 2e-20).abs() < 1e-30);
    }

    #[test]
    fn decimal_strings_have_requested_digits() {
        let c = DeepComplex::from_f64(-0.75, 0.125).unwrap().with_precision(128);
        let (re, im) = c.to_decimal_strings(10);

        assert!(re.starts_with("-0.75"), "re={re}");
        assert!(im.starts_with("0.125"), "im={im}");
    }

    #[test]
    fn zero_formats_cleanly() {
        let c = DeepComplex::zero();
        let (re, _) = c.to_decimal_strings(8);

        assert_eq!(re, "0");
    }
}
