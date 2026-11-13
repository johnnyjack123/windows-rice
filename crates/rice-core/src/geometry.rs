//! Island window geometry in physical screen pixels.
//!
//! The frontend speaks CSS px relative to the island window; Windows speaks
//! physical px on the virtual screen. `scale` is the monitor's DPI / 96.

use crate::contract::{Point, Rect, Size};

/// Rectangle in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Point in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysPoint {
    pub x: i32,
    pub y: i32,
}

impl PhysRect {
    /// Creates a rectangle from its edges (Win32 `RECT` order).
    pub fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { x: left, y: top, width: right - left, height: bottom - top }
    }

    /// Exclusive right edge.
    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    /// Exclusive bottom edge.
    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// Horizontal centre.
    pub fn center_x(&self) -> f64 {
        f64::from(self.x) + f64::from(self.width) / 2.0
    }

    /// Whether both rectangles share an area larger than zero.
    pub fn intersects(&self, other: &PhysRect) -> bool {
        self.x < other.right() && other.x < self.right() && self.y < other.bottom() && other.y < self.bottom()
    }

    /// Whether `other` lies completely inside this rectangle.
    pub fn contains_rect(&self, other: &PhysRect) -> bool {
        other.x >= self.x && other.y >= self.y && other.right() <= self.right() && other.bottom() <= self.bottom()
    }
}

/// Guards against a zero or broken DPI scale.
pub fn sane_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 { scale } else { 1.0 }
}

fn to_physical(length: f64, scale: f64) -> i32 {
    // Window sizes are bounded by the virtual screen, far below i32::MAX.
    (length * sane_scale(scale)).round() as i32
}

/// Where the island window goes: horizontally centred on the monitor, top edge
/// flush with the monitor's work area (below a top taskbar).
pub fn island_window_rect(monitor: PhysRect, work: PhysRect, scale: f64, size: Size) -> PhysRect {
    let width = to_physical(size.width.max(1.0), scale);
    let height = to_physical(size.height.max(1.0), scale);
    PhysRect { x: monitor.x + (monitor.width - width) / 2, y: work.y, width, height }
}

/// The zone at the top centre of the island monitor whose overlap with another
/// window hides the island: 480 × (offsetTop + 50) CSS px.
pub fn island_zone(monitor: PhysRect, work: PhysRect, scale: f64, offset_top: f64) -> PhysRect {
    let width = to_physical(480.0, scale);
    let height = to_physical(offset_top.max(0.0) + 50.0, scale);
    let x = (monitor.center_x() - f64::from(width) / 2.0).round() as i32;
    PhysRect { x, y: work.y, width, height }
}

/// Converts a screen point to CSS px relative to the window's top-left corner.
pub fn to_window_css(point: PhysPoint, window_origin: PhysPoint, scale: f64) -> (f64, f64) {
    let scale = sane_scale(scale);
    (f64::from(point.x - window_origin.x) / scale, f64::from(point.y - window_origin.y) / scale)
}

/// Whether a screen point lies inside one of the hit regions (CSS px, window-relative).
pub fn hits(point: PhysPoint, window_origin: PhysPoint, scale: f64, regions: &[Rect]) -> bool {
    let (x, y) = to_window_css(point, window_origin, scale);
    regions.iter().any(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
}

/// Monitor size and window position as `system://wallpaper` reports them (CSS px).
pub fn wallpaper_geometry(monitor: PhysRect, window: PhysRect, scale: f64) -> (Size, Point) {
    let scale = sane_scale(scale);
    (
        Size { width: f64::from(monitor.width) / scale, height: f64::from(monitor.height) / scale },
        Point { x: f64::from(window.x - monitor.x) / scale, y: f64::from(window.y - monitor.y) / scale },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: PhysRect = PhysRect { x: 0, y: 0, width: 2560, height: 1440 };

    #[test]
    fn centres_the_window_below_a_top_taskbar() {
        let work = PhysRect { x: 0, y: 72, width: 2560, height: 1368 };
        let rect = island_window_rect(MONITOR, work, 1.5, Size { width: 896.0, height: 662.0 });
        assert_eq!(rect, PhysRect { x: 608, y: 72, width: 1344, height: 993 });
    }

    #[test]
    fn places_on_secondary_monitors_with_negative_coordinates() {
        let monitor = PhysRect { x: -1920, y: -200, width: 1920, height: 1080 };
        let rect = island_window_rect(monitor, monitor, 1.0, Size { width: 900.0, height: 600.0 });
        assert_eq!(rect, PhysRect { x: -1410, y: -200, width: 900, height: 600 });
    }

    #[test]
    fn zone_scales_with_dpi_and_offset() {
        let zone = island_zone(MONITOR, MONITOR, 1.25, 10.0);
        assert_eq!(zone, PhysRect { x: 980, y: 0, width: 600, height: 75 });
    }

    #[test]
    fn hit_test_uses_css_pixels() {
        let origin = PhysPoint { x: 608, y: 0 };
        let regions = [Rect { x: 348.0, y: 10.0, width: 200.0, height: 38.0 }];
        // CSS (350, 20) → physical (608 + 525, 30) at 150 %.
        assert!(hits(PhysPoint { x: 1133, y: 30 }, origin, 1.5, &regions));
        assert!(!hits(PhysPoint { x: 1133, y: 5 }, origin, 1.5, &regions));
        assert!(!hits(PhysPoint { x: 1133, y: 30 }, origin, 1.5, &[]));
    }

    #[test]
    fn intersection_needs_area() {
        let a = PhysRect { x: 0, y: 0, width: 10, height: 10 };
        assert!(a.intersects(&PhysRect { x: 9, y: 9, width: 5, height: 5 }));
        assert!(!a.intersects(&PhysRect { x: 10, y: 0, width: 5, height: 5 }));
        assert!(a.contains_rect(&PhysRect { x: 2, y: 2, width: 8, height: 8 }));
        assert!(!a.contains_rect(&PhysRect { x: 2, y: 2, width: 9, height: 8 }));
    }

    #[test]
    fn wallpaper_geometry_is_relative_to_the_monitor() {
        let monitor = PhysRect { x: 2560, y: 0, width: 3840, height: 2160 };
        let window = PhysRect { x: 2560 + 1248, y: 0, width: 1344, height: 993 };
        let (size, point) = wallpaper_geometry(monitor, window, 1.5);
        assert_eq!(size, Size { width: 2560.0, height: 1440.0 });
        assert_eq!(point, Point { x: 832.0, y: 0.0 });
    }
}
