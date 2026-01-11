//! Main GUI application loop.

use egui::Context;
use egui_winit::State as EguiWinitState;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

/// Application state holding the pixels framebuffer and egui context.
struct App {
    pixels: Pixels<'static>,
    width: u32,
    height: u32,
    scale_factor: f64,
    /// Whether the window is focused. Can be used to reduce render rate when unfocused.
    #[allow(dead_code)]
    focused: bool,
    /// egui context for immediate mode UI.
    egui_ctx: Context,
    /// egui-winit state for input handling.
    egui_state: EguiWinitState,
}

impl App {
    /// Creates a new App with a pixels surface tied to the window.
    fn new(window: &'static Window, event_loop: &EventLoop<()>) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window);
        let pixels = Pixels::new(size.width, size.height, surface_texture)
            .expect("Failed to create pixels surface");

        // Initialize egui
        let egui_ctx = Context::default();
        let egui_state = EguiWinitState::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            event_loop,
            Some(scale_factor as f32),
            None, // max_texture_side, use default
        );

        Self {
            pixels,
            width: size.width,
            height: size.height,
            scale_factor,
            focused: true,
            egui_ctx,
            egui_state,
        }
    }

    /// Draws a checkerboard pattern to prove the rendering pipeline works.
    fn draw_placeholder(&mut self) {
        let frame = self.pixels.frame_mut();
        let width = self.width as usize;
        let tile_size = 32;

        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = i % width;
            let y = i / width;

            let tile_x = x / tile_size;
            let tile_y = y / tile_size;
            let is_dark = (tile_x + tile_y) % 2 == 0;

            let color = if is_dark { 60 } else { 200 };
            pixel[0] = color; // R
            pixel[1] = color; // G
            pixel[2] = color; // B
            pixel[3] = 255; // A (opaque)
        }
    }

    /// Renders the current frame to the window.
    fn render(&mut self) -> Result<(), pixels::Error> {
        // Skip rendering for invalid size (e.g., minimized window)
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }
        self.draw_placeholder();
        self.pixels.render()
    }

    /// Handles window resize by recreating the pixels surface.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.pixels
                .resize_surface(width, height)
                .expect("Failed to resize surface");
            self.pixels
                .resize_buffer(width, height)
                .expect("Failed to resize buffer");
        }
    }

    /// Runs the egui frame and returns the output.
    ///
    /// This gathers input from egui-winit, runs the UI logic, and returns
    /// the output which contains paint commands and platform output.
    fn update_ui(&mut self, window: &Window) -> egui::FullOutput {
        let raw_input = self.egui_state.take_egui_input(window);

        self.egui_ctx.run(raw_input, |ctx| {
            // Minimal debug window to prove egui is working
            egui::Window::new("Debug").show(ctx, |ui| {
                ui.label("egui is working!");
                ui.label(format!("Window size: {}x{}", self.width, self.height));
            });
        })
    }

    /// Handles a window event, forwarding it to egui first.
    ///
    /// Returns true if egui consumed the event (e.g., click on UI element).
    fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.egui_state.on_window_event(window, event);
        response.consumed
    }
}

/// Runs the GUI application.
///
/// This function does not return until the window is closed.
pub fn run_gui() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    // Leak the window to get a 'static reference for pixels
    let window: &'static Window = Box::leak(Box::new(
        WindowBuilder::new()
            .with_title("Fractal Explorer")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_min_inner_size(LogicalSize::new(200.0, 200.0))
            .build(&event_loop)
            .expect("Failed to create window"),
    ));

    let mut app = App::new(window, &event_loop);

    // Track whether we need to redraw
    let mut redraw_pending = true;

    event_loop
        .run(|event, elwt| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    // Forward event to egui first
                    let egui_consumed = app.handle_window_event(window, event);

                    // If egui consumed the event, skip our handling
                    // (except for events we always need to handle)
                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::RedrawRequested => {
                            redraw_pending = false;

                            // Run egui frame
                            let egui_output = app.update_ui(window);

                            // Handle egui platform output (e.g., clipboard, cursor changes)
                            app.egui_state
                                .handle_platform_output(window, egui_output.platform_output);

                            // Check if egui wants a repaint
                            if egui_output.viewport_output.values().any(|v| v.repaint_delay.is_zero()) {
                                redraw_pending = true;
                            }

                            // Render the frame (egui rendering will be integrated later)
                            if let Err(e) = app.render() {
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
                        _ => {
                            // For other events, request redraw if egui consumed them
                            // (indicates UI state changed)
                            if egui_consumed {
                                redraw_pending = true;
                            }
                        }
                    }
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
