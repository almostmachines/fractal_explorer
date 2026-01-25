use std::path::Path;

use crate::core::data::pixel_buffer::PixelBuffer;

pub trait FilePresenterPort {
    fn present(&self, buffer: &PixelBuffer, filepath: impl AsRef<Path>) -> std::io::Result<()>;
}
