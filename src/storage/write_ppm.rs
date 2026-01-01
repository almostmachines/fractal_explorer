use std::io::Write;
use std::path::Path;
use crate::core::data::pixel_buffer::PixelBuffer;

pub fn write_ppm(buffer: PixelBuffer, filepath: impl AsRef<Path>) -> std::io::Result<()> {
    let mut file = std::fs::File::create(filepath)?;

    // PPM header: P6 means binary RGB, then width height max_colour
    let width = buffer.pixel_rect().width();
    let height = buffer.pixel_rect().height();

    writeln!(file, "P6")?;
    writeln!(file, "{} {}", width, height)?;
    writeln!(file, "255")?;
    file.write_all(buffer.buffer())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use crate::core::data::point::Point;
    use crate::core::data::pixel_rect::PixelRect;

    fn create_pixel_rect(width: i32, height: i32) -> PixelRect {
        PixelRect::new(
            Point { x: 0, y: 0 },
            Point { x: width, y: height },
        ).unwrap()
    }

    fn create_test_buffer(width: i32, height: i32, data: Vec<u8>) -> PixelBuffer {
        let pixel_rect = create_pixel_rect(width, height);
        PixelBuffer::from_data(pixel_rect, data).unwrap()
    }

    fn read_file_bytes(path: &Path) -> Vec<u8> {
        let mut file = fs::File::open(path).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        contents
    }

    fn parse_ppm_header(data: &[u8]) -> (String, i32, i32, i32, usize) {
        // Parse PPM header and return (magic, width, height, max_val, header_end_index)
        let header_str = String::from_utf8_lossy(data);
        let mut lines = header_str.lines();

        let magic = lines.next().unwrap().to_string();
        let dimensions = lines.next().unwrap();
        let mut dim_parts = dimensions.split_whitespace();
        let width: i32 = dim_parts.next().unwrap().parse().unwrap();
        let height: i32 = dim_parts.next().unwrap().parse().unwrap();
        let max_val: i32 = lines.next().unwrap().parse().unwrap();

        // Calculate header end: "P6\n" + "W H\n" + "255\n"
        let header_len = magic.len() + 1 + dimensions.len() + 1 + 3 + 1; // "255" + newline

        (magic, width, height, max_val, header_len)
    }

    #[test]
    fn test_write_ppm_creates_file() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_creates_file.ppm");

        let buffer = create_test_buffer(2, 2, vec![0; 12]);
        let result = write_ppm(buffer, &filepath);

        assert!(result.is_ok());
        assert!(filepath.exists());

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_header_format() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_header_format.ppm");

        let buffer = create_test_buffer(10, 20, vec![0; 600]);
        write_ppm(buffer, &filepath).unwrap();

        let contents = read_file_bytes(&filepath);
        let (magic, width, height, max_val, _) = parse_ppm_header(&contents);

        assert_eq!(magic, "P6");
        assert_eq!(width, 10);
        assert_eq!(height, 20);
        assert_eq!(max_val, 255);

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_pixel_data() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_pixel_data.ppm");

        let pixel_data: Vec<u8> = vec![
            255, 0, 0,    // red
            0, 255, 0,    // green
            0, 0, 255,    // blue
            255, 255, 0,  // yellow
        ];
        let buffer = create_test_buffer(2, 2, pixel_data.clone());
        write_ppm(buffer, &filepath).unwrap();

        let contents = read_file_bytes(&filepath);
        let (_, _, _, _, header_len) = parse_ppm_header(&contents);

        let pixel_bytes = &contents[header_len..];
        assert_eq!(pixel_bytes, pixel_data.as_slice());

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_single_pixel() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_single_pixel.ppm");

        let pixel_data: Vec<u8> = vec![128, 64, 32];
        let buffer = create_test_buffer(1, 1, pixel_data.clone());
        write_ppm(buffer, &filepath).unwrap();

        let contents = read_file_bytes(&filepath);
        let (magic, width, height, max_val, header_len) = parse_ppm_header(&contents);

        assert_eq!(magic, "P6");
        assert_eq!(width, 1);
        assert_eq!(height, 1);
        assert_eq!(max_val, 255);
        assert_eq!(&contents[header_len..], pixel_data.as_slice());

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_larger_image() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_larger_image.ppm");

        let width = 100;
        let height = 50;
        let pixel_count = (width * height * 3) as usize;
        let pixel_data: Vec<u8> = (0..pixel_count).map(|i| (i % 256) as u8).collect();

        let buffer = create_test_buffer(width, height, pixel_data.clone());
        write_ppm(buffer, &filepath).unwrap();

        let contents = read_file_bytes(&filepath);
        let (magic, w, h, max_val, header_len) = parse_ppm_header(&contents);

        assert_eq!(magic, "P6");
        assert_eq!(w, width);
        assert_eq!(h, height);
        assert_eq!(max_val, 255);
        assert_eq!(&contents[header_len..], pixel_data.as_slice());

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_file_size() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_file_size.ppm");

        let width = 4;
        let height = 3;
        let pixel_data: Vec<u8> = vec![0; 36]; // 4 * 3 * 3 = 36 bytes
        let buffer = create_test_buffer(width, height, pixel_data);
        write_ppm(buffer, &filepath).unwrap();

        let metadata = fs::metadata(&filepath).unwrap();
        // Header: "P6\n" (3) + "4 3\n" (4) + "255\n" (4) = 11 bytes, plus 36 pixel bytes = 47 bytes
        assert_eq!(metadata.len(), 47);

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_overwrites_existing_file() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_overwrite.ppm");

        // Write first file
        let buffer1 = create_test_buffer(1, 1, vec![255, 0, 0]);
        write_ppm(buffer1, &filepath).unwrap();

        // Overwrite with different content
        let buffer2 = create_test_buffer(1, 1, vec![0, 255, 0]);
        write_ppm(buffer2, &filepath).unwrap();

        let contents = read_file_bytes(&filepath);
        let (_, _, _, _, header_len) = parse_ppm_header(&contents);

        assert_eq!(&contents[header_len..], &[0, 255, 0]);

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_invalid_path_returns_error() {
        let invalid_path = Path::new("nonexistent_directory_12345/test.ppm");

        let buffer = create_test_buffer(1, 1, vec![0; 3]);
        let result = write_ppm(buffer, invalid_path);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_write_ppm_with_path_buf() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_path_buf.ppm");

        let buffer = create_test_buffer(1, 1, vec![0; 3]);
        let result = write_ppm(buffer, filepath.clone());

        assert!(result.is_ok());
        assert!(filepath.exists());

        fs::remove_file(&filepath).ok();
    }

    #[test]
    fn test_write_ppm_with_string_path() {
        let temp_dir = std::env::temp_dir();
        let filepath = temp_dir.join("test_string_path.ppm");
        let path_string = filepath.to_str().unwrap();

        let buffer = create_test_buffer(1, 1, vec![0; 3]);
        let result = write_ppm(buffer, path_string);

        assert!(result.is_ok());
        assert!(filepath.exists());

        fs::remove_file(&filepath).ok();
    }
}
