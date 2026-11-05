// SPDX-License-Identifier: Apache-2.0
//! Grid snapping (4px / 8px) and minimum tap targets.
//!
//! These are post-layout helpers a host applies to computed rects; they never change what Taffy
//! computes. [`snap_to_grid`] rounds a value to a grid step, [`min_tap_target`] gives the minimum
//! interactive size of a guideline, and [`LayoutRect::snapped`] / [`LayoutRect::with_min_tap_target`]
//! apply them to a box.

use crate::tree::LayoutRect;

/// The 4px grid (the M3 spacing step).
pub const GRID_4: f32 = 4.0;
/// The 8px grid (the M3 layout grid).
pub const GRID_8: f32 = 8.0;

/// A tap-target guideline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapTarget {
    /// WCAG 2.2 SC 2.5.8 Target Size (Minimum), level AA: 24 CSS px.
    WcagAa,
    /// WCAG 2.2 SC 2.5.5 Target Size (Enhanced), level AAA: 44 CSS px.
    WcagAaa,
    /// Apple Human Interface Guidelines: 44 pt.
    Apple,
    /// Material Design 3: 48 dp.
    Material,
}

/// The minimum width and height, in logical px, of an interactive target under `guideline`.
#[must_use]
pub const fn min_tap_target(guideline: TapTarget) -> f32 {
    match guideline {
        TapTarget::WcagAa => 24.0,
        TapTarget::WcagAaa | TapTarget::Apple => 44.0,
        TapTarget::Material => 48.0,
    }
}

/// Rounds `px` to the nearest multiple of `grid` (halves round away from zero).
///
/// A `grid` that is not a positive finite number, or a `px` that is not finite, is returned
/// unchanged: snapping never turns a value into NaN.
#[must_use]
pub fn snap_to_grid(px: f32, grid: f32) -> f32 {
    if !px.is_finite() || !grid.is_finite() || grid <= 0.0 {
        return px;
    }
    let snapped = (px / grid).round() * grid;
    // `-0.2` snaps to `-0.0`; report plain zero.
    if snapped == 0.0 { 0.0 } else { snapped }
}

/// `size` raised to the guideline minimum when it is smaller; `NaN` and negatives raise to the
/// minimum too.
#[must_use]
pub fn ensure_tap_target(size: f32, guideline: TapTarget) -> f32 {
    let min = min_tap_target(guideline);
    if size.is_nan() || size < min {
        min
    } else {
        size
    }
}

impl LayoutRect {
    /// The rect with its origin snapped to `grid` and its far edges snapped too, so adjacent
    /// snapped boxes keep sharing an edge instead of drifting by a rounding step.
    #[must_use]
    pub fn snapped(self, grid: f32) -> Self {
        let x = snap_to_grid(self.x, grid);
        let y = snap_to_grid(self.y, grid);
        let right = snap_to_grid(self.x + self.width, grid);
        let bottom = snap_to_grid(self.y + self.height, grid);
        Self {
            x,
            y,
            width: (right - x).max(0.0),
            height: (bottom - y).max(0.0),
        }
    }

    /// The rect grown about its center to at least the guideline minimum on each axis.
    /// Axes already large enough are untouched.
    #[must_use]
    pub fn with_min_tap_target(self, guideline: TapTarget) -> Self {
        let width = ensure_tap_target(self.width, guideline);
        let height = ensure_tap_target(self.height, guideline);
        Self {
            x: self.x - (width - self.width) / 2.0,
            y: self.y - (height - self.height) / 2.0,
            width,
            height,
        }
    }

    /// Whether both sides meet the guideline minimum.
    #[must_use]
    pub fn meets_tap_target(&self, guideline: TapTarget) -> bool {
        let min = min_tap_target(guideline);
        self.width >= min && self.height >= min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_to_4_and_8() {
        assert_eq!(snap_to_grid(5.0, GRID_4), 4.0);
        assert_eq!(snap_to_grid(6.0, GRID_4), 8.0);
        assert_eq!(snap_to_grid(13.0, GRID_8), 16.0);
        assert_eq!(snap_to_grid(11.9, GRID_8), 8.0);
        assert_eq!(snap_to_grid(12.0, GRID_8), 16.0, "halves round up");
    }

    #[test]
    fn snap_negative_and_zero() {
        assert_eq!(snap_to_grid(-5.0, GRID_4), -4.0);
        assert_eq!(
            snap_to_grid(-6.0, GRID_4),
            -8.0,
            "halves round away from zero"
        );
        let z = snap_to_grid(-0.2, GRID_4);
        assert_eq!(z, 0.0);
        assert!(z.is_sign_positive(), "no negative zero");
    }

    #[test]
    fn snap_degenerate_inputs_pass_through() {
        assert_eq!(snap_to_grid(7.0, 0.0), 7.0);
        assert_eq!(snap_to_grid(7.0, -4.0), 7.0);
        assert_eq!(snap_to_grid(7.0, f32::NAN), 7.0);
        assert!(snap_to_grid(f32::NAN, 4.0).is_nan());
        assert_eq!(snap_to_grid(f32::INFINITY, 4.0), f32::INFINITY);
    }

    #[test]
    fn tap_target_minimums() {
        assert_eq!(min_tap_target(TapTarget::WcagAa), 24.0);
        assert_eq!(min_tap_target(TapTarget::WcagAaa), 44.0);
        assert_eq!(min_tap_target(TapTarget::Apple), 44.0);
        assert_eq!(min_tap_target(TapTarget::Material), 48.0);
    }

    #[test]
    fn ensure_tap_target_raises_only_small_sizes() {
        assert_eq!(ensure_tap_target(20.0, TapTarget::WcagAa), 24.0);
        assert_eq!(ensure_tap_target(24.0, TapTarget::WcagAa), 24.0);
        assert_eq!(ensure_tap_target(100.0, TapTarget::Material), 100.0);
        assert_eq!(ensure_tap_target(-3.0, TapTarget::Material), 48.0);
        assert_eq!(ensure_tap_target(f32::NAN, TapTarget::Material), 48.0);
    }

    #[test]
    fn rect_snapped_keeps_shared_edges() {
        let a = LayoutRect {
            x: 0.0,
            y: 0.0,
            width: 101.0,
            height: 30.0,
        }
        .snapped(GRID_8);
        let b = LayoutRect {
            x: 101.0,
            y: 0.0,
            width: 50.0,
            height: 30.0,
        }
        .snapped(GRID_8);
        assert_eq!(a.x + a.width, b.x, "adjacent boxes still touch");
        assert_eq!(a.height, 32.0);
    }

    #[test]
    fn rect_tap_target_grows_about_center() {
        let r = LayoutRect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 100.0,
        };
        assert!(!r.meets_tap_target(TapTarget::WcagAa));
        let g = r.with_min_tap_target(TapTarget::WcagAa);
        assert!(g.meets_tap_target(TapTarget::WcagAa));
        assert_eq!(
            g,
            LayoutRect {
                x: 8.0,
                y: 10.0,
                width: 24.0,
                height: 100.0
            }
        );
    }
}
