const std = @import("std");
const math = std.math;

/// Normalized RGBA structure [0.0 .. 1.0] with C-ABI layout (`callconv(.c)` in Zig 0.17).
pub const RgbaColor = extern struct {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
};

/// Converts an OKLCH color (L: 0..1, C: 0..0.4, H: 0..360 deg) to linear sRGB and gamma-encoded sRGB.
pub export fn yolo_oklch_to_srgb(l: f32, c: f32, h_deg: f32) callconv(.c) RgbaColor {
    const h_rad = h_deg * (math.pi / 180.0);
    const a = c * @cos(h_rad);
    const b = c * @sin(h_rad);

    // OKLab to non-linear LMS
    const l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    const m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    const s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    const l_cubed = l_ * l_ * l_;
    const m_cubed = m_ * m_ * m_;
    const s_cubed = s_ * s_ * s_;

    // LMS to linear sRGB
    const r_lin = 4.0767416621 * l_cubed - 3.3077115913 * m_cubed + 0.2309699292 * s_cubed;
    const g_lin = -1.2684380046 * l_cubed + 2.6097574011 * m_cubed - 0.3413193965 * s_cubed;
    const b_lin = -0.0041960863 * l_cubed - 0.7034186147 * m_cubed + 1.7076147010 * s_cubed;

    return RgbaColor{
        .r = linearToSrgb(math.clamp(r_lin, 0.0, 1.0)),
        .g = linearToSrgb(math.clamp(g_lin, 0.0, 1.0)),
        .b = linearToSrgb(math.clamp(b_lin, 0.0, 1.0)),
        .a = 1.0,
    };
}

/// Computes perceptual APCA-W3 contrast (Lc in [-108 .. +106]), polarity-aware.
pub export fn yolo_apca_contrast(
    txt_r: f32,
    txt_g: f32,
    txt_b: f32,
    bg_r: f32,
    bg_g: f32,
    bg_b: f32,
) callconv(.c) f32 {
    const y_txt = srgbToApcaLuminance(txt_r, txt_g, txt_b);
    const y_bg = srgbToApcaLuminance(bg_r, bg_g, bg_b);

    // Soft clamp at APCA black threshold (blkThrs = 0.022, blkClmp = 1.414)
    const yc_txt = if (y_txt > 0.022) y_txt else y_txt + math.pow(f32, 0.022 - y_txt, 1.414);
    const yc_bg = if (y_bg > 0.022) y_bg else y_bg + math.pow(f32, 0.022 - y_bg, 1.414);

    if (@abs(yc_bg - yc_txt) < 0.0005) return 0.0;

    // Normal polarity (dark ink on light surface) vs reverse polarity (dark mode)
    if (yc_bg > yc_txt) {
        const sapc = (math.pow(f32, yc_bg, 0.56) - math.pow(f32, yc_txt, 0.57)) * 1.14;
        return if (sapc < 0.1) 0.0 else (sapc - 0.027) * 100.0;
    } else {
        const sapc = (math.pow(f32, yc_bg, 0.65) - math.pow(f32, yc_txt, 0.62)) * 1.14;
        return if (sapc > -0.1) 0.0 else (sapc + 0.027) * 100.0;
    }
}

/// Computes standard WCAG 2.1 relative luminance contrast ratio (1.0 .. 21.0)
/// using exact piecewise IEC 61966-2-1 sRGB transfer function.
pub export fn yolo_wcag_contrast_ratio(
    fg_r: f32,
    fg_g: f32,
    fg_b: f32,
    bg_r: f32,
    bg_g: f32,
    bg_b: f32,
) callconv(.c) f32 {
    const l1 = srgbToWcagLuminance(fg_r, fg_g, fg_b);
    const l2 = srgbToWcagLuminance(bg_r, bg_g, bg_b);
    const lighter = @max(l1, l2);
    const darker = @min(l1, l2);
    return (lighter + 0.05) / (darker + 0.05);
}

/// Geometric law of concentric radii: R_inner = max(0, R_outer - Padding).
pub export fn yolo_concentric_inner_radius(outer_radius: f32, padding: f32) callconv(.c) f32 {
    return @max(0.0, outer_radius - padding);
}

/// Closed-form rounded-rect SDF coverage (vercel-labs/native `referenceRoundedRectCoverage`).
pub export fn yolo_rounded_rect_sdf_coverage(
    px: f32,
    py: f32,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
    radius: f32,
) callconv(.c) f32 {
    if (rect_w <= 0.0 or rect_h <= 0.0) return 0.0;
    const half_w = rect_w * 0.5;
    const half_h = rect_h * 0.5;
    const r = math.clamp(radius, 0.0, @min(half_w, half_h));
    const cx = rect_x + half_w;
    const cy = rect_y + half_h;
    const qx = @abs(px - cx) - half_w + r;
    const qy = @abs(py - cy) - half_h + r;

    // Fast interior and exterior short-circuits
    if (@max(qx, qy) <= -0.5) return 1.0;
    if (qx - r >= 0.5 or qy - r >= 0.5) return 0.0;

    const ox = @max(qx, 0.0);
    const oy = @max(qy, 0.0);
    const d = @sqrt(ox * ox + oy * oy) + @min(@max(qx, qy), 0.0) - r;
    return math.clamp(0.5 - d, 0.0, 1.0);
}

/// Detects whether an OKLCH color falls in the banned "AI Purple Slop" zone.
pub export fn yolo_is_ai_purple_slop(l: f32, c: f32, h_deg: f32) callconv(.c) bool {
    return (h_deg >= 265.0 and h_deg <= 305.0) and (c > 0.14) and (l >= 0.35 and l <= 0.75);
}

