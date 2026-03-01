#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlightControlsSnapshot {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub accelerate: bool,
    pub decelerate: bool,
    pub pause_toggle_edge: bool,
}

#[cfg(test)]
mod tests {
    use super::FlightControlsSnapshot;

    #[test]
    fn default_snapshot_is_all_false() {
        let snapshot = FlightControlsSnapshot::default();

        assert!(!snapshot.w);
        assert!(!snapshot.a);
        assert!(!snapshot.s);
        assert!(!snapshot.d);
        assert!(!snapshot.accelerate);
        assert!(!snapshot.decelerate);
        assert!(!snapshot.pause_toggle_edge);
    }
}
