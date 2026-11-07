// SPDX-License-Identifier: Apache-2.0
//! Content model of the templates and the copy of the six Aphrody sites.
//!
//! Every string here is original Aphrody copy describing what the products do today. Product
//! facts come from the product repositories (Shenron, Rose Griffon, Auto, Craft workbenches,
//! YOLO-Web, yolo-ui); no reference site's wording is reused.

use crate::icons::IconName;

/// A link with its visible label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Visible text, also the accessible name.
    pub label: &'static str,
    /// Site-relative or absolute URL.
    pub href: &'static str,
}

/// A titled group of links inside a flyout or mega menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkGroup {
    /// Group heading.
    pub heading: &'static str,
    /// Links of the group.
    pub links: &'static [Link],
}

/// One entry of the global navigation. No groups: a plain link. One group: a flyout. Two or
/// more groups: a mega menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavItem {
    /// Visible label.
    pub label: &'static str,
    /// Destination when the item is a plain link, and the overview link inside its panel.
    pub href: &'static str,
    /// One sentence shown at the top of the panel.
    pub summary: &'static str,
    /// Link groups of the panel.
    pub groups: &'static [LinkGroup],
}

/// The short announcement shown as a pill above heroes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Release {
    /// Tag inside the pill.
    pub tag: &'static str,
    /// Announcement text.
    pub text: &'static str,
    /// Where it leads.
    pub href: &'static str,
}

/// Hero copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hero {
    /// Small line above the title.
    pub eyebrow: &'static str,
    /// The headline.
    pub title: &'static str,
    /// One or two sentences under the headline.
    pub lede: &'static str,
    /// The one primary action.
    pub primary: Link,
    /// An optional secondary action.
    pub secondary: Option<Link>,
}

/// A section of the product page, reachable from the sticky section tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// Fragment id, unique within a page.
    pub id: &'static str,
    /// Label in the section tabs.
    pub tab: &'static str,
    /// Section heading.
    pub heading: &'static str,
    /// Paragraph.
    pub body: &'static str,
    /// Supporting points.
    pub points: &'static [&'static str],
}

/// A question and its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Faq {
    /// The question.
    pub question: &'static str,
    /// The answer.
    pub answer: &'static str,
}

/// The simplified product UI drawn next to the product hero (no screenshot).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sketch {
    /// Accessible description of the drawing.
    pub alt: &'static str,
    /// Title shown in the drawn window.
    pub window_title: &'static str,
    /// Sidebar entries drawn in the window.
    pub sidebar: &'static [&'static str],
    /// Rows drawn in the main pane.
    pub rows: &'static [&'static str],
    /// Caption of the floating card.
    pub card: &'static str,
}

/// Product page content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductPage {
    /// Product name.
    pub name: &'static str,
    /// Hero of the product page.
    pub hero: Hero,
    /// The drawn UI.
    pub sketch: Sketch,
    /// Label of the section tabs landmark.
    pub tabs_label: &'static str,
    /// Sections in page order.
    pub sections: &'static [Section],
    /// Frequently asked questions.
    pub faqs: &'static [Faq],
}

/// A release channel offered for download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Channel {
    /// Channel name.
    pub name: &'static str,
    /// How often it updates.
    pub cadence: &'static str,
    /// Who it is for.
    pub description: &'static str,
    /// Platforms the channel ships on.
    pub platforms: &'static [&'static str],
    /// Download action.
    pub action: Link,
    /// Highlight as the recommended channel.
    pub recommended: bool,
}

/// The downloads block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Downloads {
    /// Section heading.
    pub heading: &'static str,
    /// Intro paragraph.
    pub intro: &'static str,
    /// Channels, most stable first.
    pub channels: &'static [Channel],
}

/// An item of the app rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RailItem {
    /// Label, shown under the icon when collapsed and beside it when expanded.
    pub label: &'static str,
    /// Destination.
    pub href: &'static str,
    /// Icon.
    pub icon: IconName,
}

/// A tile of the dense media grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaItem {
    /// Title of the item.
    pub title: &'static str,
    /// Where it comes from (collection, team, workbench).
    pub source: &'static str,
    /// Metadata line.
    pub meta: &'static str,
    /// Badge on the artwork (length, count or kind).
    pub badge: &'static str,
    /// Destination.
    pub href: &'static str,
    /// Hue (degrees) of the generated artwork.
    pub hue: u16,
}

/// The app chrome content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppChrome {
    /// Name shown in the app bar.
    pub app_name: &'static str,
    /// Rail entries.
    pub rail: &'static [RailItem],
    /// Search field label.
    pub search_label: &'static str,
    /// Heading of the media grid.
    pub grid_heading: &'static str,
    /// Grid items.
    pub media: &'static [MediaItem],
}

/// A story of the editorial grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Story {
    /// Category line.
    pub kicker: &'static str,
    /// Headline.
    pub title: &'static str,
    /// Standfirst.
    pub dek: &'static str,
    /// Destination.
    pub href: &'static str,
}

/// The editorial block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Editorial {
    /// Small line above the oversized headline.
    pub kicker: &'static str,
    /// Oversized headline.
    pub headline: &'static str,
    /// Stories; the first one leads with the illustration panel.
    pub stories: &'static [Story],
}

/// Copy of the scroll-driven reveal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reveal {
    /// Left half of the converging title.
    pub left: &'static str,
    /// Right half of the converging title.
    pub right: &'static str,
    /// Paragraph shown once the halves meet.
    pub body: &'static str,
}

/// Interface strings in one language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// BCP 47 tag.
    pub lang: &'static str,
    /// Skip link.
    pub skip: &'static str,
    /// Primary navigation landmark.
    pub primary_nav: &'static str,
    /// Overview link at the top of a panel.
    pub overview: &'static str,
    /// Starfield pause switch.
    pub pause_motion: &'static str,
    /// Rail expand switch.
    pub expand_rail: &'static str,
    /// App navigation landmark.
    pub app_nav: &'static str,
    /// Search button.
    pub search: &'static str,
    /// Voice search button.
    pub voice_search: &'static str,
    /// Theme switch.
    pub dark_theme: &'static str,
    /// Platform picker legend.
    pub platform: &'static str,
    /// Docs navigation landmark.
    pub docs_nav: &'static str,
    /// FAQ heading.
    pub faq: &'static str,
    /// "Available on" before platform lists.
    pub available_on: &'static str,
    /// Recommended channel badge.
    pub recommended: &'static str,
    /// Column headers of the token table: role, light, dark, light ratio, dark ratio.
    pub token_columns: [&'static str; 5],
    /// Token table caption.
    pub token_caption: &'static str,
    /// Pass / not applicable markers of the token table.
    pub passes: &'static str,
    /// Marker for roles without a contrast requirement.
    pub decorative: &'static str,
}

/// English interface strings.
pub const EN: Strings = Strings {
    lang: "en",
    skip: "Skip to content",
    primary_nav: "Main",
    overview: "Overview",
    pause_motion: "Pause motion",
    expand_rail: "Show labels",
    app_nav: "Sections",
    search: "Search",
    voice_search: "Search by voice",
    dark_theme: "Dark theme",
    platform: "Platform",
    docs_nav: "Documentation",
    faq: "Questions and answers",
    available_on: "Available on",
    recommended: "Recommended",
    token_columns: ["Role", "Light", "Dark", "Light ratio", "Dark ratio"],
    token_caption: "Colour roles of the current preset, with their WCAG contrast against the surface they sit on.",
    passes: "AA",
    decorative: "surface",
};

