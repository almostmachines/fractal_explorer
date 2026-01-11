//! Main GUI application loop.

use egui::Context;
use egui_winit::State as EguiWinitState;
use egui_wgpu::Renderer as EguiRenderer;
use pixels::{wgpu, Pixels, SurfaceTexture};
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
    /// egui-wgpu renderer for drawing UI on top of pixels.
    egui_renderer: EguiRenderer,
    /// Test slider value to demonstrate UI interactivity.
    test_slider_value: f32,
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

        // Initialize egui-wgpu renderer, sharing device with pixels
        let egui_renderer = EguiRenderer::new(
            pixels.device(),
            pixels.render_texture_format(),
            None, // depth format
            1,    // msaa samples
        );

        Self {
            pixels,
            width: size.width,
            height: size.height,
            scale_factor,
            focused: true,
            egui_ctx,
            egui_state,
            egui_renderer,
            test_slider_value: 0.5,
        }
    }

    /// Draws a checkerboard pattern to prove the rendering pipeline works.
    /// The slider value controls the red channel tint.
    fn draw_placeholder(&mut self) {
        let frame = self.pixels.frame_mut();
        let width = self.width as usize;
        let tile_size = 32;

        // Use slider to control red tint (0.0 = no red, 1.0 = full red)
        let red_tint = (self.test_slider_value * 255.0) as u8;

        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = i % width;
            let y = i / width;

            let tile_x = x / tile_size;
            let tile_y = y / tile_size;
            let is_dark = (tile_x + tile_y) % 2 == 0;

            let base = if is_dark { 60 } else { 200 };
            pixel[0] = base.max(red_tint); // R (tinted by slider)
            pixel[1] = base; // G
            pixel[2] = base; // B
            pixel[3] = 255; // A (opaque)
        }
    }

    /// Renders the current frame to the window, including egui overlay.
    fn render(&mut self, egui_output: egui::FullOutput) -> Result<(), pixels::Error> {
        // Skip rendering for invalid size (e.g., minimized window)
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        self.draw_placeholder();

        // Tessellate egui shapes into primitives
        let clipped_primitives = self
            .egui_ctx
            .tessellate(egui_output.shapes, self.egui_ctx.pixels_per_point());

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.width, self.height],
            pixels_per_point: self.egui_ctx.pixels_per_point(),
        };

        // Render with egui overlay
        let textures_delta = egui_output.textures_delta;
        self.pixels.render_with(|encoder, render_target, context| {
            // First, render the pixels framebuffer (the scaling pass)
            context.scaling_renderer.render(encoder, render_target);

            // Upload new/changed egui textures
            for (id, delta) in &textures_delta.set {
                self.egui_renderer
                    .update_texture(&context.device, &context.queue, *id, delta);
            }

            // Update egui buffers (vertices, indices)
            self.egui_renderer.update_buffers(
                &context.device,
                &context.queue,
                encoder,
                &clipped_primitives,
                &screen_descriptor,
            );

            // Render egui on top of pixels framebuffer
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Keep pixels content
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });

                self.egui_renderer
                    .render(&mut render_pass, &clipped_primitives, &screen_descriptor);
            }

            // Free textures no longer needed
            for id in &textures_delta.free {
                self.egui_renderer.free_texture(id);
            }

            Ok(())
        })
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
            egui::Window::new("Debug Panel")
                .default_pos([10.0, 10.0])
                .default_size([200.0, 100.0])
                .show(ctx, |ui| {
                    ui.heading("Fractal Explorer");
                    ui.separator();

                    ui.label("Test slider:");
                    ui.add(
                        egui::Slider::new(&mut self.test_slider_value, 0.0..=1.0).text("value"),
                    );

                    ui.separator();
                    ui.label(format!("Window size: {}x{}", self.width, self.height));
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
                    let (egui_consumed, egui_repaint) = app.handle_window_event(window, event);

                    // Request redraw if egui wants one (e.g., hover state changed)
                    if egui_repaint {
                        redraw_pending = true;
                    }

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
                                .handle_platform_output(window, egui_output.platform_output.clone());

                            // Check if egui wants a repaint
                            if egui_output.viewport_output.values().any(|v| v.repaint_delay.is_zero()) {
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
