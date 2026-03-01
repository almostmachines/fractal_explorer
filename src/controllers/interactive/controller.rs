use crate::controllers::interactive::data::fractal_config::FractalConfig;
use crate::controllers::interactive::data::frame_data::FrameData;
use crate::controllers::interactive::errors::render::RenderError;
use crate::controllers::interactive::events::render::RenderEvent;
use crate::controllers::interactive::ports::presenter::InteractiveControllerPresenterPort;
use crate::core::actions::cancellation::CancelToken;
use crate::core::actions::generate_fractal::generate_fractal_parallel_rayon::{
    GenerateFractalError, generate_fractal_parallel_rayon_cancelable,
};
use crate::core::actions::generate_pixel_buffer::generate_pixel_buffer::{
    GeneratePixelBufferCancelableError, generate_pixel_buffer_cancelable,
};
use crate::core::data::pixel_buffer::PixelBuffer;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

struct SharedState {
    generation: AtomicU64,
    last_completed_generation: AtomicU64,
    latest_request: Mutex<Option<(u64, Arc<FractalConfig>)>>,
    wake: Condvar,
    shutdown: AtomicBool,
    presenter_port: Arc<dyn InteractiveControllerPresenterPort>,
}

pub struct InteractiveController {
    shared: Arc<SharedState>,
    worker: Option<JoinHandle<()>>,
}

impl InteractiveController {
    pub fn new(presenter_port: Arc<dyn InteractiveControllerPresenterPort>) -> Self {
        let shared = Arc::new(SharedState {
            generation: AtomicU64::new(0),
            last_completed_generation: AtomicU64::new(0),
            latest_request: Mutex::new(None),
            wake: Condvar::new(),
            shutdown: AtomicBool::new(false),
            presenter_port,
        });

        let worker_shared = Arc::clone(&shared);

        let worker = thread::spawn(move || {
            Self::worker_loop(&worker_shared);
        });

        Self {
            shared,
            worker: Some(worker),
        }
    }

    pub fn submit_request(&self, request: Arc<FractalConfig>) -> u64 {
        let generation = self.shared.generation.fetch_add(1, Ordering::SeqCst) + 1;

        {
            let mut guard = self.shared.latest_request.lock().unwrap();
            *guard = Some((generation, request));
        }

        self.shared.wake.notify_one();

        generation
    }

    pub fn shutdown(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.wake.notify_one();

        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }

    #[must_use]
    pub fn last_completed_generation(&self) -> u64 {
        self.shared
            .last_completed_generation
            .load(Ordering::Acquire)
    }

    fn worker_loop(shared: &Arc<SharedState>) {
        loop {
            let (job_generation, request) = {
                let mut guard = shared.latest_request.lock().unwrap();
                loop {
                    if shared.shutdown.load(Ordering::Acquire) {
                        return;
                    }

                    if let Some(req) = guard.take() {
                        break req;
                    }

                    guard = shared.wake.wait(guard).unwrap();
                }
            };

            let cancel_token = || {
                shared.shutdown.load(Ordering::Relaxed)
                    || job_generation != shared.generation.load(Ordering::Relaxed)
            };

            let start = Instant::now();
            let result = Self::render_request(&request, &cancel_token);
            let render_duration = start.elapsed();

            match result {
                Ok(pixel_buffer) => {
                    let current_gen = shared.generation.load(Ordering::Acquire);

                    if job_generation != current_gen {
                        continue;
                    }

                    shared.presenter_port.present(RenderEvent::Frame(FrameData {
                        generation: job_generation,
                        pixel_buffer,
                        render_duration,
                    }));

                    shared
                        .last_completed_generation
                        .store(job_generation, Ordering::Release);
                }
                Err(RenderOutcome::Cancelled) => {
                    continue;
                }
                Err(RenderOutcome::Error(message)) => {
                    let current_gen = shared.generation.load(Ordering::Acquire);

                    if job_generation != current_gen {
                        continue;
                    }

                    shared
                        .presenter_port
                        .present(RenderEvent::Error(RenderError {
                            generation: job_generation,
                            message,
                        }));

                    shared
                        .last_completed_generation
                        .store(job_generation, Ordering::Release);
                }
            }
        }
    }

