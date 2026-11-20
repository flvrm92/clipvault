//! Where the popup opens (HK-005): next to the text caret when known, otherwise next to the
//! mouse pointer, and always entirely inside the work area of that monitor.

use crate::ports::{ScreenPoint, TargetWindow};

/// Gap between the anchor and the popup, in physical pixels.
const GAP: i32 = 8;
/// Approximate height of a caret line, so a popup placed above does not cover the text.
const CARET_LINE: i32 = 24;

/// A rectangle in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl ScreenRect {
    fn right(self) -> i32 {
        self.x.saturating_add_unsigned(self.width)
    }

    fn bottom(self) -> i32 {
        self.y.saturating_add_unsigned(self.height)
    }
}

/// The point the popup is anchored to: the caret, else the pointer.
pub fn anchor_of(target: Option<&TargetWindow>, pointer: ScreenPoint) -> ScreenPoint {
    target.and_then(|t| t.caret).unwrap_or(pointer)
}

/// Top-left corner for a popup of `width` × `height` anchored at `anchor`: below and to the
/// right of it when it fits, above it otherwise, clamped to `work_area`.
pub fn place_popup(
    anchor: ScreenPoint,
    width: u32,
    height: u32,
    work_area: ScreenRect,
) -> ScreenPoint {
    let w = i32::try_from(width).unwrap_or(i32::MAX);
    let h = i32::try_from(height).unwrap_or(i32::MAX);

    let below = anchor.y + GAP;
    let y = if below + h <= work_area.bottom() {
        below
    } else {
        anchor.y - CARET_LINE - GAP - h
    };
    let clamp = |value: i32, min: i32, max: i32| value.min(max).max(min);
    ScreenPoint {
        x: clamp(anchor.x, work_area.x, work_area.right() - w),
        y: clamp(y, work_area.y, work_area.bottom() - h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::WindowHandle;

    const AREA: ScreenRect = ScreenRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    };

    fn point(x: i32, y: i32) -> ScreenPoint {
        ScreenPoint { x, y }
    }

    #[test]
    fn hk_005_opens_below_and_right_of_the_anchor_when_it_fits() {
        assert_eq!(
            place_popup(point(100, 100), 380, 520, AREA),
            point(100, 108)
        );
    }

    #[test]
    fn hk_005_opens_above_when_there_is_no_room_below() {
        assert_eq!(
            place_popup(point(100, 900), 380, 520, AREA),
            point(100, 900 - 24 - 8 - 520)
        );
    }

    #[test]
    fn hk_005_never_leaves_the_work_area() {
        assert_eq!(
            place_popup(point(1900, 1030), 380, 520, AREA),
            point(1540, 478)
        );
        assert_eq!(place_popup(point(-50, 10), 380, 520, AREA), point(0, 18));
        assert_eq!(
            place_popup(point(100, 300), 380, 2000, AREA),
            point(100, 0),
            "taller than the area"
        );
    }

    #[test]
    fn hk_005_respects_work_areas_of_secondary_monitors() {
        let left_monitor = ScreenRect {
            x: -1280,
            y: 0,
            width: 1280,
            height: 984,
        };

        assert_eq!(
            place_popup(point(-10, 500), 380, 520, left_monitor),
            point(-380, 0),
            "fits neither below nor above: clamped to the top of that monitor"
        );
    }

    #[test]
    fn hk_005_anchors_to_the_caret_when_known_otherwise_to_the_pointer() {
        let pointer = point(5, 5);
        let mut target = TargetWindow {
            handle: WindowHandle::new(1),
            caret: Some(point(300, 200)),
            cursor: point(9, 9),
            elevated: false,
        };

        assert_eq!(anchor_of(Some(&target), pointer), point(300, 200));
        target.caret = None;
        assert_eq!(anchor_of(Some(&target), pointer), pointer);
        assert_eq!(anchor_of(None, pointer), pointer);
    }
}
