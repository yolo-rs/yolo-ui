//! Errors with enough context to fix a bundle.

/// Everything that can go wrong while loading or computing a bundle.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    /// `theme.toml` does not parse as a yolo-theme-pack manifest.
    #[error("theme.toml: {0}")]
    Pack(String),
    /// `bundle.toml` does not parse.
    #[error("bundle.toml: {0}")]
    Extras(String),
    /// `assets.lock.json` does not parse.
    #[error("assets.lock.json: {0}")]
    Lock(String),
    /// The bundle parses but breaks a rule. Every problem is listed.
    #[error("bundle `{id}` is invalid:\n  - {}", .issues.join("\n  - "))]
    Invalid {
        /// Bundle id.
        id: String,
        /// One line per broken rule.
        issues: Vec<String>,
    },
    /// Unknown bundle id and no default registered.
    #[error("unknown theme `{0}`")]
    UnknownTheme(String),
}