/// French interface strings.
pub const FR: Strings = Strings {
    lang: "fr",
    skip: "Aller au contenu",
    primary_nav: "Principale",
    overview: "Vue d'ensemble",
    pause_motion: "Mettre en pause",
    expand_rail: "Afficher les libellés",
    app_nav: "Rubriques",
    search: "Rechercher",
    voice_search: "Recherche vocale",
    dark_theme: "Thème sombre",
    platform: "Plateforme",
    docs_nav: "Documentation",
    faq: "Questions fréquentes",
    available_on: "Disponible sur",
    recommended: "Recommandé",
    token_columns: ["Rôle", "Clair", "Sombre", "Ratio clair", "Ratio sombre"],
    token_caption: "Rôles de couleur du preset courant et leur contraste WCAG sur la surface qui les porte.",
    passes: "AA",
    decorative: "surface",
};

/// A documentation tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocNode {
    /// Label.
    pub label: &'static str,
    /// Destination.
    pub href: &'static str,
    /// Children; non-empty nodes render as collapsible groups.
    pub children: &'static [DocNode],
}

/// A platform of the docs platform switcher with its install snippet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Platform {
    /// Stable id, used in element ids.
    pub id: &'static str,
    /// Visible label.
    pub label: &'static str,
    /// Code shown when the platform is selected.
    pub snippet: &'static str,
}

/// The design-system docs content (shared by every site; it documents yolo-ui itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Docs {
    /// Product name in the docs header.
    pub product: &'static str,
    /// Navigation tree.
    pub tree: &'static [DocNode],
    /// Href of the current page, marked `aria-current`.
    pub current: &'static str,
    /// Page title.
    pub title: &'static str,
    /// Intro paragraph.
    pub intro: &'static str,
    /// Platforms of the switcher.
    pub platforms: &'static [Platform],
}

/// Everything one site needs to fill every template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    /// Stable id (`aphrody`, `yolo`, `shenron`, `rosegriffon`, `auto`, `craft`).
    pub id: &'static str,
    /// Wordmark text.
    pub name: &'static str,
    /// Public domain.
    pub domain: &'static str,
    /// Interface strings.
    pub strings: &'static Strings,
    /// Global navigation.
    pub nav: &'static [NavItem],
    /// Release pill.
    pub release: Release,
    /// Stage hero.
    pub hero: Hero,
    /// Product page.
    pub product: ProductPage,
    /// Downloads.
    pub downloads: Downloads,
    /// App chrome and media grid.
    pub app: AppChrome,
    /// Editorial grid.
    pub editorial: Editorial,
    /// Scroll reveal.
    pub reveal: Reveal,
    /// Footer line.
    pub footer: &'static str,
}

const fn l(label: &'static str, href: &'static str) -> Link {
    Link { label, href }
}

/// The yolo-ui design-system documentation shown in the docs shell.
pub const DOCS: Docs = Docs {
    product: "yolo-ui",
    tree: &[
        DocNode {
            label: "Get started",
            href: "/docs/start",
            children: &[
                DocNode { label: "Install", href: "/docs/start/install", children: &[] },
                DocNode { label: "Pick a preset", href: "/docs/start/presets", children: &[] },
            ],
        },
        DocNode {
            label: "Foundations",
            href: "/docs/foundations",
            children: &[
                DocNode { label: "Colour tokens", href: "/docs/foundations/color", children: &[] },
                DocNode { label: "Type", href: "/docs/foundations/type", children: &[] },
                DocNode { label: "Shape and spacing", href: "/docs/foundations/shape", children: &[] },
                DocNode { label: "Motion", href: "/docs/foundations/motion", children: &[] },
            ],
        },
        DocNode {
            label: "Templates",
            href: "/docs/templates",
            children: &[
                DocNode { label: "Stage hero", href: "/docs/templates/stage-hero", children: &[] },
                DocNode { label: "Global nav", href: "/docs/templates/global-nav", children: &[] },
                DocNode { label: "Product page", href: "/docs/templates/product-page", children: &[] },
                DocNode { label: "App chrome", href: "/docs/templates/app-chrome", children: &[] },
            ],
        },
        DocNode { label: "Accessibility", href: "/docs/accessibility", children: &[] },
    ],
    current: "/docs/foundations/color",
    title: "Colour tokens",
    intro: "Every template paints with eighteen roles. Each preset fills them from its own palette, then the solver nudges any text or control colour that falls short of WCAG 2.2 AA until it passes on the surfaces it is used on.",
    platforms: &[
        Platform {
            id: "dioxus",
            label: "Dioxus",
            snippet: "use yolo_templates::{render_document, Mode, Templates, SITES};\nuse yolo_shared::YoloThemePreset;\n\nlet html = render_document(&SITES[0], YoloThemePreset::Aphrody, Mode::Dark, Templates::ALL);",
        },
        Platform {
            id: "web",
            label: "Web",
            snippet: "<div class=\"yt-root\" data-yt-preset=\"yolo-aphrody\" data-yt-mode=\"dark\">\n  <!-- server-rendered templates -->\n</div>",
        },
        Platform {
            id: "gpui",
            label: "GPUI",
            snippet: "let palette = yolo_templates::Palette::resolve(YoloThemePreset::Fluent, Mode::Light);\nlet text = palette.text; // RgbaColor, AA on canvas, surface and raised",
        },
        Platform {
            id: "iced",
            label: "Iced",
            snippet: "let palette = yolo_templates::Palette::resolve(YoloThemePreset::Omarchy, Mode::Dark);\nlet focus = palette.focus; // 3:1 against every surface",
        },
    ],
};

/// The six Aphrody sites.
pub static SITES: [Site; 6] = [APHRODY, YOLO, SHENRON, ROSEGRIFFON, AUTO, CRAFT];

/// Looks a site up by id.
#[must_use]
pub fn site(id: &str) -> Option<&'static Site> {
    SITES.iter().find(|s| s.id == id)
}

