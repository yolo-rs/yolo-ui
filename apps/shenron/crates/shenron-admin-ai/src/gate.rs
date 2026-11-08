//! Admin gate: the same decisions as `requireAdmin` (page) and `requireIaAdmin` (API) of the Bun
//! site, applied to what the Bun API says about the forwarded session.
//!
//! - no session → `Unauthenticated`: the page redirects to `/signin?callbackURL=%2Fadmin`, the API
//!   answers 401 with `WWW-Authenticate: Bearer`;
//! - a session without `roleAdmin` (including a session whose business user is not linked yet)
//!   → `Forbidden`: the page redirects to `/`, the API answers 403;
//! - `roleAdmin === true` → `Admin`.
//!
//! The hub never validates a session itself: it forwards the better-auth cookies, the admin
//! cookie, `Authorization: Bearer` or `x-admin-token` to the Bun API and reads its verdict.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Admin,
    Unauthenticated,
    Forbidden,
}

/// `GET /api/me` → `{ authenticated, username, avatar, isAdmin }`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Me {
    pub authenticated: bool,
    pub username: Option<String>,
    pub is_admin: bool,
}

/// `iaAccess(me)`: `null` → unauthenticated; `roleAdmin === true` → ok; anything else → forbidden.
pub fn access_from_me(me: &Me) -> Access {
    if !me.authenticated {
        Access::Unauthenticated
    } else if me.is_admin {
        Access::Admin
    } else {
        Access::Forbidden
    }
}

/// Verdict of a gated upstream route (`requireIaAdmin` answers 401 / 403 / 2xx).
pub fn access_from_status(status: u16) -> Option<Access> {
    match status {
        200..=299 => Some(Access::Admin),
        401 => Some(Access::Unauthenticated),
        403 => Some(Access::Forbidden),
        _ => None,
    }
}

/// `requireAdmin()` redirect targets for the HTML page.
pub fn page_redirect(access: Access) -> Option<&'static str> {
    match access {
        Access::Admin => None,
        Access::Unauthenticated => Some("/signin?callbackURL=%2Fadmin"),
        Access::Forbidden => Some("/"),
    }
}

/// HTTP status and i18n key of an API denial (`iaDenied`).
pub fn api_denial(access: Access) -> Option<(u16, &'static str)> {
    match access {
        Access::Admin => None,
        Access::Unauthenticated => Some((401, "adminAi.gate.unauthenticated")),
        Access::Forbidden => Some((403, "adminAi.gate.forbidden")),
    }
}

/// Cookie name prefixes forwarded upstream; every other cookie stays on this hop.
pub const DEFAULT_COOKIE_PREFIXES: &[&str] = &[
    "better-auth.",
    "__Secure-better-auth.",
    "shenron_ba.",
    "__Secure-shenron_ba.",
    "dbfr_admin",
];

/// The credentials of one request, as they will be forwarded.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Credentials {
    pub cookie: Option<String>,
    pub authorization: Option<String>,
    pub admin_token: Option<String>,
}

impl Credentials {
    /// Reads `cookie`, `authorization` (Bearer only) and `x-admin-token` from header pairs.
    pub fn from_headers<'a>(
        headers: impl IntoIterator<Item = (&'a str, &'a str)>,
        cookie_prefixes: &[&str],
    ) -> Self {
        let mut out = Credentials::default();
        let mut cookies: Vec<String> = Vec::new();
        for (name, value) in headers {
            match name.to_ascii_lowercase().as_str() {
                "cookie" => cookies.extend(filter_cookies(value, cookie_prefixes)),
                "authorization" => {
                    let v = value.trim();
                    if v.len() > 7
                        && v[..7].eq_ignore_ascii_case("bearer ")
                        && !v[7..].trim().is_empty()
                    {
                        out.authorization = Some(format!("Bearer {}", v[7..].trim()));
                    }
                }
                "x-admin-token" if !value.trim().is_empty() => {
                    out.admin_token = Some(value.trim().to_owned());
                }
                _ => {}
            }
        }
        if !cookies.is_empty() {
            out.cookie = Some(cookies.join("; "));
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.cookie.is_none() && self.authorization.is_none() && self.admin_token.is_none()
    }

    /// Automation credentials (token admins) are not visible to `/api/me`, which reads the
    /// session only: the hub confirms them on a gated route instead.
    pub fn is_token(&self) -> bool {
        self.authorization.is_some() || self.admin_token.is_some()
    }

    /// Header pairs to forward upstream.
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut h = Vec::new();
        if let Some(c) = &self.cookie {
            h.push(("cookie", c.clone()));
        }
        if let Some(a) = &self.authorization {
            h.push(("authorization", a.clone()));
        }
        if let Some(t) = &self.admin_token {
            h.push(("x-admin-token", t.clone()));
        }
        h
    }

    /// Stable string to hash for the decision cache (never logged).
    pub fn cache_material(&self) -> String {
        format!(
            "c={}\u{1f}a={}\u{1f}t={}",
            self.cookie.as_deref().unwrap_or(""),
            self.authorization.as_deref().unwrap_or(""),
            self.admin_token.as_deref().unwrap_or("")
        )
    }
}

