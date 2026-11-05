# `yolo-shared`: neutral foundation, manifest and 7-preset theme engine (`crates/yolo-shared`)

`yolo-shared` is the zero-overhead, vendor-neutral foundation crate at the bottom of the `yolo-rs/yolo-ui` dependency graph.

## Neutral by default, with 8 built-in theme presets (`YoloThemePreset`)

`yolo-ui` is neutral and customizable by default (`YoloThemePreset::Neutral`) and ships 8 built-in theme presets:

1. `YoloThemePreset::Neutral` (`"yolo-neutral"`) is the customizable default theme, with zinc/slate surfaces, a `#6366f1` accent and no vendor lock-in.
2. `YoloThemePreset::M3` (`"yolo-m3"`) follows the upstream Google Material Design 3 / Jetpack Compose theme: seed `#6750A4`, HCT tonal spot, `9999px` pill buttons, elevation 0..5 and expressive spring motion.
3. `YoloThemePreset::Fluent` (`"yolo-fluent"`) follows the Microsoft Fluent UI 2 / WinUI 3 design system: brand blue `#0078D4`, Mica / Acrylic / Smoke backdrop vibrancy tokens, `4px` control and `8px` overlay corner radii, and a subtle `1px` reveal stroke.
4. `YoloThemePreset::Apple` (`"yolo-apple"`) covers Apple macOS (Sequoia / Tahoe Liquid Glass `NSGlassEffectView` / AppKit) and iOS (Cupertino / SwiftUI HIG). It uses `#007AFF` system blue, `#34C759` green, `#FF9500` orange and `#FF3B30` red, a `24px` Liquid Glass vibrancy blur, continuous squircle corner radii of `10px`/`14px`, traffic-light window controls in `#FF5F56`/`#FFBD2E`/`#27C93F`, and SF Pro / SF Mono typography.
5. `YoloThemePreset::Omarchy` (`"yolo-omarchy"`) follows the TUI-inspired terminal design system of Omacom Omarchy Linux and `omarchy-site`. It defaults to `Tokyo Night` (`#1a1b26`/`#7aa2f7`/`#bb9af7`) and includes all 23 official `OmarchySubTheme`s from `submodules/omarchy/themes/*/colors.toml`. Borders are sharp (`0px`/`2px`), the font stack is `'Geist Variable'` / `'JetBrains Mono Variable'` / `'CaskaydiaMono Nerd Font'`, and surfaces match Waybar/Walker/Hyprland/Ghostty/Alacritty/Neovim/Btop.
6. `YoloThemePreset::Hyprland` (`"yolo-hyprland"`) follows the Wayland compositor design system of Hyprwm / Hyprland / `hyprtoolkit` / `hyprqt6engine`: an active border gradient from `#33ccff` to `#00ff99` at `45deg`, inactive border `#595959aa`, dark glass surfaces `#0b0e14`/`#111622`/`#192030`, dual-pass kawase blur `size = 8, passes = 3, vibrancy = 0.1696`, rounding `10px`, gaps `5px`/`20px` and bezier `[0.05, 0.9, 0.1, 1.05]`.
7. `YoloThemePreset::Aphrody` (`"yolo-aphrody"`) is the Byron Love (Afuro Terumi / Aphrody) pet and brand theme. It combines the 10 Oklch/sRGB colors measured from the Byron Love 8×11 sprite atlas (`#17335C` azur seed, `#131420` nuit abyss, `#F7F3F0` creme, `#EADBA6` blond, etc.) with celestial gold `#FFD700`, divine violet `#7C5CFF`, cyan `#00D4FF` and `ByronLovePetSpec`.
8. `YoloThemePreset::Chromium` (`"yolo-chromium"`) uses the Chromium Views + WebUI palette, 8px card geometry, a 56px toolbar baseline and an editable HCT accent seed (`#1A73E8`). [`docs/CHROMIUM_UI_DESIGN_SYSTEM.md`](../../docs/CHROMIUM_UI_DESIGN_SYSTEM.md) catalogs the source token contracts.
