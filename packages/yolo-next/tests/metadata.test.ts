// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";

import { defaultViewport, headElements, mergeMetadata, normalizeMetadata, renderHeadTags, resolveMetadataChain, resolveTitle, viewportContent } from "../src/metadata.ts";
import type { Metadata } from "../src/types.ts";

const root: Metadata = {
  metadataBase: "https://fixture.example.com",
  title: { default: "Yolo Fixture", template: "%s | Yolo Fixture" },
  description: "Next.js 16 migration fixture",
  applicationName: "Yolo Fixture",
  keywords: ["yolo", "next", "bun"],
  authors: [{ name: "Aphrody", url: "https://aphrody.com" }],
  openGraph: { title: "Yolo Fixture", url: "https://fixture.example.com", siteName: "Yolo Fixture", images: [{ url: "/opengraph-image", width: 1200, height: 630 }], locale: "en_US", type: "website" },
  twitter: { card: "summary_large_image", site: "@aphrody" },
  robots: { index: true, follow: true, googleBot: { index: true, "max-image-preview": "large" } },
  alternates: { canonical: "/", languages: { "fr-FR": "/fr" } },
  icons: { icon: "/icon.svg", apple: "/apple-icon.png" },
  manifest: "/manifest.webmanifest",
};

describe("titles", () => {
  test("resolveTitle", () => {
    expect(resolveTitle(root, { title: "Pricing" })).toBe("Pricing | Yolo Fixture");
    expect(resolveTitle(root, { title: { absolute: "Raw" } })).toBe("Raw");
    expect(resolveTitle(root, { title: { default: "Blog" } })).toBe("Blog | Yolo Fixture");
    expect(resolveTitle(root, {})).toBe("Yolo Fixture");
    expect(resolveTitle(undefined, { title: "Solo" })).toBe("Solo");
  });

  test("mergeMetadata and chains", () => {
    const merged = mergeMetadata(root, { title: "Pricing", description: "Plans" });
    expect(merged.title).toEqual({ absolute: "Pricing | Yolo Fixture" });
    expect(merged.description).toBe("Plans");
    expect(merged.metadataBase).toBe("https://fixture.example.com");
    const inherited = mergeMetadata(root, { description: "No title" });
    expect(inherited.title).toEqual({ absolute: "Yolo Fixture", template: "%s | Yolo Fixture" });
    const chain = resolveMetadataChain([root, { title: { template: "%s - Blog", default: "Blog" } }, { title: "Hello" }]);
    expect(chain.title).toEqual({ absolute: "Hello - Blog" });
    expect(normalizeMetadata({ title: "x", keywords: "a, b", metadataBase: new URL("https://h.io/") }).keywords).toEqual(["a", "b"]);
  });
});

describe("head rendering", () => {
  test("viewport", () => {
    expect(viewportContent(defaultViewport())).toBe("width=device-width, initial-scale=1");
    expect(viewportContent({ width: "device-width", initialScale: 1, maximumScale: 1, userScalable: false, viewportFit: "cover" })).toBe("width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover");
  });

  test("renderHeadTags on the fixture root metadata", () => {
    const html = renderHeadTags(root, { width: "device-width", initialScale: 1, themeColor: [{ media: "(prefers-color-scheme: dark)", color: "#09090b" }], colorScheme: "dark light" });
    expect(html.split("\n")[0]).toBe('<meta charset="utf-8">');
    expect(html).toContain('<meta name="viewport" content="width=device-width, initial-scale=1">');
    expect(html).toContain("<title>Yolo Fixture</title>");
    expect(html).toContain('<meta name="description" content="Next.js 16 migration fixture">');
    expect(html).toContain('<meta name="application-name" content="Yolo Fixture">');
    expect(html).toContain('<link rel="author" href="https://aphrody.com">');
    expect(html).toContain('<meta name="author" content="Aphrody">');
    expect(html).toContain('<meta name="keywords" content="yolo,next,bun">');
    expect(html).toContain('<meta name="robots" content="index, follow">');
    expect(html).toContain('<meta name="googlebot" content="index, max-image-preview:large">');
    expect(html).toContain('<meta name="theme-color" content="#09090b" media="(prefers-color-scheme: dark)">');
    expect(html).toContain('<meta name="color-scheme" content="dark light">');
    expect(html).toContain('<link rel="canonical" href="https://fixture.example.com/">');
    expect(html).toContain('<link rel="alternate" hreflang="fr-FR" href="https://fixture.example.com/fr">');
    expect(html).toContain('<link rel="manifest" href="/manifest.webmanifest">');
    expect(html).toContain('<link rel="icon" href="/icon.svg">');
    expect(html).toContain('<link rel="apple-touch-icon" href="/apple-icon.png">');
    expect(html).toContain('<meta property="og:title" content="Yolo Fixture">');
    expect(html).toContain('<meta property="og:image" content="https://fixture.example.com/opengraph-image">');
    expect(html).toContain('<meta property="og:image:width" content="1200">');
    expect(html).toContain('<meta property="og:type" content="website">');
    expect(html).toContain('<meta name="twitter:card" content="summary_large_image">');
    expect(html).toContain('<meta name="twitter:site" content="@aphrody">');
    expect(html.indexOf("<title>")).toBeLessThan(html.indexOf('name="description"'));
  });

  test("headElements descriptors, escaping and other keys", () => {
    const elements = headElements({ title: 'A "quoted" <title>', other: { "x-custom": ["1", "2"] }, verification: { google: "g", other: { me: "m" } }, appleWebApp: { capable: true, title: "App" }, formatDetection: { telephone: false } });
    expect(elements[0]).toEqual({ tag: "meta", attrs: { charSet: "utf-8" } });
    expect(elements.find((e) => e.tag === "title")?.children).toBe('A "quoted" <title>');
    expect(elements.filter((e) => e.attrs.name === "x-custom")).toHaveLength(2);
    expect(elements.find((e) => e.attrs.name === "google-site-verification")?.attrs.content).toBe("g");
    expect(elements.find((e) => e.attrs.name === "me")?.attrs.content).toBe("m");
    expect(elements.find((e) => e.attrs.name === "apple-mobile-web-app-title")?.attrs.content).toBe("App");
    expect(elements.find((e) => e.attrs.name === "format-detection")?.attrs.content).toBe("telephone=no");
    expect(renderHeadTags({ title: 'A "quoted" <title>' })).toContain("<title>A &quot;quoted&quot; &lt;title&gt;</title>");
  });
});
