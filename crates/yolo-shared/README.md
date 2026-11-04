# `yolo-shared` — Neutral Foundation, Manifest & 7-Preset Theme Engine (`crates/yolo-shared`)

`yolo-shared` is the zero-overhead, vendor-neutral foundation crate at the bottom of the `yolo-rs/yolo-ui` dependency graph.

## Neutral-by-Default & 7 Built-in Theme Presets (`YoloThemePreset`)

`yolo-ui` is **100% neutral and customizable by default** (`YoloThemePreset::Neutral`), while offering **7 first-class built-in theme presets**:

1. **`YoloThemePreset::Neutral` (`"yolo-neutral"`)** — Clean, neutral customizable default theme (zinc/slate surfaces, `#6366f1` accent, zero vendor lock-in).
2. **`YoloThemePreset::M3` (`"yolo-m3"`)** — Google Material Design 3 / Jetpack Compose upstream theme (seed `#6750A4`, HCT tonal spot, pill buttons `9999px`, elevation 0..5, expressive spring motion).
3. **`YoloThemePreset::Fluent` (`"yolo-fluent"`)** — Microsoft Fluent UI 2 / WinUI 3 Design system (brand blue `#0078D4`, Mica / Acrylic / Smoke backdrop vibrancy tokens, `4px` control / `8px` overlay corner radii, subtle `1px` reveal stroke).
4. **`YoloThemePreset::Apple` (`"yolo-apple"`)** — Apple macOS (Sequoia / Tahoe Liquid Glass `NSGlassEffectView` / AppKit) & iOS (Cupertino / SwiftUI HIG) theme (`#007AFF` system blue, `#34C759` green, `#FF9500` orange, `#FF3B30` red, Liquid Glass `24px` vibrancy blur, continuous squircle corner radii `10px`/`14px`, traffic-light window controls `#FF5F56`/`#FFBD2E`/`#27C93F`, SF Pro / SF Mono typography).
5. **`YoloThemePreset::Omarchy` (`"yolo-omarchy"`)** — Omacom Omarchy Linux & `omarchy-site` TUI-inspired terminal-modern design system (`Tokyo Night` default `#1a1b26`/`#7aa2f7`/`#bb9af7` + all **23 official `OmarchySubTheme`s** from `submodules/omarchy/themes/*/colors.toml`, sharp `0px`/`2px` borders, `'Geist Variable'` / `'JetBrains Mono Variable'` / `'CaskaydiaMono Nerd Font'` stack, Waybar/Walker/Hyprland/Ghostty/Alacritty/Neovim/Btop surface parity).
6. **`YoloThemePreset::Hyprland` (`"yolo-hyprland"`)** — Hyprwm / Hyprland / `hyprtoolkit` / `hyprqt6engine` Wayland compositor design system (signature active border gradient `#33ccff` -> `#00ff99` at `45deg`, inactive border `#595959aa`, deep dark-glass surfaces `#0b0e14`/`#111622`/`#192030`, dual-pass kawase blur `size = 8, passes = 3, vibrancy = 0.1696`, rounding `10px`, gaps `5px`/`20px`, bezier `[0.05, 0.9, 0.1, 1.05]`).
7. **`YoloThemePreset::Aphrody` (`"yolo-aphrody"`)** — Signature Byron Love (Afuro Terumi / Aphrody) pet & brand theme combining the 10 Oklch/sRGB colors measured from the Byron Love 8×11 sprite atlas (`#17335C` azur seed, `#131420` nuit abyss, `#F7F3F0` creme, `#EADBA6` blond, etc.) with celestial gold `#FFD700`, divine violet `#7C5CFF`, cyan `#00D4FF`, and `ByronLovePetSpec`.
