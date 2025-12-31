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
