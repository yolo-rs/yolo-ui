use axum::{
    http::header::{CONTENT_TYPE, HeaderValue},
    response::IntoResponse,
};

use crate::{
    content::{all_paths, url},
    i18n::Lang,
};

/// Builds the sitemap: one `<url>` per (locale, page) with `xhtml:link`
/// alternates for every locale and `x-default`.
pub fn render_sitemap() -> String {
    let mut xml = String::with_capacity(16 * 1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(
        r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">"#,
    );
    xml.push('\n');
    for path in all_paths() {
        for lang in Lang::ALL {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", escape(&url(lang, &path))));
            for alt in Lang::ALL {
                xml.push_str(&format!(
                    r#"    <xhtml:link rel="alternate" hreflang="{}" href="{}"/>"#,
                    alt.code(),
                    escape(&url(alt, &path))
                ));
                xml.push('\n');
            }
            xml.push_str(&format!(
                r#"    <xhtml:link rel="alternate" hreflang="x-default" href="{}"/>"#,
                escape(&url(Lang::DEFAULT, &path))
            ));
            xml.push_str("\n  </url>\n");
        }
    }
    xml.push_str("</urlset>\n");
    xml
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `GET /sitemap.xml`
pub async fn sitemap_xml() -> impl IntoResponse {
    (
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("application/xml; charset=utf-8"),
        )],
        render_sitemap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sitemap_lists_every_locale_of_every_page() {
        let xml = render_sitemap();
        let urls = xml.matches("<url>").count();
        assert_eq!(urls, all_paths().len() * Lang::ALL.len());
        assert!(xml.contains("https://dragonballfr.com/ja/manga/1/1"));
        assert!(xml.contains(r#"hreflang="x-default""#));
    }
}