const APHRODY: Site = Site {
    id: "aphrody",
    name: "Aphrody",
    domain: "aphrody.com",
    strings: &EN,
    nav: &[
        NavItem {
            label: "Products",
            href: "/products",
            summary: "Six products, one account, one design system.",
            groups: &[
                LinkGroup {
                    heading: "Build",
                    links: &[l("YOLO browser", "/yolo"), l("yolo-ui", "/yolo/ui"), l("Craft workbenches", "/craft")],
                },
                LinkGroup {
                    heading: "Run",
                    links: &[l("Auto", "/auto"), l("Rose Griffon", "/rosegriffon")],
                },
                LinkGroup {
                    heading: "Read",
                    links: &[l("Shenron", "/shenron"), l("Engineering notes", "/notes")],
                },
            ],
        },
        NavItem {
            label: "Developers",
            href: "/developers",
            summary: "Open source crates and Bun packages behind every product.",
            groups: &[LinkGroup {
                heading: "Start here",
                links: &[
                    l("Source on GitHub", "https://github.com/yolo-rs"),
                    l("Design system docs", "/docs"),
                    l("MCP servers", "/developers/mcp"),
                ],
            }],
        },
        NavItem { label: "Notes", href: "/notes", summary: "", groups: &[] },
        NavItem { label: "Account", href: "/account", summary: "", groups: &[] },
    ],
    release: Release {
        tag: "New",
        text: "One design system now dresses every Aphrody site",
        href: "/notes/one-design-system",
    },
    hero: Hero {
        eyebrow: "Aphrody",
        title: "Tools we use every day, opened up for you.",
        lede: "A browser, a design system, creative workbenches, an automation platform and two community sites, built in Rust and Bun and shipped from the same repositories.",
        primary: l("See the products", "/products"),
        secondary: Some(l("Read the source", "https://github.com/yolo-rs")),
    },
    product: ProductPage {
        name: "Aphrody account",
        hero: Hero {
            eyebrow: "One account",
            title: "Sign in once, carry your preferences everywhere.",
            lede: "Your theme, language and pet companion follow you from the browser to Craft to the Shenron reader.",
            primary: l("Create an account", "/account/new"),
            secondary: Some(l("How sign-in works", "/account/security")),
        },
        sketch: Sketch {
            alt: "Drawing of the account page: a sidebar with Profile, Sign-in and Appearance, and a list of connected products.",
            window_title: "Account",
            sidebar: &["Profile", "Sign-in", "Appearance", "Products"],
            rows: &["YOLO browser", "Craft", "Auto", "Shenron"],
            card: "Theme: Aphrody, dark",
        },
        tabs_label: "Account page sections",
        sections: &[
            Section {
                id: "account-signin",
                tab: "Sign-in",
                heading: "Sign in with what you already use",
                body: "Google, Microsoft, GitHub, Discord, Steam or Epic Games. Password managers and passkeys work everywhere a field asks for a secret.",
                points: &["No puzzle or memory test to prove you are human", "Paste works in every field"],
            },
            Section {
                id: "account-appearance",
                tab: "Appearance",
                heading: "Pick a preset once",
                body: "Eight presets, light or dark. The choice is stored with your account and applied by every product that renders yolo-ui.",
                points: &["Neutral, Material, Fluent, Apple, Omarchy, Hyprland, Aphrody, Chromium", "Contrast is checked for every preset"],
            },
            Section {
                id: "account-data",
                tab: "Your data",
                heading: "Export or delete in one place",
                body: "Every product reports what it stores about you on one page. Export downloads it as JSON; delete removes it from every product.",
                points: &["Deletion reaches every product", "Exports are plain JSON"],
            },
        ],
        faqs: &[
            Faq {
                question: "Do I need an account to use the browser?",
                answer: "No. The YOLO browser and Craft run without an account; signing in only syncs preferences.",
            },
            Faq {
                question: "Which data leaves my machine?",
                answer: "Only what you sync: preset, language and companion settings. Browsing history stays local.",
            },
        ],
    },
    downloads: Downloads {
        heading: "Get the desktop apps",
        intro: "The YOLO browser and Craft install side by side. Pick the channel that matches how much change you want.",
        channels: &[
            Channel {
                name: "Stable",
                cadence: "Every four weeks",
                description: "For daily work. Updates arrive after a full cycle on Preview.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Stable", "/download/stable"),
                recommended: true,
            },
            Channel {
                name: "Preview",
                cadence: "Every week",
                description: "The next release, a cycle early. Good for testing your own sites.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Preview", "/download/preview"),
                recommended: false,
            },
        ],
    },
    app: AppChrome {
        app_name: "Aphrody",
        rail: &[
            RailItem { label: "Home", href: "/", icon: IconName::Home },
            RailItem { label: "Products", href: "/products", icon: IconName::Grid },
            RailItem { label: "Notes", href: "/notes", icon: IconName::Library },
            RailItem { label: "Recent", href: "/recent", icon: IconName::Clock },
        ],
        search_label: "Search Aphrody",
        grid_heading: "Product tours",
        media: &[
            MediaItem { title: "Install the YOLO browser on Windows 11", source: "YOLO", meta: "Tour, setup", badge: "Tour", href: "/tours/yolo-install", hue: 220 },
            MediaItem { title: "Theme a site with one preset switch", source: "yolo-ui", meta: "Tour, design system", badge: "Tour", href: "/tours/presets", hue: 265 },
            MediaItem { title: "Draft a chapter in WordCraft", source: "Craft", meta: "Tour, writing", badge: "Tour", href: "/tours/wordcraft", hue: 32 },
            MediaItem { title: "Send invoices on a schedule with Auto", source: "Auto", meta: "Tour, automation", badge: "Tour", href: "/tours/auto-invoices", hue: 150 },
            MediaItem { title: "Search the Dragon Ball databooks", source: "Shenron", meta: "Tour, reading", badge: "Tour", href: "/tours/shenron-search", hue: 18 },
            MediaItem { title: "Run a ranked season on Achillea", source: "Rose Griffon", meta: "Tour, esport", badge: "Tour", href: "/tours/achillea", hue: 340 },
        ],
    },
    editorial: Editorial {
        kicker: "Engineering notes",
        headline: "How we build in the open",
        stories: &[
            Story {
                kicker: "Design",
                title: "Eight presets, one contrast rule",
                dek: "Why every template solves its own colours against WCAG instead of trusting the palette.",
                href: "/notes/contrast-solver",
            },
            Story {
                kicker: "Browser",
                title: "Shipping Chromium through CEF",
                dek: "What the YOLO browser keeps from Chromium and what it changes.",
                href: "/notes/cef",
            },
            Story {
                kicker: "Tooling",
                title: "Bun and Rust, nothing else",
                dek: "One runtime for scripts, one language for engines.",
                href: "/notes/bun-and-rust",
            },
            Story {
                kicker: "Motion",
                title: "Motion that stops when you ask",
                dek: "Reduced motion and a pause control on every animated surface.",
                href: "/notes/motion",
            },
        ],
    },
    reveal: Reveal {
        left: "Built",
        right: "together",
        body: "Six products share one account, one design system and one set of open repositories.",
    },
    footer: "Aphrody builds open tools in Rust and Bun.",
};

