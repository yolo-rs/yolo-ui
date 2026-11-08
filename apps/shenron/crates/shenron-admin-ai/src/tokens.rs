// Design tokens of the admin AI cockpit: the single source of the generated stylesheet.
//
// This file is `include!`d by `build.rs` (which resolves every colour role through the HCT /
// CAM16 space of the in-repo `yolo-ui-color` pillar, enforces the WCAG contrast pairs below and
// renders the stylesheet into OUT_DIR) and compiled as a normal module of the crate, so it must
// stay dependency-free. No hand-written CSS file exists: everything below is data plus the
// structural rules rendered by `render_css`.
//
// Identity: Shenron dark surfaces (warm neutral on the ki-orange hue), orange primary, Dragon
// Ball yellow accent. Dark is the default scheme; light is opt-in (`data-theme="light"`).

/// Brand seeds (ARGB). Hues are read from them in HCT; tones are chosen per role.
pub const SEED_ORANGE: u32 = 0xFFE8_641B;
pub const SEED_YELLOW: u32 = 0xFFF5_C518;
pub const SEED_GREEN: u32 = 0xFF2E_A86B;
pub const SEED_RED: u32 = 0xFFE5_3935;
pub const SEED_BLUE: u32 = 0xFF1F_5FA8;
pub const SEED_AMBER: u32 = 0xFFF0_9A1A;

/// Which seed a role takes its hue from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hue {
    Orange,
    Yellow,
    Green,
    Red,
    Blue,
    Amber,
}

impl Hue {
    pub const fn seed(self) -> u32 {
        match self {
            Hue::Orange => SEED_ORANGE,
            Hue::Yellow => SEED_YELLOW,
            Hue::Green => SEED_GREEN,
            Hue::Red => SEED_RED,
            Hue::Blue => SEED_BLUE,
            Hue::Amber => SEED_AMBER,
        }
    }
}

/// A colour role: hue source, chroma, starting tone, and the contrast it must reach.
///
/// `against` lists roles this one is drawn on; the build moves the tone away from them
/// (lighter on dark backgrounds, darker on light ones) until every ratio is at least `min_ratio`.
#[derive(Clone, Copy, Debug)]
pub struct Role {
    pub name: &'static str,
    pub hue: Hue,
    pub chroma: f64,
    pub tone: f64,
    pub against: &'static [&'static str],
    pub min_ratio: f64,
}

const fn role(name: &'static str, hue: Hue, chroma: f64, tone: f64) -> Role {
    Role { name, hue, chroma, tone, against: &[], min_ratio: 1.0 }
}

const fn text(
    name: &'static str,
    hue: Hue,
    chroma: f64,
    tone: f64,
    against: &'static [&'static str],
    min_ratio: f64,
) -> Role {
    Role { name, hue, chroma, tone, against, min_ratio }
}

/// WCAG 2.2: 4.5:1 for text, 3:1 for non-text UI (borders of controls, focus rings, icons).
pub const TEXT_RATIO: f64 = 4.5;
pub const UI_RATIO: f64 = 3.0;

const SURFACES: &[&str] = &["bg", "surface", "surface-2", "surface-3"];

