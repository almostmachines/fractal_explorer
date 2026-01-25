use std::sync::Arc;
use std::time::Duration;
use egui::Context;
use egui_winit::State as EguiWinitState;
use crate::input::gui::app::events::gui::GuiEvent;
use crate::input::gui::app::state::GuiAppState;
use crate::{core::fractals::mandelbrot::colour_mapping::kinds::MandelbrotColourMapKinds, input::gui::app::ports::presenter::GuiPresenterPort};
use crate::controllers::interactive::InteractiveController;
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

pub struct GuiApp<T: GuiPresenterPort>
{
    window: &'static Window,
    width: u32,
    height: u32,
    pub scale_factor: f64,
    presenter: T,
    pub controller: InteractiveController,
    ui_state: GuiAppState,
    last_render_duration: Option<Duration>,
    last_error_message: Option<String>,
    pub egui_ctx: Context,
    pub egui_state: EguiWinitState,
}

impl<T: GuiPresenterPort> GuiApp<T>
{
    pub fn new(
        window: &'static Window,
        event_loop: &EventLoop<GuiEvent>,
        presenter: T,
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
            window,
            width: size.width,
            height: size.height,
            scale_factor,
            presenter,
            controller,
            ui_state: GuiAppState::default(),
            last_render_duration: None,
            last_error_message: None,
            egui_ctx,
            egui_state,
        }
    }

    pub fn render(&mut self, egui_output: egui::FullOutput) -> Result<(), pixels::Error> {
        self.presenter.render(egui_output, &self.egui_ctx, self.ui_state.latest_submitted_generation)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        if width == 0 || height == 0 {
            return;
        }

        self.presenter.resize(width, height);
    }

    pub fn submit_render_request_if_needed(&mut self) {
        if self.width < 1 || self.height < 1 {
            return;
        }

        let pixel_rect = match PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: (self.width as i32) - 1,
                y: (self.height as i32) - 1,
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

    pub fn update_ui(&mut self, window: &Window) -> egui::FullOutput {
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
                                for &kind in MandelbrotColourMapKinds::ALL {
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

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> (bool, bool) {
        let response = self.egui_state.on_window_event(window, event);
        (response.consumed, response.repaint)
    }

    pub fn run(mut self, event_loop: EventLoop<GuiEvent>) {
        event_loop
            .run(move |event, elwt| {
                match event {
                    Event::UserEvent(GuiEvent::Wake) => {
                        self.ui_state.redraw_pending = true;
                    }
                    Event::WindowEvent {
                        ref event,
                        window_id,
                    } if window_id == self.window.id() => {
                        // Forward event to egui first
                        let (egui_consumed, egui_repaint) = self.handle_window_event(self.window, event);

                        if egui_repaint {
                            self.ui_state.redraw_pending = true;
                        }

                        // If egui consumed the event, skip our handling
                        // (except for events we always need to handle)
                        match event {
                            WindowEvent::CloseRequested => {
                                self.controller.shutdown();
                                elwt.exit();
                            }
                            WindowEvent::RedrawRequested => {
                                self.ui_state.redraw_pending = false;

                                // Run egui frame
                                let egui_output = self.update_ui(self.window);
                                self.submit_render_request_if_needed();

                                // Handle egui platform output (e.g., clipboard, cursor changes)
                                self.egui_state.handle_platform_output(
                                    self.window,
                                    egui_output.platform_output.clone(),
                                );

                                // Check if egui wants a repaint
                                if egui_output
                                    .viewport_output
                                    .values()
                                    .any(|v| v.repaint_delay.is_zero())
                                {
                                    self.ui_state.redraw_pending = true;
                                }

                                // Render the frame with egui overlay
                                if let Err(e) = self.render(egui_output) {
                                    eprintln!("Render error: {e}");
                                    elwt.exit();
                                }
                            }
                            WindowEvent::Resized(size) => {
                                self.resize(size.width, size.height);
                                self.ui_state.redraw_pending = true;
                            }
                            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                                self.scale_factor = *scale_factor;
                                self.egui_ctx.set_pixels_per_point(*scale_factor as f32);
                                // Get the new physical size after scale factor change
                                let size = self.window.inner_size();
                                self.resize(size.width, size.height);
                                self.ui_state.redraw_pending = true;
                            }
                            _ => {}
                        }

                        // Suppress unused variable warning - consumed will be used
                        // when we add pan/zoom to avoid passing clicks through UI
                        let _ = egui_consumed;
                    }
                    Event::AboutToWait => {
                        // Only request redraw if state changed
                        if self.ui_state.redraw_pending {
                            self.window.request_redraw();
                        }
                    }
                    _ => {}
                }
            })
            .expect("Event loop error");
    }
}
