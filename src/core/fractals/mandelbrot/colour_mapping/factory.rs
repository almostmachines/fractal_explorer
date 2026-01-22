use crate::core::fractals::mandelbrot::colour_mapping::{kinds::MandelbrotColourMapKinds, map::MandelbrotColourMap, maps::{blue_white_gradient::MandelbrotBlueWhiteGradient, fire_gradient::MandelbrotFireGradient}};

#[must_use]
pub fn mandelbrot_colour_map_factory(
    kind: MandelbrotColourMapKinds,
    max_iterations: u32,
) -> Box<dyn MandelbrotColourMap> {
    match kind {
        MandelbrotColourMapKinds::FireGradient => {
            Box::new(MandelbrotFireGradient::new(max_iterations))
        }
        MandelbrotColourMapKinds::BlueWhiteGradient => {
            Box::new(MandelbrotBlueWhiteGradient::new(max_iterations))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::generate_pixel_buffer::ports::colour_map::ColourMap;

    #[test]
    fn all_array_has_default_first() {
        assert_eq!(
            MandelbrotColourMapKinds::ALL.first(),
            Some(&MandelbrotColourMapKinds::default())
        );
    }

    #[test]
    fn factory_round_trip_for_all_kinds() {
        for &kind in MandelbrotColourMapKinds::ALL {
            let map = mandelbrot_colour_map_factory(kind, 256);
            assert_eq!(map.kind(), kind);
        }
    }

    #[test]
    fn display_names_match_between_kind_and_concrete() {
        for &kind in MandelbrotColourMapKinds::ALL {
            let map = mandelbrot_colour_map_factory(kind, 256);
            assert_eq!(map.display_name(), kind.display_name());
        }
    }

    #[test]
    fn display_names_are_unique() {
        let names: Vec<&str> = MandelbrotColourMapKinds::ALL
            .iter()
            .map(|k| k.display_name())
            .collect();
        for (i, name) in names.iter().enumerate() {
            for (j, other) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(name, other, "Duplicate display name: {}", name);
                }
            }
        }
    }
}
