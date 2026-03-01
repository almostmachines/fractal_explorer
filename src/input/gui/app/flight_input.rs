use crate::core::flight::FlightControlsSnapshot;
use winit::event::ElementState;
use winit::keyboard::KeyCode;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FlightInputState {
    w_held: bool,
    a_held: bool,
    s_held: bool,
    d_held: bool,
    j_held: bool,
    k_held: bool,
    p_edge_pending: bool,
}

impl FlightInputState {
    pub fn handle_key_event(&mut self, key_code: KeyCode, state: ElementState) {
        let pressed = state == ElementState::Pressed;

        match key_code {
            KeyCode::KeyW => self.w_held = pressed,
            KeyCode::KeyA => self.a_held = pressed,
            KeyCode::KeyS => self.s_held = pressed,
            KeyCode::KeyD => self.d_held = pressed,
            KeyCode::KeyJ => self.j_held = pressed,
            KeyCode::KeyK => self.k_held = pressed,
            KeyCode::KeyP if pressed => {
                self.p_edge_pending = true;
            }
            _ => {}
        }
    }

    pub fn snapshot(&mut self, text_editing: bool) -> FlightControlsSnapshot {
        if text_editing {
            self.p_edge_pending = false;
            return FlightControlsSnapshot::default();
        }

        let snapshot = FlightControlsSnapshot {
            w: self.w_held,
            a: self.a_held,
            s: self.s_held,
            d: self.d_held,
            accelerate: self.k_held,
            decelerate: self.j_held,
            pause_toggle_edge: self.p_edge_pending,
        };

        self.p_edge_pending = false;
        snapshot
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::FlightInputState;
    use winit::{event::ElementState, keyboard::KeyCode};

    #[test]
    fn press_and_release_updates_held_flags() {
        let mut input = FlightInputState::default();

        input.handle_key_event(KeyCode::KeyW, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyA, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyS, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyD, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyJ, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyK, ElementState::Pressed);

        let pressed_snapshot = input.snapshot(false);
        assert!(pressed_snapshot.w);
        assert!(pressed_snapshot.a);
        assert!(pressed_snapshot.s);
        assert!(pressed_snapshot.d);
        assert!(pressed_snapshot.decelerate);
        assert!(pressed_snapshot.accelerate);

        input.handle_key_event(KeyCode::KeyW, ElementState::Released);
        input.handle_key_event(KeyCode::KeyA, ElementState::Released);
        input.handle_key_event(KeyCode::KeyS, ElementState::Released);
        input.handle_key_event(KeyCode::KeyD, ElementState::Released);
        input.handle_key_event(KeyCode::KeyJ, ElementState::Released);
        input.handle_key_event(KeyCode::KeyK, ElementState::Released);

        let released_snapshot = input.snapshot(false);
        assert!(!released_snapshot.w);
        assert!(!released_snapshot.a);
        assert!(!released_snapshot.s);
        assert!(!released_snapshot.d);
        assert!(!released_snapshot.decelerate);
        assert!(!released_snapshot.accelerate);
    }

    #[test]
    fn p_press_sets_single_pending_edge_even_with_repeats() {
        let mut input = FlightInputState::default();

        input.handle_key_event(KeyCode::KeyP, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyP, ElementState::Pressed);

        let first = input.snapshot(false);
        let second = input.snapshot(false);

        assert!(first.pause_toggle_edge);
        assert!(!second.pause_toggle_edge);
    }

    #[test]
    fn snapshot_consumes_pause_edge_once() {
        let mut input = FlightInputState::default();

        input.handle_key_event(KeyCode::KeyP, ElementState::Pressed);

        let first = input.snapshot(false);
        let second = input.snapshot(false);
        let third = input.snapshot(false);

        assert!(first.pause_toggle_edge);
        assert!(!second.pause_toggle_edge);
        assert!(!third.pause_toggle_edge);
    }

    #[test]
    fn focus_suppression_returns_neutral_snapshot_and_clears_edge() {
        let mut input = FlightInputState::default();

        input.handle_key_event(KeyCode::KeyW, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyP, ElementState::Pressed);

        let suppressed = input.snapshot(true);
        assert_eq!(
            suppressed,
            crate::core::flight::FlightControlsSnapshot::default()
        );

        let after_focus = input.snapshot(false);
        assert!(after_focus.w);
        assert!(!after_focus.pause_toggle_edge);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut input = FlightInputState::default();
        input.handle_key_event(KeyCode::KeyW, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyK, ElementState::Pressed);
        input.handle_key_event(KeyCode::KeyP, ElementState::Pressed);

        input.reset();

        let snapshot = input.snapshot(false);
        assert!(!snapshot.w);
        assert!(!snapshot.a);
        assert!(!snapshot.s);
        assert!(!snapshot.d);
        assert!(!snapshot.accelerate);
        assert!(!snapshot.decelerate);
        assert!(!snapshot.pause_toggle_edge);
    }
}
