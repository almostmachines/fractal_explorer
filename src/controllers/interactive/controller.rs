//! Interactive controller for real-time fractal rendering.
//!
//! This controller manages the render loop, handling user input
//! events and dispatching rendered frames to the presentation layer.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::core::actions::generate_fractal::generate_fractal_parallel_rayon::generate_fractal_parallel_rayon;
use crate::core::actions::generate_pixel_buffer::generate_pixel_buffer::generate_pixel_buffer;
use crate::core::data::pixel_buffer::PixelBuffer;
use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;
use crate::core::fractals::mandelbrot::colour_maps::blue_white_gradient::MandelbrotBlueWhiteGradient;

use super::ports::{FrameMessage, FrameSink, RenderErrorMessage, RenderEvent};
use super::types::{ColourSchemeKind, FractalKind, FractalParams, RenderRequest};

/// Shared state between the controller and its worker thread.
///
/// This struct contains all synchronization primitives needed for
/// thread-safe communication between the GUI thread and the render worker.
struct SharedState {
    /// Monotonically increasing counter for request versioning.
    /// Each new request increments this, allowing the worker to detect stale work.
    generation: AtomicU64,

    /// Slot holding the most recent render request.
    /// Tuple contains (generation_at_submit, request).
    /// Uses Option to allow empty slot when no work is pending.
    latest_request: Mutex<Option<(u64, RenderRequest)>>,

    /// Condition variable to wake the worker when:
    /// - A new request arrives
    /// - Shutdown is requested
    wake: Condvar,

    /// Flag to signal graceful shutdown to the worker thread.
    shutdown: AtomicBool,

    /// Output port for delivering rendered frames to the presentation layer.
    frame_sink: Arc<dyn FrameSink>,
}

/// Interactive controller for real-time fractal rendering.
///
/// Manages the render lifecycle:
/// - Accepts a `FrameSink` for output
/// - Processes `RenderRequest` inputs via `submit_request()`
/// - Coordinates parallel rendering on a background worker thread
/// - Uses generation IDs to suppress stale frames (soft cancellation)
///
/// # Threading Model
///
/// The controller spawns a dedicated worker thread that:
/// 1. Waits for new requests or shutdown signal
/// 2. Takes the most recent request (coalescing rapid requests)
/// 3. Renders using core domain actions
/// 4. Checks if the result is still current before emitting
///
/// # Example
///
/// ```ignore
/// let sink: Arc<dyn FrameSink> = /* ... */;
/// let controller = InteractiveController::new(sink);
///
/// // Submit render requests (returns generation ID)
/// let gen = controller.submit_request(request);
///
/// // When done, shutdown gracefully
/// controller.shutdown();
/// ```
pub struct InteractiveController {
    /// Shared state accessible by both the controller and worker thread.
    shared: Arc<SharedState>,

    /// Handle to join the worker thread on shutdown.
    /// Option because it's taken (and joined) during shutdown.
    worker: Option<JoinHandle<()>>,
}

impl InteractiveController {
    /// Creates a new interactive controller with the given frame sink.
    ///
    /// Spawns a background worker thread that will process render requests
    /// and deliver results via the provided `FrameSink`.
    ///
    /// # Arguments
    ///
    /// * `frame_sink` - Output port for receiving rendered frames
    ///
    /// # Returns
    ///
    /// A new controller ready to accept render requests via `submit_request()`.
    pub fn new(frame_sink: Arc<dyn FrameSink>) -> Self {
        let shared = Arc::new(SharedState {
            generation: AtomicU64::new(0),
            latest_request: Mutex::new(None),
            wake: Condvar::new(),
            shutdown: AtomicBool::new(false),
            frame_sink,
        });

        // Clone Arc for the worker thread
        let worker_shared = Arc::clone(&shared);

        let worker = thread::spawn(move || {
            Self::worker_loop(&worker_shared);
        });

        Self {
            shared,
            worker: Some(worker),
        }
    }

    /// Submits a render request to be processed by the worker thread.
    ///
    /// This method is non-blocking. If a previous request is still pending,
    /// it will be superseded (overwrite semantics / request coalescing).
    ///
    /// # Arguments
    ///
    /// * `request` - The render parameters to use
    ///
    /// # Returns
    ///
    /// The generation ID assigned to this request. Can be used to correlate
    /// with rendered frames (though frames may be silently dropped if superseded).
    pub fn submit_request(&self, request: RenderRequest) -> u64 {
        // Increment generation atomically and get the new value
        let generation = self.shared.generation.fetch_add(1, Ordering::SeqCst) + 1;

        // Store the request in the slot (overwriting any pending request)
        {
            let mut guard = self.shared.latest_request.lock().unwrap();
            *guard = Some((generation, request));
        }

        // Wake the worker thread
        self.shared.wake.notify_one();

        generation
    }

    /// Shuts down the controller gracefully.
    ///
    /// Signals the worker thread to stop and waits for it to finish
    /// processing any in-flight work. After this call, the controller
    /// should not be used.
    pub fn shutdown(&mut self) {
        // Signal shutdown
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.wake.notify_one();

        // Wait for worker to finish
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }

