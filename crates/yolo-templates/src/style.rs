// SPDX-License-Identifier: Apache-2.0
//! The template stylesheet.
//!
//! Box layout lives in [`LAYOUT_CSS`] and [`LAYOUT_COMPACT_CSS`], written in the `yolo-layout`
//! subset (flat simple selectors, px and %, flex and grid), so the very same rules run in the
//! browser and through Taffy for native hosts and tests. Paint (colour, type, borders, motion,
//! sticky, popovers, media queries) lives in [`PAINT_CSS`] and reads only `--yt-*` tokens from
//! [`crate::theme`].

use std::fmt::Write as _;

use crate::content::DOCS;
use crate::docs::platform_css;
use crate::motion::stars;

/// Viewport width at or below which the compact layout applies.
pub const COMPACT_MAX_WIDTH: u32 = 720;

/// Desktop box layout, `yolo-layout` subset.
pub const LAYOUT_CSS: &str = r"
.yt-nav { display: flex; flex-direction: row; align-items: center; gap: 24px; height: 52px; padding: 0px 24px; }
.yt-navlist { display: flex; flex-direction: row; align-items: center; gap: 4px; flex-grow: 1; }
.yt-navlink { display: flex; flex-direction: row; align-items: center; gap: 4px; min-height: 44px; padding: 0px 12px; }
.yt-stage { display: flex; flex-direction: column; align-items: center; gap: 48px; padding: 96px 24px 112px 24px; min-height: 640px; }
.yt-ctas { display: flex; flex-direction: row; flex-wrap: wrap; justify-content: center; gap: 12px; }
.yt-btn { display: flex; flex-direction: row; align-items: center; justify-content: center; gap: 8px; min-height: 44px; min-width: 44px; padding: 0px 22px; }
.yt-tabs-list { display: flex; flex-direction: row; gap: 4px; padding: 0px 24px; }
.yt-tab { display: flex; align-items: center; min-height: 48px; padding: 0px 16px; flex-shrink: 0; }
.yt-product-hero { display: grid; grid-template-columns: minmax(0px, 1fr) minmax(0px, 1fr); gap: 48px; align-items: center; padding: 72px 24px; }
.yt-sketch { aspect-ratio: 4 / 3; padding: 24px; }
.yt-sections { display: flex; flex-direction: column; gap: 64px; padding: 64px 24px; }
.yt-channels { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 16px; }
.yt-channel { display: flex; flex-direction: column; gap: 12px; padding: 24px; }
.yt-docs { display: grid; grid-template-columns: 280px minmax(0px, 1fr); grid-template-rows: 64px minmax(0px, 1fr); min-height: 720px; }
.yt-docs-top { grid-column: 1 / span 2; grid-row: 1; display: flex; flex-direction: row; align-items: center; justify-content: space-between; gap: 16px; padding: 0px 24px; }
.yt-docs-tree { grid-column: 1; grid-row: 2; display: flex; flex-direction: column; gap: 2px; padding: 16px; }
.yt-docs-main { grid-column: 2; grid-row: 2; display: flex; flex-direction: column; gap: 24px; padding: 40px 48px; }
.yt-app { display: grid; grid-template-columns: 72px minmax(0px, 1fr); grid-template-rows: 56px minmax(0px, 1fr); min-height: 720px; }
.yt-app-bar { grid-column: 1 / span 2; grid-row: 1; display: grid; grid-template-columns: minmax(0px, 1fr) minmax(0px, 640px) minmax(0px, 1fr); align-items: center; gap: 16px; padding: 0px 16px; }
.yt-app-start { display: flex; flex-direction: row; align-items: center; gap: 12px; }
.yt-search { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 44px; }
.yt-search-field { display: flex; flex-grow: 1; height: 44px; }
.yt-icon-btn { display: flex; align-items: center; justify-content: center; width: 44px; height: 44px; flex-shrink: 0; }
.yt-app-end { display: flex; flex-direction: row; justify-content: flex-end; }
.yt-rail { grid-column: 1; grid-row: 2; padding: 8px 4px; }
.yt-rail-list { display: flex; flex-direction: column; gap: 4px; }
.yt-rail-item { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; min-height: 64px; padding: 8px 0px; }
.yt-app-main { grid-column: 2; grid-row: 2; display: flex; flex-direction: column; gap: 16px; padding: 16px 24px; }
.yt-media { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 32px 16px; }
.yt-media-item { display: flex; flex-direction: column; gap: 12px; }
.yt-thumb { aspect-ratio: 16 / 9; }
.yt-media-meta { display: flex; flex-direction: row; gap: 12px; }
.yt-avatar { width: 36px; height: 36px; flex-shrink: 0; }
.yt-editorial-grid { display: grid; grid-template-columns: repeat(3, minmax(0px, 1fr)); gap: 24px; }
.yt-story { display: flex; flex-direction: column; gap: 12px; padding: 24px; }
.yt-story-lead { grid-column: 1 / span 2; grid-row: 1 / span 2; }
";

