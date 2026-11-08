use axum::{
    http::header::{CONTENT_TYPE, HeaderValue},
    response::IntoResponse,
};

use crate::content::SITE_ORIGIN;

/// `GET /robots.txt`
pub async fn robots_txt() -> impl IntoResponse {
    let body = format!("User-agent: *\nAllow: /\n\nSitemap: {SITE_ORIGIN}/sitemap.xml\n");
    (
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        body,
    )
}
