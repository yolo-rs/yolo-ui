//! Design tokens as Rust constants. No hand-written CSS: every style string is
//! assembled from these tokens at render time.

pub const BG: &str = "#0e0f12";
pub const SURFACE: &str = "#17191f";
pub const SURFACE_HI: &str = "#22252d";
pub const OUTLINE: &str = "#2f3340";
pub const TEXT: &str = "#e8e6df";
pub const TEXT_DIM: &str = "#9a9ea8";
pub const ACCENT: &str = "#f2a53b"; // Shenron amber
pub const ACCENT_INK: &str = "#1a1206";

pub const FONT_UI: &str = "\"IBM Plex Sans\", \"Noto Sans\", system-ui, sans-serif";
pub const FONT_MONO: &str = "\"Iosevka\", \"JetBrains Mono\", ui-monospace, monospace";

pub const RADIUS: f32 = 6.0;
pub const TOOLBAR_H: f32 = 48.0;
pub const STRIP_W: f32 = 112.0;
pub const GAP: f32 = 8.0;
pub const PAD: f32 = 8.0;

/// Absolute positioning string for a Taffy rect (CSS px).
pub fn abs(rect: crate::layout::Rect) -> String {
    format!(
        "position:absolute;left:{}px;top:{}px;width:{}px;height:{}px;box-sizing:border-box;",
        rect.x, rect.y, rect.w, rect.h
    )
}

pub fn root_style() -> String {
    format!(
        "position:relative;width:100vw;height:100vh;overflow:hidden;background:{BG};color:{TEXT};font-family:{FONT_UI};font-size:14px;user-select:none;margin:0;"
    )
}

pub fn toolbar_style(rect: crate::layout::Rect) -> String {
    format!(
        "{}display:flex;align-items:center;gap:{GAP}px;padding:0 {PAD}px;background:{SURFACE};border-bottom:1px solid {OUTLINE};",
        abs(rect)
    )
}

pub fn strip_style(rect: crate::layout::Rect) -> String {
    format!(
        "{}display:flex;flex-direction:column;gap:{GAP}px;padding:{PAD}px;overflow-y:auto;background:{SURFACE};border-left:1px solid {OUTLINE};",
        abs(rect)
    )
}

/// Canvas area: transparent so the wgpu surface underneath (desktop) is visible;
/// on web the <canvas> element fills this box.
pub fn canvas_area_style(rect: crate::layout::Rect) -> String {
    format!(
        "{}background:transparent;cursor:grab;touch-action:none;overflow:hidden;",
        abs(rect)
    )
}

pub fn canvas_el_style() -> String {
    "display:block;width:100%;height:100%;".to_string()
}

pub fn button_style(active: bool) -> String {
    let (bg, fg) = if active {
        (ACCENT, ACCENT_INK)
    } else {
        (SURFACE_HI, TEXT)
    };
    format!(
        "appearance:none;border:1px solid {OUTLINE};border-radius:{RADIUS}px;background:{bg};color:{fg};padding:6px 10px;font:inherit;cursor:pointer;line-height:1;"
    )
}

pub fn label_style() -> String {
    format!("color:{TEXT_DIM};font-family:{FONT_MONO};font-size:12px;margin-left:auto;")
}

pub fn title_style() -> String {
    format!("color:{ACCENT};font-weight:600;letter-spacing:0.04em;margin-right:{GAP}px;")
}

pub fn thumb_style(active: bool) -> String {
    let border = if active { ACCENT } else { OUTLINE };
    format!(
        "flex:none;width:100%;aspect-ratio:3/4;border-radius:{RADIUS}px;border:2px solid {border};background:{SURFACE_HI};display:flex;align-items:center;justify-content:center;color:{TEXT_DIM};font-family:{FONT_MONO};font-size:12px;cursor:pointer;"
    )
}