    /// The main worker loop that processes render requests.
    ///
    /// Runs until shutdown is signaled. Each iteration:
    /// 1. Waits for a request or shutdown signal
    /// 2. Takes the latest request (coalescing multiple rapid requests)
    /// 3. Performs the render
    /// 4. Checks if result is still current before emitting
    fn worker_loop(shared: &Arc<SharedState>) {
        loop {
            // Wait for work or shutdown
            let (job_generation, request) = {
                let mut guard = shared.latest_request.lock().unwrap();
                loop {
                    // Check shutdown first
                    if shared.shutdown.load(Ordering::Acquire) {
                        return;
                    }
                    // Try to take a request
                    if let Some(req) = guard.take() {
                        break req;
                    }
                    // Wait for notification (releases lock while waiting)
                    guard = shared.wake.wait(guard).unwrap();
                }
            };

            // Perform the render (outside the lock)
            let start = Instant::now();
            let result = Self::render_request(&request);
            let render_duration = start.elapsed();

            // Check if this job has been superseded before emitting
            let current_gen = shared.generation.load(Ordering::Acquire);
            if job_generation != current_gen {
                // A newer request arrived; discard this result
                continue;
            }

            // Emit the result
            match result {
                Ok(pixel_buffer) => {
                    shared.frame_sink.submit(RenderEvent::Frame(FrameMessage {
                        generation: job_generation,
                        pixel_rect: request.pixel_rect,
                        pixel_buffer,
                        render_duration,
                    }));
                }
                Err(message) => {
                    shared
                        .frame_sink
                        .submit(RenderEvent::Error(RenderErrorMessage {
                            generation: job_generation,
                            message,
                        }));
                }
            }
        }
    }

    /// Performs the actual rendering based on the request parameters.
    ///
    /// Returns the RGB pixel buffer on success.
    fn render_request(request: &RenderRequest) -> Result<PixelBuffer, String> {
        // Validate request
        let width = request.pixel_rect.width();
        let height = request.pixel_rect.height();

        if width < 2 || height < 2 {
            return Err(format!("Invalid dimensions: {}x{}", width, height));
        }

        // Dispatch based on fractal type
        match (&request.fractal, &request.params) {
            (
                FractalKind::Mandelbrot,
                FractalParams::Mandelbrot {
                    region,
                    max_iterations,
                },
            ) => {
                if *max_iterations == 0 {
                    return Err("max_iterations must be greater than 0".to_string());
                }

                // Create the algorithm
                let algorithm =
                    MandelbrotAlgorithm::new(request.pixel_rect, *region, *max_iterations)
                        .map_err(|e| e.to_string())?;

                // Generate iteration counts
                let fractal = generate_fractal_parallel_rayon(request.pixel_rect, &algorithm)
                    .map_err(|e| e.to_string())?;

                // Select colour map based on scheme
                let pixel_buffer = match request.colour_scheme {
                    ColourSchemeKind::BlueWhiteGradient => {
                        let colour_map = MandelbrotBlueWhiteGradient::new(*max_iterations);
                        generate_pixel_buffer(fractal, &colour_map, request.pixel_rect)
                            .map_err(|e| e.to_string())?
                    }
                };

                Ok(pixel_buffer)
            }
        }
    }
}

impl Drop for InteractiveController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::core::data::complex::Complex;
    use crate::core::data::complex_rect::ComplexRect;
    use crate::core::data::pixel_rect::PixelRect;
    use crate::core::data::point::Point;

    #[derive(Default)]
    struct MockFrameSink {
        events: Mutex<Vec<RenderEvent>>,
    }

    impl MockFrameSink {
        fn take_events(&self) -> Vec<RenderEvent> {
            let mut guard = self.events.lock().unwrap();
            std::mem::take(&mut *guard)
        }
    }

    impl FrameSink for MockFrameSink {
        fn submit(&self, event: RenderEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn wait_for_events(sink: &MockFrameSink, timeout: Duration) -> Vec<RenderEvent> {
        let start = Instant::now();
        loop {
            let events = sink.take_events();
            if !events.is_empty() {
                return events;
            }
            if start.elapsed() >= timeout {
                return events;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn create_test_request(pixel_rect: PixelRect) -> RenderRequest {
        let region = ComplexRect::new(
            Complex {
                real: -2.5,
                imag: -1.0,
            },
            Complex {
                real: 1.0,
                imag: 1.0,
            },
        )
        .expect("test region is valid");

        RenderRequest {
            pixel_rect,
            fractal: FractalKind::Mandelbrot,
            params: FractalParams::Mandelbrot {
                region,
                max_iterations: 10,
            },
            colour_scheme: ColourSchemeKind::BlueWhiteGradient,
        }
    }

    #[test]
    fn test_submit_request_emits_frame() {
        let frame_sink = Arc::new(MockFrameSink::default());
        let mut controller =
            InteractiveController::new(Arc::clone(&frame_sink) as Arc<dyn FrameSink>);

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = create_test_request(pixel_rect);

        let generation = controller.submit_request(request.clone());
        let events = wait_for_events(frame_sink.as_ref(), Duration::from_secs(2));
        assert!(!events.is_empty(), "expected a render event");

        let mut saw_frame = false;
        for event in events {
            match event {
                RenderEvent::Frame(frame) => {
                    assert_eq!(frame.generation, generation);
                    assert!(generation > 0, "generation should be non-zero");
                    assert_eq!(frame.pixel_rect, request.pixel_rect);
                    assert_eq!(
                        frame.pixel_buffer.buffer().len(),
                        (request.pixel_rect.width() * request.pixel_rect.height() * 3) as usize
                    );
                    saw_frame = true;
                }
                RenderEvent::Error(error) => {
                    panic!("unexpected render error: {}", error.message);
                }
            }
        }

        assert!(saw_frame, "expected a frame event");
        controller.shutdown();
    }
}
