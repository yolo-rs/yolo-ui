// SPDX-License-Identifier: Apache-2.0
//! Motion models shared by the web and native hosts.
//!
//! On the web the reveal runs on CSS scroll timelines and the starfield on CSS keyframes; both
//! are switched off by `prefers-reduced-motion` and fall back to their final, static frame. GPU
//! and native hosts (wgpu, Bevy, Blitz) drive the same visuals from these functions, so the two
//! paths agree on what each scroll position or clock tick shows.

/// Progress of a scroll-driven reveal for an element `track_top` px from the document top,
/// `track_height` px tall, in a viewport of `viewport` px scrolled to `scroll`.
///
/// `0.0` when the track's top reaches the viewport top, `1.0` when its bottom reaches the
/// viewport bottom (the CSS `contain` range of a view timeline), clamped.
#[must_use]
pub fn reveal_progress(scroll: f32, viewport: f32, track_top: f32, track_height: f32) -> f32 {
    let travel = track_height - viewport;
    if travel <= 0.0 {
        return 1.0;
    }
    ((scroll - track_top) / travel).clamp(0.0, 1.0)
}

/// One frame of the reveal: where the two title halves sit and how the 3D object is posed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevealFrame {
    /// Horizontal offset of each title half from its resting place, as a fraction of the
    /// viewport width (left half moves by `-split`, right half by `+split`).
    pub split: f32,
    /// Rotation of the object around the vertical axis, degrees.
    pub turn_deg: f32,
    /// Tilt of the object toward the viewer, degrees.
    pub tilt_deg: f32,
    /// Scale of the object.
    pub scale: f32,
    /// Opacity of the body copy.
    pub body_opacity: f32,
}

impl RevealFrame {
    /// The resting frame: title joined, object facing the viewer, copy visible. This is what
    /// reduced motion and unsupported browsers show.
    pub const FINAL: Self = Self { split: 0.0, turn_deg: 0.0, tilt_deg: 12.0, scale: 1.0, body_opacity: 1.0 };

    /// The frame at `progress` (`0.0..=1.0`). With `reduced_motion` it is always [`Self::FINAL`].
    #[must_use]
    pub fn at(progress: f32, reduced_motion: bool) -> Self {
        if reduced_motion {
            return Self::FINAL;
        }
        let p = progress.clamp(0.0, 1.0);
        // Halves converge over the first 60 % of the track, then hold.
        let converge = ease_out_cubic((p / 0.6).min(1.0));
        // Copy fades in over the last 30 %.
        let copy = ((p - 0.7) / 0.3).clamp(0.0, 1.0);
        Self {
            split: 0.32 * (1.0 - converge),
            turn_deg: -150.0 * (1.0 - converge),
            tilt_deg: 12.0 + 18.0 * (1.0 - converge),
            scale: 0.72 + 0.28 * converge,
            body_opacity: copy,
        }
    }
}

/// Cubic ease-out, `0.0..=1.0` to `0.0..=1.0`.
#[must_use]
pub fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// The starfield clock: advances only while running, so pausing freezes the frame where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarfieldClock {
    /// Seconds of animation shown so far.
    pub time: f32,
    /// Whether the user paused the field.
    pub paused: bool,
    /// Whether the system asks for reduced motion.
    pub reduced_motion: bool,
}

impl StarfieldClock {
    /// A clock at zero. Reduced motion starts paused.
    #[must_use]
    pub const fn new(reduced_motion: bool) -> Self {
        Self { time: 0.0, paused: reduced_motion, reduced_motion }
    }

    /// Whether frames should be requested at all.
    #[must_use]
    pub const fn running(&self) -> bool {
        !self.paused && !self.reduced_motion
    }

    /// Advances by `dt` seconds if running.
    pub fn tick(&mut self, dt: f32) {
        if self.running() && dt.is_finite() && dt > 0.0 {
            // Wrap to keep f32 precision: the shader drift repeats every 600 s.
            self.time = (self.time + dt) % 600.0;
        }
    }

    /// Flips the pause switch.
    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }

    /// Uniform block for `shaders/starfield.wgsl`.
    #[must_use]
    pub fn uniforms(&self, width: f32, height: f32, density: f32) -> StarfieldUniforms {
        StarfieldUniforms { resolution: [width, height], time: self.time, density }
    }
}

/// Uniforms of `starfield.wgsl`, laid out like the WGSL struct (16 bytes, no padding).
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct StarfieldUniforms {
    /// Render target size in px.
    pub resolution: [f32; 2],
    /// Seconds from [`StarfieldClock::time`].
    pub time: f32,
    /// Stars per cell, `0.0..=1.0`.
    pub density: f32,
}

