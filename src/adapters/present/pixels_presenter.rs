use std::sync::{Arc, Mutex};

use pixels::Pixels;
use winit::event_loop::EventLoopProxy;

use crate::adapters::pixel_format::copy_rgb_to_rgba;
use crate::controllers::interactive::ports::{FrameSink, RenderEvent};
use crate::controllers::interactive::data::frame_data::FrameData;
use crate::input::gui::GuiEvent;

struct PresenterInner {
    render_event: Mutex<Option<RenderEvent>>,
    event_loop_proxy: EventLoopProxy<GuiEvent>,
}

pub struct PixelsPresenter {
    inner: Arc<PresenterInner>,
}

impl PixelsPresenter {
    pub fn new(event_loop_proxy: EventLoopProxy<GuiEvent>) -> Self {
        Self {
            inner: Arc::new(PresenterInner {
                render_event: Mutex::new(None),
                event_loop_proxy,
            }),
        }
    }

    pub fn frame_sink(&self) -> Arc<dyn FrameSink> {
        Arc::clone(&self.inner) as Arc<dyn FrameSink>
    }

    #[must_use]
    pub fn take_render_event(&self) -> Option<RenderEvent> {
        self.inner.render_event.lock().unwrap().take()
    }

    pub fn copy_pixel_buffer_into_pixels_frame(frame: &FrameData, pixels: &mut Pixels) {
        let pixel_rect = frame.pixel_rect;
        let width = pixel_rect.width();
        let height = pixel_rect.height();
        let expected_rgb_len = (width * height * 3) as usize;
        let src = frame.pixel_buffer.buffer();
        assert_eq!(
            src.len(),
            expected_rgb_len,
            "frame data length {} does not match expected {} for {}x{}",
            src.len(),
            expected_rgb_len,
            width,
            height
        );

        let expected_rgba_len = (width * height * 4) as usize;
        let dest = pixels.frame_mut();
        assert_eq!(
            dest.len(),
            expected_rgba_len,
            "pixels frame length {} does not match expected {} for {}x{}",
            dest.len(),
            expected_rgba_len,
            width,
            height
        );

        copy_rgb_to_rgba(src, dest);
    }
}

impl FrameSink for PresenterInner {
    fn submit(&self, event: RenderEvent) {
        *self.render_event.lock().unwrap() = Some(event);
        let _ = self.event_loop_proxy.send_event(GuiEvent::Wake);
    }
}
