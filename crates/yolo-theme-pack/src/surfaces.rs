//! Which colour role paints which surface. Every consumer (yolo-chromium's `cef_color_id_t` map,
//! the new tab page, the extension, Dioxus and Leptos sites) reads the same role names from
//! [`ResolvedTheme::roles`](crate::ResolvedTheme) and follows this table, so a theme looks the same
//! everywhere. Role names are the stable kebab-case names of `yolo_tokens::role::Role`.

/// (`surface`, background role, foreground role).
pub const ROLE_SURFACES: &[(&str, &str, &str)] = &[
    ("window frame, active", "surface-container-high", "on-surface"),
    ("window frame, inactive", "surface-container", "on-surface-variant"),
    ("toolbar", "surface-container", "on-surface"),
    ("toolbar icon", "surface-container", "on-surface-variant"),
    ("toolbar separator", "surface-container", "outline-variant"),
    ("location bar", "surface-container-highest", "on-surface"),
    ("location bar, hover", "surface-bright", "on-surface"),
    ("omnibox results", "surface-container-low", "on-surface"),
    ("omnibox selected result", "secondary-container", "on-secondary-container"),
    ("tab, active", "surface", "on-surface"),
    ("tab, inactive", "surface-container", "on-surface-variant"),
    ("page background", "surface", "on-surface"),
    ("card", "surface-container-low", "on-surface"),
    ("elevated card, dialog", "surface-container-high", "on-surface"),
    ("primary button", "primary", "on-primary"),
    ("tonal button, selected chip", "secondary-container", "on-secondary-container"),
    ("accent highlight, badge", "tertiary-container", "on-tertiary-container"),
    ("text field", "surface-container-highest", "on-surface"),
    ("text field border, divider", "surface", "outline"),
    ("focus ring", "surface", "primary"),
    ("error", "error-container", "on-error-container"),
    ("snackbar, tooltip", "inverse-surface", "inverse-on-surface"),
    ("scrim behind dialogs", "scrim", "on-surface"),
];

#[cfg(test)]
mod tests {
    use super::ROLE_SURFACES;
    use yolo_tokens::role::Role;

    #[test]
    fn every_role_in_the_table_exists() {
        let names: Vec<&str> = Role::ALL.iter().map(|r| r.name()).collect();
        for (surface, bg, fg) in ROLE_SURFACES {
            assert!(names.contains(bg), "{surface}: unknown role {bg}");
            assert!(names.contains(fg), "{surface}: unknown role {fg}");
        }
    }
}