/// Compact (phone) overrides, `yolo-layout` subset; applied after [`LAYOUT_CSS`] at or below
/// [`COMPACT_MAX_WIDTH`].
pub const LAYOUT_COMPACT_CSS: &str = r"
.yt-nav { gap: 8px; padding: 0px 12px; }
.yt-navlist { gap: 0px; }
.yt-stage { gap: 32px; padding: 64px 16px 96px 16px; min-height: 560px; }
.yt-tabs-list { padding: 0px 8px; }
.yt-product-hero { grid-template-columns: 100%; gap: 32px; padding: 40px 16px; }
.yt-sections { gap: 48px; padding: 40px 16px; }
.yt-docs { grid-template-columns: 100%; grid-template-rows: 64px auto minmax(0px, 1fr); }
.yt-docs-top { grid-column: 1; padding: 0px 16px; }
.yt-docs-tree { grid-column: 1; grid-row: 2; }
.yt-docs-main { grid-column: 1; grid-row: 3; padding: 24px 16px; }
.yt-app { grid-template-columns: 100%; grid-template-rows: 56px minmax(0px, 1fr) 64px; }
.yt-app-bar { grid-column: 1; grid-template-columns: 100%; gap: 8px; padding: 0px 8px; }
.yt-app-start { display: none; }
.yt-app-end { display: none; }
.yt-rail { grid-column: 1; grid-row: 3; padding: 0px; }
.yt-rail-list { flex-direction: row; justify-content: space-around; gap: 0px; }
.yt-rail-item { min-width: 64px; padding: 4px 0px; }
.yt-app-main { grid-column: 1; grid-row: 2; padding: 12px; }
.yt-media { grid-template-columns: 100%; }
.yt-editorial-grid { grid-template-columns: 100%; }
.yt-story-lead { grid-column: 1; grid-row: auto; }
";

/// Paint rules. Colours come only from `--yt-*` custom properties.
pub const PAINT_CSS: &str = r#"
.yt-root{background:var(--yt-canvas);color:var(--yt-text);font-family:var(--yt-font-body);font-size:16px;line-height:1.5;-webkit-text-size-adjust:100%;text-size-adjust:100%}
.yt-root *,.yt-root *::before,.yt-root *::after{box-sizing:border-box}
.yt-root h1,.yt-root h2,.yt-root h3,.yt-root p,.yt-root ul,.yt-root figure,.yt-root pre{margin:0}
.yt-root ul{padding:0;list-style:none}
.yt-root a{color:var(--yt-accent);text-underline-offset:.2em}
.yt-root :focus-visible{outline:3px solid var(--yt-focus);outline-offset:2px}
.yt-root main:focus{outline:none}
.yt-sr{position:absolute!important;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap;border:0}
.yt-icon{width:20px;height:20px;flex:none}
.yt-skip{position:absolute;left:12px;top:-120px;z-index:100;background:var(--yt-accent-fill);color:var(--yt-on-accent);padding:10px 16px;border-radius:var(--yt-radius-md);font-weight:600}
.yt-skip:focus{top:12px}

.yt-globalnav{position:relative;z-index:20;background:var(--yt-surface);border-bottom:1px solid var(--yt-border)}
.yt-nav{max-width:1200px;margin-inline:auto}
.yt-wordmark{font-family:var(--yt-font-display);font-weight:700;font-size:20px;letter-spacing:-.02em;color:var(--yt-text)!important;text-decoration:none;white-space:nowrap}
.yt-navlist{overflow-x:auto;scrollbar-width:none}
.yt-navlink{font:inherit;font-size:15px;font-weight:500;color:var(--yt-text)!important;background:none;border:0;border-radius:999px;cursor:pointer;text-decoration:none;white-space:nowrap}
.yt-navlink:hover{background:var(--yt-raised)}
.yt-navtrigger .yt-icon{width:16px;height:16px;transition:transform .2s}
.yt-navtrigger:has(+ .yt-flyout:popover-open) .yt-icon{transform:rotate(180deg)}
.yt-navtrigger:has(+ .yt-flyout:popover-open){background:var(--yt-raised)}
.yt-flyout{position:fixed;inset:52px 0 auto 0;margin:0;width:100%;max-width:none;max-height:calc(100vh - 52px);overflow:auto;border:0;padding:0;background:var(--yt-raised);color:var(--yt-text);border-bottom:1px solid var(--yt-border);box-shadow:0 28px 48px -12px rgba(0,0,0,.28)}
.yt-flyout-inner{max-width:1200px;margin-inline:auto;padding:32px 24px 40px;display:grid;grid-template-columns:minmax(200px,280px) 1fr;gap:48px}
.yt-flyout-summary{color:var(--yt-muted);font-size:15px;margin-bottom:8px}
.yt-flyout-overview{display:inline-flex;align-items:center;gap:6px;font-weight:600;min-height:44px}
.yt-flyout-groups{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:32px}
.yt-flyout-heading{font-size:12px;letter-spacing:.08em;text-transform:uppercase;color:var(--yt-muted);margin-bottom:8px;font-weight:600}
.yt-flyout-link{display:flex;align-items:center;min-height:40px;color:var(--yt-text)!important;text-decoration:none;font-size:18px;font-weight:600;letter-spacing:-.01em}
.yt-mega .yt-flyout-link{font-size:15px;font-weight:500}
.yt-flyout-link:hover{text-decoration:underline}