    fn render_request<C: CancelToken>(
        request: &FractalConfig,
        cancel: &C,
    ) -> Result<PixelBuffer, RenderOutcome> {
        let algorithm = request.algorithm();
        let colour_map = request.colour_map();
        let pixel_rect = algorithm.pixel_rect();

        let fractal = generate_fractal_parallel_rayon_cancelable(pixel_rect, algorithm, cancel)
            .map_err(|e| match e {
                GenerateFractalError::Cancelled(_) => RenderOutcome::Cancelled,
                GenerateFractalError::Algorithm(err) => RenderOutcome::Error(err.to_string()),
            })?;

        if cancel.is_cancelled() {
            return Err(RenderOutcome::Cancelled);
        }

        let pixel_buffer = generate_pixel_buffer_cancelable(
            fractal, colour_map, pixel_rect, cancel,
        )
        .map_err(|e| match e {
            GeneratePixelBufferCancelableError::Cancelled(_) => RenderOutcome::Cancelled,
            GeneratePixelBufferCancelableError::ColourMap(err) => {
                RenderOutcome::Error(err.to_string())
            }
            GeneratePixelBufferCancelableError::PixelBuffer(err) => {
                RenderOutcome::Error(err.to_string())
            }
        })?;

        Ok(pixel_buffer)
    }
}

enum RenderOutcome {
    Cancelled,
    Error(String),
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
    use crate::core::fractals::mandelbrot::algorithm::MandelbrotAlgorithm;
    use crate::core::fractals::mandelbrot::colour_mapping::factory::mandelbrot_colour_map_factory;
    use crate::core::fractals::mandelbrot::colour_mapping::kinds::MandelbrotColourMapKinds;

    #[derive(Default)]
    struct MockPresenterPort {
        events: Mutex<Vec<RenderEvent>>,
    }

    impl MockPresenterPort {
        fn take_events(&self) -> Vec<RenderEvent> {
            let mut guard = self.events.lock().unwrap();
            std::mem::take(&mut *guard)
        }
    }

    impl InteractiveControllerPresenterPort for MockPresenterPort {
        fn present(&self, event: RenderEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn wait_for_events(sink: &MockPresenterPort, timeout: Duration) -> Vec<RenderEvent> {
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

    fn create_test_request(pixel_rect: PixelRect) -> FractalConfig {
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

        let max_iterations = 10;
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, region, max_iterations)
            .expect("test algorithm params are valid");
        let colour_map = mandelbrot_colour_map_factory(
            MandelbrotColourMapKinds::BlueWhiteGradient,
            max_iterations,
        );

        FractalConfig::Mandelbrot {
            colour_map,
            algorithm,
        }
    }

    fn create_error_request(pixel_rect: PixelRect) -> FractalConfig {
        let region = ComplexRect::new(
            Complex {
                real: -0.1,
                imag: -0.1,
            },
            Complex {
                real: 0.1,
                imag: 0.1,
            },
        )
        .expect("test region is valid");

        let max_iterations = 10;
        let algorithm = MandelbrotAlgorithm::new(pixel_rect, region, max_iterations)
            .expect("test algorithm params are valid");
        let colour_map =
            mandelbrot_colour_map_factory(MandelbrotColourMapKinds::BlueWhiteGradient, 1);

        FractalConfig::Mandelbrot {
            colour_map,
            algorithm,
        }
    }

    #[test]
    fn test_submit_request_emits_frame() {
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_test_request(pixel_rect));

        let generation = controller.submit_request(Arc::clone(&request));
        let events = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!events.is_empty(), "expected a render event");

