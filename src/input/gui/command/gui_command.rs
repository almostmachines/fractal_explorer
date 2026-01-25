use std::marker::PhantomData;

use winit::{dpi::LogicalSize, event::{Event, WindowEvent}, event_loop::EventLoopBuilder, window::{Window, WindowBuilder}};
use crate::{controllers::interactive::InteractiveController, input::gui::{app::{events::gui::GuiEvent, gui_app::GuiApp, ports::presenter::GuiPresenterPort}, command::ports::presenter_factory::GuiPresenterFactoryPort}};

pub struct GuiCommand<F, P>
where
    P: GuiPresenterPort,
    F: GuiPresenterFactoryPort<P>,
{
    presenter_factory: F,
    _phantom: PhantomData<fn() -> P>,
}

impl<F, P> GuiCommand<F, P>
where
    P: GuiPresenterPort,
    F: GuiPresenterFactoryPort<P>,
{
    pub fn new(presenter_factory: F) -> Self {
        Self { presenter_factory, _phantom: PhantomData }
    }

    pub fn run(&self) {
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
        let mut app = GuiApp::new(window, &event_loop, presenter, controller);
        let mut redraw_pending = true;

        event_loop
            .run(|event, elwt| {
                match event {
                    Event::UserEvent(GuiEvent::Wake) => {
                        redraw_pending = true;
                    }
                    Event::WindowEvent {
                        ref event,
                        window_id,
                    } if window_id == window.id() => {
                        // Forward event to egui first
                        let (egui_consumed, egui_repaint) = app.handle_window_event(window, event);

                        if egui_repaint {
                            redraw_pending = true;
                        }

                        // If egui consumed the event, skip our handling
                        // (except for events we always need to handle)
                        match event {
                            WindowEvent::CloseRequested => {
                                app.controller.shutdown();
                                elwt.exit();
                            }
                            WindowEvent::RedrawRequested => {
                                redraw_pending = false;

                                // Run egui frame
                                let egui_output = app.update_ui(window);
                                app.submit_render_request_if_needed();

                                // Handle egui platform output (e.g., clipboard, cursor changes)
                                app.egui_state.handle_platform_output(
                                    window,
                                    egui_output.platform_output.clone(),
                                );

                                // Check if egui wants a repaint
                                if egui_output
                                    .viewport_output
                                    .values()
                                    .any(|v| v.repaint_delay.is_zero())
                                {
                                    redraw_pending = true;
                                }

                                // Render the frame with egui overlay
                                if let Err(e) = app.render(egui_output) {
                                    eprintln!("Render error: {e}");
                                    elwt.exit();
                                }
                            }
                            WindowEvent::Resized(size) => {
                                app.resize(size.width, size.height);
                                redraw_pending = true;
                            }
                            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                                app.scale_factor = *scale_factor;
                                app.egui_ctx.set_pixels_per_point(*scale_factor as f32);
                                // Get the new physical size after scale factor change
                                let size = window.inner_size();
                                app.resize(size.width, size.height);
                                redraw_pending = true;
                            }
                            _ => {}
                        }

                        // Suppress unused variable warning - consumed will be used
                        // when we add pan/zoom to avoid passing clicks through UI
                        let _ = egui_consumed;
                    }
                    Event::AboutToWait => {
                        // Only request redraw if state changed
                        if redraw_pending {
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
            })
            .expect("Event loop error");
    }
}