/// Dark scheme (default). Order matters: a role may only reference roles listed before it.
pub const DARK: &[Role] = &[
    role("bg", Hue::Orange, 4.0, 5.0),
    role("surface", Hue::Orange, 5.0, 9.0),
    role("surface-2", Hue::Orange, 6.0, 13.0),
    role("surface-3", Hue::Orange, 7.0, 19.0),
    role("divider", Hue::Orange, 6.0, 24.0),
    text("outline", Hue::Orange, 8.0, 46.0, SURFACES, UI_RATIO),
    text("text", Hue::Orange, 6.0, 94.0, SURFACES, TEXT_RATIO),
    text("text-muted", Hue::Orange, 8.0, 72.0, SURFACES, TEXT_RATIO),
    text("primary", Hue::Orange, 70.0, 68.0, SURFACES, TEXT_RATIO),
    text(
        "on-primary",
        Hue::Orange,
        30.0,
        10.0,
        &["primary"],
        TEXT_RATIO,
    ),
    text("accent", Hue::Yellow, 70.0, 84.0, SURFACES, TEXT_RATIO),
    text("focus", Hue::Yellow, 80.0, 84.0, SURFACES, UI_RATIO),
    role("ok-container", Hue::Green, 18.0, 16.0),
    text(
        "ok",
        Hue::Green,
        50.0,
        78.0,
        &["ok-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("warn-container", Hue::Amber, 20.0, 17.0),
    text(
        "warn",
        Hue::Amber,
        55.0,
        80.0,
        &["warn-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("danger-container", Hue::Red, 24.0, 16.0),
    text(
        "danger",
        Hue::Red,
        55.0,
        74.0,
        &["danger-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("info-container", Hue::Blue, 22.0, 17.0),
    text(
        "info",
        Hue::Blue,
        45.0,
        76.0,
        &["info-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("danger-solid", Hue::Red, 60.0, 40.0),
    text(
        "on-danger-solid",
        Hue::Red,
        10.0,
        98.0,
        &["danger-solid"],
        TEXT_RATIO,
    ),
    role("mark", Hue::Orange, 40.0, 32.0),
    text("on-mark", Hue::Orange, 8.0, 96.0, &["mark"], TEXT_RATIO),
];

/// Light scheme (`data-theme="light"`).
pub const LIGHT: &[Role] = &[
    role("bg", Hue::Orange, 4.0, 97.0),
    role("surface", Hue::Orange, 3.0, 99.0),
    role("surface-2", Hue::Orange, 5.0, 95.0),
    role("surface-3", Hue::Orange, 7.0, 91.0),
    role("divider", Hue::Orange, 6.0, 86.0),
    text("outline", Hue::Orange, 8.0, 56.0, SURFACES, UI_RATIO),
    text("text", Hue::Orange, 8.0, 10.0, SURFACES, TEXT_RATIO),
    text("text-muted", Hue::Orange, 8.0, 38.0, SURFACES, TEXT_RATIO),
    text("primary", Hue::Orange, 70.0, 44.0, SURFACES, TEXT_RATIO),
    text(
        "on-primary",
        Hue::Orange,
        10.0,
        99.0,
        &["primary"],
        TEXT_RATIO,
    ),
    text("accent", Hue::Yellow, 60.0, 40.0, SURFACES, TEXT_RATIO),
    text("focus", Hue::Blue, 60.0, 40.0, SURFACES, UI_RATIO),
    role("ok-container", Hue::Green, 20.0, 93.0),
    text(
        "ok",
        Hue::Green,
        50.0,
        34.0,
        &["ok-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("warn-container", Hue::Amber, 24.0, 93.0),
    text(
        "warn",
        Hue::Amber,
        55.0,
        36.0,
        &["warn-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("danger-container", Hue::Red, 24.0, 93.0),
    text(
        "danger",
        Hue::Red,
        60.0,
        38.0,
        &["danger-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("info-container", Hue::Blue, 22.0, 93.0),
    text(
        "info",
        Hue::Blue,
        50.0,
        36.0,
        &["info-container", "surface-2", "surface-3"],
        TEXT_RATIO,
    ),
    role("danger-solid", Hue::Red, 60.0, 40.0),
    text(
        "on-danger-solid",
        Hue::Red,
        10.0,
        98.0,
        &["danger-solid"],
        TEXT_RATIO,
    ),
    role("mark", Hue::Yellow, 50.0, 86.0),
    text("on-mark", Hue::Orange, 8.0, 10.0, &["mark"], TEXT_RATIO),
];

/// 4/8 px spacing scale, exposed as `--space-N` (px).
pub const SPACE_PX: &[u16] = &[0, 4, 8, 12, 16, 24, 32, 48, 64];
/// Radii (px), exposed as `--radius-<name>`.
pub const RADII: &[(&str, u16)] = &[("sm", 4), ("md", 8), ("lg", 12), ("full", 999)];
/// Type scale (px, line-height px), exposed as `--text-<name>` / `--leading-<name>`.
pub const TYPE_SCALE: &[(&str, u16, u16)] = &[
    ("xs", 12, 16),
    ("sm", 13, 20),
    ("md", 14, 20),
    ("lg", 16, 24),
    ("xl", 20, 28),
    ("2xl", 24, 32),
];
/// Icon sizes (px), exposed as `--icon-<name>`.
pub const ICON_PX: &[(&str, u16)] = &[("sm", 16), ("md", 20), ("lg", 24)];
/// Minimum pointer target (WCAG 2.2 SC 2.5.8 asks 24 x 24 CSS px; controls use 32).
pub const TARGET_MIN_PX: u16 = 24;
pub const CONTROL_PX: u16 = 32;
pub const SHELL_MAX_PX: u16 = 1440;
pub const MOTION_MS: u16 = 160;

/// Brand type stacks; no generic default faces.
pub const FONT_BODY: &str = "\"Shenron Sans\", \"Noto Sans JP\", system-ui, sans-serif";
pub const FONT_DISPLAY: &str = "\"Shenron Display\", \"Noto Sans JP\", system-ui, sans-serif";
pub const FONT_MONO: &str = "\"Shenron Mono\", ui-monospace, \"Cascadia Code\", monospace";

/// A resolved role: CSS name and `#rrggbb`.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub name: &'static str,
    pub hex: String,
}

fn push_colors(out: &mut String, colors: &[Resolved]) {
    for c in colors {
        out.push_str(&format!("  --c-{}: {};\n", c.name, c.hex));
    }
}

/// Renders the whole stylesheet from the tokens and the resolved palettes.
pub fn render_css(dark: &[Resolved], light: &[Resolved]) -> String {
    let mut css =
        String::from("/* Generated by build.rs from src/tokens.rs. Do not edit. */\n:root {\n");
    push_colors(&mut css, dark);
    for (i, px) in SPACE_PX.iter().enumerate() {
        css.push_str(&format!("  --space-{i}: {px}px;\n"));
    }
    for (name, px) in RADII {
        css.push_str(&format!("  --radius-{name}: {px}px;\n"));
    }
    for (name, size, leading) in TYPE_SCALE {
        css.push_str(&format!(
            "  --text-{name}: {size}px;\n  --leading-{name}: {leading}px;\n"
        ));
    }
    for (name, px) in ICON_PX {
        css.push_str(&format!("  --icon-{name}: {px}px;\n"));
    }
    css.push_str(&format!(
        "  --target-min: {TARGET_MIN_PX}px;\n  --control: {CONTROL_PX}px;\n  --shell-max: {SHELL_MAX_PX}px;\n  --motion: {MOTION_MS}ms;\n  --font-body: {FONT_BODY};\n  --font-display: {FONT_DISPLAY};\n  --font-mono: {FONT_MONO};\n  color-scheme: dark;\n}}\n"
    ));
    css.push_str(":root[data-theme=\"light\"], .theme-light {\n");
    push_colors(&mut css, light);
    css.push_str("  color-scheme: light;\n}\n");
    css.push_str(RULES);
    css
}

/// Structural rules. They only reference the custom properties above.
const RULES: &str = r#"
*, *::before, *::after { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body { margin: 0; background: var(--c-bg); color: var(--c-text); font-family: var(--font-body); font-size: var(--text-md); line-height: var(--leading-md); overflow-x: hidden; }
h1, h2, h3, p { margin: 0; }
h1, h2, h3 { font-family: var(--font-display); }
a { color: var(--c-primary); text-underline-offset: 2px; }
code, .mono { font-family: var(--font-mono); font-size: var(--text-xs); }
.tabular, .metric__value, .table td, .mono { font-variant-numeric: tabular-nums; }
:focus { outline: none; }
:focus-visible { outline: 2px solid var(--c-focus); outline-offset: 2px; border-radius: var(--radius-sm); }
.app { min-height: 100vh; background: var(--c-bg); color: var(--c-text); }
.plain { margin: 0; padding: 0; list-style: none; }
.chips { display: flex; flex-wrap: wrap; gap: var(--space-2); margin: 0; padding: 0; list-style: none; }
.skeleton--chip { width: 120px; height: 24px; }
.field--narrow { flex: 0 1 140px; }
.field--mid { flex: 0 1 220px; }
.card--link { text-decoration: none; color: inherit; }
.card--link:hover { border-color: var(--c-primary); }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0; }
.skip-link { position: absolute; left: var(--space-2); top: -64px; z-index: 50; background: var(--c-primary); color: var(--c-on-primary); padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); }
.skip-link:focus-visible { top: var(--space-2); }
.icon { width: var(--icon-sm); height: var(--icon-sm); flex: none; fill: currentColor; }
.icon--md { width: var(--icon-md); height: var(--icon-md); }
.icon--lg { width: var(--icon-lg); height: var(--icon-lg); }
.shell { max-width: var(--shell-max); margin: 0 auto; padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-4); }
.header { background: var(--c-surface); border: 1px solid var(--c-divider); border-radius: var(--radius-lg); padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-4); }
.header__row { display: flex; flex-wrap: wrap; gap: var(--space-3); align-items: center; justify-content: space-between; }
.brand { display: flex; gap: var(--space-3); align-items: center; min-width: 0; }
.brand__mark { width: 40px; height: 40px; border-radius: var(--radius-md); background: var(--c-primary); color: var(--c-on-primary); display: grid; place-items: center; flex: none; }
.title { font-size: var(--text-xl); line-height: var(--leading-xl); font-weight: 800; letter-spacing: 0.02em; }
.subtitle { color: var(--c-text-muted); font-size: var(--text-sm); line-height: var(--leading-sm); }
.toolbar { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; }
.row { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; min-width: 0; }
.spread { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; justify-content: space-between; min-width: 0; }
.stack { display: flex; flex-direction: column; gap: var(--space-3); min-width: 0; }
.stack--tight { gap: var(--space-1); }
.grow { flex: 1 1 200px; min-width: 0; }
.muted { color: var(--c-text-muted); }
.small { font-size: var(--text-xs); line-height: var(--leading-xs); }
.strong { font-weight: 700; }
.break { overflow-wrap: anywhere; }
.pre { white-space: pre-wrap; overflow-wrap: anywhere; }
.conn { display: inline-flex; align-items: center; gap: var(--space-2); min-height: var(--control); padding: 0 var(--space-3); border-radius: var(--radius-full); border: 1px solid var(--c-outline); background: var(--c-surface-2); font-size: var(--text-sm); }
.conn--live { color: var(--c-ok); border-color: var(--c-ok); }
.conn--reconnecting, .conn--connecting { color: var(--c-warn); border-color: var(--c-warn); }
.conn--offline { color: var(--c-danger); border-color: var(--c-danger); }
.dot { width: 8px; height: 8px; border-radius: var(--radius-full); background: currentColor; flex: none; }
.conn--live .dot { animation: pulse 2s ease-in-out infinite; }
@keyframes pulse { 50% { opacity: 0.35; } }
.tabs { display: flex; flex-wrap: wrap; gap: var(--space-1); border-top: 1px solid var(--c-divider); padding-top: var(--space-3); }
.tab { display: inline-flex; align-items: center; gap: var(--space-2); min-height: 40px; padding: 0 var(--space-3); border-radius: var(--radius-md); border: 1px solid transparent; background: transparent; color: var(--c-text-muted); font: inherit; font-weight: 700; font-size: var(--text-sm); cursor: pointer; }
.tab:hover { color: var(--c-text); background: var(--c-surface-2); }
.tab[aria-selected="true"] { background: var(--c-primary); color: var(--c-on-primary); }
.tab__badge { min-width: 20px; padding: 0 var(--space-1); border-radius: var(--radius-full); background: var(--c-surface); color: var(--c-text); font-size: var(--text-xs); text-align: center; font-variant-numeric: tabular-nums; }
.panel { display: flex; flex-direction: column; gap: var(--space-4); min-width: 0; }
.grid { display: grid; gap: var(--space-3); grid-template-columns: repeat(auto-fit, minmax(min(100%, 280px), 1fr)); }
.grid--metrics { grid-template-columns: repeat(auto-fit, minmax(min(100%, 160px), 1fr)); }
.grid--wide { grid-template-columns: repeat(auto-fit, minmax(min(100%, 420px), 1fr)); }
.card { background: var(--c-surface); border: 1px solid var(--c-divider); border-radius: var(--radius-lg); padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-3); min-width: 0; }
.card--inset { background: var(--c-surface-2); border-radius: var(--radius-md); padding: var(--space-3); gap: var(--space-2); }
.card__title { font-size: var(--text-lg); line-height: var(--leading-lg); font-weight: 700; display: flex; align-items: center; gap: var(--space-2); }
.card__title--sm { font-size: var(--text-md); line-height: var(--leading-md); }
.card__title .icon { color: var(--c-primary); }
.card__desc { color: var(--c-text-muted); font-size: var(--text-sm); line-height: var(--leading-sm); }
.metric { background: var(--c-surface); border: 1px solid var(--c-divider); border-radius: var(--radius-lg); padding: var(--space-3) var(--space-4); display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.metric__label { color: var(--c-text-muted); font-size: var(--text-xs); line-height: var(--leading-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; }
.metric__value { font-size: var(--text-2xl); line-height: var(--leading-2xl); font-weight: 800; overflow-wrap: anywhere; }
.metric__value--text { font-size: var(--text-lg); line-height: var(--leading-lg); }
.metric__hint { color: var(--c-text-muted); font-size: var(--text-xs); line-height: var(--leading-xs); display: flex; gap: var(--space-1); align-items: center; overflow-wrap: anywhere; }
.badge { display: inline-flex; align-items: center; gap: var(--space-1); min-height: var(--target-min); padding: 0 var(--space-2); border-radius: var(--radius-sm); border: 1px solid currentColor; font-size: var(--text-xs); font-weight: 700; white-space: nowrap; }
.badge--ok { color: var(--c-ok); background: var(--c-ok-container); }
.badge--warn { color: var(--c-warn); background: var(--c-warn-container); }
.badge--danger { color: var(--c-danger); background: var(--c-danger-container); }
.badge--info { color: var(--c-info); background: var(--c-info-container); }
.badge--neutral { color: var(--c-text-muted); background: var(--c-surface-3); border-color: var(--c-outline); }
.tone-ok { color: var(--c-ok); } .tone-warn { color: var(--c-warn); } .tone-danger { color: var(--c-danger); } .tone-info { color: var(--c-info); } .tone-accent { color: var(--c-accent); } .tone-primary { color: var(--c-primary); }
.btn { display: inline-flex; align-items: center; justify-content: center; gap: var(--space-2); min-height: var(--control); min-width: var(--target-min); padding: 0 var(--space-3); border-radius: var(--radius-md); border: 1px solid var(--c-outline); background: var(--c-surface-2); color: var(--c-text); font: inherit; font-size: var(--text-sm); font-weight: 700; cursor: pointer; text-decoration: none; white-space: nowrap; }
.btn:hover { background: var(--c-surface-3); }
.btn--primary { background: var(--c-primary); border-color: var(--c-primary); color: var(--c-on-primary); }
.btn--primary:hover { background: var(--c-primary); filter: brightness(1.08); }
.btn--danger { background: var(--c-danger-solid); border-color: var(--c-danger-solid); color: var(--c-on-danger-solid); }
.btn--ghost { background: transparent; border-color: transparent; }
.btn--sm { min-height: 28px; padding: 0 var(--space-2); font-size: var(--text-xs); }
.btn[disabled], .btn[aria-disabled="true"] { opacity: 0.55; cursor: not-allowed; filter: none; }
.chip { display: inline-flex; align-items: center; min-height: 28px; padding: 0 var(--space-2); border-radius: var(--radius-md); border: 1px solid var(--c-outline); background: var(--c-surface-2); color: var(--c-text); font: inherit; font-size: var(--text-xs); cursor: pointer; }
.chip:hover { background: var(--c-surface-3); }
.field { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; flex: 1 1 220px; }
.field__label { font-size: var(--text-sm); font-weight: 700; }
.field__hint { color: var(--c-text-muted); font-size: var(--text-xs); line-height: var(--leading-xs); }
.field__error { color: var(--c-danger); font-size: var(--text-xs); line-height: var(--leading-xs); display: flex; gap: var(--space-1); align-items: center; }
.input, .select, .textarea { width: 100%; min-height: var(--control); padding: var(--space-1) var(--space-3); border-radius: var(--radius-md); border: 1px solid var(--c-outline); background: var(--c-surface-2); color: var(--c-text); font: inherit; font-size: var(--text-sm); }
.textarea { min-height: 72px; resize: vertical; line-height: var(--leading-sm); padding: var(--space-2) var(--space-3); }
.input::placeholder, .textarea::placeholder { color: var(--c-text-muted); opacity: 1; }
.input[aria-invalid="true"], .textarea[aria-invalid="true"] { border-color: var(--c-danger); }
.select { width: auto; }
.form { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-3); border: 1px solid var(--c-divider); border-radius: var(--radius-md); background: var(--c-surface-2); }
.form__row { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: flex-end; }
.scroll { max-height: 360px; overflow-y: auto; display: flex; flex-direction: column; gap: var(--space-2); padding-right: var(--space-1); }
.scroll--tall { max-height: 520px; }
.item { background: var(--c-surface-2); border: 1px solid var(--c-divider); border-radius: var(--radius-md); padding: var(--space-3); display: flex; flex-direction: column; gap: var(--space-2); min-width: 0; }
.item__head { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; justify-content: space-between; min-width: 0; }
.table-wrap { overflow-x: auto; border: 1px solid var(--c-divider); border-radius: var(--radius-md); }
.table { width: 100%; border-collapse: collapse; font-size: var(--text-sm); }
.table caption { text-align: left; padding: var(--space-2) var(--space-3); color: var(--c-text-muted); font-size: var(--text-xs); }
.table th { text-align: left; font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.04em; color: var(--c-text-muted); padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--c-divider); white-space: nowrap; }
.table td { padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--c-divider); white-space: nowrap; }
.table tr:last-child td { border-bottom: 0; }
.notice { display: flex; gap: var(--space-2); align-items: flex-start; justify-content: space-between; padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); border: 1px solid currentColor; font-size: var(--text-sm); }
.notice__text { display: flex; gap: var(--space-2); align-items: flex-start; min-width: 0; overflow-wrap: anywhere; }
.notice--ok { color: var(--c-ok); background: var(--c-ok-container); }
.notice--info { color: var(--c-info); background: var(--c-info-container); }
.notice--warn { color: var(--c-warn); background: var(--c-warn-container); }
.notice--error { color: var(--c-danger); background: var(--c-danger-container); }
.notice .btn { color: var(--c-text); }
.empty { padding: var(--space-5) var(--space-3); text-align: center; color: var(--c-text-muted); font-size: var(--text-sm); border: 1px dashed var(--c-outline); border-radius: var(--radius-md); display: flex; flex-direction: column; gap: var(--space-2); align-items: center; }
.skeleton { background: linear-gradient(90deg, var(--c-surface-2) 0%, var(--c-surface-3) 50%, var(--c-surface-2) 100%); background-size: 200% 100%; animation: shimmer 1.4s linear infinite; border-radius: var(--radius-sm); }
.skeleton--line { height: 12px; width: 100%; }
.skeleton--value { height: 28px; width: 60%; }
.skeleton--block { height: 96px; width: 100%; border-radius: var(--radius-md); }
@keyframes shimmer { from { background-position: 200% 0; } to { background-position: -200% 0; } }
.quote { border-left: 3px solid var(--c-primary); background: var(--c-surface-3); padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); font-style: italic; }
.quote__by { font-style: normal; color: var(--c-text-muted); font-size: var(--text-xs); margin-top: var(--space-1); }
mark { background: var(--c-mark); color: var(--c-on-mark); border-radius: 2px; padding: 0 2px; }
.lab { background: var(--c-bg); border: 1px solid var(--c-divider); border-radius: var(--radius-md); padding: var(--space-3); max-height: 300px; overflow-y: auto; display: flex; flex-direction: column; gap: var(--space-2); }
.lab__msg { border-bottom: 1px solid var(--c-divider); padding-bottom: var(--space-2); display: flex; flex-direction: column; gap: var(--space-1); }
.lab__msg:last-child { border-bottom: 0; }
.lab__author { font-weight: 700; color: var(--c-info); }
.lab__author--shenron { color: var(--c-primary); }
.activity { display: flex; flex-direction: column; gap: var(--space-1); font-size: var(--text-xs); color: var(--c-text-muted); }
.dialog-backdrop { position: fixed; inset: 0; background: rgb(0 0 0 / 0.6); display: grid; place-items: center; padding: var(--space-4); z-index: 40; }
.dialog { width: min(100%, 480px); background: var(--c-surface); border: 1px solid var(--c-outline); border-radius: var(--radius-lg); padding: var(--space-5); display: flex; flex-direction: column; gap: var(--space-4); }
.dialog__title { font-size: var(--text-lg); line-height: var(--leading-lg); font-weight: 800; }
.dialog__actions { display: flex; flex-wrap: wrap; gap: var(--space-2); justify-content: flex-end; }
.swatch { width: 20px; height: 20px; border-radius: var(--radius-sm); border: 1px solid var(--c-outline); flex: none; }
.sticky { position: sticky; top: 0; z-index: 10; }
.details > summary { cursor: pointer; min-height: var(--control); display: flex; align-items: center; gap: var(--space-2); font-weight: 700; }
@media (max-width: 640px) {
  .shell { padding: var(--space-2); gap: var(--space-3); }
  .header, .card { padding: var(--space-3); }
  .tab { flex: 1 1 auto; justify-content: center; }
  .metric__value { font-size: var(--text-xl); line-height: var(--leading-xl); }
}
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; transition-duration: 0.01ms !important; scroll-behavior: auto !important; }
  .skeleton { animation: none; }
}
@media (forced-colors: active) {
  .tab[aria-selected="true"] { outline: 2px solid CanvasText; }
  .badge, .btn, .conn { border-color: CanvasText; }
}
"#;
