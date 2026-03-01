use crate::controllers::interactive::InteractiveController;
use crate::controllers::interactive::data::fractal_config::FractalConfig;
use crate::controllers::interactive::flight::{FlightSimulator, RenderScheduler, SchedulerAction};
use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::flight::{FlightLimits, FlightWarning};
use crate::core::fractals::fractal_kinds::FractalKinds;
use crate::core::fractals::julia::colour_mapping::kinds::JuliaColourMapKinds;
use crate::core::fractals::julia::flight as julia_flight;
use crate::core::fractals::mandelbrot::colour_mapping::kinds::MandelbrotColourMapKinds;
use crate::core::fractals::mandelbrot::flight as mandelbrot_flight;
use crate::input::gui::app::events::gui::GuiEvent;
use crate::input::gui::app::flight_input::FlightInputState;
use crate::input::gui::app::ports::presenter::GuiPresenterPort;
use crate::input::gui::app::state::GuiAppState;
use egui::Context;
use egui_winit::State as EguiWinitState;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::PhysicalKey,
    window::Window,
};

pub struct GuiApp<T: GuiPresenterPort> {
    window: &'static Window,
    width: u32,
    height: u32,
    pub scale_factor: f64,
    presenter: T,
    pub controller: InteractiveController,
    ui_state: GuiAppState,
    flight_input: FlightInputState,
    flight_sim: FlightSimulator,
    scheduler: RenderScheduler,
    last_redraw_instant: Instant,
    last_selected_fractal: FractalKinds,
    last_render_duration: Option<Duration>,
    last_error_message: Option<String>,
    pub egui_ctx: Context,
    pub egui_state: EguiWinitState,
}