const YOLO: Site = Site {
    id: "yolo",
    name: "YOLO",
    domain: "yolo.aphrody.com",
    strings: &EN,
    nav: &[
        NavItem {
            label: "Browser",
            href: "/browser",
            summary: "A Chromium desktop browser with a Rust shell.",
            groups: &[LinkGroup {
                heading: "Browser",
                links: &[l("Features", "/browser/features"), l("Download", "/download"), l("Release notes", "/browser/releases")],
            }],
        },
        NavItem {
            label: "Platform",
            href: "/platform",
            summary: "Everything under the browser, open source.",
            groups: &[
                LinkGroup {
                    heading: "Engines",
                    links: &[l("Chromium shell", "/platform/chromium"), l("Servo crates", "/platform/servo"), l("V8 runtime", "/platform/v8")],
                },
                LinkGroup {
                    heading: "Interface",
                    links: &[l("yolo-ui", "/platform/ui"), l("Templates", "/platform/templates"), l("Presets", "/platform/presets")],
                },
                LinkGroup {
                    heading: "Tooling",
                    links: &[l("Bun packages", "/platform/bun"), l("MCP servers", "/platform/mcp")],
                },
            ],
        },
        NavItem { label: "Docs", href: "/docs", summary: "", groups: &[] },
        NavItem { label: "Download", href: "/download", summary: "", groups: &[] },
    ],
    release: Release {
        tag: "Preview",
        text: "Browser shell now built on Chromium 156",
        href: "/browser/releases",
    },
    hero: Hero {
        eyebrow: "YOLO browser",
        title: "Chromium, rebuilt around a Rust shell.",
        lede: "Real Chromium rendering with a shell, theme engine and tooling written in Rust and Bun. Eight presets, light and dark, on Windows 11 and Linux.",
        primary: l("Download", "/download"),
        secondary: Some(l("See what changed", "/browser/releases")),
    },
    product: ProductPage {
        name: "YOLO browser",
        hero: Hero {
            eyebrow: "YOLO browser",
            title: "The browser that matches your desktop.",
            lede: "Pick a preset and the toolbar, new tab page and menus follow it. Websites keep rendering exactly as Chromium renders them.",
            primary: l("Download", "/download"),
            secondary: Some(l("Compare channels", "/download#channels")),
        },
        sketch: Sketch {
            alt: "Drawing of the browser window: a tab strip, a toolbar with a centered address field and a new tab page with shortcuts.",
            window_title: "New tab",
            sidebar: &["Tabs", "Bookmarks", "History", "Downloads"],
            rows: &["aphrody.com", "dragonballfr.com", "rosegriffon.fr", "auto.aphrody.com"],
            card: "Preset: Hyprland",
        },
        tabs_label: "Browser page sections",
        sections: &[
            Section {
                id: "browser-themes",
                tab: "Themes",
                heading: "Your preset, all the way through",
                body: "The shell reads the same tokens as every Aphrody site, so the frame, menus and new tab page match the preset you chose.",
                points: &["Eight presets, light and dark", "Omarchy subthemes follow your desktop"],
            },
            Section {
                id: "browser-engine",
                tab: "Engine",
                heading: "Chromium where it counts",
                body: "Pages render with Chromium through CEF. The shell around it, the policy layer and the theme engine are Rust.",
                points: &["Standard extensions load", "Managed policies for search and privacy"],
            },
            Section {
                id: "browser-privacy",
                tab: "Privacy",
                heading: "Search that does not follow you",
                body: "The default search goes through Aphrody with no tracking parameters, and you can switch engines in one setting.",
                points: &["No account required", "History stays on your machine"],
            },
        ],
        faqs: &[
            Faq {
                question: "Will my extensions work?",
                answer: "Manifest V3 extensions load as they do in Chromium.",
            },
            Faq {
                question: "Which systems are supported?",
                answer: "Windows 11 and Linux x64 are tested on every build.",
            },
        ],
    },
    downloads: Downloads {
        heading: "Choose a channel",
        intro: "All channels install side by side, each with its own profile.",
        channels: &[
            Channel {
                name: "Stable",
                cadence: "Every four weeks",
                description: "For everyday browsing. Each build has spent a full cycle on Preview.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Stable", "/download/stable"),
                recommended: true,
            },
            Channel {
                name: "Preview",
                cadence: "Every week",
                description: "Next month's browser today, for checking your sites early.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Preview", "/download/preview"),
                recommended: false,
            },
            Channel {
                name: "Nightly",
                cadence: "Every night",
                description: "Built from main. Expect rough edges and report them.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Nightly", "/download/nightly"),
                recommended: false,
            },
        ],
    },
    app: AppChrome {
        app_name: "YOLO",
        rail: &[
            RailItem { label: "Home", href: "/", icon: IconName::Home },
            RailItem { label: "Explore", href: "/explore", icon: IconName::Compass },
            RailItem { label: "Library", href: "/library", icon: IconName::Library },
            RailItem { label: "History", href: "/history", icon: IconName::Clock },
        ],
        search_label: "Search YOLO",
        grid_heading: "Platform walkthroughs",
        media: &[
            MediaItem { title: "Build the Chromium shell from a clean checkout", source: "Chromium shell", meta: "Walkthrough, build", badge: "Build", href: "/watch/build-shell", hue: 210 },
            MediaItem { title: "Switch presets without restarting", source: "yolo-ui", meta: "Walkthrough, themes", badge: "Themes", href: "/watch/presets", hue: 280 },
            MediaItem { title: "Load a Manifest V3 extension", source: "Browser", meta: "Walkthrough, extensions", badge: "Extensions", href: "/watch/extensions", hue: 190 },
            MediaItem { title: "Drive the browser from an MCP client", source: "MCP servers", meta: "Walkthrough, automation", badge: "MCP", href: "/watch/mcp", hue: 160 },
            MediaItem { title: "Lay out a page with Taffy from CSS", source: "yolo-layout", meta: "Walkthrough, layout", badge: "Layout", href: "/watch/taffy", hue: 30 },
            MediaItem { title: "Run Bun scripts with no Node.js", source: "Bun packages", meta: "Walkthrough, tooling", badge: "Tooling", href: "/watch/bun", hue: 50 },
        ],
    },
    editorial: Editorial {
        kicker: "From the build log",
        headline: "What ships next",
        stories: &[
            Story { kicker: "Shell", title: "A toolbar drawn from tokens", dek: "The frame colours come from the active preset, not from a bitmap theme.", href: "/notes/toolbar-tokens" },
            Story { kicker: "Policy", title: "Managed search, one switch", dek: "How the browser picks its default engine and lets you change it.", href: "/notes/search-policy" },
            Story { kicker: "Layout", title: "CSS in, Taffy out", dek: "One layout engine for the web and for native windows.", href: "/notes/taffy" },
            Story { kicker: "Release", title: "Three channels, one profile each", dek: "Why Stable, Preview and Nightly never share data.", href: "/notes/channels" },
        ],
    },
    reveal: Reveal {
        left: "Chromium",
        right: "in Rust",
        body: "The engine renders the web; Rust draws everything around it.",
    },
    footer: "YOLO is open source under MIT or Apache-2.0.",
};

