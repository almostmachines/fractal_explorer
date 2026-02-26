use crate::core::data::pixel_rect::PixelRect;
use crate::core::util::calculate_bands_in_pixel_rect::calculate_bands_in_pixel_rect;
use std::num::NonZeroU32;

pub fn calculate_threads_for_pixel_rect_banding(pixel_rect: PixelRect) -> u32 {
    let num_avail_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as u32;

    calculate_bands_in_pixel_rect(NonZeroU32::new(num_avail_threads).unwrap(), pixel_rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;

    #[test]
    fn test_pixel_rect_height_1_gives_1_thread() {
        let threads = calculate_threads_for_pixel_rect_banding(
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 0, y: 0 }).unwrap(),
        );

        assert_eq!(threads, 1);
    }

    #[test]
    fn test_pixel_rect_height_2_gives_1_thread() {
        let threads = calculate_threads_for_pixel_rect_banding(
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 1 }).unwrap(),
        );

        assert_eq!(threads, 1);
    }

    #[test]
    fn test_pixel_rect_height_3_gives_1_thread() {
        let threads = calculate_threads_for_pixel_rect_banding(
            PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 2 }).unwrap(),
        );

        assert_eq!(threads, 1);
    }

    #[test]
    fn test_sanity_check() {
        let num_avail_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as u32;

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 10, y: 3 }).unwrap();
        let threads = calculate_threads_for_pixel_rect_banding(pixel_rect);

        if num_avail_threads > 2 {
            assert_eq!(threads, 2);
        } else {
            assert_eq!(threads, num_avail_threads);
        }
    }

    #[test]
    fn test_sanity_check_2() {
        let num_avail_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as i32;

        let pixel_rect = PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: 10,
                y: (num_avail_threads * 3),
            },
        )
        .unwrap();
        let threads = calculate_threads_for_pixel_rect_banding(pixel_rect);

        assert_eq!(threads, num_avail_threads as u32);
    }
}
