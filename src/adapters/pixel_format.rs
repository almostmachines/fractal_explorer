//! Pixel format conversion helpers for presentation adapters.

/// Copies RGB pixel data to RGBA format, setting alpha to 255.
///
/// # Arguments
/// * `src` - Source buffer with RGB data (3 bytes per pixel)
/// * `dst` - Destination buffer for RGBA data (4 bytes per pixel)
///
/// # Panics
/// Panics if buffer sizes don't match (dst.len() must equal src.len() / 3 * 4)
/// or if `src` is not a multiple of 3.
pub fn copy_rgb_to_rgba(src: &[u8], dst: &mut [u8]) {
    assert!(
        src.len() % 3 == 0,
        "src length {} is not a multiple of 3",
        src.len()
    );
    let expected_dst_len = (src.len() / 3) * 4;
    assert_eq!(
        dst.len(),
        expected_dst_len,
        "dst length {} does not match expected {}",
        dst.len(),
        expected_dst_len
    );

    for (src_pixel, dst_pixel) in src.chunks_exact(3).zip(dst.chunks_exact_mut(4)) {
        dst_pixel[0] = src_pixel[0];
        dst_pixel[1] = src_pixel[1];
        dst_pixel[2] = src_pixel[2];
        dst_pixel[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_rgb_to_rgba_known_values() {
        let src = vec![
            255, 0, 0, // red
            0, 255, 0, // green
            0, 0, 255, // blue
            255, 255, 255, // white
        ];
        let mut dst = vec![0; (src.len() / 3) * 4];

        copy_rgb_to_rgba(&src, &mut dst);

        assert_eq!(
            dst,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255
            ]
        );
    }

    #[test]
    fn test_copy_rgb_to_rgba_empty_buffers() {
        let src: Vec<u8> = vec![];
        let mut dst: Vec<u8> = vec![];

        copy_rgb_to_rgba(&src, &mut dst);

        assert!(dst.is_empty());
    }

    #[test]
    fn test_copy_rgb_to_rgba_single_pixel() {
        let src = vec![128, 64, 32];
        let mut dst = vec![0; 4];

        copy_rgb_to_rgba(&src, &mut dst);

        assert_eq!(dst, vec![128, 64, 32, 255]);
    }

    #[test]
    fn test_copy_rgb_to_rgba_multiple_pixels() {
        let src = vec![10, 20, 30, 40, 50, 60];
        let mut dst = vec![0; 8];

        copy_rgb_to_rgba(&src, &mut dst);

        assert_eq!(dst, vec![10, 20, 30, 255, 40, 50, 60, 255]);
    }
}