const SHENRON: Site = Site {
    id: "shenron",
    name: "Shenron",
    domain: "dragonballfr.com",
    strings: &FR,
    nav: &[
        NavItem {
            label: "Encyclopédie",
            href: "/wiki",
            summary: "Personnages, techniques et lieux, chaque fiche sourcée.",
            groups: &[
                LinkGroup {
                    heading: "Fiches",
                    links: &[l("Personnages", "/wiki/personnages"), l("Techniques", "/wiki/techniques"), l("Lieux", "/wiki/lieux")],
                },
                LinkGroup {
                    heading: "Repères",
                    links: &[l("Chronologie", "/chronologie"), l("Canon", "/canon"), l("Lexique japonais", "/lexique")],
                },
            ],
        },
        NavItem {
            label: "Lire",
            href: "/lire",
            summary: "Le manga et les databooks, page par page.",
            groups: &[LinkGroup {
                heading: "Lecteurs",
                links: &[l("Manga", "/manga"), l("Databooks", "/databooks")],
            }],
        },
        NavItem { label: "Actualités", href: "/actualites", summary: "", groups: &[] },
        NavItem { label: "Studio", href: "/studio", summary: "", groups: &[] },
    ],
    release: Release {
        tag: "Nouveau",
        text: "La recherche couvre désormais les databooks",
        href: "/actualites/recherche-databooks",
    },
    hero: Hero {
        eyebrow: "Shenron",
        title: "Dragon Ball, sources à l'appui.",
        lede: "Une encyclopédie où chaque fait renvoie au manga ou aux databooks officiels, avec les lecteurs pour vérifier par vous-même.",
        primary: l("Explorer l'encyclopédie", "/wiki"),
        secondary: Some(l("Lire le manga", "/manga")),
    },
    product: ProductPage {
        name: "Shenron Studio",
        hero: Hero {
            eyebrow: "Shenron Studio",
            title: "Traduire une planche sans perdre le texte d'origine.",
            lede: "L'éditeur de traduction de Shenron garde le japonais, la traduction et les notes côte à côte, bulle par bulle.",
            primary: l("Ouvrir le Studio", "/studio"),
            secondary: Some(l("Voir un exemple", "/studio/exemple")),
        },
        sketch: Sketch {
            alt: "Dessin de l'éditeur : une liste de pages à gauche, des bulles numérotées au centre et leur traduction à droite.",
            window_title: "Studio",
            sidebar: &["Pages", "Bulles", "Lexique", "Relecture"],
            rows: &["Bulle 1, japonais", "Bulle 1, français", "Bulle 2, japonais", "Bulle 2, français"],
            card: "Lexique : 3 termes vérifiés",
        },
        tabs_label: "Sections de la page Studio",
        sections: &[
            Section {
                id: "studio-bulles",
                tab: "Bulles",
                heading: "Une bulle, deux langues",
                body: "Chaque bulle garde son texte japonais, sa traduction et la personne qui l'a relue.",
                points: &["Ordre de lecture japonais respecté", "Historique de chaque modification"],
            },
            Section {
                id: "studio-lexique",
                tab: "Lexique",
                heading: "Les noms restent justes",
                body: "Le lexique attesté propose la graphie validée des noms propres et des techniques pendant la saisie.",
                points: &["Graphies tirées des sources officielles", "Les écarts sont signalés à la relecture"],
            },
            Section {
                id: "studio-relecture",
                tab: "Relecture",
                heading: "Relire avant de publier",
                body: "Une planche n'est publiée qu'après relecture, et la relecture cite la page source.",
                points: &["Commentaires par bulle", "Publication en un clic après validation"],
            },
        ],
        faqs: &[
            Faq {
                question: "Le Studio publie-t-il des scans complets ?",
                answer: "Non. Il sert à préparer et relire des traductions ; la lecture complète passe par les éditions officielles.",
            },
            Faq {
                question: "Faut-il un compte ?",
                answer: "La lecture de l'encyclopédie est libre ; le Studio demande un compte de contributeur.",
            },
        ],
    },
    downloads: Downloads {
        heading: "Shenron chez vous",
        intro: "Le bot Discord et le serveur autonome reprennent la recherche et l'encyclopédie hors du site.",
        channels: &[
            Channel {
                name: "Bot Discord",
                cadence: "Mis à jour en continu",
                description: "Recherche, chronologie et vérification du canon directement dans votre serveur.",
                platforms: &["Discord"],
                action: l("Ajouter le bot", "/bot"),
                recommended: true,
            },
            Channel {
                name: "Serveur autonome",
                cadence: "À chaque version",
                description: "L'API et la recherche sur votre machine, pour un usage hors ligne.",
                platforms: &["Linux x64", "Windows 11"],
                action: l("Télécharger le serveur", "/serveur"),
                recommended: false,
            },
        ],
    },
    app: AppChrome {
        app_name: "Shenron",
        rail: &[
            RailItem { label: "Accueil", href: "/", icon: IconName::Home },
            RailItem { label: "Explorer", href: "/wiki", icon: IconName::Compass },
            RailItem { label: "Bibliothèque", href: "/lire", icon: IconName::Library },
            RailItem { label: "Historique", href: "/historique", icon: IconName::Clock },
        ],
        search_label: "Rechercher dans Shenron",
        grid_heading: "Databooks et volumes",
        media: &[
            MediaItem { title: "Daizenshuu 1 : les illustrations", source: "Databooks", meta: "Lecteur, illustrations", badge: "Databook", href: "/databooks/daizenshuu-1", hue: 24 },
            MediaItem { title: "Daizenshuu 2 : l'histoire", source: "Databooks", meta: "Lecteur, récit", badge: "Databook", href: "/databooks/daizenshuu-2", hue: 40 },
            MediaItem { title: "Daizenshuu 4 : le monde", source: "Databooks", meta: "Lecteur, univers", badge: "Databook", href: "/databooks/daizenshuu-4", hue: 200 },
            MediaItem { title: "Daizenshuu 7 : le guide", source: "Databooks", meta: "Lecteur, encyclopédie", badge: "Databook", href: "/databooks/daizenshuu-7", hue: 120 },
            MediaItem { title: "Chronologie de l'âge 749 à 774", source: "Chronologie", meta: "Repères, dates", badge: "Repères", href: "/chronologie", hue: 280 },
            MediaItem { title: "Ce qui est canon, et pourquoi", source: "Canon", meta: "Doctrine, références", badge: "Canon", href: "/canon", hue: 0 },
        ],
    },
    editorial: Editorial {
        kicker: "La rédaction",
        headline: "Vérifier avant de raconter",
        stories: &[
            Story { kicker: "Méthode", title: "Le manga d'abord", dek: "Comment la rédaction tranche quand deux sources se contredisent.", href: "/actualites/methode" },
            Story { kicker: "Databooks", title: "Relire une page OCR", dek: "De la planche scannée au texte cité dans une fiche.", href: "/actualites/ocr" },
            Story { kicker: "Lexique", title: "Écrire les noms juste", dek: "Le lexique attesté et ses sources japonaises.", href: "/lexique" },
            Story { kicker: "Studio", title: "Une traduction relue", dek: "Le parcours d'une bulle, de la saisie à la publication.", href: "/studio/parcours" },
        ],
    },
    reveal: Reveal {
        left: "Chaque fait",
        right: "sa source",
        body: "Les fiches citent la page du manga ou du databook dont elles viennent.",
    },
    footer: "Shenron est un site de fans, sans affiliation avec les ayants droit de Dragon Ball.",
};