.yt-pill{display:inline-flex;align-items:center;gap:10px;min-height:36px;padding:4px 14px 4px 4px;border-radius:999px;border:1px solid color-mix(in srgb,var(--yt-stage-text) 24%,transparent);background:color-mix(in srgb,var(--yt-stage-text) 6%,transparent);color:var(--yt-stage-text)!important;text-decoration:none;font-size:14px;text-align:left}
.yt-pill:hover{border-color:var(--yt-stage-muted)}
.yt-pill-tag{background:var(--yt-stage-fill);color:var(--yt-stage-on-fill);border-radius:999px;padding:3px 10px;font-weight:700;font-size:12px}
.yt-pill .yt-icon{width:16px;height:16px}

.yt-stage{position:relative;overflow:hidden;isolation:isolate;justify-content:center;background:radial-gradient(70% 55% at 50% 108%,color-mix(in srgb,var(--yt-stage-accent) 14%,transparent),transparent 70%),var(--yt-stage);color:var(--yt-stage-text);text-align:center;color-scheme:dark}
.yt-stage :focus-visible{outline-color:var(--yt-stage-focus)}
.yt-stage-copy{position:relative;z-index:1;display:flex;flex-direction:column;align-items:center;gap:20px;max-width:900px}
.yt-eyebrow{font-weight:600;font-size:15px;color:var(--yt-accent)}
.yt-stage .yt-eyebrow{color:var(--yt-stage-accent)}
.yt-stage-title{font-family:var(--yt-font-display);font-size:clamp(40px,7vw,88px);line-height:1.02;letter-spacing:-.035em;font-weight:700;text-wrap:balance}
.yt-lede{font-size:clamp(17px,1.6vw,21px);color:var(--yt-muted);max-width:62ch;text-wrap:pretty}
.yt-stage .yt-lede{color:var(--yt-stage-muted)}
.yt-btn{font:inherit;font-weight:600;font-size:16px;border-radius:999px;text-decoration:none!important;cursor:pointer;border:1.5px solid transparent;transition:box-shadow .15s,background-color .15s}
.yt-btn-primary{background:var(--yt-accent-fill);color:var(--yt-on-accent)!important}
.yt-btn-primary:hover{box-shadow:0 0 0 4px color-mix(in srgb,var(--yt-accent-fill) 28%,transparent)}
.yt-btn-secondary{background:transparent;color:var(--yt-text)!important;border-color:var(--yt-control-border)}
.yt-btn-secondary:hover{background:var(--yt-raised)}
.yt-stage .yt-btn-primary{background:var(--yt-stage-fill);color:var(--yt-stage-on-fill)!important}
.yt-stage .yt-btn-primary:hover{box-shadow:0 0 0 4px color-mix(in srgb,var(--yt-stage-fill) 30%,transparent)}
.yt-stage .yt-btn-secondary{color:var(--yt-stage-text)!important;border-color:var(--yt-stage-muted)}
.yt-stage .yt-btn-secondary:hover{background:color-mix(in srgb,var(--yt-stage-text) 10%,transparent)}
.yt-stage-object{position:relative;z-index:1;width:min(320px,70vw);aspect-ratio:1;display:grid;place-items:center;perspective:900px}

