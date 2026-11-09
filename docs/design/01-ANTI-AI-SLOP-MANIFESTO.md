# Mathematical Anti-AI-Slop Manifesto

Every interface generated or modified by an agent in `yolo-design` (`C:\src\yolo-ui`) must satisfy the following 6 mathematical invariants before passing the Rust/Zig verification gate:

## Invariant 1: Concentric Radius Geometry
A parent container with outer radius $R_{\text{ext}}$ and internal padding $P$ must never contain an adjacent child element with the same radius. The inner radius $R_{\text{int}}$ is strictly governed by:
$$R_{\text{int}} = \max(0, R_{\text{ext}} - P)$$

## Invariant 2: Perceptual Colorimetry (OKLCH / Material 3 HCT) & APCA / WCAG Contrast
1. All colors are defined in perceptual `oklch(L C H)` space or derived via the Material Design 3 **HCT** (`CAM16` Hue + Chroma, `CIE L*` Tone) solver (`yolo-tokens`).
2. The hue band $H \in [265^\circ, 305^\circ]$ combined with chroma $C > 0.14$ and lightness $L \in [0.35, 0.75]$ in background fills/gradients is rejected by `yolo_is_ai_purple_slop`.
3. Perceptual **APCA ($L_c$)** and **WCAG 2.1** contrast computed by the Zig kernel (`yolo_apca_contrast`, `yolo_wcag_contrast_ratio`) must satisfy:
   - Standard body text: $|L_c| \ge 60$ and WCAG ratio $\ge 4.5:1$ ($\ge 7.0:1$ for AAA).
   - Display headlines (`>= 24px` / `1.5rem`): $|L_c| \ge 45$ and WCAG ratio $\ge 3.0:1$.
   - Structural hairline borders: relative luminance delta $\Delta L \in [0.06, 0.12]$.

## Invariant 3: Parametric Typography (`Google Sans Flex` & Tabular Metrics)
1. Display and section headings use **Google Sans Flex** with optical tracking and balanced line wrapping:
   $$\text{letter-spacing} = -0.03\text{em}, \quad \text{text-wrap: balance}$$
2. Every numeric surface (tables, prices, counters, telemetry metrics, timestamps) enables mandatory tabular figures:
   $$\text{font-variant-numeric: tabular-nums} \quad (\text{font-feature-settings: "tnum" 1})$$

## Invariant 4: Kinematics & Damped Harmonic Oscillator Physics (`emilkowalski` / M3 Expressive)
1. User-triggered UI feedback transitions never exceed $T = 180\text{ ms}$.
2. Permitted curves are `cubic-bezier(0.22, 1, 0.36, 1)` or the analytical second-order damped harmonic oscillator (`yolo_spring_position`) across all three regimes:
   - Underdamped ($\zeta < 1.0$) for spatial geometry morphs (`zeta = 0.8, k = 380`).
   - Critically damped ($\zeta = 1.0$) for opacity and color effects (`zeta = 1.0, k = 450..1600`, zero overshoot).
   - Overdamped ($\zeta > 1.0$) for heavy resistive drags.

## Invariant 5: Luminance Stratification (Zero Fuzzy Drop Shadows)
Dispersed large blur shadows (`shadow-xl`, `shadow-2xl`) are replaced by:
- Progressive surface container luminance steps (`surface-0` -> `surface-1` -> `surface-2` or M3 `surface-container-lowest` to `highest`).
- A `1px solid oklch(100% 0 0 / 0.08)` hairline border paired with a top inner highlight (`inset 0 1px 0 0 oklch(100% 0 0 / 0.12)`).

## Invariant 6: Pinned Goldens & Closed-Form SDF Coverage (`vercel-labs/native`)
Every critical component and generated token bundle carries a 64-bit FNV-1a signature (`yolo_golden_signature`) and evaluates rounded-rect anti-aliasing via the exact closed-form SDF (`yolo_rounded_rect_sdf_coverage`). Unapproved signature drift fails the CI gate.
