use std::sync::Arc;
use std::time::Duration;
use egui::Context;
use egui_winit::State as EguiWinitState;
use super::{GuiEvent, UiState};
use crate::presenters::PixelsPresenter;
use crate::controllers::interactive::InteractiveController;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::fractals::mandelbrot::colour_map::MandelbrotColourMapKind;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

struct App {
    width: u32,
    height: u32,
    scale_factor: f64,
    presenter: PixelsPresenter,
    controller: InteractiveController,
    ui_state: UiState,
    last_render_duration: Option<Duration>,
    last_error_message: Option<String>,
    /// Whether the window is focused. Can be used to reduce render rate when unfocused.
    #[allow(dead_code)]
    focused: bool,
    egui_ctx: Context,
    egui_state: EguiWinitState,
}

impl App {
    fn new(
        window: &'static Window,
        event_loop: &EventLoop<GuiEvent>,
        presenter: PixelsPresenter,
        controller: InteractiveController,
    ) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let egui_ctx = Context::default();

        let egui_state = EguiWinitState::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            event_loop,
            Some(scale_factor as f32),
            None, // max_texture_side, use default
        );

        Self {
            width: size.width,
            height: size.height,
            scale_factor,
            presenter,
            controller,
            ui_state: UiState::default(),
            last_render_duration: None,
            last_error_message: None,
            focused: true,
            egui_ctx,
            egui_state,
        }
    }

    fn render(&mut self, egui_output: egui::FullOutput) -> Result<(), pixels::Error> {
        self.presenter.render(egui_output, &self.egui_ctx, self.ui_state.latest_submitted_generation)
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        if width == 0 || height == 0 {
            return;
        }

        // resizing the surface keeps the swapchain in sync with the actual window size, which wgpu/pixels expects on every resize (except size 0). The buffer size is independent and is gated at >=2 because PixelRect and the render path require at least 2×2, so a 1×1 window just reuses the last buffer and scales it.
        self.presenter.resize(width, height);

        if width >= 2 && height >= 2 {
            self.presenter.resize_pixels_buffer(width, height);
        }
    }

    fn submit_render_request_if_needed(&mut self) {
        if self.width < 2 || self.height < 2 {
            return;
        }

        let width = i32::try_from(self.width).ok();
        let height = i32::try_from(self.height).ok();
        let (width, height) = match (width, height) {
            (Some(width), Some(height)) => (width, height),
            _ => return,
        };

        let pixel_rect = match PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: width - 1,
                y: height - 1,
            },
        ) {
            Ok(rect) => rect,
            Err(_) => return,
        };

        let request = self.ui_state.build_render_request(pixel_rect);
        if self.ui_state.should_submit(&request) {
            let request = Arc::new(request);
            let generation = self.controller.submit_request(Arc::clone(&request));
            self.ui_state.record_submission(request, generation);
            self.last_error_message = None;
        }
    }

    fn update_ui(&mut self, window: &Window) -> egui::FullOutput {
        let raw_input = self.egui_state.take_egui_input(window);

        self.egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("Debug Panel")
                .default_pos([10.0, 10.0])
                .default_size([260.0, 220.0])
                .show(ctx, |ui| {
                    ui.heading("Fractal Explorer");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Max iterations:");
                        ui.add(egui::Slider::new(
                            &mut self.ui_state.max_iterations,
                            1..=1000,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Colour map:");
                        egui::ComboBox::from_id_source("mandelbrot_colour_map")
                            .selected_text(self.ui_state.colour_map_kind.display_name())
                            .show_ui(ui, |ui| {
                                for &kind in MandelbrotColourMapKind::ALL {
                                    ui.selectable_value(
                                        &mut self.ui_state.colour_map_kind,
                                        kind,
                                        kind.display_name(),
                                    );
                                }
                            });
                    });

                    ui.separator();
                    ui.label("View region:");
                    let top_left = self.ui_state.region.top_left();
                    let bottom_right = self.ui_state.region.bottom_right();
                    ui.label(format!(
                        "Real: [{:.4}, {:.4}]",
                        top_left.real, bottom_right.real
                    ));
                    ui.label(format!(
                        "Imag: [{:.4}, {:.4}]",
                        top_left.imag, bottom_right.imag
                    ));

                    if ui.button("Reset view").clicked() {
                        self.ui_state.reset_view();
                    }

                    ui.separator();
                    ui.label(format!("Window size: {}x{}", self.width, self.height));
                    ui.label(format!(
                        "Latest generation: {}",
                        self.ui_state.latest_submitted_generation
                    ));
                    if let Some(render_duration) = self.last_render_duration {
                        ui.label(format!("Last render: {} ms", render_duration.as_millis()));
                    }
                    if let Some(message) = &self.last_error_message {
                        ui.separator();
                        ui.colored_label(egui::Color32::LIGHT_RED, message);
                    }
                });
        })
    }

    /// Handles a window event, forwarding it to egui first.
    ///
    /// Returns (consumed, repaint) where:
    /// - consumed: egui wants exclusive use of the event
    /// - repaint: egui wants a redraw (e.g., hover state changed)
    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> (bool, bool) {
        let response = self.egui_state.on_window_event(window, event);
        (response.consumed, response.repaint)
    }
}

/// This function does not return until the window is closed.
pub fn run_gui() {
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

    let presenter = PixelsPresenter::new(window, event_loop_proxy);
    let presenter_port = presenter.share_presenter_port();
    let controller = InteractiveController::new(presenter_port);
    let mut app = App::new(window, &event_loop, presenter, controller);
    let mut redraw_pending = true;

    event_loop
        .run(|event, elwt| {
            match event {
                Event::UserEvent(GuiEvent::Wake) => {
                    redraw_pending = true;
                    window.request_redraw();
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
                        WindowEvent::Focused(focused) => {
                            app.focused = *focused;
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