.yt-object{--yt-size:176px;position:relative;width:var(--yt-size);height:var(--yt-size);transform-style:preserve-3d;transform:rotateX(-18deg) rotateY(-32deg)}
.yt-cube{position:absolute;inset:0;transform-style:preserve-3d;--yt-d:calc(var(--yt-size) / 2)}
.yt-cube-core{inset:30%;--yt-d:calc(var(--yt-size) * .2)}
.yt-face{position:absolute;inset:0;border-radius:var(--yt-radius-md);border:1px solid color-mix(in srgb,var(--yt-stage-accent) 60%,transparent);background:linear-gradient(135deg,color-mix(in srgb,var(--yt-stage-accent) 28%,transparent),color-mix(in srgb,var(--yt-stage-accent) 6%,transparent))}
.yt-cube-core .yt-face{border-color:transparent;background:linear-gradient(135deg,var(--yt-stage-accent),color-mix(in srgb,var(--yt-stage-accent) 40%,var(--yt-stage)))}
.yt-face-front{transform:translateZ(var(--yt-d))}
.yt-face-back{transform:rotateY(180deg) translateZ(var(--yt-d))}
.yt-face-right{transform:rotateY(90deg) translateZ(var(--yt-d))}
.yt-face-left{transform:rotateY(-90deg) translateZ(var(--yt-d))}
.yt-face-top{transform:rotateX(90deg) translateZ(var(--yt-d))}
.yt-face-bottom{transform:rotateX(-90deg) translateZ(var(--yt-d))}
.yt-object-idle{animation:yt-turn 30s linear infinite}
@keyframes yt-turn{from{transform:rotateX(-18deg) rotateY(-32deg)}to{transform:rotateX(-18deg) rotateY(328deg)}}

.yt-starfield{position:absolute;inset:0;z-index:0;pointer-events:none}
.yt-stars{position:absolute;inset:0;background-repeat:repeat;background-size:100% 720px}
.yt-stars-1{animation:yt-drift 260s linear infinite}
.yt-stars-2{animation:yt-drift 170s linear infinite}
.yt-stars-3{animation:yt-drift 100s linear infinite}
@keyframes yt-drift{from{background-position:0 0}to{background-position:0 720px}}
.yt-motion-switch{position:absolute;right:16px;bottom:16px;z-index:2;display:inline-flex;align-items:center;gap:10px;min-height:44px;padding:0 14px;border-radius:999px;background:color-mix(in srgb,var(--yt-stage-text) 8%,transparent);color:var(--yt-stage-text);font-size:14px;cursor:pointer}
.yt-pause-switch,.yt-dark-switch{appearance:none;position:relative;margin:0;flex:none;cursor:pointer;border-radius:999px}
.yt-pause-switch{width:38px;height:22px;border:1.5px solid var(--yt-stage-muted);background:transparent}
.yt-pause-switch::before{content:"";position:absolute;top:2.5px;left:2.5px;width:14px;height:14px;border-radius:50%;background:var(--yt-stage-text);transition:transform .15s}
.yt-pause-switch:checked{background:var(--yt-stage-fill);border-color:var(--yt-stage-fill)}
.yt-pause-switch:checked::before{transform:translateX(16px);background:var(--yt-stage-on-fill)}
.yt-root:has(.yt-pause-switch:checked) .yt-motion{animation-play-state:paused}

.yt-sectiontabs{position:sticky;top:0;z-index:10;background:var(--yt-canvas);border-block:1px solid var(--yt-border)}
.yt-tabs-list{max-width:1200px;margin-inline:auto;overflow-x:auto;scrollbar-width:none;justify-content:safe center}
.yt-tab{color:var(--yt-muted)!important;text-decoration:none;font-weight:500;font-size:15px;border-bottom:2px solid transparent;white-space:nowrap}
.yt-tab:hover{color:var(--yt-text)!important}
.yt-tab[aria-current="true"]{color:var(--yt-text)!important;border-bottom-color:var(--yt-accent);font-weight:600}
.yt-section{scroll-margin-top:72px}

.yt-product-hero{max-width:1200px;margin-inline:auto}
.yt-product-copy{display:flex;flex-direction:column;gap:20px;align-items:flex-start}
.yt-product-title{font-family:var(--yt-font-display);font-size:clamp(34px,4.6vw,60px);line-height:1.05;letter-spacing:-.03em;font-weight:700;text-wrap:balance}
.yt-ctas-start{justify-content:flex-start}
.yt-sketch{position:relative}
.yt-sketch-window{height:100%;display:flex;flex-direction:column;overflow:hidden;border-radius:var(--yt-radius-lg);background:var(--yt-surface);border:1px solid var(--yt-border);box-shadow:0 30px 60px -24px rgba(0,0,0,.3)}
.yt-sketch-bar{display:flex;align-items:center;gap:6px;padding:10px 14px;border-bottom:1px solid var(--yt-border)}
.yt-sketch-dot{width:9px;height:9px;border-radius:50%;background:var(--yt-control-border)}
.yt-sketch-title{margin-inline-start:10px;font-size:12px;color:var(--yt-muted)}
.yt-sketch-body{display:grid;grid-template-columns:34% 1fr;flex:1}
.yt-sketch-side{display:flex;flex-direction:column;gap:6px;padding:14px;border-right:1px solid var(--yt-border)}
.yt-sketch-side-item{font-size:12px;color:var(--yt-muted);padding:6px 10px;border-radius:var(--yt-radius-sm)}
.yt-sketch-side-item.is-active{background:var(--yt-raised);color:var(--yt-text);font-weight:600}
.yt-sketch-rows{display:flex;flex-direction:column;gap:10px;padding:16px}
.yt-sketch-row{font-size:12px;padding:10px 12px;border-radius:var(--yt-radius-sm);background:var(--yt-canvas);border:1px solid var(--yt-border)}
.yt-sketch-card{position:absolute;right:0;bottom:0;font-size:13px;font-weight:600;padding:12px 16px;border-radius:var(--yt-radius-md);background:var(--yt-raised);border:1px solid var(--yt-border);box-shadow:0 12px 32px -8px rgba(0,0,0,.3)}

