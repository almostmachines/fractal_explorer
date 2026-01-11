//! Main GUI application loop.

use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

/// Runs the GUI application.
///
/// This function does not return until the window is closed.
pub fn run_gui() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Fractal Explorer")
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_min_inner_size(LogicalSize::new(200.0, 200.0))
        .build(&event_loop)
        .expect("Failed to create window");

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
                            // Rendering will happen here in future milestones
                        }
                        WindowEvent::Resized(_) => {
                            // Mark for redraw on resize
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