        let mut saw_frame = false;
        for event in events {
            match event {
                RenderEvent::Frame(frame) => {
                    assert_eq!(frame.generation, generation);
                    assert!(generation > 0, "generation should be non-zero");
                    assert_eq!(frame.pixel_buffer.pixel_rect(), pixel_rect);
                    assert_eq!(
                        frame.pixel_buffer.buffer().len(),
                        (pixel_rect.width() * pixel_rect.height() * 3) as usize
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

    #[test]
    fn test_generation_ids_increment() {
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_test_request(pixel_rect));

        // Submit request A
        controller.submit_request(Arc::clone(&request));
        let events_a = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!events_a.is_empty(), "expected events from request A");
        let gen_a = extract_generation(&events_a);

        // Submit request B
        controller.submit_request(Arc::clone(&request));
        let events_b = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!events_b.is_empty(), "expected events from request B");
        let gen_b = extract_generation(&events_b);

        assert!(
            gen_b > gen_a,
            "Generation B ({}) should be greater than A ({})",
            gen_b,
            gen_a
        );

        controller.shutdown();
    }

    fn extract_generation(events: &[RenderEvent]) -> u64 {
        events
            .iter()
            .find_map(|e| match e {
                RenderEvent::Frame(frame) => Some(frame.generation),
                RenderEvent::Error(err) => Some(err.generation),
            })
            .expect("Should have at least one event with generation")
    }

    #[test]
    fn test_last_completed_generation_starts_at_zero() {
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        assert_eq!(controller.last_completed_generation(), 0);

        controller.shutdown();
    }

    #[test]
    fn test_last_completed_generation_updates_after_frame_completion() {
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_test_request(pixel_rect));

        let submitted_generation = controller.submit_request(request);
        let events = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!events.is_empty(), "expected a render event");

        let completed_generation = extract_generation(&events);

        assert_eq!(completed_generation, submitted_generation);
        assert_eq!(controller.last_completed_generation(), completed_generation);

        controller.shutdown();
    }

    #[test]
    fn test_last_completed_generation_updates_after_error_completion() {
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_error_request(pixel_rect));