const ROSEGRIFFON: Site = Site {
    id: "rosegriffon",
    name: "Rose Griffon",
    domain: "rosegriffon.fr",
    strings: &FR,
    nav: &[
        NavItem {
            label: "Achillea",
            href: "/achillea",
            summary: "La plateforme classée de la communauté.",
            groups: &[LinkGroup {
                heading: "Compétition",
                links: &[l("Classement", "/achillea/classement"), l("Saisons", "/achillea/saisons"), l("Règlement", "/achillea/reglement")],
            }],
        },
        NavItem {
            label: "Communauté",
            href: "/communaute",
            summary: "Le serveur Discord, ses bots et ses équipes.",
            groups: &[
                LinkGroup {
                    heading: "Discord",
                    links: &[l("Rejoindre le serveur", "/discord"), l("Bot Rose Griffon", "/bot"), l("Azalée", "/azalee")],
                },
                LinkGroup {
                    heading: "Équipes",
                    links: &[l("Effectifs", "/equipes"), l("Recrutement", "/recrutement")],
                },
            ],
        },
        NavItem { label: "Actualités", href: "/actualites", summary: "", groups: &[] },
        NavItem { label: "Profil", href: "/profil", summary: "", groups: &[] },
    ],
    release: Release {
        tag: "Saison",
        text: "Les inscriptions à la nouvelle saison Achillea sont ouvertes",
        href: "/achillea/saisons",
    },
    hero: Hero {
        eyebrow: "Rose Griffon",
        title: "Jouer, se classer, progresser ensemble.",
        lede: "Une communauté esport avec son classement, ses saisons et ses bots, du serveur Discord jusqu'au profil en ligne.",
        primary: l("Rejoindre Achillea", "/achillea"),
        secondary: Some(l("Entrer sur le Discord", "/discord")),
    },
    product: ProductPage {
        name: "Achillea",
        hero: Hero {
            eyebrow: "Achillea",
            title: "Des matchs classés qui comptent.",
            lede: "Inscrivez-vous, jouez vos matchs et suivez votre cote saison après saison, sans quitter Discord.",
            primary: l("S'inscrire à la saison", "/achillea/inscription"),
            secondary: Some(l("Lire le règlement", "/achillea/reglement")),
        },
        sketch: Sketch {
            alt: "Dessin du classement : des onglets de saison à gauche, une liste de joueurs avec leur cote au centre.",
            window_title: "Classement",
            sidebar: &["Saison", "Matchs", "Équipes", "Profil"],
            rows: &["1. Joueur, cote 1840", "2. Joueur, cote 1795", "3. Joueur, cote 1762", "4. Joueur, cote 1730"],
            card: "Prochain match : ce soir",
        },
        tabs_label: "Sections de la page Achillea",
        sections: &[
            Section {
                id: "achillea-saisons",
                tab: "Saisons",
                heading: "Une saison, un objectif",
                body: "Chaque saison repart d'une cote commune et se termine par un classement final archivé.",
                points: &["Inscription depuis Discord ou le site", "Classements passés consultables"],
            },
            Section {
                id: "achillea-matchs",
                tab: "Matchs",
                heading: "Des matchs sans paperasse",
                body: "Le bot propose les rencontres, récupère les résultats et met la cote à jour.",
                points: &["Résultats confirmés par les deux camps", "Litiges traités par l'équipe"],
            },
            Section {
                id: "achillea-profil",
                tab: "Profil",
                heading: "Votre progression en un coup d'œil",
                body: "Le profil réunit votre cote, vos matchs et vos équipes, sur le site comme sur Discord.",
                points: &["Historique complet", "Visibilité réglable"],
            },
        ],
        faqs: &[
            Faq {
                question: "Faut-il être sur le Discord pour jouer ?",
                answer: "Oui, les rencontres et les résultats passent par le serveur Rose Griffon.",
            },
            Faq {
                question: "Comment la cote est-elle calculée ?",
                answer: "Elle évolue après chaque match confirmé ; le règlement détaille la formule.",
            },
        ],
    },
    downloads: Downloads {
        heading: "Rose Griffon chez vous",
        intro: "Ajoutez les bots à votre serveur ou hébergez l'exécutable rosegriffon vous-même.",
        channels: &[
            Channel {
                name: "Bot Discord",
                cadence: "Mis à jour en continu",
                description: "Profils, classement et rencontres Achillea dans votre serveur.",
                platforms: &["Discord"],
                action: l("Ajouter le bot", "/bot"),
                recommended: true,
            },
            Channel {
                name: "Exécutable rosegriffon",
                cadence: "À chaque version",
                description: "Le site, l'API et les bots sur votre propre serveur.",
                platforms: &["Linux x64", "Windows 11"],
                action: l("Télécharger l'exécutable", "/auto-hebergement"),
                recommended: false,
            },
        ],
    },
    app: AppChrome {
        app_name: "Rose Griffon",
        rail: &[
            RailItem { label: "Accueil", href: "/", icon: IconName::Home },
            RailItem { label: "Achillea", href: "/achillea", icon: IconName::Compass },
            RailItem { label: "Équipes", href: "/equipes", icon: IconName::Grid },
            RailItem { label: "Historique", href: "/historique", icon: IconName::Clock },
        ],
        search_label: "Rechercher un joueur ou une équipe",
        grid_heading: "Saisons et rencontres",
        media: &[
            MediaItem { title: "Finale de la saison passée", source: "Achillea", meta: "Rediffusion, finale", badge: "Finale", href: "/achillea/finale", hue: 340 },
            MediaItem { title: "Présentation des équipes inscrites", source: "Équipes", meta: "Annonce, effectifs", badge: "Équipes", href: "/equipes", hue: 300 },
            MediaItem { title: "Le règlement expliqué en dix minutes", source: "Achillea", meta: "Guide, règlement", badge: "Guide", href: "/achillea/reglement", hue: 210 },
            MediaItem { title: "Configurer Azalée sur votre serveur", source: "Azalée", meta: "Guide, Discord", badge: "Guide", href: "/azalee", hue: 140 },
            MediaItem { title: "Les meilleures actions de la semaine", source: "Communauté", meta: "Sélection, temps forts", badge: "Temps forts", href: "/temps-forts", hue: 20 },
            MediaItem { title: "Recrutement : ce que cherchent les équipes", source: "Recrutement", meta: "Annonce, équipes", badge: "Annonce", href: "/recrutement", hue: 260 },
        ],
    },
    editorial: Editorial {
        kicker: "Le journal",
        headline: "La saison vue de l'intérieur",
        stories: &[
            Story { kicker: "Achillea", title: "Une cote plus lisible", dek: "Ce qui change dans le calcul du classement cette saison.", href: "/actualites/cote" },
            Story { kicker: "Discord", title: "Azalée arrive sur votre serveur", dek: "Le second bot de la communauté et ses commandes.", href: "/actualites/azalee" },
            Story { kicker: "Équipes", title: "Monter une équipe", dek: "Effectif, remplaçants et inscription en saison.", href: "/actualites/equipes" },
            Story { kicker: "Communauté", title: "Les règles du respect", dek: "La charte qui s'applique au serveur et aux matchs.", href: "/charte" },
        ],
    },
    reveal: Reveal {
        left: "Rose",
        right: "Griffon",
        body: "Une communauté, un classement et des saisons qui se jouent ensemble.",
    },
    footer: "Rose Griffon, communauté esport francophone.",
};

