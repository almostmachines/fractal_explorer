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
use crate::input::gui::app::frame_overlay::FrameOverlay;
use crate::input::gui::app::flight_input::FlightInputState;
use crate::input::gui::app::ports::presenter::GuiPresenterPort;
use crate::input::gui::app::state::GuiAppState;
use egui::{Color32, Context, Rounding, Stroke};
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
        configure_egui_style(&egui_ctx);
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
        let frame_overlay = self.build_frame_overlay();
        self.presenter.render(
            egui_output,
            &self.egui_ctx,
            self.ui_state.latest_submitted_generation,
            &frame_overlay,
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

    fn build_frame_overlay(&self) -> FrameOverlay {
        build_frame_overlay_from_paused(self.flight_sim.status().paused)
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
            egui::Window::new("Settings")
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
                                    1..=10000,
                                ));
                            }
                            FractalKinds::Julia => {
                                ui.add(egui::Slider::new(
                                    &mut self.ui_state.julia.max_iterations,
                                    1..=10000,
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
                        "Real: [{:.8}, {:.8}]",
                        top_left.real, bottom_right.real
                    ));

                    ui.label(format!(
                        "Imag: [{:.8}, {:.8}]",
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
                        "Extent: w={:.8}, h={:.8}",
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
                                self.ui_state.redraw_pending = true;
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

fn build_frame_overlay_from_paused(paused: bool) -> FrameOverlay {
    FrameOverlay { paused }
}

fn configure_egui_style(ctx: &Context) {
    // Colours drawn from the PAUSED overlay palette:
    //   [88,6,0]  [168,30,0]  [230,88,8]  [255,166,48]  [255,232,180]
    // Backplate: [6,4,10] at ~69% opacity
    let amber = Color32::from_rgb(255, 166, 48);
    let bright_amber = Color32::from_rgb(255, 232, 180);
    let deep_orange = Color32::from_rgb(230, 88, 8);
    let dark_orange = Color32::from_rgb(168, 30, 0);
    let darkest = Color32::from_rgb(88, 6, 0);

    let backplate = Color32::from_rgba_premultiplied(6, 4, 10, 200);
    let backplate_solid = Color32::from_rgb(16, 12, 24);

    let mut visuals = egui::Visuals::dark();

    // Window chrome
    visuals.window_fill = backplate;
    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_shadow = egui::epaint::Shadow {
        extrusion: 12.0,
        color: Color32::from_black_alpha(120),
    };
    visuals.window_stroke = Stroke::new(1.0, dark_orange);

    // Panel backgrounds
    visuals.panel_fill = backplate;

    // Selection highlight
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(230, 88, 8, 100);
    visuals.selection.stroke = Stroke::new(1.0, bright_amber);

    // Widget states — inactive
    visuals.widgets.inactive.bg_fill = Color32::from_rgba_premultiplied(88, 6, 0, 80);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgba_premultiplied(168, 30, 0, 120));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, amber);
    visuals.widgets.inactive.rounding = Rounding::same(4.0);

    // Widget states — hovered
    visuals.widgets.hovered.bg_fill = Color32::from_rgba_premultiplied(168, 30, 0, 120);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, deep_orange);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, bright_amber);
    visuals.widgets.hovered.rounding = Rounding::same(4.0);

    // Widget states — active (pressed)
    visuals.widgets.active.bg_fill = Color32::from_rgba_premultiplied(230, 88, 8, 160);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, amber);
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, bright_amber);
    visuals.widgets.active.rounding = Rounding::same(4.0);

    // Widget states — open (e.g. open combo box)
    visuals.widgets.open.bg_fill = Color32::from_rgba_premultiplied(168, 30, 0, 140);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, deep_orange);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, bright_amber);
    visuals.widgets.open.rounding = Rounding::same(4.0);

    // Non-interactive widgets (labels, separators)
    visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 166, 48, 60));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, amber);
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);

    // Override text colour globally to warm amber
    visuals.override_text_color = Some(amber);

    // Popup menus (combo box dropdowns)
    visuals.popup_shadow = egui::epaint::Shadow {
        extrusion: 8.0,
        color: Color32::from_black_alpha(100),
    };

    // Hyperlinks and special text
    visuals.hyperlink_color = bright_amber;
    visuals.warn_fg_color = deep_orange;
    visuals.error_fg_color = darkest;

    // Extreme background (behind popups, dropdown area)
    visuals.extreme_bg_color = backplate_solid;

    // Faint background (text edit fields, sliders track)
    visuals.faint_bg_color = Color32::from_rgba_premultiplied(20, 14, 30, 180);

    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::build_frame_overlay_from_paused;
    use crate::input::gui::app::frame_overlay::FrameOverlay;

    #[test]
    fn build_frame_overlay_reflects_paused_state() {
        assert_eq!(
            build_frame_overlay_from_paused(true),
            FrameOverlay { paused: true }
        );
        assert_eq!(
            build_frame_overlay_from_paused(false),
            FrameOverlay { paused: false }
        );
    }
}
