// SPDX-License-Identifier: Apache-2.0
//! Aphrody.com Flagship Ecosystem & Evolving Google Identity (2026)
//!
//! Serves `GET /api/identity` and `GET /api/apps` for the Aphrody (`aphrody.com`) flagship
//! product experience:
//! - Evolving Google Identity 2026 4-stop gradient (`#4285F4` -> `#9B72CB` -> `#D96570` -> `#F4B400`)
//! - Aphrody brand palette (`#FFD700` Gold, `#7C5CFF` Violet, `#00D4FF` Cyan, `#17335C` Deep Navy, `#8C6C88` Pet Mauve)
//! - Material 3 Expressive HCT/CAM16 tokens & Product Sans / Google Sans geometric typography
//! - 10 instant SERP vertical tabs (`all`, `ai`, `images`, `videos`, `news`, `maps`, `shopping`, `books`, `code`, `scholar`)
//! - 16 Google LLC -> Aphrody.com Web & Android 17 x64 Ecosystem applications

use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AppsQuery {
    pub category: Option<String>,
    pub q: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct EcosystemApp {
    pub id: &'static str,
    pub name: &'static str,
    #[serde(rename = "googleEquivalent")]
    pub google_equivalent: &'static str,
    pub url: &'static str,
    pub category: &'static str,
    pub icon: &'static str,
    #[serde(rename = "accentHex")]
    pub accent_hex: &'static str,
    #[serde(rename = "descriptionEn")]
    pub description_en: &'static str,
    #[serde(rename = "descriptionFr")]
    pub description_fr: &'static str,
    pub runtime: &'static str,
    #[serde(rename = "androidPackage")]
    pub android_package: &'static str,
}

pub const APPS: &[EcosystemApp] = &[
    EcosystemApp {
        id: "search",
        name: "Aphrody Search",
        google_equivalent: "Google Search",
        url: "https://aphrody.com/search",
        category: "search_ai",
        icon: "search",
        accent_hex: "#4285F4",
        description_en: "Universal web index, Knowledge Graph, Voice, Lens & AI Overview.",
        description_fr: "Index web universel, Knowledge Graph, Recherche vocale, Lens et Aperçu IA.",
        runtime: "yolo-web + aphrody-site",
        android_package: "com.aphrody.android.googlequicksearchbox",
    },
    EcosystemApp {
        id: "ai",
        name: "Aphrody AI / Gemini",
        google_equivalent: "Google Gemini & AI Studio",
        url: "https://ai.aphrody.com",
        category: "search_ai",
        icon: "auto_awesome",
        accent_hex: "#9B72CB",
        description_en: "Multimodal AI assistant, Deep Research, Canvas & local-first RAG.",
        description_fr: "Assistant IA multimodal, Deep Research, Canvas et RAG local-first.",
        runtime: "yolo-ai + aphrody-gemini-runtime",
        android_package: "com.aphrody.android.apps.bard",
    },
    EcosystemApp {
        id: "drive",
        name: "Aphrody Drive",
        google_equivalent: "Google Drive",
        url: "https://drive.aphrody.com",
        category: "workspace",
        icon: "cloud_done",
        accent_hex: "#34A853",
        description_en: "Cloud & local-first VFS storage powered by Spacedrive + Google Drive sync.",
        description_fr: "Stockage cloud et VFS local-first propulsé par Spacedrive + synchro Google Drive.",
        runtime: "aphrody-drive + spacedrive",
        android_package: "com.aphrody.android.apps.docs",
    },
    EcosystemApp {
        id: "docs",
        name: "Docs / WordCraft",
        google_equivalent: "Google Docs",
        url: "https://docs.aphrody.com",
        category: "workspace",
        icon: "description",
        accent_hex: "#4285F4",
        description_en: "Collaborative rich-text & Markdown studio with native Google Docs sync.",
        description_fr: "Studio de traitement de texte et Markdown collaboratif avec synchro Google Docs.",
        runtime: "wordcraft-engine + @yolo-web/google",
        android_package: "com.aphrody.android.apps.docs.editors.docs",
    },
    EcosystemApp {
        id: "sheets",
        name: "Sheets / GridCraft",
        google_equivalent: "Google Sheets",
        url: "https://sheets.aphrody.com",
        category: "workspace",
        icon: "table_chart",
        accent_hex: "#34A853",
        description_en: "High-speed GPU spreadsheet engine, formulas, and data analytics.",
        description_fr: "Tableur haute performance accéléré par GPU, formules et analytique.",
        runtime: "gridcraft-engine + yolo-ui",
        android_package: "com.aphrody.android.apps.docs.editors.sheets",
    },
    EcosystemApp {
        id: "slides",
        name: "Slides / DeckCraft",
        google_equivalent: "Google Slides",
        url: "https://slides.aphrody.com",
        category: "workspace",
        icon: "slideshow",
        accent_hex: "#F4B400",
        description_en: "Expressive presentation studio with PPTX & Google Slides parity.",
        description_fr: "Studio de présentation expressif compatible PPTX et Google Slides.",
        runtime: "deckcraft + @yolo-web/google",
        android_package: "com.aphrody.android.apps.docs.editors.slides",
    },
    EcosystemApp {
        id: "mail",
        name: "Aphrody Mail",
        google_equivalent: "Gmail",
        url: "https://mail.aphrody.com",
        category: "communication",
        icon: "mail",
        accent_hex: "#EA4335",
        description_en: "Zero-trust encrypted email & Gmail API workspace client with AI triage.",
        description_fr: "Client mail chiffré zéro-trust et espace Gmail avec tri IA.",
        runtime: "aphrody-backend + @yolo-web/google",
        android_package: "com.aphrody.android.gm",
    },
    EcosystemApp {
        id: "meet",
        name: "Aphrody Meet",
        google_equivalent: "Google Meet",
        url: "https://meet.aphrody.com",
        category: "communication",
        icon: "videocam",
        accent_hex: "#00D4FF",
        description_en: "Low-latency WebRTC video conferencing with live AI captions & translation.",
        description_fr: "Visioconférence WebRTC basse latence avec sous-titres et traduction IA en direct.",
        runtime: "yolo-web + aphrody-voice",
        android_package: "com.aphrody.android.apps.tachyon",
    },
    EcosystemApp {
        id: "calendar",
        name: "Aphrody Calendar",
        google_equivalent: "Google Calendar",
        url: "https://calendar.aphrody.com",
        category: "workspace",
        icon: "calendar_month",
        accent_hex: "#4285F4",
        description_en: "Intelligent time orchestration, autonomous scheduling & Temporal API.",
        description_fr: "Orchestration intelligente du temps, planification autonome et API Temporal.",
        runtime: "aphrody-jobs + yolo-ui",
        android_package: "com.aphrody.android.calendar",
    },
    EcosystemApp {
        id: "photos",
        name: "Aphrody Photos",
        google_equivalent: "Google Photos",
        url: "https://photos.aphrody.com",
        category: "media_geo",
        icon: "photo_library",
        accent_hex: "#D96570",
        description_en: "PhotoCraft & LightCraft RAW/WebGPU gallery with semantic vision search.",
        description_fr: "Galerie RAW/WebGPU PhotoCraft & LightCraft avec recherche visuelle sémantique.",
        runtime: "photocraft-engine + lightcraft-engine",
        android_package: "com.aphrody.android.apps.photos",
    },
    EcosystemApp {
        id: "translate",
        name: "Aphrody Translate",
        google_equivalent: "Google Translate",
        url: "https://translate.aphrody.com",
        category: "search_ai",
        icon: "translate",
        accent_hex: "#4285F4",
        description_en: "Neural multilingual translation across 130+ languages (FR/EN/JA native).",
        description_fr: "Traduction neuronale multilingue dans plus de 130 langues (FR/EN/JA natif).",
        runtime: "aphrody-translate + aphrody-translate-core",
        android_package: "com.aphrody.android.apps.translate",
    },
    EcosystemApp {
        id: "maps",
        name: "Aphrody Maps",
        google_equivalent: "Google Maps",
        url: "https://maps.aphrody.com",
        category: "media_geo",
        icon: "explore",
        accent_hex: "#34A853",
        description_en: "3D WebGPU vector cartography, geospatial intelligence & routing.",
        description_fr: "Cartographie vectorielle 3D WebGPU, intelligence géospatiale et itinéraires.",
        runtime: "yolo-web + vectorcraft-render",
        android_package: "com.aphrody.android.apps.maps",
    },
    EcosystemApp {
        id: "video",
        name: "Aphrody Video / Media",
        google_equivalent: "YouTube",
        url: "https://video.aphrody.com",
        category: "media_geo",
        icon: "play_circle",
        accent_hex: "#EA4335",
        description_en: "FilmCraft & EffectCraft streaming studio, 4K WebGPU playback & creator hub.",
        description_fr: "Studio de streaming FilmCraft & EffectCraft, lecture 4K WebGPU et hub créateur.",
        runtime: "filmcraft-engine + effectcraft-gpu",
        android_package: "com.aphrody.android.youtube",
    },
    EcosystemApp {
        id: "play",
        name: "Aphrody Play Store",
        google_equivalent: "Google Play Store",
        url: "https://play.aphrody.com",
        category: "platform_os",
        icon: "shop",
        accent_hex: "#00D4FF",
        description_en: "Android 17 x64 Google Play apps, F-Droid, WebGPU/WASM & signed YOLO plugins.",
        description_fr: "Applications Android 17 x64 Google Play, F-Droid, WebGPU/WASM et plugins YOLO signés.",
        runtime: "yolo-os + yolo-plugins + bunos-flutter-store",
        android_package: "com.android.vending",
    },
    EcosystemApp {
        id: "console",
        name: "Aphrody Cloud Console",
        google_equivalent: "Google Cloud Console",
        url: "https://console.aphrody.com",
        category: "platform_os",
        icon: "terminal",
        accent_hex: "#FFD700",
        description_en: "Fleet orchestration, OVH/Cloudflare edge workers, AI models & telemetry.",
        description_fr: "Orchestration de flotte, workers edge OVH/Cloudflare, modèles IA et télémétrie.",
        runtime: "aphrody-infra + aphrody-ovh + yolo-vps",
        android_package: "com.aphrody.android.apps.cloudconsole",
    },
    EcosystemApp {
        id: "os",
        name: "Chromium OS / GoogleBook Shell",
        google_equivalent: "ChromeOS / Aluminium OS",
        url: "https://os.aphrody.com",
        category: "platform_os",
        icon: "laptop_chromebook",
        accent_hex: "#7C5CFF",
        description_en: "2026 GoogleBook Laptop OS: Aluminium OS + Chromium OS + Android 17 x64 + Bun v1.4.4+.",
        description_fr: "OS GoogleBook 2026 : Aluminium OS + Chromium OS + Android 17 x64 + Bun v1.4.4+.",
        runtime: "yolo-os + aphrody-webos-server",
        android_package: "com.aphrody.os.shell",
    },
];

pub fn identity_payload() -> Value {
    json!({
        "brand": "Aphrody",
        "domain": "aphrody.com",
        "org": "aphrody-labs",
        "coreLibrary": "yolo",
        "inspirationUrl": "https://design.google/library/evolving-google-identity",
        "evolvingGradient": {
            "stops": [
                { "stop": "0%", "hex": "#4285F4", "name": "Google Blue" },
                { "stop": "33%", "hex": "#9B72CB", "name": "Evolving Violet" },
                { "stop": "66%", "hex": "#D96570", "name": "Evolving Coral" },
                { "stop": "100%", "hex": "#F4B400", "name": "Google Gold" },
            ],
            "css": "linear-gradient(90deg, #4285F4 0%, #9B72CB 33%, #D96570 66%, #F4B400 100%)",
            "aphrodyFusionCss": "linear-gradient(105deg, #4285F4 0%, #7C5CFF 25%, #9B72CB 45%, #D96570 68%, #F4B400 85%, #FFD700 100%)",
        },
        "aphrodyBrandPalette": {
            "gold": "#FFD700",
            "violet": "#7C5CFF",
            "cyan": "#00D4FF",
            "deepNavy": "#17335C",
            "petMauveSeed": "#8C6C88",
            "blondSeed": "#EADBA6",
        },
        "m3Expressive": {
            "colorSpace": "HCT/CAM16",
            "preset": "aphrody",
            "searchCapsuleRadius": "28px",
        },
        "verticals": [
            { "id": "all", "labelEn": "All", "labelFr": "Tous", "icon": "search" },
            { "id": "ai", "labelEn": "AI Mode", "labelFr": "Mode IA", "icon": "auto_awesome" },
            { "id": "images", "labelEn": "Images", "labelFr": "Images", "icon": "image" },
            { "id": "videos", "labelEn": "Videos", "labelFr": "Vidéos", "icon": "smart_display" },
            { "id": "news", "labelEn": "News", "labelFr": "Actualités", "icon": "newspaper" },
            { "id": "maps", "labelEn": "Maps", "labelFr": "Maps", "icon": "location_on" },
            { "id": "shopping", "labelEn": "Shopping", "labelFr": "Shopping", "icon": "shopping_bag" },
            { "id": "books", "labelEn": "Books", "labelFr": "Livres", "icon": "menu_book" },
            { "id": "code", "labelEn": "Code", "labelFr": "Code", "icon": "terminal" },
            { "id": "scholar", "labelEn": "Scholar", "labelFr": "Scholar", "icon": "school" },
        ]
    })
}

pub async fn identity() -> Json<Value> {
    Json(identity_payload())
}

pub async fn apps(Query(query): Query<AppsQuery>) -> Json<Value> {
    let category = query
        .category
        .as_deref()
        .filter(|c| !c.is_empty() && *c != "all");
    let q = query.q.as_deref().unwrap_or("").trim().to_lowercase();

    let filtered: Vec<&EcosystemApp> = APPS
        .iter()
        .filter(|app| {
            if let Some(cat) = category
                && app.category != cat
            {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            app.id.to_lowercase().contains(&q)
                || app.name.to_lowercase().contains(&q)
                || app.google_equivalent.to_lowercase().contains(&q)
                || app.description_en.to_lowercase().contains(&q)
                || app.description_fr.to_lowercase().contains(&q)
        })
        .collect();

    Json(json!({
        "brand": "Aphrody",
        "domain": "aphrody.com",
        "total": filtered.len(),
        "apps": filtered,
    }))
}
