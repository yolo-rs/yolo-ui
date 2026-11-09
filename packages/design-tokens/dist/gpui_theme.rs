//! AUTO-GENERATED GPUI THEME FROM DESIGN.md

#[derive(Debug, Clone, Copy)]
pub struct YoloGpuiTheme {
    pub color_primary: &'static str,
    pub color_on_primary: &'static str,
    pub color_secondary: &'static str,
    pub color_tertiary: &'static str,
    pub color_neutral: &'static str,
    pub color_surface_0: &'static str,
    pub color_surface_1: &'static str,
    pub color_surface_2: &'static str,
    pub color_surface_dark: &'static str,
    pub color_on_surface: &'static str,
    pub color_on_surface_variant: &'static str,
    pub color_border_hairline: &'static str,
    pub radius_xs: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_xl: f32,
}

pub const YOLO_GPUI_THEME: YoloGpuiTheme = YoloGpuiTheme {
    color_primary: "#ff73a8",
    color_on_primary: "#1a0911",
    color_secondary: "#00d4b2",
    color_tertiary: "#ff9e3b",
    color_neutral: "#0f1014",
    color_surface_0: "#0b0c0f",
    color_surface_1: "#12141a",
    color_surface_2: "#1a1d26",
    color_surface_dark: "#0f1014",
    color_on_surface: "#f3f4f6",
    color_on_surface_variant: "#9ca3af",
    color_border_hairline: "oklch(100% 0 0 / 0.08)",
    radius_xs: 4.0,
    radius_sm: 8.0,
    radius_md: 12.0,
    radius_lg: 20.0,
    radius_xl: 28.0,
};

impl YoloGpuiTheme {
    #[inline]
    pub const fn concentric_radius(outer_px: f32, padding_px: f32) -> f32 {
        let diff = outer_px - padding_px;
        if diff > 0.0 { diff } else { 0.0 }
    }
}
