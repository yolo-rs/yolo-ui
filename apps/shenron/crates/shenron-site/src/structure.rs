// Structural stylesheet: layout, spacing, a11y and component rules.
//
// Included by `build.rs` and compiled as a module of the crate, so it stays dependency-free.
// It contains no colour literal: every colour is `var(--yolo-<role>)`, computed by
// yolo-stack-theme from the active bundle (see `tests/no_hex.rs`).

/// Rules shared by every theme.
pub fn render_structure() -> String {
    let mut css = String::with_capacity(8192);
    css.push_str(concat!(
        ":root {\n",
        "  --space-1: calc(var(--yolo-space) * 0.5);\n",
        "  --space-2: var(--yolo-space);\n",
        "  --space-3: calc(var(--yolo-space) * 1.5);\n",
        "  --space-4: calc(var(--yolo-space) * 2);\n",
        "  --space-5: calc(var(--yolo-space) * 3);\n",
        "  --space-6: calc(var(--yolo-space) * 4);\n",
        "  --measure: 72ch;\n",
        "  --content-max: 72rem;\n",
        "}\n",
        "*, *::before, *::after { box-sizing: border-box; }\n",
        "html { font-family: var(--yolo-font-body); background: var(--yolo-surface); color: var(--yolo-on-surface); }\n",
        "body { margin: 0; min-height: 100dvh; display: grid; grid-template-rows: auto 1fr auto; }\n",
        ".theme-wallpaper { position: fixed; inset: 0; z-index: -1; background-color: var(--yolo-wallpaper-dominant); background-image: var(--yolo-wallpaper); background-size: cover; background-position: center; }\n",
        "a { color: var(--yolo-primary); text-decoration-thickness: from-font; }\n",
        "h1, h2, h3 { font-family: var(--yolo-font-display); line-height: 1.1; margin: 0 0 var(--space-4); }\n",
        ".site-header, .site-footer { padding: var(--space-3) var(--space-5); background: var(--yolo-surface-container); color: var(--yolo-on-surface); border-block: 1px solid var(--yolo-outline-variant); }\n",
        ".site-header nav { display: flex; gap: var(--space-4); flex-wrap: wrap; align-items: center; }\n",
        ".brand { display: inline-flex; gap: var(--space-2); align-items: center; font-family: var(--yolo-font-header); font-weight: 700; color: var(--yolo-on-surface); text-decoration: none; }\n",
        ".brand-logo { display: inline-block; width: 2rem; height: 2rem; background: var(--yolo-primary); mask: var(--brand-mask) center / contain no-repeat; -webkit-mask: var(--brand-mask) center / contain no-repeat; }\n",
        ".lang-switch { margin-inline-start: auto; display: flex; gap: var(--space-2); }\n",
        ".lang-switch a[aria-current='true'] { font-weight: 700; text-decoration: underline; }\n",
        "main { width: min(100% - 2 * var(--space-5), var(--content-max)); margin-inline: auto; margin-block: var(--space-5); padding: var(--space-5); background: var(--yolo-surface); color: var(--yolo-on-surface); border-radius: var(--yolo-radius-lg); }\n",
        "main p { max-width: var(--measure); }\n",
        ".card-grid { display: grid; gap: var(--space-4); grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr)); padding: 0; list-style: none; }\n",
        ".card { padding: var(--space-4); border: 1px solid var(--yolo-outline-variant); border-radius: var(--yolo-radius-md); background: var(--yolo-surface-container-low); color: var(--yolo-on-surface); }\n",
        ".reader { display: grid; gap: var(--space-3); }\n",
        ".reader canvas { width: 100%; aspect-ratio: 2 / 3; background: var(--yolo-scrim); border-radius: var(--yolo-radius-sm); display: block; }\n",
        ".reader-nav { display: flex; justify-content: space-between; gap: var(--space-3); }\n",
        ".muted { color: var(--yolo-on-surface-variant); }\n",
        ".site-footer { margin-block-start: auto; }\n",
        // Controls
        "button, select, input { font: inherit; color: var(--yolo-on-surface); }\n",
        "button, select { background: var(--yolo-surface-container-high); border: 1px solid var(--yolo-outline); border-radius: var(--yolo-radius-sm); padding: var(--space-1) var(--space-3); }\n",
        "button.primary { background: var(--yolo-primary); color: var(--yolo-on-primary); border-color: var(--yolo-primary); }\n",
        "input[type='range'] { accent-color: var(--yolo-primary); width: 100%; }\n",
        // Theme switcher and settings panel
        ".theme-switcher { display: inline-flex; gap: var(--space-2); align-items: center; }\n",
        ".theme-panel { display: grid; gap: var(--space-5); }\n",
        ".theme-grid { display: grid; gap: var(--space-3); grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr)); padding: 0; list-style: none; margin: 0; }\n",
        ".theme-choice { display: grid; gap: var(--space-2); justify-items: start; text-align: start; padding: var(--space-3); width: 100%; background: var(--yolo-surface-container-low); border: 2px solid var(--yolo-outline-variant); border-radius: var(--yolo-radius-md); }\n",
        ".theme-choice[aria-pressed='true'] { border-color: var(--yolo-primary); }\n",
        ".theme-choice img { width: 100%; aspect-ratio: 16 / 10; object-fit: cover; border-radius: var(--yolo-radius-sm); }\n",
        ".field { display: grid; gap: var(--space-1); }\n",
        ".field-row { display: grid; gap: var(--space-4); grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr)); }\n",
        ".preview { display: grid; gap: var(--space-3); padding: var(--space-4); border: 1px solid var(--yolo-outline-variant); border-radius: var(--yolo-radius-lg); background: var(--yolo-surface-container); color: var(--yolo-on-surface); }\n",
        ".swatches { display: flex; flex-wrap: wrap; gap: var(--space-2); }\n",
        ".swatch { padding: var(--space-2) var(--space-3); border-radius: var(--yolo-radius-sm); font-size: 0.875rem; }\n",
        ".swatch-primary { background: var(--yolo-primary); color: var(--yolo-on-primary); }\n",
        ".swatch-secondary { background: var(--yolo-secondary-container); color: var(--yolo-on-secondary-container); }\n",
        ".swatch-tertiary { background: var(--yolo-tertiary-container); color: var(--yolo-on-tertiary-container); }\n",
        ".swatch-error { background: var(--yolo-error-container); color: var(--yolo-on-error-container); }\n",
        ".audit { border-collapse: collapse; width: 100%; font-size: 0.875rem; }\n",
        ".audit th, .audit td { text-align: start; padding: var(--space-1) var(--space-2); border-block-end: 1px solid var(--yolo-outline-variant); }\n",
        ".audit [data-pass='false'] { font-weight: 700; text-decoration: underline wavy; }\n",
        ".overrides { display: grid; gap: var(--space-2); padding: 0; list-style: none; }\n",
        ".sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }\n",
        // Companion pet
        ".pet { position: fixed; bottom: var(--space-4); width: var(--pet-size, 6rem); height: var(--pet-size, 6rem); z-index: 6; cursor: pointer; transition: translate var(--yolo-motion-duration) var(--yolo-motion-easing); user-select: none; }\n",
        ".pet[data-anchor='bottom-right'] { right: var(--space-4); }\n",
        ".pet[data-anchor='bottom-left'] { left: var(--space-4); }\n",
        ".pet[data-anchor='dock'] { right: var(--space-4); bottom: 6rem; }\n",
        ".pet img { width: 100%; height: 100%; pointer-events: none; }\n",
        ".pet[data-idle~='float'] { animation: pet-float 3.2s ease-in-out infinite; }\n",
        ".pet[data-idle~='bob'] { animation: pet-float 2.4s ease-in-out infinite; }\n",
        ".pet[data-idle~='sway'] { animation: pet-sway 3.6s ease-in-out infinite; }\n",
        ".pet[data-idle~='scan'] { animation: pet-sway 2.2s ease-in-out infinite; }\n",
        ".pet.pet-greet { animation: pet-greet 0.6s ease-out 1; }\n",
        "@keyframes pet-float { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-0.5rem); } }\n",
        "@keyframes pet-sway { 0%, 100% { transform: rotate(-3deg); } 50% { transform: rotate(3deg); } }\n",
        "@keyframes pet-greet { 0% { transform: scale(1); } 40% { transform: scale(1.15) rotate(-6deg); } 100% { transform: scale(1); } }\n",
    ));

    // Accessibility: visible focus, forced colors, reduced motion.
    css.push_str(concat!(
        ":focus-visible { outline: 3px solid var(--yolo-primary); outline-offset: 2px; border-radius: var(--yolo-radius-sm); }\n",
        "@media (forced-colors: active) {\n",
        "  :focus-visible { outline-color: Highlight; }\n",
        "  .card, .preview, .theme-choice, button, select { border: 1px solid CanvasText; }\n",
        "  .theme-choice[aria-pressed='true'] { outline: 3px solid Highlight; }\n",
        "  .brand-logo { background: CanvasText; forced-color-adjust: none; }\n",
        "  .theme-wallpaper { display: none; }\n",
        "  .swatch { border: 1px solid CanvasText; forced-color-adjust: none; }\n",
        "}\n",
        "@media (prefers-reduced-motion: reduce) {\n",
        "  *, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; transition-duration: 0.01ms !important; scroll-behavior: auto !important; }\n",
        "  .pet { display: none; }\n",
        "}\n",
        ":root[data-motion='reduce'] *, :root[data-motion='reduce'] *::before, :root[data-motion='reduce'] *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; transition-duration: 0.01ms !important; scroll-behavior: auto !important; }\n",
        ":root[data-motion='reduce'] .pet { display: none; }\n",
    ));
    css
}

