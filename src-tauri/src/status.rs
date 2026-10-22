use crate::rclone::RcdState;
use serde::Serialize;

/// The single state shown by the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OverallStatus {
    Synced,
    Syncing,
    Disconnected,
}

/// Maps rclone health and current activity onto one tray state.
///
/// Rclone not being healthy wins over everything else. Any active transfer or sync job
/// means `Syncing`. Otherwise everything is `Synced`.
pub fn overall(rcd: &RcdState, active_transfers: usize, active_jobs: usize) -> OverallStatus {
    match rcd {
        RcdState::Healthy { .. } => {}
        _ => return OverallStatus::Disconnected,
    }
    if active_transfers + active_jobs > 0 {
        OverallStatus::Syncing
    } else {
        OverallStatus::Synced
    }
}

/// Human-readable line for the tray menu and status card.
pub fn describe(status: OverallStatus) -> &'static str {
    match status {
        OverallStatus::Synced => "All connections are up to date",
        OverallStatus::Syncing => "Working…",
        OverallStatus::Disconnected => "rclone is not running",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> RcdState {
        RcdState::Healthy {
            version: "v1".into(),
        }
    }

    #[test]
    fn idle_and_healthy_is_synced() {
        assert_eq!(overall(&healthy(), 0, 0), OverallStatus::Synced);
    }

    #[test]
    fn any_activity_is_syncing() {
        assert_eq!(overall(&healthy(), 2, 0), OverallStatus::Syncing);
        assert_eq!(overall(&healthy(), 0, 1), OverallStatus::Syncing);
    }

    #[test]
    fn unhealthy_rcd_is_disconnected_even_with_activity() {
        for state in [
            RcdState::Starting,
            RcdState::Restarting { attempt: 1 },
            RcdState::Failed {
                message: "boom".into(),
            },
            RcdState::Stopped,
        ] {
            assert_eq!(
                overall(&state, 5, 5),
                OverallStatus::Disconnected,
                "{state:?}"
            );
        }
    }
}