.yt-sections{max-width:860px;margin-inline:auto}
.yt-section h3,.yt-faq h3{font-family:var(--yt-font-display);font-size:28px;line-height:1.15;letter-spacing:-.02em;margin-bottom:12px}
.yt-section p{color:var(--yt-muted);font-size:18px;max-width:65ch}
.yt-points{margin-top:16px;display:grid;gap:8px}
.yt-points li{padding-left:22px;position:relative}
.yt-points li::before{content:"";position:absolute;left:0;top:.7em;width:12px;height:2px;background:var(--yt-accent)}
.yt-faq-list{border-top:1px solid var(--yt-border)}
.yt-faq-item{border-bottom:1px solid var(--yt-border)}
.yt-faq-item summary{cursor:pointer;min-height:56px;display:flex;align-items:center;gap:16px;font-weight:600;font-size:18px;list-style:none}
.yt-faq-item summary::-webkit-details-marker{display:none}
.yt-faq-item summary::after{content:"";margin-inline-start:auto;width:9px;height:9px;flex:none;border-right:2px solid currentColor;border-bottom:2px solid currentColor;transform:rotate(45deg);transition:transform .15s}
.yt-faq-item[open] summary::after{transform:rotate(-135deg)}
.yt-faq-item p{padding:0 0 20px;color:var(--yt-muted)}

.yt-downloads{max-width:1200px;margin-inline:auto;padding:80px 24px;display:flex;flex-direction:column;gap:16px}
.yt-downloads h2,.yt-media-heading,.yt-docs-title{font-family:var(--yt-font-display);letter-spacing:-.025em;line-height:1.1}
.yt-downloads h2{font-size:clamp(30px,3.6vw,44px)}
.yt-channels{margin-top:16px}
.yt-channel{border:1px solid var(--yt-border);border-radius:var(--yt-radius-lg);background:var(--yt-surface)}
.yt-channel.is-recommended{border:2px solid var(--yt-accent)}
.yt-channel-head{display:flex;align-items:center;justify-content:space-between;gap:12px}
.yt-channel h3{font-family:var(--yt-font-display);font-size:24px;letter-spacing:-.01em}
.yt-channel p{color:var(--yt-muted)}
.yt-channel .yt-platforms-label{color:var(--yt-text);font-size:13px;font-weight:600}
.yt-badge{font-size:12px;font-weight:700;padding:4px 10px;border-radius:999px;background:var(--yt-accent-fill);color:var(--yt-on-accent);white-space:nowrap}
.yt-channel .yt-cadence{font-family:var(--yt-font-mono);font-size:13px}
.yt-platforms{display:flex;flex-wrap:wrap;gap:6px}
.yt-platforms li{font-size:13px;padding:4px 10px;border-radius:999px;border:1px solid var(--yt-border)}
.yt-channel .yt-btn{margin-top:auto;align-self:flex-start}