impl StarfieldUniforms {
    /// Little-endian bytes for a uniform buffer write.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        for (i, v) in [self.resolution[0], self.resolution[1], self.time, self.density].iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }
}

/// The WGSL source of the starfield.
pub const STARFIELD_WGSL: &str = include_str!("../shaders/starfield.wgsl");

/// A star of the static (CSS) starfield, in percent of the field and px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Horizontal position, `0.0..100.0` %.
    pub x: f32,
    /// Vertical position, `0.0..100.0` %.
    pub y: f32,
    /// Radius in px.
    pub radius: f32,
    /// Opacity.
    pub alpha: f32,
}

/// `count` stars from `seed`, deterministic across platforms (xorshift32), so server and
/// client renders match and screenshots are stable.
#[must_use]
pub fn stars(seed: u32, count: usize) -> Vec<Star> {
    let mut state = seed.max(1);
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        // 24 high-quality bits to an f32 in [0, 1).
        (state >> 8) as f32 / 16_777_216.0
    };
    (0..count)
        .map(|_| {
            let x = next() * 100.0;
            let y = next() * 100.0;
            let m = next();
            Star { x, y, radius: 0.5 + m * m * 1.1, alpha: 0.35 + next() * 0.6 }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_clamped_and_monotonic() {
        let (vp, top, h) = (900.0, 1000.0, 2700.0);
        assert_eq!(reveal_progress(0.0, vp, top, h), 0.0);
        assert_eq!(reveal_progress(top, vp, top, h), 0.0);
        assert_eq!(reveal_progress(top + (h - vp), vp, top, h), 1.0);
        assert_eq!(reveal_progress(1e6, vp, top, h), 1.0);
        let mut last = -1.0;
        for s in (0..5000).step_by(50) {
            let p = reveal_progress(s as f32, vp, top, h);
            assert!(p >= last);
            last = p;
        }
        // A track no taller than the viewport is already revealed.
        assert_eq!(reveal_progress(0.0, 900.0, 0.0, 800.0), 1.0);
    }

    #[test]
    fn reveal_ends_on_the_static_frame() {
        assert_eq!(RevealFrame::at(1.0, false), RevealFrame::FINAL);
        assert_eq!(RevealFrame::at(0.3, true), RevealFrame::FINAL);
        let start = RevealFrame::at(0.0, false);
        assert!(start.split > 0.3 && start.body_opacity == 0.0);
        // The split only closes as progress grows.
        let mut last = f32::MAX;
        for i in 0..=100 {
            let f = RevealFrame::at(i as f32 / 100.0, false);
            assert!(f.split <= last + 1e-6);
            last = f.split;
        }
    }

    #[test]
    fn pause_freezes_the_clock_and_reduced_motion_never_runs() {
        let mut c = StarfieldClock::new(false);
        c.tick(1.5);
        assert!((c.time - 1.5).abs() < 1e-6);
        c.toggle();
        c.tick(10.0);
        assert!((c.time - 1.5).abs() < 1e-6);
        c.toggle();
        c.tick(f32::NAN);
        c.tick(-1.0);
        assert!((c.time - 1.5).abs() < 1e-6);

        let mut r = StarfieldClock::new(true);
        assert!(!r.running());
        r.toggle();
        r.tick(5.0);
        assert_eq!(r.time, 0.0);
    }

    #[test]
    fn uniform_bytes_match_the_wgsl_layout() {
        let u = StarfieldClock::new(false).uniforms(1440.0, 900.0, 0.6);
        assert_eq!(std::mem::size_of::<StarfieldUniforms>(), 16);
        let b = u.to_bytes();
        assert_eq!(f32::from_le_bytes([b[0], b[1], b[2], b[3]]), 1440.0);
        assert_eq!(f32::from_le_bytes([b[12], b[13], b[14], b[15]]), 0.6);
        assert!(STARFIELD_WGSL.contains("struct Uniforms"));
    }

    #[test]
    fn stars_are_deterministic_and_in_bounds() {
        let a = stars(7, 200);
        assert_eq!(a, stars(7, 200));
        assert_ne!(a, stars(8, 200));
        for s in &a {
            assert!((0.0..100.0).contains(&s.x) && (0.0..100.0).contains(&s.y));
            assert!(s.radius >= 0.5 && s.radius <= 1.6 && s.alpha > 0.0 && s.alpha <= 0.95);
        }
    }
}
