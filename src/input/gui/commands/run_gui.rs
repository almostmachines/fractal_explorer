use std::marker::PhantomData;

use winit::{dpi::LogicalSize, event_loop::EventLoopBuilder, window::{Window, WindowBuilder}};
use crate::{controllers::interactive::InteractiveController, input::gui::{app::{events::gui::GuiEvent, gui_app::GuiApp, ports::presenter::GuiPresenterPort}, commands::ports::presenter_factory::GuiPresenterFactoryPort}};

pub struct RunGuiCommand<F, P>
where
    P: GuiPresenterPort,
    F: GuiPresenterFactoryPort<P>,
{
    presenter_factory: F,
    _phantom: PhantomData<fn() -> P>,
}

impl<F, P> RunGuiCommand<F, P>
where
    P: GuiPresenterPort,
    F: GuiPresenterFactoryPort<P>,
{
    pub fn new(presenter_factory: F) -> Self {
        Self { presenter_factory, _phantom: PhantomData }
    }

    pub fn execute(&self) {
        let event_loop = EventLoopBuilder::<GuiEvent>::with_user_event()
            .build()
            .expect("Failed to create event loop");

        let event_loop_proxy = event_loop.create_proxy();

        let window: &'static Window = Box::leak(Box::new(
            WindowBuilder::new()
                .with_title("Fractal Explorer")
                .with_inner_size(LogicalSize::new(800.0, 600.0))
                .with_min_inner_size(LogicalSize::new(200.0, 200.0))
                .build(&event_loop)
                .expect("Failed to create window"),
        ));

        let presenter: P = self.presenter_factory.build(window, event_loop_proxy);
        let controller = InteractiveController::new(presenter.share_adapter());
        let app = GuiApp::new(window, &event_loop, presenter, controller);

        app.run(event_loop);
    }
}