.yt-frame{padding:40px 24px}
.yt-app,.yt-docs{max-width:1440px;margin-inline:auto;overflow:hidden;background:var(--yt-canvas);border:1px solid var(--yt-border);border-radius:var(--yt-radius-lg)}
.yt-app-bar{background:var(--yt-canvas);border-bottom:1px solid var(--yt-border)}
.yt-app-name{font-family:var(--yt-font-display);font-weight:700;font-size:18px;white-space:nowrap}
.yt-icon-btn{border-radius:999px;background:transparent;border:0;color:var(--yt-text);cursor:pointer;position:relative}
.yt-icon-btn:hover{background:var(--yt-raised)}
.yt-rail-toggle:has(:focus-visible){outline:3px solid var(--yt-focus);outline-offset:2px}
.yt-search-field{align-items:center;border:1px solid var(--yt-control-border);border-radius:999px;background:var(--yt-surface);padding:0 18px}
.yt-search-field:has(:focus-visible){outline:3px solid var(--yt-focus);outline-offset:1px}
.yt-search-input{flex:1;min-width:0;height:100%;border:0;background:transparent;color:var(--yt-text);font:inherit;font-size:16px}
.yt-search-input:focus-visible{outline:none}
.yt-search-input::placeholder{color:var(--yt-muted);opacity:1}
.yt-voice{background:var(--yt-surface)}
.yt-rail{background:var(--yt-canvas)}
.yt-rail-item{border-radius:var(--yt-radius-md);color:var(--yt-text)!important;text-decoration:none;font-size:12px;font-weight:500;text-align:center}
.yt-rail-item:hover{background:var(--yt-raised)}
.yt-rail-item[aria-current="page"]{background:var(--yt-surface);font-weight:700}
.yt-rail-item[aria-current="page"] .yt-icon{color:var(--yt-accent)}
.yt-app:has(.yt-rail-switch:checked){grid-template-columns:232px minmax(0,1fr)}
.yt-app:has(.yt-rail-switch:checked) .yt-rail-item{flex-direction:row;justify-content:flex-start;gap:16px;padding:0 16px;min-height:44px;font-size:14px}
.yt-media-heading{font-size:22px}
.yt-media-item{position:relative;content-visibility:auto;contain-intrinsic-size:auto 320px}
.yt-thumb{position:relative;overflow:hidden;border-radius:var(--yt-radius-lg);background:radial-gradient(120% 90% at 18% 12%,hsl(var(--yt-hue) 80% 62% / .95),transparent 55%),radial-gradient(90% 80% at 85% 90%,hsl(calc(var(--yt-hue) + 40) 75% 45% / .9),transparent 60%),linear-gradient(160deg,hsl(var(--yt-hue) 45% 22%),hsl(calc(var(--yt-hue) + 70) 40% 12%))}
.yt-thumb-badge{position:absolute;right:8px;bottom:8px;font-size:12px;font-weight:600;padding:2px 8px;border-radius:var(--yt-radius-sm);background:color-mix(in srgb,var(--yt-stage) 84%,transparent);color:var(--yt-stage-text)}
.yt-avatar{border-radius:50%;display:grid;place-items:center;font-weight:700;font-size:14px;background:hsl(var(--yt-hue) 55% 26%);color:var(--yt-stage-text)}
.yt-media-text{min-width:0}
.yt-media-title{font-size:16px;line-height:1.35;font-weight:600;font-family:var(--yt-font-body)}
.yt-media-title a{color:var(--yt-text)!important;text-decoration:none}
.yt-media-title a::after{content:"";position:absolute;inset:0;border-radius:var(--yt-radius-lg)}
.yt-media-title a:focus-visible{outline:none}
.yt-media-item:has(a:focus-visible){outline:3px solid var(--yt-focus);outline-offset:4px;border-radius:var(--yt-radius-lg)}
.yt-media-item:hover .yt-media-title a{text-decoration:underline}
.yt-media-source,.yt-media-line{font-size:14px;color:var(--yt-muted)}

.yt-editorial{max-width:1200px;margin-inline:auto;padding:88px 24px;display:flex;flex-direction:column;gap:40px}
.yt-kicker{font-size:13px;font-weight:600;letter-spacing:.08em;text-transform:uppercase;color:var(--yt-accent)}
.yt-display{font-family:var(--yt-font-serif);font-size:clamp(44px,8vw,112px);line-height:.95;letter-spacing:-.04em;font-weight:600;max-width:14ch}
.yt-story{position:relative;border-top:1px solid var(--yt-border)}
.yt-story-lead{background:var(--yt-surface);border:1px solid var(--yt-border);border-radius:var(--yt-radius-lg)}
.yt-illus{min-height:280px;display:grid;place-items:center;perspective:900px;border-radius:var(--yt-radius-md);background:var(--yt-stage)}
.yt-story-title{font-family:var(--yt-font-serif);font-size:26px;line-height:1.15;font-weight:600}
.yt-story-lead .yt-story-title{font-size:clamp(28px,3vw,40px)}
.yt-story-title a{color:var(--yt-text)!important;text-decoration:none}
.yt-story-title a:hover{text-decoration:underline}
.yt-dek{color:var(--yt-muted)}