/// Shell layout rules for one theme, keyed by `:root[data-theme]`.
pub fn render_layout(theme_id: &str, layout: &str) -> String {
    let root = format!(":root[data-theme=\"{theme_id}\"]");
    match layout {
        "sidebar" => format!(
            concat!(
                "{r} body {{ grid-template-columns: minmax(12rem, 16rem) 1fr; grid-template-rows: 1fr auto; }}\n",
                "{r} .site-header {{ grid-row: 1 / 3; position: sticky; top: 0; height: 100dvh; border-block: 0; border-inline-end: 1px solid var(--yolo-outline-variant); }}\n",
                "{r} .site-header nav {{ flex-direction: column; align-items: stretch; }}\n",
                "{r} .lang-switch {{ margin-inline-start: 0; }}\n",
                "{r} .site-footer {{ grid-column: 2; }}\n",
            ),
            r = root
        ),
        "floating" => format!(
            concat!(
                "{r} .site-header {{ position: sticky; top: var(--space-3); margin: var(--space-3); z-index: 4; border: 1px solid var(--yolo-outline-variant); border-radius: var(--yolo-radius-lg); backdrop-filter: blur(var(--yolo-glass-blur)); }}\n",
                "{r} .site-footer {{ margin: var(--space-3); border: 1px solid var(--yolo-outline-variant); border-radius: var(--yolo-radius-lg); }}\n",
            ),
            r = root
        ),
        "dock" => format!(
            concat!(
                "{r} body {{ padding-block-end: 6rem; }}\n",
                "{r} .site-header {{ position: fixed; inset-block-end: var(--space-3); inset-inline: 0; width: fit-content; margin-inline: auto; z-index: 5; border: 1px solid var(--yolo-outline-variant); border-radius: var(--yolo-radius-lg); }}\n",
                "{r} .site-header .lang-switch {{ margin-inline-start: var(--space-3); }}\n",
            ),
            r = root
        ),
        // `top-bar` is the base layout.
        _ => String::new(),
    }
}
