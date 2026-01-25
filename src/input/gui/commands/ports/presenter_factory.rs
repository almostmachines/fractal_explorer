use winit::{event_loop::EventLoopProxy, window::Window};
use crate::input::gui::app::{events::gui::GuiEvent, ports::presenter::GuiPresenterPort};

pub trait GuiPresenterFactoryPort<T: GuiPresenterPort> {
    fn build(&self, window: &'static Window, event_loop_proxy: EventLoopProxy<GuiEvent>) -> T;
}