.yt-reveal{background:var(--yt-stage);color:var(--yt-stage-text);color-scheme:dark;overflow:clip}
.yt-reveal :focus-visible{outline-color:var(--yt-stage-focus)}
.yt-reveal-sticky{min-height:100vh;display:grid;place-items:center;align-content:center;gap:32px;padding:80px 24px;text-align:center}
.yt-split{font-family:var(--yt-font-display);font-size:clamp(44px,9vw,128px);line-height:1;letter-spacing:-.04em;font-weight:600;display:flex;flex-wrap:wrap;justify-content:center;column-gap:.25em}
.yt-split-a,.yt-split-b{display:inline-block}
.yt-reveal-object{width:220px;height:220px;display:grid;place-items:center;perspective:900px}
.yt-reveal-object .yt-object{transform:rotateX(-12deg) rotateY(0deg)}
.yt-reveal-body{color:var(--yt-stage-muted);font-size:20px;max-width:46ch}
@supports (animation-timeline: view()){
@media (prefers-reduced-motion: no-preference){
.yt-reveal-track{height:300vh;view-timeline-name:--yt-reveal}
.yt-reveal-sticky{position:sticky;top:0;height:100vh}
.yt-split-a,.yt-split-b,.yt-object-scroll{animation-timing-function:cubic-bezier(.33,1,.68,1);animation-fill-mode:both;animation-timeline:--yt-reveal;animation-range:contain 0% contain 60%}
.yt-split-a{animation-name:yt-split-left}
.yt-split-b{animation-name:yt-split-right}
.yt-object-scroll{animation-name:yt-pose}
.yt-reveal-body{animation:yt-fade linear both;animation-timeline:--yt-reveal;animation-range:contain 70% contain 100%}
}
}
@keyframes yt-split-left{from{transform:translateX(-32vw)}to{transform:none}}
@keyframes yt-split-right{from{transform:translateX(32vw)}to{transform:none}}
@keyframes yt-pose{from{transform:rotateX(-30deg) rotateY(-150deg) scale(.72)}to{transform:rotateX(-12deg) rotateY(0deg) scale(1)}}
@keyframes yt-fade{from{opacity:0}to{opacity:1}}

.yt-docs-top{background:var(--yt-surface);border-bottom:1px solid var(--yt-border)}
.yt-docs-controls{display:flex;align-items:center;gap:16px;flex-wrap:wrap}
.yt-segmented{display:flex;align-items:center;gap:2px;margin:0;padding:3px;border:1px solid var(--yt-control-border);border-radius:999px}
.yt-segmented-legend{position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap}
.yt-segment{position:relative;display:flex;align-items:center;min-height:36px;padding:0 14px;border-radius:999px;font-size:14px;font-weight:500;cursor:pointer;color:var(--yt-muted)}
.yt-segment:has(:checked){background:var(--yt-accent-fill);color:var(--yt-on-accent)}
.yt-segment:has(:focus-visible){outline:3px solid var(--yt-focus);outline-offset:2px}
.yt-switch{display:inline-flex;align-items:center;gap:10px;min-height:44px;font-size:14px;cursor:pointer}
.yt-dark-switch{width:42px;height:24px;border:1.5px solid var(--yt-control-border);background:var(--yt-surface)}
.yt-dark-switch::before{content:"";position:absolute;top:3px;left:3px;width:15px;height:15px;border-radius:50%;background:var(--yt-muted);transition:transform .15s}
.yt-dark-switch:checked{background:var(--yt-accent-fill);border-color:var(--yt-accent-fill)}
.yt-dark-switch:checked::before{transform:translateX(18px);background:var(--yt-on-accent)}
.yt-docs-tree{background:var(--yt-surface);border-right:1px solid var(--yt-border);font-size:14px}
.yt-docs-tree ul ul{padding-left:12px;margin-top:2px}
.yt-tree-summary{min-height:36px;display:flex;align-items:center;padding:0 10px;border-radius:var(--yt-radius-sm);cursor:pointer;font-weight:600}
.yt-tree-link{display:flex;align-items:center;min-height:36px;padding:0 10px;border-radius:var(--yt-radius-sm);color:var(--yt-text)!important;text-decoration:none}
.yt-tree-link:hover,.yt-tree-summary:hover{background:var(--yt-raised)}
.yt-tree-link[aria-current="page"]{background:var(--yt-raised);font-weight:700;box-shadow:inset 3px 0 0 var(--yt-accent)}
.yt-docs-title{font-size:clamp(30px,3.4vw,44px)}
.yt-code{padding:16px 20px;border-radius:var(--yt-radius-md);background:var(--yt-surface);border:1px solid var(--yt-border);font-family:var(--yt-font-mono);font-size:13px;line-height:1.6;overflow-x:auto;white-space:pre}
.yt-table-wrap{overflow-x:auto}
.yt-tokens{border-collapse:collapse;width:100%;font-size:14px}
.yt-tokens caption{text-align:left;color:var(--yt-muted);padding-bottom:12px;caption-side:top}
.yt-tokens th,.yt-tokens td{text-align:left;padding:10px 12px;border-bottom:1px solid var(--yt-border);white-space:nowrap}
.yt-tokens thead th{font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:var(--yt-muted)}
.yt-tokens code{font-family:var(--yt-font-mono);font-size:13px}
.yt-swatch{display:inline-block;width:18px;height:18px;border-radius:5px;vertical-align:middle;margin-right:8px;border:1px solid var(--yt-control-border)}
.yt-footer{padding:40px 24px;border-top:1px solid var(--yt-border);color:var(--yt-muted);text-align:center;font-size:14px}

