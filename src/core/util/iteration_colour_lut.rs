use crate::core::data::colour::Colour;

#[derive(Debug)]
pub struct IterationColourLut {
    entries: Box<[Colour]>,
}

impl IterationColourLut {
    #[must_use]
    pub fn new(max_iterations: u32, mut colour_from_t: impl FnMut(f64) -> Colour) -> Self {
        if max_iterations == 0 {
            return Self {
                entries: vec![Colour { r: 0, g: 0, b: 0 }].into_boxed_slice(),
            };
        }

        let mut entries = Vec::with_capacity(max_iterations as usize + 1);
        for i in 0..max_iterations {
            let t = i as f64 / max_iterations as f64;
            entries.push(colour_from_t(t));
        }

        entries.push(Colour { r: 0, g: 0, b: 0 });

        Self {
            entries: entries.into_boxed_slice(),
        }
    }

    #[inline]
    #[must_use]
    pub fn get(&self, iterations: u32) -> Option<Colour> {
        self.entries.get(iterations as usize).copied()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_max_iterations_builds_single_black_entry() {
        let lut = IterationColourLut::new(0, |_| Colour { r: 1, g: 2, b: 3 });

        assert_eq!(lut.len(), 1);
        let c = lut.get(0).expect("entry at 0 must exist");
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert!(lut.get(1).is_none());
    }

    #[test]
    fn non_zero_max_has_max_plus_one_entries_and_black_tail() {
        let lut = IterationColourLut::new(4, |t| Colour {
            r: (t * 10.0) as u8,
            g: 0,
            b: 0,
        });

        assert_eq!(lut.len(), 5);
        let tail = lut.get(4).expect("tail entry at max must exist");
        assert_eq!(tail.r, 0);
        assert_eq!(tail.g, 0);
        assert_eq!(tail.b, 0);
    }
}
