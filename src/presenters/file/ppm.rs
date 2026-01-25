use crate::controllers::ports::file_presenter::FilePresenterPort;
use crate::core::data::pixel_buffer::PixelBuffer;
use std::io::Write;
use std::path::Path;

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
        file.write_all(buffer.buffer())?;

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