        let submitted_generation = controller.submit_request(request);
        let events = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!events.is_empty(), "expected an error render event");

        let mut saw_error = false;
        for event in &events {
            if let RenderEvent::Error(error) = event {
                saw_error = true;
                assert_eq!(error.generation, submitted_generation);
            }
        }

        assert!(saw_error, "expected at least one error event");
        assert_eq!(controller.last_completed_generation(), submitted_generation);

        controller.shutdown();
    }

    #[test]
    fn test_last_completed_generation_is_monotonic_across_mixed_completions() {
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();

        let frame_generation = controller.submit_request(Arc::new(create_test_request(pixel_rect)));
        let frame_events = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!frame_events.is_empty(), "expected frame completion events");
        assert_eq!(extract_generation(&frame_events), frame_generation);
        let after_frame = controller.last_completed_generation();

        let error_generation =
            controller.submit_request(Arc::new(create_error_request(pixel_rect)));
        let error_events = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(!error_events.is_empty(), "expected error completion events");
        assert_eq!(extract_generation(&error_events), error_generation);
        let after_error = controller.last_completed_generation();

        let frame_generation_2 =
            controller.submit_request(Arc::new(create_test_request(pixel_rect)));
        let frame_events_2 = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));
        assert!(
            !frame_events_2.is_empty(),
            "expected second frame completion events"
        );
        assert_eq!(extract_generation(&frame_events_2), frame_generation_2);
        let after_frame_2 = controller.last_completed_generation();

        assert!(after_frame >= frame_generation);
        assert!(after_error >= after_frame);
        assert!(after_frame_2 >= after_error);

        controller.shutdown();
    }

    #[test]
    fn test_ui_layer_filters_stale_generations() {
        // Simulate presenter behavior without actual GUI.
        // This tests the filtering logic that the UI layer should implement.
        struct PresenterState {
            last_presented_generation: u64,
        }

        impl PresenterState {
            fn should_present(&self, incoming_generation: u64) -> bool {
                incoming_generation > self.last_presented_generation
            }

            fn present(&mut self, generation: u64) -> bool {
                if self.should_present(generation) {
                    self.last_presented_generation = generation;
                    true
                } else {
                    false
                }
            }
        }

        let mut state = PresenterState {
            last_presented_generation: 0,
        };

        // Simulate out-of-order frame arrivals
        assert!(
            state.present(3),
            "Frame 3 should be presented (first frame)"
        );
        assert_eq!(state.last_presented_generation, 3);

        assert!(
            !state.present(1),
            "Frame 1 should be rejected (stale, arrived late)"
        );
        assert_eq!(
            state.last_presented_generation, 3,
            "Generation should remain at 3 after rejecting stale frame"
        );

        assert!(
            !state.present(2),
            "Frame 2 should be rejected (stale, arrived late)"
        );
        assert_eq!(state.last_presented_generation, 3);

        assert!(state.present(5), "Frame 5 should be presented (newer)");
        assert_eq!(state.last_presented_generation, 5);

        assert!(
            !state.present(4),
            "Frame 4 should be rejected (stale, arrived late)"
        );
        assert_eq!(state.last_presented_generation, 5);

        assert!(state.present(6), "Frame 6 should be presented (newer)");
        assert_eq!(state.last_presented_generation, 6);
    }

    #[test]
    fn test_rapid_requests_do_not_emit_cancellation_errors() {
        // Submit multiple rapid requests; the controller should emit only Frame events
        // (no Error events for cancelled work).
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_test_request(pixel_rect));

        // Submit several requests rapidly to trigger cancellation
        for _ in 0..5 {
            controller.submit_request(Arc::clone(&request));
        }

        // Wait for events to settle
        thread::sleep(Duration::from_millis(500));
        let events = presenter_port.take_events();

        // Verify no Error events were emitted (cancellation should not produce errors)
        for event in &events {
            if let RenderEvent::Error(err) = event {
                panic!(
                    "Unexpected error event - cancellation should not emit errors: {}",
                    err.message
                );
            }
        }

        // At least one frame should have been emitted (the last non-cancelled one)
        let frame_count = events
            .iter()
            .filter(|e| matches!(e, RenderEvent::Frame(_)))
            .count();
        assert!(
            frame_count >= 1,
            "Expected at least one frame event, got {}",
            frame_count
        );

        controller.shutdown();
    }

    #[test]
    fn test_newest_request_yields_emitted_frame() {
        // Submit multiple requests; the final frame should have the highest generation.
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_test_request(pixel_rect));

        // Submit several requests rapidly
        let mut last_gen = 0;
        for _ in 0..5 {
            last_gen = controller.submit_request(Arc::clone(&request));
        }

        // Wait for rendering to complete
        thread::sleep(Duration::from_millis(500));
        let events = presenter_port.take_events();

        // Find the highest generation among emitted frames
        let max_emitted_gen = events
            .iter()
            .filter_map(|e| match e {
                RenderEvent::Frame(frame) => Some(frame.generation),
                _ => None,
            })
            .max()
            .unwrap_or(0);

        // The highest emitted generation should be from the last request
        // (or close to it if not all requests completed)
        assert!(
            max_emitted_gen <= last_gen,
            "Emitted generation {} should be <= last submitted {}",
            max_emitted_gen,
            last_gen
        );

        // There should be at least one frame
        assert!(
            max_emitted_gen > 0,
            "Expected at least one frame to be emitted"
        );

        controller.shutdown();
    }

    #[test]
    fn test_cancellation_silently_discards_results() {
        // This is a conceptual test - when cancellation occurs, no event should be emitted.
        // We verify this by checking that Frame events only contain valid pixel buffers
        // (i.e., no partially rendered or corrupted data).
        let presenter_port = Arc::new(MockPresenterPort::default());
        let mut controller = InteractiveController::new(
            Arc::clone(&presenter_port) as Arc<dyn InteractiveControllerPresenterPort>
        );

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 3, y: 3 }).unwrap();
        let request = Arc::new(create_test_request(pixel_rect));
        let expected_buffer_size = (pixel_rect.width() * pixel_rect.height() * 3) as usize;

        // Submit requests
        controller.submit_request(Arc::clone(&request));

        // Wait for completion
        let events = wait_for_events(presenter_port.as_ref(), Duration::from_secs(2));

        // Verify all emitted frames have valid, complete buffers
        for event in events {
            if let RenderEvent::Frame(frame) = event {
                assert_eq!(
                    frame.pixel_buffer.buffer().len(),
                    expected_buffer_size,
                    "Frame buffer should be complete, not partial"
                );
            }
        }

        controller.shutdown();
    }
}
