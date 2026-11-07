# yolo-templates assets

`catalog/assets.lock.toml` does not exist on `origin/main` yet, so the asset record of this crate lives here.
Move these rows into the lock file when it lands.

The crate ships no image, video, model or font file. Everything visual is code written for this crate.

| Asset | Kind | Source | License | Notes |
| --- | --- | --- | --- | --- |
| Line icons (`src/icons.rs`: search, mic, chevron, arrow, menu, home, compass, library, clock, grid, download) | SVG path data | Original, drawn for yolo-templates on a 24 px grid | MIT OR Apache-2.0 (crate license) | Replaceable by `yolo-icons` through `IconName` |
| Cube object (`Object3d`) | CSS 3D faces | Original geometry | MIT OR Apache-2.0 | Twelve faces tinted from `--yt-stage-accent`; no model file |
| Product UI drawings (`UiSketch`) | HTML boxes | Original, generated from `content.rs` | MIT OR Apache-2.0 | Shapes and text only, no screenshot |
| Media tile artwork (`MediaTile`) | CSS gradients | Original, generated from a hue per item | MIT OR Apache-2.0 | No image file |
| Starfield (`starfield_css`, `shaders/starfield.wgsl`) | CSS gradients, WGSL | Original, deterministic xorshift32 star positions | MIT OR Apache-2.0 | No texture |
| Copy (`src/content.rs`) | Text | Original Aphrody copy | MIT OR Apache-2.0 | Product facts from the product repositories; no reference wording |

## Fonts (referenced by family name, not bundled)

All are SIL Open Font License 1.1 variable fonts. A host that bundles one must ship the font file together with
its `OFL.txt` and record the release it took them from.

| Family | Role | Upstream | License |
| --- | --- | --- | --- |
| Instrument Sans | body | https://github.com/Instrument/instrument-sans | OFL-1.1 |
| Bricolage Grotesque | display | https://github.com/ateliertriay/bricolage | OFL-1.1 |
| Fraunces | editorial serif | https://github.com/undercasetype/Fraunces | OFL-1.1 |
| JetBrains Mono | code, Omarchy identity | https://github.com/JetBrains/JetBrainsMono | OFL-1.1 |
| Space Grotesk | Hyprland display | https://github.com/floriankarsten/space-grotesk | OFL-1.1 |
| Atkinson Hyperlegible Next | body fallback | https://github.com/googlefonts/atkinson-hyperlegible-next | OFL-1.1 |
| Source Serif 4 | serif fallback | https://github.com/adobe-fonts/source-serif | OFL-1.1 |
| Iosevka | mono fallback | https://github.com/be5invis/Iosevka | OFL-1.1 |

Proprietary faces used by the design references (Google Sans, Product Sans, SF Pro, Segoe UI Variable) are not
referenced by these stacks.
