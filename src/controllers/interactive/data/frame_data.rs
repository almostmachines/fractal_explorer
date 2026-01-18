use std::time::Duration;
use crate::core::data::pixel_buffer::PixelBuffer;

#[derive(Debug)]
pub struct FrameData {
    pub generation: u64,
    pub pixel_buffer: PixelBuffer,
    pub render_duration: Duration,
}
