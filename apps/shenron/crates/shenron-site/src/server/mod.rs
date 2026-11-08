//! Axum-side handlers (ssr feature only): sitemap, robots, legacy redirects.

pub mod redirects;
pub mod robots;
pub mod sitemap;

pub use redirects::{legacy_redirect, root_redirect};
pub use robots::robots_txt;
pub use sitemap::sitemap_xml;
