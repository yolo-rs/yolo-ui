---
name: "YOLO Sovereign Design System"
description: "Canonical Anti-AI-Slop Generative UI specification for YOLO Engine, GPUI, Dioxus 0.8, COSMIC, Tailwind v4, and Vercel Native SDK"
colors:
  primary: "#ff73a8"
  on-primary: "#1a0911"
  secondary: "#00d4b2"
  tertiary: "#ff9e3b"
  neutral: "#0f1014"
  surface-0: "#0b0c0f"
  surface-1: "#12141a"
  surface-2: "#1a1d26"
  surface-dark: "#0f1014"
  on-surface: "#f3f4f6"
  on-surface-variant: "#9ca3af"
  border-hairline: "oklch(100% 0 0 / 0.08)"
typography:
  display:
    fontFamily: "Google Sans Flex"
    fontSize: "2.75rem"
    fontWeight: 680
    lineHeight: 1.12
    letterSpacing: "-0.03em"
  body:
    fontFamily: "Google Sans Flex"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.5
  mono:
    fontFamily: "JetBrains Mono"
    fontSize: "0.875rem"
    fontWeight: 500
    lineHeight: 1.4
    fontFeature: "tnum"
rounded:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "20px"
  xl: "28px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "20px"
  xl: "32px"
---

## Overview

The YOLO Sovereign Design System unifies Material Design 3 HCT perceptual color mathematics, OKLCH high-contrast neutral surfaces, Google Sans Flex 6-axis variable typography, and deterministic Anti-AI-Slop invariants across Web (Bun + Tailwind v4 + WebGPU), Desktop (Rust GPUI v0.8 + Dioxus 0.8 + COSMIC), and Native Zig (`vercel-labs/native`).

```yaml
colors:
  primary:
    value: "#ff73a8"
    type: color
    description: "Primary brand accent (Aphrody Rose HCT Tone 72)"
  on-primary:
    value: "#1a0911"
    type: color
    description: "Primary foreground ink on primary accent (WCAG > 10:1)"
  surface:
    value: "#0b0c0f"
    type: color
    description: "Base dark neutral canvas (N1 Tone 4)"
  on-surface:
    value: "#f3f4f6"
    type: color
    description: "Primary foreground ink on dark surface (APCA Lc > 90)"
typography:
  fontFamily:
    sans:
      value: "Google Sans Flex, Inter, system-ui, sans-serif"
      type: fontFamily
    mono:
      value: "JetBrains Mono, monospace"
      type: fontFamily
```

## Colors

All colors are derived in perceptual `oklch(L C H)` or Material Design 3 `HCT` space:
- Primary Brand Seed: `#ff73a8` paired with `#1a0911` (`on-primary`).
- Surface Hierarchy: `surface-0` (`#0b0c0f`) -> `surface-1` (`#12141a`) -> `surface-2` (`#1a1d26`).
- Hairline Borders: `oklch(100% 0 0 / 0.08)` with top specular highlight `inset 0 1px 0 0 oklch(100% 0 0 / 0.12)`.

## Typography

- Display and Headlines: `Google Sans Flex` (`wght 680`, `opsz 36`, `GRAD 0`, `letter-spacing: -0.03em`, `text-wrap: balance`).
- Body Prose: `Google Sans Flex` (`1rem`, `line-height: 1.5`, `text-wrap: pretty`, max measure `65ch`).
- Numeric & Telemetry Data: `JetBrains Mono` with mandatory `font-variant-numeric: tabular-nums` (`"tnum" 1`).

## Layout

- Base Spatial Grid: `4px` sub-grid / `8px` structural rhythm.
- Asymmetric Bento and Swiss grids replace generic 3-column equal marketing cards.
- Interactive touch/pointer targets enforce a minimum `44x44px` hit box.

## Elevation & Depth

- Zero fuzzy large drop shadows (`shadow-xl` and `shadow-2xl` are banned).
- Depth is communicated exclusively through tonal surface luminance steps (`N1` tone delta) and `1px` hairline borders.

## Shapes

- Concentric Radius Law: Every nested child container at distance $P$ from an outer container of radius $R_{\text{ext}}$ satisfies $R_{\text{int}} = \max(0, R_{\text{ext}} - P)$.
- Standard radii: `xs = 4px`, `sm = 8px`, `md = 12px`, `lg = 20px`, `xl = 28px`.

## Components

- Cards (`concentric-card`): `border-radius: 20px`, `padding: 12px`, inner media/action radius `8px` (`20 - 12 = 8`).
- Micro-interactions: Under-180ms feedback using `cubic-bezier(0.22, 1, 0.36, 1)` or critically damped M3 Expressive springs ($\zeta = 1.0, k = 450$).

## Do's and Don'ts

- Do enforce `font-variant-numeric: tabular-nums` on every counter, table cell, price, latency metric, and timestamp.
- Do compute APCA contrast ($|L_c| \ge 60$ for body text, $|L_c| \ge 45$ for `>= 24px` headlines) and WCAG 2.1 contrast ($\ge 4.5:1$) via the Zig FFI kernel.
- Don't use AI purple/indigo background gradients ($H \in [265^\circ, 305^\circ], C > 0.14$).
- Don't nest a child element with `border-radius: 20px` inside a padded parent with `border-radius: 20px`.
