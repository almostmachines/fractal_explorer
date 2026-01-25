use std::sync::Mutex;
use winit::event_loop::EventLoopProxy;
use crate::controllers::interactive::{events::render::RenderEvent, ports::presenter::InteractiveControllerPresenterPort};
use crate::input::gui::app::events::gui::GuiEvent;

pub struct PixelsAdapter {
    render_event: Mutex<Option<RenderEvent>>,
    event_loop_proxy: EventLoopProxy<GuiEvent>,
}

impl InteractiveControllerPresenterPort for PixelsAdapter {
    fn present(&self, event: RenderEvent) {
        *self.render_event.lock().unwrap() = Some(event);
        let _ = self.event_loop_proxy.send_event(GuiEvent::Wake);
    }
}

impl PixelsAdapter {
    pub fn new(event_loop_proxy: EventLoopProxy<GuiEvent>) -> Self {
        Self {
            render_event: Mutex::new(None),
            event_loop_proxy,
        }
    }

    pub fn render_event(&self) -> Option<RenderEvent> {
        self.render_event.lock().unwrap().take()
    }
}
