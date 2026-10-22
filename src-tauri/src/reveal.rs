use std::sync::Mutex;

/// Decides when the main window replaces the splash.
///
/// The main window opens only after the splash has finished (or was skipped) and rclone has
/// answered, healthy or not. Either side may finish first.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RevealGate {
    splash_done: bool,
    rclone_settled: bool,
    revealed: bool,
}

impl RevealGate {
    pub fn new(splash_enabled: bool) -> Self {
        Self {
            splash_done: !splash_enabled,
            ..Self::default()
        }
    }

    pub fn splash_finished(&mut self) -> bool {
        self.splash_done = true;
        self.take_reveal()
    }

    pub fn rclone_settled(&mut self) -> bool {
        self.rclone_settled = true;
        self.take_reveal()
    }

    /// Returns true exactly once, when both conditions have been met.
    fn take_reveal(&mut self) -> bool {
        if self.revealed || !(self.splash_done && self.rclone_settled) {
            return false;
        }
        self.revealed = true;
        true
    }
}

/// Managed app state wrapper so commands and event hooks can share the gate.
pub struct RevealState(pub Mutex<RevealGate>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_both_sides() {
        let mut gate = RevealGate::new(true);
        assert!(!gate.rclone_settled());
        assert!(gate.splash_finished());
    }

    #[test]
    fn splash_first_then_rclone() {
        let mut gate = RevealGate::new(true);
        assert!(!gate.splash_finished());
        assert!(gate.rclone_settled());
    }

    #[test]
    fn reveals_only_once() {
        let mut gate = RevealGate::new(true);
        gate.splash_finished();
        assert!(gate.rclone_settled());
        assert!(!gate.rclone_settled());
        assert!(!gate.splash_finished());
    }

    #[test]
    fn disabled_splash_does_not_block() {
        let mut gate = RevealGate::new(false);
        assert!(gate.rclone_settled());
    }
}