impl<T: GuiPresenterPort> GuiApp<T> {
    pub fn new(
        window: &'static Window,
        event_loop: &EventLoop<GuiEvent>,
        presenter: T,
        controller: InteractiveController,
    ) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let egui_ctx = Context::default();
        let ui_state = GuiAppState::default();
        let last_selected_fractal = ui_state.selected_fractal;

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
            ui_state,
            flight_input: FlightInputState::default(),
            flight_sim: FlightSimulator::new(FlightLimits::default()),
            scheduler: RenderScheduler::new(),
            last_redraw_instant: Instant::now(),
            last_selected_fractal,
            last_render_duration: None,
            last_error_message: None,
            egui_ctx,
            egui_state,
        }
    }

    pub fn render(&mut self, egui_output: egui::FullOutput) -> Result<(), pixels::Error> {
        self.presenter.render(
            egui_output,
            &self.egui_ctx,
            self.ui_state.latest_submitted_generation,
        )
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        if width == 0 || height == 0 {
            return;
        }

        self.presenter.resize(width, height);
    }

    fn build_desired_request(&self) -> Option<Arc<FractalConfig>> {
        if self.width < 1 || self.height < 1 {
            return None;
        }

        let pixel_rect = match PixelRect::new(
            Point { x: 0, y: 0 },
            Point {
                x: (self.width as i32) - 1,
                y: (self.height as i32) - 1,
            },
        ) {
            Ok(rect) => rect,
            Err(_) => return None,
        };

        Some(Arc::new(self.ui_state.build_render_request(pixel_rect)))
    }

    fn warning_label(warning: FlightWarning) -> &'static str {
        match warning {
            FlightWarning::SpeedClamped => "Speed clamped",
            FlightWarning::CenterClamped => "Center clamped",
            FlightWarning::ExtentClamped => "Extent clamped",
            FlightWarning::NonFiniteReset => "Non-finite reset",
        }
    }

    fn update_flight_simulation(&mut self, elapsed: Duration, text_editing: bool) {
        let selected_fractal = self.ui_state.selected_fractal;
        let flight_input = &mut self.flight_input;
        let ui_state = &mut self.ui_state;

        let _ = self.flight_sim.advance(
            elapsed,
            || flight_input.snapshot(text_editing),
            |motion, dt, limits| match selected_fractal {
                FractalKinds::Mandelbrot => {
                    mandelbrot_flight::step_flight(&mut ui_state.mandelbrot, motion, dt, limits)
                }
                FractalKinds::Julia => {
                    julia_flight::step_flight(&mut ui_state.julia, motion, dt, limits)
                }
            },
        );
    }

    fn schedule_desired_request(&mut self, desired_request: Arc<FractalConfig>) {
        let action = self.scheduler.update(
            Arc::clone(&desired_request),
            self.flight_sim.is_active(),
            self.controller.last_completed_generation(),
            |request| self.controller.submit_request(request),
        );

        if let SchedulerAction::Submitted { generation } = action {
            self.ui_state.record_submission(desired_request, generation);
            self.last_error_message = None;
        }
    }

    pub fn update_ui(&mut self, window: &Window) -> egui::FullOutput {
        let raw_input = self.egui_state.take_egui_input(window);

        self.egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("Debug Panel")
                .default_pos([10.0, 10.0])
                .default_size([300.0, 320.0])
                .show(ctx, |ui| {
                    ui.heading("Fractal Explorer");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Fractal:");

                        egui::ComboBox::from_id_source("fractal_kind")
                            .selected_text(self.ui_state.selected_fractal.display_name())
                            .show_ui(ui, |ui| {
                                for &kind in FractalKinds::ALL {
                                    ui.selectable_value(
                                        &mut self.ui_state.selected_fractal,
                                        kind,
                                        kind.display_name(),
                                    );
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max iterations:");
                        match self.ui_state.selected_fractal {
                            FractalKinds::Mandelbrot => {
                                ui.add(egui::Slider::new(
                                    &mut self.ui_state.mandelbrot.max_iterations,
                                    1..=1000,
                                ));
                            }
                            FractalKinds::Julia => {
                                ui.add(egui::Slider::new(
                                    &mut self.ui_state.julia.max_iterations,
                                    1..=1000,
                                ));
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Colour map:");

                        match self.ui_state.selected_fractal {
                            FractalKinds::Mandelbrot => {
                                egui::ComboBox::from_id_source("fractal_colour_map")
                                    .selected_text(
                                        self.ui_state.mandelbrot.colour_map_kind.display_name(),
                                    )
                                    .show_ui(ui, |ui| {
                                        for &kind in MandelbrotColourMapKinds::ALL {
                                            ui.selectable_value(
                                                &mut self.ui_state.mandelbrot.colour_map_kind,
                                                kind,
                                                kind.display_name(),
                                            );
                                        }
                                    });
                            }
                            FractalKinds::Julia => {
                                egui::ComboBox::from_id_source("fractal_colour_map")
                                    .selected_text(
                                        self.ui_state.julia.colour_map_kind.display_name(),
                                    )
                                    .show_ui(ui, |ui| {
                                        for &kind in JuliaColourMapKinds::ALL {
                                            ui.selectable_value(
                                                &mut self.ui_state.julia.colour_map_kind,
                                                kind,
                                                kind.display_name(),
                                            );
                                        }
                                    });
                            }
                        }
                    });

                    ui.separator();
                    ui.label("View region:");

                    let active_region = self.ui_state.active_region();
                    let top_left = active_region.top_left();
                    let bottom_right = active_region.bottom_right();

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

                    ui.separator();
                    ui.heading("Flight");

                    let flight_status = self.flight_sim.status();
                    let activity_label = if flight_status.paused {
                        "Paused"
                    } else if self.flight_sim.is_active() {
                        "Active"
                    } else {
                        "Idle"
                    };

                    ui.label(format!("Status: {}", activity_label));
                    ui.label(format!("Speed: {:.2} zoom/s", flight_status.speed));
                    ui.label(format!(
                        "Heading: ({:.2}, {:.2})",
                        flight_status.heading[0], flight_status.heading[1]
                    ));
                    ui.label(format!(
                        "Extent: w={:.4}, h={:.4}",
                        active_region.width(),
                        active_region.height()
                    ));

                    if let Some(warning) = flight_status.last_warning {
                        ui.label(format!("Warning: {}", Self::warning_label(warning)));
                    }

                    if let Some(in_flight_generation) = self.scheduler.in_flight_generation() {
                        ui.label(format!("In-flight gen: {}", in_flight_generation));
                    }
                    ui.label(format!(
                        "Scheduler pending: {}",
                        self.scheduler.has_pending()
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
                        let (egui_consumed, egui_repaint) =
                            self.handle_window_event(self.window, event);

                        if egui_repaint {
                            self.ui_state.redraw_pending = true;
                        }

                        if let WindowEvent::KeyboardInput { event, .. } = event {
                            if let PhysicalKey::Code(key_code) = event.physical_key {
                                self.flight_input.handle_key_event(key_code, event.state);
                            }
                        }

                        match event {
                            WindowEvent::CloseRequested => {
                                self.controller.shutdown();
                                elwt.exit();
                            }
                            WindowEvent::RedrawRequested => {
                                self.ui_state.redraw_pending = false;
                                self.scheduler.observe_completion(
                                    self.controller.last_completed_generation(),
                                );

                                let egui_output = self.update_ui(self.window);

                                if self.ui_state.selected_fractal != self.last_selected_fractal {
                                    self.flight_sim.reset_motion();
                                    self.flight_input.reset();
                                    self.scheduler.reset();
                                    self.last_selected_fractal = self.ui_state.selected_fractal;
                                }

                                let now = Instant::now();
                                let elapsed =
                                    now.saturating_duration_since(self.last_redraw_instant);
                                self.last_redraw_instant = now;

                                let text_editing = self.egui_ctx.wants_keyboard_input();
                                self.update_flight_simulation(elapsed, text_editing);

                                let mut request_to_schedule: Option<Arc<FractalConfig>> = None;
                                if let Some(desired_request) = self.build_desired_request() {
                                    let request_changed =
                                        self.ui_state.should_submit(desired_request.as_ref());
                                    let should_schedule =
                                        request_changed || self.scheduler.has_pending();

                                    if should_schedule {
                                        request_to_schedule = Some(desired_request);
                                    }
                                }

                                self.ui_state.redraw_pending =
                                    self.flight_sim.is_active() || self.scheduler.has_pending();

                                self.egui_state.handle_platform_output(
                                    self.window,
                                    egui_output.platform_output.clone(),
                                );

                                if egui_output
                                    .viewport_output
                                    .values()
                                    .any(|v| v.repaint_delay.is_zero())
                                {
                                    self.ui_state.redraw_pending = true;
                                }

                                if let Err(e) = self.render(egui_output) {
                                    eprintln!("Render error: {e}");
                                    elwt.exit();
                                }

                                if let Some(desired_request) = request_to_schedule {
                                    self.schedule_desired_request(desired_request);
                                    self.ui_state.redraw_pending = true;
                                }

                                self.ui_state.redraw_pending |=
                                    self.flight_sim.is_active() || self.scheduler.has_pending();
                            }
                            WindowEvent::Resized(size) => {
                                self.resize(size.width, size.height);
                                self.ui_state.redraw_pending = true;
                            }
                            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                                self.scale_factor = *scale_factor;
                                self.egui_ctx.set_pixels_per_point(*scale_factor as f32);

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
