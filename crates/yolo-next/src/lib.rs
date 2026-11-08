// SPDX-License-Identifier: Apache-2.0
//! `yolo-next`: Next.js 16 -> Yolo migration compiler.
//!
//! Reads a Next.js project (App Router and legacy Pages Router) and emits an equivalent
//! project for one of three lanes:
//!
//! | Lane | Runtime | Routing | Server functions |
//! |------|---------|---------|------------------|
//! | [`Target::Bun`] | Bun 1.4.4 `Bun.serve` + React 19 (`@yolo-rs/next`) | TanStack Router tree | Server Actions RPC (`Next-Action`) |
//! | [`Target::Dioxus`] | Dioxus 0.8 fullstack (axum) | `#[derive(Routable)]` | `#[server]` / `#[get]` |
//! | [`Target::Leptos`] | Leptos 0.8 (axum) | `leptos_router` `path!()` | `#[server]` |
//!
//! Module map (each module is independent and unit-tested; all share [`types`]):
//!
//! - [`config`]: static evaluation of `next.config.(js|mjs|ts)` with OXC -> [`NextConfig`] -> [`YoloWebConfig`].
//! - [`turbopack`]: `turbopack.{rules,resolveAlias,resolveExtensions,root}` -> [`BunBundlerPlan`] + `bunfig.toml`.
//! - [`rules`]: `redirects()` / `rewrites()` / `headers()` path-to-regexp sources -> matchers for Bun and axum.
//! - [`proxy`]: `proxy.ts` / `middleware.ts` analysis -> [`ProxySpec`] + Bun / tower skeletons.
//! - [`cdn`]: `assetPrefix`, `basePath`, `deploymentId`, `output` -> [`CdnPlan`] + upload manifest.
//! - [`routes`]: `app/` + `pages/` file-system scanner -> [`RouteManifest`] with Bun / TanStack / Dioxus / Leptos patterns.
//! - [`segment`]: route segment config exports, `generate*` detection, directives -> [`SegmentConfig`].
//! - [`metadata`]: `metadata` / `viewport` objects -> merged head tags (HTML, Dioxus `document::*`, Leptos `leptos_meta`).
//! - [`fonts`]: `next/font/google|local` calls -> CSS2 URLs, self-hosting plan, size-adjust fallbacks -> [`FontPlan`].
//! - [`images`]: `next/image` usages, `remotePatterns`, loaders, `srcset` algorithm -> [`ImageOptimizerPolicy`].
//! - [`rsc`]: `"use client"` / `"use server"` / `"use cache"` module graph -> Flight manifests -> [`ModuleGraph`].
//! - [`emit_bun`], [`emit_dioxus`], [`emit_leptos`]: project emitters -> [`EmittedFile`] lists.
//! - [`report`]: feature parity + diagnostics -> [`MigrationReport`] (JSON + Markdown).
//! - [`project`]: orchestrator (`analyze_project`, `migrate`).

#![forbid(unsafe_code)]
// Config structs mirror Next.js option bags, whose booleans are independent flags, not state machines.
#![allow(clippy::struct_excessive_bools)]
// Docs name products (TanStack, CloudFront, Bun) rather than Rust items.
#![allow(clippy::doc_markdown)]
// Parsers and emitters are single-pass walkers over Next.js option tables; length tracks the table.
#![allow(clippy::too_many_lines)]

pub mod types;

pub mod cdn;
pub mod config;
pub mod emit_bun;
pub mod emit_dioxus;
pub mod emit_leptos;
pub mod fonts;
pub mod images;
pub mod metadata;
pub mod project;
pub mod proxy;
pub mod report;
pub mod routes;
pub mod rsc;
pub mod rules;
pub mod segment;
pub mod turbopack;

pub use types::*;
pub use yolo_shared::{YoloThemePreset, css_preset_variables};

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Next.js version the compiler was verified against.
pub const NEXT_VERSION_BASELINE: &str = "16.4.5";

/// Bun version the Bun lane targets.
pub const BUN_VERSION_BASELINE: &str = "1.4.4";
