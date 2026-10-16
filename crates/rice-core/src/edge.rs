//! "Push against the top edge" detection (`island://edge`).
//!
//! Reference behaviour: `trackEdge()` in `web/src/backend/mock/mock-backend.ts`,
//! plus the dwell time from docs/BACKEND.md §3.1.

use crate::contract::Rect;
use crate::geometry::{PhysPoint, PhysRect, sane_scale, to_window_css};

/// How long the cursor must rest at the edge before the island is revealed.
pub const DWELL_MS: u64 = 120;
/// Maximum horizontal distance from the monitor centre, CSS px.
pub const REACH: f64 = 160.0;
/// Margin around the hit regions that still counts as "inside", CSS px.
pub const MARGIN: f64 = 32.0;
/// The edge state ends only below this depth, CSS px from the monitor top.
pub const RELEASE_DEPTH: f64 = 40.0;

/// One cursor sample with the geometry it is judged against.
#[derive(Debug, Clone, Copy)]
pub struct EdgeInput<'a> {
    pub now_ms: u64,
    pub cursor: PhysPoint,
    /// Full rectangle of the island monitor.
    pub monitor: PhysRect,
    pub scale: f64,
    /// Top-left corner of the island window.
    pub window_origin: PhysPoint,
    /// Hit regions reported by the frontend (CSS px, window-relative).
    pub regions: &'a [Rect],
}

/// State machine behind `island://edge`.
#[derive(Debug, Clone, Default)]
pub struct EdgeTracker {
    active: bool,
    since: Option<u64>,
}

impl EdgeTracker {
    /// Whether the edge state is currently on.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Feeds one sample; returns the new state when it changed.
    pub fn update(&mut self, input: &EdgeInput<'_>) -> Option<bool> {
        let scale = sane_scale(input.scale);
        let cursor = input.cursor;
        let monitor = input.monitor;
        let at_edge = cursor.y >= monitor.y
            && cursor.y <= monitor.y + 1
            && (f64::from(cursor.x) - monitor.center_x()).abs() <= REACH * scale;

        if !self.active {
            if !at_edge {
                self.since = None;
                return None;
            }
            let since = *self.since.get_or_insert(input.now_ms);
            if input.now_ms.saturating_sub(since) >= DWELL_MS {
                self.active = true;
                self.since = None;
                return Some(true);
            }
            return None;
        }

        if at_edge {
            return None;
        }
        let (x, y) = to_window_css(cursor, input.window_origin, scale);
        let inside = input
            .regions
            .iter()
            .any(|r| x >= r.x - MARGIN && x <= r.x + r.width + MARGIN && y <= r.y + r.height + MARGIN);
        let depth = f64::from(cursor.y - monitor.y) / scale;
        if !inside && depth > RELEASE_DEPTH {
            self.active = false;
            return Some(false);
        }
        None
    }

    /// Ends the edge state (e.g. when the island moves to another monitor).
    pub fn reset(&mut self) -> Option<bool> {
        self.since = None;
        if self.active {
            self.active = false;
            Some(false)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: PhysRect = PhysRect { x: 0, y: 0, width: 1920, height: 1080 };
    const ORIGIN: PhysPoint = PhysPoint { x: 512, y: 0 };
    const REGIONS: [Rect; 1] = [Rect { x: 348.0, y: 10.0, width: 200.0, height: 38.0 }];

    fn sample(tracker: &mut EdgeTracker, now_ms: u64, x: i32, y: i32) -> Option<bool> {
        tracker.update(&EdgeInput {
            now_ms,
            cursor: PhysPoint { x, y },
            monitor: MONITOR,
            scale: 1.0,
            window_origin: ORIGIN,
            regions: &REGIONS,
        })
    }

    #[test]
    fn activates_after_the_dwell_time() {
        let mut t = EdgeTracker::default();
        assert_eq!(sample(&mut t, 0, 960, 0), None);
        assert_eq!(sample(&mut t, 100, 1000, 1), None);
        assert_eq!(sample(&mut t, 120, 1000, 0), Some(true));
        assert!(t.is_active());
        assert_eq!(sample(&mut t, 200, 1000, 0), None);
    }

    #[test]
    fn leaving_the_edge_restarts_the_dwell() {
        let mut t = EdgeTracker::default();
        sample(&mut t, 0, 960, 0);
        sample(&mut t, 100, 960, 5);
        assert_eq!(sample(&mut t, 150, 960, 0), None);
        assert_eq!(sample(&mut t, 260, 960, 0), None);
        assert_eq!(sample(&mut t, 270, 960, 0), Some(true));
    }

    #[test]
    fn ignores_the_edge_away_from_the_centre() {
        let mut t = EdgeTracker::default();
        sample(&mut t, 0, 200, 0);
        assert_eq!(sample(&mut t, 500, 200, 0), None);
        // A monitor stacked above: the cursor is not on the island monitor.
        sample(&mut t, 600, 960, -1);
        assert_eq!(sample(&mut t, 900, 960, -1), None);
    }

    #[test]
    fn stays_active_over_the_island_and_ends_below_it() {
        let mut t = EdgeTracker::default();
        sample(&mut t, 0, 960, 0);
        sample(&mut t, 120, 960, 0);
        // Over the island (CSS x 448 → screen 960) and inside its margin.
        assert_eq!(sample(&mut t, 200, 960, 60), None);
        assert_eq!(sample(&mut t, 220, 960, 75), None);
        // Shallow but outside the regions: still active (depth ≤ 40).
        assert_eq!(sample(&mut t, 240, 100, 30), None);
        // Outside and deep enough.
        assert_eq!(sample(&mut t, 260, 100, 200), Some(false));
        assert!(!t.is_active());
    }

    #[test]
    fn reset_reports_only_a_change() {
        let mut t = EdgeTracker::default();
        assert_eq!(t.reset(), None);
        sample(&mut t, 0, 960, 0);
        sample(&mut t, 120, 960, 0);
        assert_eq!(t.reset(), Some(false));
    }
}