const AUTO: Site = Site {
    id: "auto",
    name: "Auto",
    domain: "auto.aphrody.com",
    strings: &FR,
    nav: &[
        NavItem {
            label: "Produit",
            href: "/produit",
            summary: "Automatiser le travail répétitif d'une petite entreprise.",
            groups: &[
                LinkGroup {
                    heading: "Automatiser",
                    links: &[l("Workflows visuels", "/produit/workflows"), l("Déclencheurs", "/produit/declencheurs"), l("Agents IA", "/produit/agents")],
                },
                LinkGroup {
                    heading: "Gérer",
                    links: &[l("Facturation électronique", "/produit/facturation"), l("Achats", "/produit/achats"), l("Documents", "/produit/documents")],
                },
                LinkGroup {
                    heading: "Connecter",
                    links: &[l("Slack, Discord, Telegram", "/produit/messageries"), l("Mail", "/produit/mail"), l("Tableurs", "/produit/tableurs")],
                },
            ],
        },
        NavItem {
            label: "Développeurs",
            href: "/developpeurs",
            summary: "Une seule API pour toutes les surfaces.",
            groups: &[LinkGroup {
                heading: "Outils",
                links: &[l("API /v1", "/developpeurs/api"), l("CLI", "/developpeurs/cli"), l("Serveur MCP", "/developpeurs/mcp")],
            }],
        },
        NavItem { label: "Tarifs", href: "/tarifs", summary: "", groups: &[] },
        NavItem { label: "Connexion", href: "/connexion", summary: "", groups: &[] },
    ],
    release: Release {
        tag: "Nouveau",
        text: "La facturation électronique devient un déclencheur",
        href: "/nouveautes/facturation",
    },
    hero: Hero {
        eyebrow: "Auto",
        title: "Votre entreprise tourne, Auto s'occupe du reste.",
        lede: "Des workflows visuels déclenchés par vos factures, vos mails ou un horaire, pour les indépendants et les TPE.",
        primary: l("Créer un espace", "/inscription"),
        secondary: Some(l("Voir les modèles", "/modeles")),
    },
    product: ProductPage {
        name: "Workflows Auto",
        hero: Hero {
            eyebrow: "Workflows",
            title: "Un déclencheur, quelques étapes, c'est réglé.",
            lede: "Assemblez des étapes HTTP, mail, tableur ou IA autour d'un événement de votre entreprise, sans écrire de code.",
            primary: l("Créer un workflow", "/workflows/nouveau"),
            secondary: Some(l("Parcourir les modèles", "/modeles")),
        },
        sketch: Sketch {
            alt: "Dessin de l'éditeur : un déclencheur facture relié à trois étapes, mail, tableur et Slack.",
            window_title: "Workflow",
            sidebar: &["Déclencheurs", "Étapes", "Exécutions", "Réglages"],
            rows: &["Facture émise", "Envoyer le PDF par mail", "Ajouter au tableur", "Prévenir sur Slack"],
            card: "Dernière exécution : réussie",
        },
        tabs_label: "Sections de la page Workflows",
        sections: &[
            Section {
                id: "auto-declencheurs",
                tab: "Déclencheurs",
                heading: "Partir d'un vrai événement",
                body: "Une facture émise, un mail reçu, un webhook ou un horaire : chaque workflow démarre sur un fait de votre activité.",
                points: &["Événements de facturation et d'achats", "Cron et webhooks"],
            },
            Section {
                id: "auto-etapes",
                tab: "Étapes",
                heading: "Des étapes qui parlent métier",
                body: "HTTP, web, RSS, tableurs, messageries, mail et modèles d'IA, reliés dans un éditeur visuel.",
                points: &["Données transmises d'une étape à l'autre", "Nouvelle tentative automatique"],
            },
            Section {
                id: "auto-surfaces",
                tab: "Partout",
                heading: "Une API, toutes les surfaces",
                body: "Web, bureau, mobile, CLI, serveur MCP et extension Chrome passent par la même API /v1.",
                points: &["Mêmes droits partout", "Historique d'exécution commun"],
            },
        ],
        faqs: &[
            Faq {
                question: "Faut-il savoir coder ?",
                answer: "Non. L'éditeur visuel suffit ; l'API et la CLI sont là si vous voulez aller plus loin.",
            },
            Faq {
                question: "Où sont hébergées mes données ?",
                answer: "Dans l'Union européenne, sur une base PostgreSQL située à Francfort.",
            },
        ],
    },
    downloads: Downloads {
        heading: "Auto sur tous vos appareils",
        intro: "Le même espace, sur le web, le bureau, le téléphone et le terminal.",
        channels: &[
            Channel {
                name: "Application de bureau",
                cadence: "À chaque version",
                description: "Une fenêtre native pour suivre vos exécutions.",
                platforms: &["Windows 11", "macOS", "Linux x64"],
                action: l("Télécharger pour le bureau", "/telecharger/bureau"),
                recommended: true,
            },
            Channel {
                name: "Application mobile",
                cadence: "À chaque version",
                description: "Notifications et validations en déplacement.",
                platforms: &["Android", "iOS"],
                action: l("Installer sur mobile", "/telecharger/mobile"),
                recommended: false,
            },
            Channel {
                name: "CLI",
                cadence: "À chaque version",
                description: "Lancer et inspecter vos workflows depuis un terminal.",
                platforms: &["Windows 11", "macOS", "Linux x64"],
                action: l("Installer la CLI", "/telecharger/cli"),
                recommended: false,
            },
        ],
    },
    app: AppChrome {
        app_name: "Auto",
        rail: &[
            RailItem { label: "Accueil", href: "/", icon: IconName::Home },
            RailItem { label: "Workflows", href: "/workflows", icon: IconName::Grid },
            RailItem { label: "Modèles", href: "/modeles", icon: IconName::Library },
            RailItem { label: "Exécutions", href: "/executions", icon: IconName::Clock },
        ],
        search_label: "Rechercher un workflow",
        grid_heading: "Modèles prêts à l'emploi",
        media: &[
            MediaItem { title: "Relancer une facture impayée après 15 jours", source: "Facturation", meta: "Modèle, mail", badge: "3 étapes", href: "/modeles/relance", hue: 150 },
            MediaItem { title: "Copier chaque achat dans un tableur", source: "Achats", meta: "Modèle, tableur", badge: "2 étapes", href: "/modeles/achats-tableur", hue: 100 },
            MediaItem { title: "Résumer les mails du jour avec un modèle d'IA", source: "Mail", meta: "Modèle, IA", badge: "4 étapes", href: "/modeles/resume-mails", hue: 260 },
            MediaItem { title: "Publier un flux RSS sur Discord", source: "Messageries", meta: "Modèle, RSS", badge: "2 étapes", href: "/modeles/rss-discord", hue: 230 },
            MediaItem { title: "Prévenir sur Telegram quand un paiement arrive", source: "Facturation", meta: "Modèle, Telegram", badge: "2 étapes", href: "/modeles/paiement-telegram", hue: 190 },
            MediaItem { title: "Archiver les pièces jointes dans Documents", source: "Documents", meta: "Modèle, mail", badge: "3 étapes", href: "/modeles/archives", hue: 40 },
        ],
    },
    editorial: Editorial {
        kicker: "Le blog",
        headline: "Moins de clics, plus de temps",
        stories: &[
            Story { kicker: "Facturation", title: "La facture électronique sans stress", dek: "Ce que la réforme change pour une TPE, et ce qu'Auto fait pour vous.", href: "/blog/facture-electronique" },
            Story { kicker: "IA", title: "Un agent qui trie vos mails", dek: "Quand confier une étape à un modèle, et quand l'éviter.", href: "/blog/agent-mails" },
            Story { kicker: "Méthode", title: "Votre premier workflow", dek: "De l'événement à la notification en cinq minutes.", href: "/blog/premier-workflow" },
            Story { kicker: "API", title: "Une API pour tout", dek: "Pourquoi le web, le mobile et la CLI partagent les mêmes routes.", href: "/blog/api-v1" },
        ],
    },
    reveal: Reveal {
        left: "Un événement",
        right: "une action",
        body: "Chaque workflow relie un fait de votre activité à ce qu'il faut faire ensuite.",
    },
    footer: "Auto, automatisation pour indépendants et TPE.",
};

