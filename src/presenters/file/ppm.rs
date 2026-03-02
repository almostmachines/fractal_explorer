use crate::controllers::ports::file_presenter::FilePresenterPort;
use crate::core::data::pixel_buffer::PixelBuffer;
use std::io::Write;
use std::path::Path;

const PPM_BYTES_PER_PIXEL: usize = 3;

pub struct PpmFilePresenter {}

impl FilePresenterPort for PpmFilePresenter {
    fn present(&self, buffer: &PixelBuffer, filepath: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = std::fs::File::create(filepath)?;
        let width = buffer.pixel_rect().width();
        let height = buffer.pixel_rect().height();

        // PPM header: P6 means binary RGB, then width, height and max_colour
        writeln!(file, "P6")?;
        writeln!(file, "{} {}", width, height)?;
        writeln!(file, "255")?;

        let pixel_count = (width * height) as usize;
        let mut rgb_data = Vec::with_capacity(pixel_count * PPM_BYTES_PER_PIXEL);
        for pixel in buffer.buffer().chunks_exact(PixelBuffer::BYTES_PER_PIXEL) {
            rgb_data.extend_from_slice(&pixel[..PPM_BYTES_PER_PIXEL]);
        }

        file.write_all(&rgb_data)?;

        Ok(())
    }
}

impl Default for PpmFilePresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl PpmFilePresenter {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_pixel_rect(width: i32, height: i32) -> PixelRect {
        PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        )
        .unwrap()
    }

    fn temp_file_path(test_name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "fractal_explorer_{}_{}_{}.ppm",
            test_name,
            std::process::id(),
            timestamp
        ))
    }

    #[test]
    fn test_present_writes_ppm_header_and_rgb_payload() {
        let pixel_rect = create_pixel_rect(2, 1);
        let buffer = PixelBuffer::from_data(
            pixel_rect,
            vec![
                10, 20, 30, 1, // pixel 1
                40, 50, 60, 2, // pixel 2
            ],
        )
        .unwrap();

        let output_path = temp_file_path("header_and_payload");
        PpmFilePresenter::new().present(&buffer, &output_path).unwrap();

        let output = fs::read(&output_path).unwrap();
        fs::remove_file(&output_path).unwrap();

        let expected_header = b"P6\n2 1\n255\n";
        let expected_payload = [10, 20, 30, 40, 50, 60];
        assert!(output.starts_with(expected_header));
        assert_eq!(&output[expected_header.len()..], expected_payload.as_slice());
    }

    #[test]
    fn test_present_strips_alpha_bytes_from_output_payload() {
        let pixel_rect = create_pixel_rect(1, 2);
        let buffer = PixelBuffer::from_data(
            pixel_rect,
            vec![
                1, 2, 3, 9, // pixel 1
                4, 5, 6, 8, // pixel 2
            ],
        )
        .unwrap();

        let output_path = temp_file_path("strips_alpha");
        PpmFilePresenter::new().present(&buffer, &output_path).unwrap();

        let output = fs::read(&output_path).unwrap();
        fs::remove_file(&output_path).unwrap();

        let expected_header = b"P6\n1 2\n255\n";
        let payload = &output[expected_header.len()..];
        let expected_payload_len = pixel_rect.size() as usize * PPM_BYTES_PER_PIXEL;

        assert!(output.starts_with(expected_header));
        assert_eq!(payload.len(), expected_payload_len);
        assert_eq!(payload, &[1, 2, 3, 4, 5, 6]);
    }
}