/// Analytical damped harmonic oscillator solver (Emil Kowalski / M3 Expressive springs).
/// Returns normalized position [0.0 -> 1.0] at time `t_sec` across underdamped, critically
/// damped, and overdamped regimes.
pub export fn yolo_spring_position(t_sec: f32, stiffness: f32, damping_ratio: f32) callconv(.c) f32 {
    if (t_sec <= 0.0) return 0.0;
    const omega_n = @sqrt(@max(stiffness, 0.0001));
    const zeta = math.clamp(damping_ratio, 0.05, 5.0);

    if (zeta < 0.9999) {
        // Underdamped regime (spatial springs)
        const omega_d = omega_n * @sqrt(1.0 - zeta * zeta);
        const env = @exp(-zeta * omega_n * t_sec);
        const cos_part = @cos(omega_d * t_sec);
        const sin_part = (zeta / @sqrt(1.0 - zeta * zeta)) * @sin(omega_d * t_sec);
        return 1.0 - env * (cos_part + sin_part);
    } else if (zeta <= 1.0001) {
        // Critically damped regime (effects springs - zero overshoot)
        const env = @exp(-omega_n * t_sec);
        return 1.0 - env * (1.0 + omega_n * t_sec);
    } else {
        // Overdamped regime (zeta > 1.0)
        const s = @sqrt(zeta * zeta - 1.0);
        const r1 = -omega_n * (zeta - s);
        const r2 = -omega_n * (zeta + s);
        const c2 = -r1 / (r2 - r1);
        const c1 = 1.0 - c2;
        return 1.0 - (c1 * @exp(r1 * t_sec) + c2 * @exp(r2 * t_sec));
    }
}

/// Deterministic 64-bit FNV-1a fingerprint for "Pinned Goldens" (`vercel-labs/native` style).
pub export fn yolo_golden_signature(ptr: [*]const u8, len: usize) callconv(.c) u64 {
    var hash: u64 = 0xcbf29ce484222325;
    var i: usize = 0;
    while (i < len) : (i += 1) {
        hash ^= @as(u64, ptr[i]);
        hash *%= 0x100000001b3;
    }
    return hash;
}

inline fn linearToSrgb(x: f32) f32 {
    return if (x <= 0.0031308) 12.92 * x else 1.055 * math.pow(f32, x, 1.0 / 2.4) - 0.055;
}

inline fn srgbChannelToLinearWcag(c: f32) f32 {
    const clamped = math.clamp(c, 0.0, 1.0);
    return if (clamped <= 0.04045)
        clamped / 12.92
    else
        math.pow(f32, (clamped + 0.055) / 1.055, 2.4);
}

inline fn srgbToWcagLuminance(r: f32, g: f32, b: f32) f32 {
    return 0.2126 * srgbChannelToLinearWcag(r) +
        0.7152 * srgbChannelToLinearWcag(g) +
        0.0722 * srgbChannelToLinearWcag(b);
}

inline fn srgbToApcaLuminance(r: f32, g: f32, b: f32) f32 {
    const r_lin = math.pow(f32, math.clamp(r, 0.0, 1.0), 2.4);
    const g_lin = math.pow(f32, math.clamp(g, 0.0, 1.0), 2.4);
    const b_lin = math.pow(f32, math.clamp(b, 0.0, 1.0), 2.4);
    return 0.2126729 * r_lin + 0.7151522 * g_lin + 0.0721750 * b_lin;
}

test "anti-slop invariants: concentric radii, APCA, WCAG piecewise, SDF, and spring regimes" {
    try std.testing.expectEqual(@as(f32, 8.0), yolo_concentric_inner_radius(20.0, 12.0));
    try std.testing.expect(yolo_is_ai_purple_slop(0.55, 0.22, 280.0));
    try std.testing.expect(!yolo_is_ai_purple_slop(0.62, 0.19, 38.0));

    const lc = yolo_apca_contrast(0.1, 0.1, 0.12, 0.97, 0.97, 0.96);
    try std.testing.expect(lc > 85.0);

    const wcag = yolo_wcag_contrast_ratio(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    try std.testing.expectApproxEqAbs(@as(f32, 21.0), wcag, 0.01);

    // M3 Expressive default effects spring (k = 1600 -> omega_n = 40 rad/s, zeta = 1.0)
    // At t = 0.18s: 1 - exp(-7.2) * (1 + 7.2) = 0.99387 (> 0.99 settled)
    const pos_crit_m3 = yolo_spring_position(0.18, 1600.0, 1.0);
    try std.testing.expect(pos_crit_m3 > 0.99 and pos_crit_m3 <= 1.0);

    // At k = 450 (omega_n = 21.213 rad/s, zeta = 1.0) at t = 0.18s: 1 - exp(-3.818) * 4.818 = 0.8942
    const pos_crit_450 = yolo_spring_position(0.18, 450.0, 1.0);
    try std.testing.expectApproxEqAbs(@as(f32, 0.8942), pos_crit_450, 0.001);

    const pos_over = yolo_spring_position(0.18, 1600.0, 1.35);
    try std.testing.expect(pos_over > 0.90 and pos_over <= 1.0);

    const cov_inside = yolo_rounded_rect_sdf_coverage(50.0, 50.0, 0.0, 0.0, 100.0, 100.0, 12.0);
    try std.testing.expectEqual(@as(f32, 1.0), cov_inside);
}
