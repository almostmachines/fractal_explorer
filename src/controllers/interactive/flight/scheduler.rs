use crate::controllers::interactive::data::fractal_config::FractalConfig;
use std::sync::Arc;

pub struct RenderScheduler {
    pending_request: Option<Arc<FractalConfig>>,
    in_flight_generation: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerAction {
    Submitted { generation: u64 },
    Coalesced,
    NothingToDo,
}

impl RenderScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_request: None,
            in_flight_generation: None,
        }
    }

    pub fn update(
        &mut self,
        desired: Arc<FractalConfig>,
        flight_active: bool,
        last_completed_gen: u64,
        submit: impl FnOnce(Arc<FractalConfig>) -> u64,
    ) -> SchedulerAction {
        self.mark_completed(last_completed_gen);
        self.pending_request = Some(desired);

        if self.in_flight_generation.is_none() || !flight_active {
            return self.submit_pending(submit);
        }

        SchedulerAction::Coalesced
    }

    pub fn reset(&mut self) {
        self.pending_request = None;
        self.in_flight_generation = None;
    }

    pub fn observe_completion(&mut self, last_completed_gen: u64) {
        self.mark_completed(last_completed_gen);
    }

    #[must_use]
    pub fn has_pending(&self) -> bool {
        self.pending_request.is_some()
    }

    #[must_use]
    pub fn in_flight_generation(&self) -> Option<u64> {
        self.in_flight_generation
    }

    fn mark_completed(&mut self, last_completed_gen: u64) {
        if self
            .in_flight_generation
            .is_some_and(|generation| last_completed_gen >= generation)
        {
            self.in_flight_generation = None;
        }
    }

    fn submit_pending(
        &mut self,
        submit: impl FnOnce(Arc<FractalConfig>) -> u64,
    ) -> SchedulerAction {
        let Some(request) = self.pending_request.take() else {
            return SchedulerAction::NothingToDo;
        };

        let generation = submit(request);
        self.in_flight_generation = Some(generation);

        SchedulerAction::Submitted { generation }
    }
}

impl Default for RenderScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderScheduler, SchedulerAction};
    use crate::{
        controllers::interactive::data::fractal_config::FractalConfig,
        core::{
            data::{pixel_rect::PixelRect, point::Point},
            fractals::mandelbrot::mandelbrot_config::MandelbrotConfig,
        },
    };
    use std::sync::Arc;

    fn request(max_iterations: u32) -> Arc<FractalConfig> {
        let mut config = MandelbrotConfig::default();
        config.max_iterations = max_iterations;

        let pixel_rect = PixelRect::new(Point { x: 0, y: 0 }, Point { x: 1, y: 1 })
            .expect("pixel rect should be valid");

        Arc::new(config.build_render_request(pixel_rect))
    }

    #[test]
    fn submits_immediately_when_nothing_is_in_flight() {
        let mut scheduler = RenderScheduler::new();

        let action = scheduler.update(request(10), true, 0, |_| 1);

        assert_eq!(action, SchedulerAction::Submitted { generation: 1 });
        assert_eq!(scheduler.in_flight_generation(), Some(1));
        assert!(!scheduler.has_pending());
    }

    #[test]
    fn submits_immediately_when_in_flight_and_flight_inactive() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 1);

        let action = scheduler.update(request(11), false, 0, |_| 2);

        assert_eq!(action, SchedulerAction::Submitted { generation: 2 });
        assert_eq!(scheduler.in_flight_generation(), Some(2));
        assert!(!scheduler.has_pending());
    }

    #[test]
    fn coalesces_when_in_flight_and_flight_active() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 1);

        let mut submitted = false;
        let next = request(11);
        let action = scheduler.update(Arc::clone(&next), true, 0, |_| {
            submitted = true;
            2
        });

        assert_eq!(action, SchedulerAction::Coalesced);
        assert!(!submitted);
        assert_eq!(scheduler.in_flight_generation(), Some(1));
        assert!(scheduler.has_pending());
        assert!(Arc::ptr_eq(
            scheduler.pending_request.as_ref().expect("pending exists"),
            &next
        ));
    }

    #[test]
    fn multiple_coalesced_updates_keep_only_the_newest_pending_request() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 1);

        let second = request(11);
        let third = request(12);

        let _ = scheduler.update(Arc::clone(&second), true, 0, |_| panic!("must not submit"));
        let _ = scheduler.update(Arc::clone(&third), true, 0, |_| panic!("must not submit"));

        assert!(Arc::ptr_eq(
            scheduler.pending_request.as_ref().expect("pending exists"),
            &third
        ));
    }

    #[test]
    fn completion_allows_pending_request_to_submit() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 1);
        let _ = scheduler.update(request(11), true, 0, |_| panic!("must not submit"));

        let newest = request(12);
        let mut submitted_request: Option<Arc<FractalConfig>> = None;
        let action = scheduler.update(Arc::clone(&newest), true, 1, |request| {
            submitted_request = Some(request);
            2
        });

        assert_eq!(action, SchedulerAction::Submitted { generation: 2 });
        assert_eq!(scheduler.in_flight_generation(), Some(2));
        assert!(!scheduler.has_pending());
        assert!(Arc::ptr_eq(
            submitted_request
                .as_ref()
                .expect("a request should have been submitted"),
            &newest
        ));
    }

    #[test]
    fn completion_mismatch_keeps_in_flight_generation() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 5);

        let action = scheduler.update(request(11), true, 4, |_| panic!("must not submit"));

        assert_eq!(action, SchedulerAction::Coalesced);
        assert_eq!(scheduler.in_flight_generation(), Some(5));
        assert!(scheduler.has_pending());
    }

    #[test]
    fn reset_clears_pending_and_in_flight_state() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 1);
        let _ = scheduler.update(request(11), true, 0, |_| panic!("must not submit"));

        scheduler.reset();

        assert!(!scheduler.has_pending());
        assert_eq!(scheduler.in_flight_generation(), None);
    }

    #[test]
    fn observe_completion_clears_in_flight_when_done() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 7);

        scheduler.observe_completion(6);
        assert_eq!(scheduler.in_flight_generation(), Some(7));

        scheduler.observe_completion(7);
        assert_eq!(scheduler.in_flight_generation(), None);
    }

    #[test]
    fn repeated_identical_desired_requests_are_handled_consistently() {
        let mut scheduler = RenderScheduler::new();
        let same = request(10);

        let first = scheduler.update(Arc::clone(&same), true, 0, |_| 1);
        assert_eq!(first, SchedulerAction::Submitted { generation: 1 });

        let second = scheduler.update(Arc::clone(&same), true, 0, |_| panic!("must not submit"));
        assert_eq!(second, SchedulerAction::Coalesced);

        let third = scheduler.update(Arc::clone(&same), true, 1, |_| 2);
        assert_eq!(third, SchedulerAction::Submitted { generation: 2 });
    }

    #[test]
    fn rapid_updates_during_flight_leave_only_last_pending_request() {
        let mut scheduler = RenderScheduler::new();
        let _ = scheduler.update(request(10), true, 0, |_| 1);

        let mut last = request(11);
        for max_iterations in 12..=20 {
            let next = request(max_iterations);
            let _ = scheduler.update(Arc::clone(&next), true, 0, |_| panic!("must not submit"));
            last = next;
        }

        assert!(Arc::ptr_eq(
            scheduler.pending_request.as_ref().expect("pending exists"),
            &last
        ));
    }
}
