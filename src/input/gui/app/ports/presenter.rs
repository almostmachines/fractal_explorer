use std::sync::Arc;

use egui::Context as EguiContext;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{controllers::interactive::ports::presenter::InteractiveControllerPresenterPort, input::gui::app::events::gui::GuiEvent};

pub trait GuiPresenterPort {
    fn new(window: &'static Window, event_loop_proxy: EventLoopProxy<GuiEvent>) -> Self;
    fn render(&mut self, egui_output: egui::FullOutput, egui_ctx: &EguiContext, requested_generation: u64) -> Result<(), pixels::Error>;
    fn share_adapter(&self) -> Arc<dyn InteractiveControllerPresenterPort>;
    fn resize(&mut self, width: u32, height: u32);
}
