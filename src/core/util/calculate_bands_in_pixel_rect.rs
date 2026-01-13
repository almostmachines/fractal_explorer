use crate::core::data::pixel_rect::PixelRect;
use std::num::NonZeroU32;

pub fn calculate_bands_in_pixel_rect(max_bands: NonZeroU32, pixel_rect: PixelRect) -> u32 {
    if pixel_rect.height() == 2 {
        1
    } else if max_bands.get() > (pixel_rect.height() / 2) {
        pixel_rect.height() / 2
    } else {
        max_bands.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::point::Point;

    #[test]
    fn test_height_2_gives_1_band() {
        let bands = calculate_bands_in_pixel_rect(
            NonZeroU32::new(10).unwrap(),
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 1 }).unwrap(),
        );

        assert_eq!(bands, 1);
    }

    #[test]
    fn test_height_3_gives_1_band() {
        let bands = calculate_bands_in_pixel_rect(
            NonZeroU32::new(10).unwrap(),
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 2 }).unwrap(),
        );

        assert_eq!(bands, 1);
    }

    #[test]
    fn bands_do_not_exceed_half_pixel_rect_height() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 5, y: 5 }).unwrap();
        let pixel_rect2 = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 6, y: 6 }).unwrap();
        let bands = calculate_bands_in_pixel_rect(NonZeroU32::new(10).unwrap(), pixel_rect);
        let bands2 = calculate_bands_in_pixel_rect(NonZeroU32::new(3).unwrap(), pixel_rect);
        let bands3 = calculate_bands_in_pixel_rect(NonZeroU32::new(10).unwrap(), pixel_rect2);
        let bands4 = calculate_bands_in_pixel_rect(NonZeroU32::new(3).unwrap(), pixel_rect2);

        assert_eq!(bands, pixel_rect.height() / 2);
        assert_eq!(bands2, pixel_rect.height() / 2);
        assert_eq!(bands3, pixel_rect2.height() / 2);
        assert_eq!(bands4, pixel_rect2.height() / 2);
    }

    #[test]
    fn bands_correctly_calculated() {
        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 19, y: 19 }).unwrap();
        let pixel_rect2 = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 20, y: 20 }).unwrap();
        let bands = calculate_bands_in_pixel_rect(NonZeroU32::new(4).unwrap(), pixel_rect);
        let bands2 = calculate_bands_in_pixel_rect(NonZeroU32::new(5).unwrap(), pixel_rect);
        let bands3 = calculate_bands_in_pixel_rect(NonZeroU32::new(4).unwrap(), pixel_rect2);
        let bands4 = calculate_bands_in_pixel_rect(NonZeroU32::new(5).unwrap(), pixel_rect2);

        assert_eq!(bands, 4);
        assert_eq!(bands2, 5);
        assert_eq!(bands3, 4);
        assert_eq!(bands4, 5);
    }
}