@media (forced-colors: active){.yt-root :focus-visible{outline-color:CanvasText}.yt-btn,.yt-pill,.yt-channel,.yt-search-field{border:1px solid CanvasText}}
@media (prefers-reduced-motion: reduce){.yt-root .yt-motion,.yt-root .yt-object{animation:none!important}.yt-root *,.yt-root *::before,.yt-root *::after{transition-duration:0s!important}}
"#;

/// Compact-only paint rules.
const PAINT_COMPACT_CSS: &str = "
.yt-flyout-inner{grid-template-columns:1fr;gap:24px;padding:24px 16px 32px}
.yt-rail{border-top:1px solid var(--yt-border)}
.yt-docs-tree{border-right:0;border-bottom:1px solid var(--yt-border)}
.yt-app:has(.yt-rail-switch:checked){grid-template-columns:100%}
.yt-app:has(.yt-rail-switch:checked) .yt-rail-item{flex-direction:column;gap:4px;padding:4px 0;min-height:64px;font-size:12px}
.yt-frame{padding:24px 8px}
.yt-motion-switch{right:8px;bottom:8px}
";

/// Background layers of the CSS starfield: three tiles of generated stars.
#[must_use]
pub fn starfield_css() -> String {
    let mut css = String::new();
    for (layer, (seed, count, scale)) in [(11u32, 70usize, 0.8f32), (23, 45, 1.0), (37, 24, 1.35)].into_iter().enumerate() {
        let _ = write!(css, ".yt-stars-{}{{background-image:", layer + 1);
        for (i, s) in stars(seed, count).iter().enumerate() {
            if i > 0 {
                css.push(',');
            }
            let r = s.radius * scale;
            let _ = write!(
                css,
                "radial-gradient({r:.2}px {r:.2}px at {:.2}% {:.2}%,rgba(236,240,255,{:.2}) 55%,transparent)",
                s.x, s.y, s.alpha
            );
        }
        css.push('}');
    }
    css
}

/// The complete stylesheet: theme tokens, layout, paint, compact overrides, stars, docs.
#[must_use]
pub fn stylesheet() -> String {
    let mut css = crate::theme::theme_css();
    css.push_str(LAYOUT_CSS);
    css.push_str(PAINT_CSS);
    let _ = write!(css, "@media (max-width:{COMPACT_MAX_WIDTH}px){{{LAYOUT_COMPACT_CSS}{PAINT_COMPACT_CSS}}}");
    css.push_str(&starfield_css());
    css.push_str(&platform_css(&DOCS));
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_uses_tokens_not_raw_hex() {
        // Colour literals are allowed only in shadows (rgba black) and the star tint.
        for (i, _) in PAINT_CSS.match_indices('#') {
            let tail = &PAINT_CSS[i..(i + 4).min(PAINT_CSS.len())];
            assert!(
                !tail[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "raw hex colour in PAINT_CSS near {tail}"
            );
        }
    }

    #[test]
    fn every_animation_is_switched_off_by_reduced_motion() {
        assert!(PAINT_CSS.contains("@media (prefers-reduced-motion: reduce)"));
        // Infinite animations are all marked `.yt-motion` (paused by the switch and by reduced
        // motion) or belong to `.yt-object`.
        for selector in [".yt-stars-1", ".yt-stars-2", ".yt-stars-3", ".yt-object-idle"] {
            assert!(PAINT_CSS.contains(selector));
        }
        // Scroll-driven animations only exist inside the no-preference block.
        let scroll = PAINT_CSS.find("animation-timeline:--").unwrap();
        let guard = PAINT_CSS.find("@media (prefers-reduced-motion: no-preference)").unwrap();
        assert!(guard < scroll);
    }

    #[test]
    fn starfield_css_is_stable() {
        assert_eq!(starfield_css(), starfield_css());
        assert_eq!(starfield_css().matches("radial-gradient").count(), 70 + 45 + 24);
    }

    #[test]
    fn stylesheet_wraps_compact_rules_in_a_media_query() {
        let css = stylesheet();
        assert!(css.contains("@media (max-width:720px){"));
        assert!(css.contains(".yt-code{display:none}"));
    }
}