/// Keeps the `name=value` pairs whose name starts with one of `prefixes`.
pub fn filter_cookies(header: &str, prefixes: &[&str]) -> Vec<String> {
    header
        .split(';')
        .map(str::trim)
        .filter(|pair| {
            let name = pair.split('=').next().unwrap_or("").trim();
            !name.is_empty() && pair.contains('=') && prefixes.iter().any(|p| name.starts_with(p))
        })
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn me_maps_like_ia_access() {
        assert_eq!(access_from_me(&Me::default()), Access::Unauthenticated);
        let user = Me { authenticated: true, username: Some("Goku".into()), is_admin: false };
        assert_eq!(access_from_me(&user), Access::Forbidden);
        let admin = Me { is_admin: true, ..user };
        assert_eq!(access_from_me(&admin), Access::Admin);
        let parsed: Me = serde_json::from_str(
            r#"{"authenticated":true,"username":"x","avatar":null,"isAdmin":true}"#,
        )
        .unwrap();
        assert_eq!(access_from_me(&parsed), Access::Admin);
    }

    #[test]
    fn status_and_redirects_match_require_admin() {
        assert_eq!(access_from_status(200), Some(Access::Admin));
        assert_eq!(access_from_status(401), Some(Access::Unauthenticated));
        assert_eq!(access_from_status(403), Some(Access::Forbidden));
        assert_eq!(access_from_status(502), None);
        assert_eq!(
            page_redirect(Access::Unauthenticated),
            Some("/signin?callbackURL=%2Fadmin")
        );
        assert_eq!(page_redirect(Access::Forbidden), Some("/"));
        assert_eq!(page_redirect(Access::Admin), None);
        assert_eq!(api_denial(Access::Unauthenticated).unwrap().0, 401);
        assert_eq!(api_denial(Access::Forbidden).unwrap().0, 403);
    }

    #[test]
    fn only_session_cookies_and_bearer_are_forwarded() {
        let c = Credentials::from_headers(
            [
                (
                    "Cookie",
                    "theme=dark; better-auth.session_token=abc.def; _ga=1; dbfr_admin=sig",
                ),
                ("Authorization", "Basic Zm9v"),
                ("x-admin-token", " "),
            ],
            DEFAULT_COOKIE_PREFIXES,
        );
        assert_eq!(
            c.cookie.as_deref(),
            Some("better-auth.session_token=abc.def; dbfr_admin=sig")
        );
        assert_eq!(c.authorization, None, "only Bearer is forwarded");
        assert_eq!(c.admin_token, None);
        assert!(!c.is_token());

        let t =
            Credentials::from_headers([("authorization", "bearer  tok ")], DEFAULT_COOKIE_PREFIXES);
        assert_eq!(t.authorization.as_deref(), Some("Bearer tok"));
        assert!(t.is_token());
        assert_eq!(
            t.headers(),
            vec![("authorization", "Bearer tok".to_string())]
        );

        let none = Credentials::from_headers([("cookie", "theme=dark")], DEFAULT_COOKIE_PREFIXES);
        assert!(none.is_empty());
    }
}
