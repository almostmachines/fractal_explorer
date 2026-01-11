//! Main GUI application loop.

use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

/// Application state holding the pixels framebuffer.
struct App {
    pixels: Pixels<'static>,
    width: u32,
    height: u32,
}

impl App {
    /// Creates a new App with a pixels surface tied to the window.
    fn new(window: &'static Window) -> Self {
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window);
        let pixels = Pixels::new(size.width, size.height, surface_texture)
            .expect("Failed to create pixels surface");

        Self {
            pixels,
            width: size.width,
            height: size.height,
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

    let mut app = App::new(window);

    // Track whether we need to redraw
    let mut redraw_pending = true;

    event_loop
        .run(|event, elwt| {
            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::RedrawRequested => {
                            redraw_pending = false;
                            if let Err(e) = app.render() {
                                eprintln!("Render error: {e}");
                                elwt.exit();
                            }
                        }
                        WindowEvent::Resized(size) => {
                            app.resize(size.width, size.height);
                            redraw_pending = true;
                        }
                        _ => {}
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
