use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{input::gui::{app::{events::gui::GuiEvent, ports::presenter::GuiPresenterPort}, command::ports::presenter_factory::GuiPresenterFactoryPort}, presenters::pixels::presenter::PixelsPresenter};

pub struct PixelsPresenterFactory {}

impl GuiPresenterFactoryPort<PixelsPresenter> for PixelsPresenterFactory {
    fn build(&self, window: &'static Window, event_loop_proxy: EventLoopProxy<GuiEvent>) -> PixelsPresenter {
        PixelsPresenter::new(window, event_loop_proxy)
    }
}