const CRAFT: Site = Site {
    id: "craft",
    name: "Craft",
    domain: "craft.aphrody.com",
    strings: &EN,
    nav: &[
        NavItem {
            label: "Workbenches",
            href: "/workbenches",
            summary: "Thirteen workbenches sharing one docking, toolbar and theme chrome.",
            groups: &[
                LinkGroup {
                    heading: "Words and pages",
                    links: &[l("WordCraft", "/wordcraft"), l("DeckCraft", "/deckcraft"), l("PdfCraft", "/pdfcraft"), l("GridCraft", "/gridcraft")],
                },
                LinkGroup {
                    heading: "Images",
                    links: &[l("PhotoCraft", "/photocraft"), l("LightCraft", "/lightcraft"), l("VectorCraft", "/vectorcraft"), l("DesignCraft", "/designcraft")],
                },
                LinkGroup {
                    heading: "Motion and sound",
                    links: &[l("FilmCraft", "/filmcraft"), l("EffectCraft", "/effectcraft"), l("SoundCraft", "/soundcraft")],
                },
                LinkGroup {
                    heading: "Space and ideas",
                    links: &[l("CadCraft", "/cadcraft"), l("FigCraft", "/figcraft")],
                },
            ],
        },
        NavItem {
            label: "Learn",
            href: "/learn",
            summary: "Guides for each workbench.",
            groups: &[LinkGroup {
                heading: "Guides",
                links: &[l("First project", "/learn/first-project"), l("Docking and layouts", "/learn/docking"), l("Printing", "/learn/printing")],
            }],
        },
        NavItem { label: "Download", href: "/download", summary: "", groups: &[] },
        NavItem { label: "Community", href: "/community", summary: "", groups: &[] },
    ],
    release: Release {
        tag: "Update",
        text: "Every workbench now follows your yolo-ui preset",
        href: "/notes/presets",
    },
    hero: Hero {
        eyebrow: "Craft",
        title: "Thirteen workbenches, one way of working.",
        lede: "Write, lay out, edit photos, cut film, mix sound and model parts in apps that share the same panels, shortcuts and theme.",
        primary: l("Download Craft", "/download"),
        secondary: Some(l("Tour the workbenches", "/workbenches")),
    },
    product: ProductPage {
        name: "WordCraft",
        hero: Hero {
            eyebrow: "WordCraft",
            title: "Long-form writing, ready for print.",
            lede: "Structured chapters, styles, a bibliography and print pagination in one document.",
            primary: l("Open WordCraft", "/wordcraft/open"),
            secondary: Some(l("See a sample book", "/wordcraft/sample")),
        },
        sketch: Sketch {
            alt: "Drawing of WordCraft: a chapter outline on the left, a page with headings in the middle and a styles panel on the right.",
            window_title: "WordCraft",
            sidebar: &["Outline", "Styles", "Sources", "Print"],
            rows: &["Chapter 1", "Chapter 2", "Chapter 3", "Bibliography"],
            card: "Pages: 212, print ready",
        },
        tabs_label: "WordCraft page sections",
        sections: &[
            Section {
                id: "wordcraft-structure",
                tab: "Structure",
                heading: "Chapters you can move",
                body: "The outline is the document. Drag a chapter and its pages, notes and references move with it.",
                points: &["Outline and page stay in sync", "Cross-references update themselves"],
            },
            Section {
                id: "wordcraft-styles",
                tab: "Styles",
                heading: "Styles, not formatting",
                body: "Headings, quotes and captions are styles that follow your preset on screen and your print profile on paper.",
                points: &["One change restyles the book", "Print profiles per publisher"],
            },
            Section {
                id: "wordcraft-print",
                tab: "Print",
                heading: "Paginate with confidence",
                body: "Widows, orphans and running heads are handled as you type, and the preflight lists what a printer will reject.",
                points: &["Preflight before export", "PDF output through PdfCraft"],
            },
        ],
        faqs: &[
            Faq {
                question: "Can I open my existing documents?",
                answer: "WordCraft imports Markdown and plain text today; richer formats arrive workbench by workbench.",
            },
            Faq {
                question: "Does it work offline?",
                answer: "Yes. Every workbench runs locally and saves to your disk.",
            },
        ],
    },
    downloads: Downloads {
        heading: "Install Craft",
        intro: "One installer brings every workbench; open only the ones you need.",
        channels: &[
            Channel {
                name: "Stable",
                cadence: "Every four weeks",
                description: "For projects with deadlines.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Stable", "/download/stable"),
                recommended: true,
            },
            Channel {
                name: "Preview",
                cadence: "Every week",
                description: "New tools a cycle early. Keep backups.",
                platforms: &["Windows 11", "Linux x64"],
                action: l("Download Preview", "/download/preview"),
                recommended: false,
            },
        ],
    },
    app: AppChrome {
        app_name: "Craft",
        rail: &[
            RailItem { label: "Home", href: "/", icon: IconName::Home },
            RailItem { label: "Workbenches", href: "/workbenches", icon: IconName::Grid },
            RailItem { label: "Projects", href: "/projects", icon: IconName::Library },
            RailItem { label: "Recent", href: "/recent", icon: IconName::Clock },
        ],
        search_label: "Search projects and workbenches",
        grid_heading: "Workbenches",
        media: &[
            MediaItem { title: "Parametric 3D CAD and solid modelling", source: "CadCraft", meta: "Workbench, 3D", badge: "3D", href: "/cadcraft", hue: 200 },
            MediaItem { title: "Multi-track video editing with colour scopes", source: "FilmCraft", meta: "Workbench, video", badge: "Video", href: "/filmcraft", hue: 0 },
            MediaItem { title: "Arrange, record and mix audio", source: "SoundCraft", meta: "Workbench, audio", badge: "Audio", href: "/soundcraft", hue: 280 },
            MediaItem { title: "Layered raster editing and brushes", source: "PhotoCraft", meta: "Workbench, images", badge: "Images", href: "/photocraft", hue: 30 },
            MediaItem { title: "Precise vector illustration for print", source: "VectorCraft", meta: "Workbench, vector", badge: "Vector", href: "/vectorcraft", hue: 320 },
            MediaItem { title: "Reactive spreadsheets and pivot tables", source: "GridCraft", meta: "Workbench, data", badge: "Data", href: "/gridcraft", hue: 140 },
        ],
    },
    editorial: Editorial {
        kicker: "Studio notes",
        headline: "Made with Craft",
        stories: &[
            Story { kicker: "Writing", title: "A novel, from outline to print", dek: "How one author moved a manuscript into WordCraft chapter by chapter.", href: "/notes/novel" },
            Story { kicker: "Film", title: "Colour scopes you can read", dek: "Waveform and vectorscope in FilmCraft, explained.", href: "/notes/scopes" },
            Story { kicker: "Sound", title: "One mixer for every track", dek: "Routing, buses and effects in SoundCraft.", href: "/notes/mixer" },
            Story { kicker: "Design", title: "Tokens in DesignCraft", dek: "Design with the same tokens the apps are built from.", href: "/notes/designcraft-tokens" },
        ],
    },
    reveal: Reveal {
        left: "Thirteen",
        right: "workbenches",
        body: "Panels, shortcuts and themes are shared, so learning one teaches you the rest.",
    },
    footer: "Craft is part of yolo-ui, open source under MIT or Apache-2.0.",
};
