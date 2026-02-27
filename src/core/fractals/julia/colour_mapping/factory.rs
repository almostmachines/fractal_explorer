use crate::core::fractals::julia::colour_mapping::{kinds::JuliaColourMapKinds, map::JuliaColourMap, maps::{blue_white_gradient::JuliaBlueWhiteGradient, fire_gradient::JuliaFireGradient}};

#[must_use]
pub fn julia_colour_map_factory(
    kind: JuliaColourMapKinds,
    max_iterations: u32,
) -> Box<dyn JuliaColourMap> {
    match kind {
        JuliaColourMapKinds::FireGradient => {
            Box::new(JuliaFireGradient::new(max_iterations))
        }
        JuliaColourMapKinds::BlueWhiteGradient => {
            Box::new(JuliaBlueWhiteGradient::new(max_iterations))
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
            JuliaColourMapKinds::ALL.first(),
            Some(&JuliaColourMapKinds::default())
        );
    }

    #[test]
    fn factory_round_trip_for_all_kinds() {
        for &kind in JuliaColourMapKinds::ALL {
            let map = julia_colour_map_factory(kind, 256);
            assert_eq!(map.kind(), kind);
        }
    }

    #[test]
    fn display_names_match_between_kind_and_concrete() {
        for &kind in JuliaColourMapKinds::ALL {
            let map = julia_colour_map_factory(kind, 256);
            assert_eq!(map.display_name(), kind.display_name());
        }
    }

    #[test]
    fn display_names_are_unique() {
        let names: Vec<&str> = JuliaColourMapKinds::ALL
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
