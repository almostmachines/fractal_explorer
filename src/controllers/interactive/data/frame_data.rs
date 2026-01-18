use std::time::Duration;
use crate::core::data::pixel_buffer::PixelBuffer;
use crate::core::data::pixel_rect::PixelRect;

#[derive(Debug)]
pub struct FrameData {
    pub generation: u64,
    pub pixel_rect: PixelRect,
    pub pixel_buffer: PixelBuffer,
    pub render_duration: Duration,
}
