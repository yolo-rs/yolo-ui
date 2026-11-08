// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/metadata: Metadata API (merge semantics, title templates, head tag rendering).

import type { Metadata, ThemeColor, TitleSpec, Viewport } from "./types.ts";

/** One `<head>` element as a React-free descriptor (`attrs` are already strings; `children` is text). */
export interface HeadElement {
  readonly tag: "title" | "meta" | "link";
  readonly attrs: Record<string, string>;
  readonly children?: string;
}

type Dict = Record<string, unknown>;

const isDict = (value: unknown): value is Dict => typeof value === "object" && value !== null && !Array.isArray(value);
const asString = (value: unknown): string | undefined =>
  typeof value === "string" ? value : typeof value === "number" || typeof value === "boolean" ? String(value) : value instanceof URL ? value.href : undefined;
const asList = <T,>(value: T | ReadonlyArray<T> | undefined | null): T[] => (value === undefined || value === null ? [] : Array.isArray(value) ? [...value] : [value as T]);

/** Default viewport of Next.js (`width=device-width, initial-scale=1`). */
export function defaultViewport(): Viewport {
  return { width: "device-width", initialScale: 1 };
}

function titleSpec(title: Metadata["title"]): TitleSpec | undefined {
  if (title === undefined || title === null) return undefined;
  return typeof title === "string" ? { absolute: title } : title;
}

/**
 * Normalizes a metadata object: string titles become `{ absolute }`, comma-separated keywords
 * become arrays, `metadataBase` becomes a string, single authors/icons become arrays.
 */
export function normalizeMetadata(metadata: Metadata): Metadata {
  const out: Dict = { ...metadata };
  if (metadata.title !== undefined) out.title = titleSpec(metadata.title);
  if (typeof metadata.keywords === "string") out.keywords = metadata.keywords.split(",").map((k) => k.trim()).filter(Boolean);
  if (metadata.metadataBase instanceof URL) out.metadataBase = metadata.metadataBase.href;
  if (metadata.authors !== undefined && !Array.isArray(metadata.authors)) out.authors = [metadata.authors];
  if (typeof metadata.icons === "string") out.icons = { icon: [metadata.icons] };
  if (isDict(metadata.openGraph) && metadata.openGraph.images !== undefined && !Array.isArray(metadata.openGraph.images)) {
    out.openGraph = { ...metadata.openGraph, images: [metadata.openGraph.images] };
  }
  if (isDict(metadata.twitter) && metadata.twitter.images !== undefined && !Array.isArray(metadata.twitter.images)) {
    out.twitter = { ...metadata.twitter, images: [metadata.twitter.images] };
  }
  for (const key of Object.keys(out)) if (out[key] === undefined) delete out[key];
  return out as Metadata;
}

function applyTemplate(template: string | undefined, title: string): string {
  return template ? template.replace(/%s/g, title) : title;
}

function absoluteTitle(spec: TitleSpec | undefined): string | undefined {
  if (!spec) return undefined;
  return spec.absolute ?? spec.default;
}

/**
 * Resolves the `<title>` text of `child` under `parent`: `absolute` wins, string and `default`
 * titles go through the parent's `%s` template, a missing child title keeps the parent's title.
 */
export function resolveTitle(parent: Metadata | undefined, child: Metadata): string | undefined {
  const parentSpec = titleSpec(parent?.title);
  if (typeof child.title === "string") return applyTemplate(parentSpec?.template, child.title);
  const childSpec = titleSpec(child.title);
  if (!childSpec) return absoluteTitle(parentSpec);
  if (childSpec.absolute !== undefined) return childSpec.absolute;
  if (childSpec.default !== undefined) return applyTemplate(parentSpec?.template, childSpec.default);
  return absoluteTitle(parentSpec);
}

/**
 * Shallow merge root -> leaf: keys defined by `child` win; the title is resolved with the parent's
 * template and the parent template is inherited while the child does not define a title.
 */
export function mergeMetadata(parent: Metadata, child: Metadata): Metadata {
  const base = normalizeMetadata(parent);
  const next = normalizeMetadata(child);
  const out: Dict = { ...base };
  for (const [key, value] of Object.entries(next)) if (value !== undefined && key !== "title") out[key] = value;
  const childSpec = titleSpec(child.title);
  const resolved = resolveTitle(parent, child);
  const template = childSpec ? childSpec.template : titleSpec(parent.title)?.template;
  if (resolved !== undefined || template !== undefined) {
    const spec: Dict = {};
    if (resolved !== undefined) spec.absolute = resolved;
    if (template !== undefined) spec.template = template;
    out.title = spec;
  }
  if (base.metadataBase && !next.metadataBase) out.metadataBase = base.metadataBase;
  return out as Metadata;
}

/** Folds a root -> leaf chain of metadata objects into the final metadata. */
export function resolveMetadataChain(chain: ReadonlyArray<Metadata>): Metadata {
  let acc: Metadata = {};
  for (const item of chain) acc = mergeMetadata(acc, item);
  return acc;
}

/** Final `<title>` of a resolved metadata object. */
export function finalTitle(metadata: Metadata): string | undefined {
  return absoluteTitle(titleSpec(metadata.title));
}

/** Builds the `content` of the viewport meta tag from `Viewport` fields. */
export function viewportContent(viewport: Viewport): string {
  const parts: string[] = [];
  if (viewport.width !== undefined) parts.push(`width=${viewport.width}`);
  if (viewport.height !== undefined) parts.push(`height=${viewport.height}`);
  if (viewport.initialScale !== undefined) parts.push(`initial-scale=${viewport.initialScale}`);
  if (viewport.minimumScale !== undefined) parts.push(`minimum-scale=${viewport.minimumScale}`);
  if (viewport.maximumScale !== undefined) parts.push(`maximum-scale=${viewport.maximumScale}`);
  if (viewport.userScalable !== undefined) parts.push(`user-scalable=${viewport.userScalable ? "yes" : "no"}`);
  if (viewport.viewportFit !== undefined) parts.push(`viewport-fit=${viewport.viewportFit}`);
  if (viewport.interactiveWidget !== undefined) parts.push(`interactive-widget=${viewport.interactiveWidget}`);
  return parts.join(", ");
}

function resolveUrl(value: unknown, base: string | undefined): string | undefined {
  const text = asString(value);
  if (text === undefined) return undefined;
  if (!base || /^[a-z][a-z0-9+.-]*:/i.test(text) || text.startsWith("//")) return text;
  try {
    return new URL(text, base).href;
  } catch {
    return text;
  }
}

function meta(attrs: Record<string, string | undefined>): HeadElement {
  const clean: Record<string, string> = {};
  for (const [key, value] of Object.entries(attrs)) if (value !== undefined) clean[key] = value;
  return { tag: "meta", attrs: clean };
}

function link(attrs: Record<string, string | undefined>): HeadElement {
  const clean: Record<string, string> = {};
  for (const [key, value] of Object.entries(attrs)) if (value !== undefined) clean[key] = value;
  return { tag: "link", attrs: clean };
}

function robotsContent(value: unknown): string | undefined {
  if (typeof value === "string") return value;
  if (!isDict(value)) return undefined;
  const parts: string[] = [];
  const flag = (key: string, yes: string, no: string): void => {
    if (value[key] === true) parts.push(yes);
    else if (value[key] === false) parts.push(no);
  };
  flag("index", "index", "noindex");
  flag("follow", "follow", "nofollow");
  if (value.noarchive === true) parts.push("noarchive");
  if (value.nosnippet === true) parts.push("nosnippet");
  if (value.noimageindex === true) parts.push("noimageindex");
  if (value.nocache === true) parts.push("nocache");
  if (value.notranslate === true) parts.push("notranslate");
  if (value.indexifembedded === true) parts.push("indexifembedded");
  if (value.nositelinkssearchbox === true) parts.push("nositelinkssearchbox");
  if (value.unavailable_after !== undefined) parts.push(`unavailable_after: ${String(value.unavailable_after)}`);
  if (value["max-video-preview"] !== undefined) parts.push(`max-video-preview:${String(value["max-video-preview"])}`);
  if (value["max-image-preview"] !== undefined) parts.push(`max-image-preview:${String(value["max-image-preview"])}`);
  if (value["max-snippet"] !== undefined) parts.push(`max-snippet:${String(value["max-snippet"])}`);
  return parts.length ? parts.join(", ") : undefined;
}

function themeColors(value: unknown): ThemeColor[] {
  const out: ThemeColor[] = [];
  for (const item of asList(value as ThemeColor | string | ReadonlyArray<ThemeColor | string> | undefined)) {
    if (typeof item === "string") out.push({ color: item });
    else if (isDict(item) && typeof item.color === "string") out.push({ color: item.color, media: asString(item.media) });
  }
  return out;
}

function imageEntries(value: unknown, base: string | undefined): Array<{ url: string; width?: string; height?: string; alt?: string; type?: string }> {
  const out: Array<{ url: string; width?: string; height?: string; alt?: string; type?: string }> = [];
  for (const item of asList(value as unknown as ReadonlyArray<unknown>)) {
    if (typeof item === "string" || item instanceof URL) {
      const url = resolveUrl(item, base);
      if (url) out.push({ url });
    } else if (isDict(item)) {
      const url = resolveUrl(item.url, base);
      if (url) out.push({ url, width: asString(item.width), height: asString(item.height), alt: asString(item.alt), type: asString(item.type) });
    }
  }
  return out;
}

function iconEntries(value: unknown, rel: string): Array<Record<string, string | undefined>> {
  const out: Array<Record<string, string | undefined>> = [];
  for (const item of asList(value as unknown as ReadonlyArray<unknown>)) {
    if (typeof item === "string" || item instanceof URL) out.push({ rel, href: asString(item) });
    else if (isDict(item)) {
      out.push({ rel: asString(item.rel) ?? rel, href: asString(item.url) ?? asString(item.href), sizes: asString(item.sizes), type: asString(item.type), media: asString(item.media), color: asString(item.color) });
    }
  }
  return out.filter((entry) => entry.href !== undefined);
}

/**
 * Produces the `<head>` elements of a resolved metadata + viewport pair as React-free descriptors
 * (same order as Next.js: charset, viewport, title, description, ..., Open Graph, Twitter, other).
 */
export function headElements(metadata: Metadata, viewport?: Viewport): HeadElement[] {
  const m = normalizeMetadata(metadata);
  const base = asString(m.metadataBase);
  const vp = viewport ?? defaultViewport();
  const out: HeadElement[] = [meta({ charSet: "utf-8" })];
  const content = viewportContent(vp);
  if (content) out.push(meta({ name: "viewport", content }));
  const title = finalTitle(m);
  if (title !== undefined) out.push({ tag: "title", attrs: {}, children: title });
  if (m.description) out.push(meta({ name: "description", content: m.description }));
  if (m.applicationName) out.push(meta({ name: "application-name", content: m.applicationName }));
  for (const author of asList(m.authors)) {
    if (!isDict(author)) continue;
    const url = asString(author.url);
    if (url) out.push(link({ rel: "author", href: url }));
    const name = asString(author.name);
    if (name) out.push(meta({ name: "author", content: name }));
  }
  if (m.generator) out.push(meta({ name: "generator", content: m.generator }));
  const keywords = asList(m.keywords as string | ReadonlyArray<string> | undefined);
  if (keywords.length) out.push(meta({ name: "keywords", content: keywords.join(",") }));
  if (m.referrer) out.push(meta({ name: "referrer", content: m.referrer }));
  if (m.creator) out.push(meta({ name: "creator", content: m.creator }));
  if (m.publisher) out.push(meta({ name: "publisher", content: m.publisher }));
  const robots = robotsContent(m.robots);
  if (robots) out.push(meta({ name: "robots", content: robots }));
  if (isDict(m.robots)) {
    const googleBot = robotsContent(m.robots.googleBot);
    if (googleBot) out.push(meta({ name: "googlebot", content: googleBot }));
  }
  for (const tc of themeColors(vp.themeColor ?? m.themeColor)) out.push(meta({ name: "theme-color", content: tc.color, media: tc.media }));
  const colorScheme = vp.colorScheme ?? m.colorScheme;
  if (colorScheme) out.push(meta({ name: "color-scheme", content: colorScheme }));
  if (isDict(m.alternates)) {
    const canonical = isDict(m.alternates.canonical) ? m.alternates.canonical.url : m.alternates.canonical;
    const canonicalUrl = resolveUrl(canonical, base);
    if (canonicalUrl) out.push(link({ rel: "canonical", href: canonicalUrl }));
    if (isDict(m.alternates.languages)) {
      for (const [lang, target] of Object.entries(m.alternates.languages)) {
        for (const item of asList(target as unknown as ReadonlyArray<unknown>)) {
          const href = resolveUrl(isDict(item) ? item.url : item, base);
          if (href) out.push(link({ rel: "alternate", hrefLang: lang, href }));
        }
      }
    }
    if (isDict(m.alternates.media)) {
      for (const [media, target] of Object.entries(m.alternates.media)) {
        const href = resolveUrl(isDict(target) ? target.url : target, base);
        if (href) out.push(link({ rel: "alternate", media, href }));
      }
    }
    if (isDict(m.alternates.types)) {
      for (const [type, target] of Object.entries(m.alternates.types)) {
        const href = resolveUrl(isDict(target) ? target.url : target, base);
        if (href) out.push(link({ rel: "alternate", type, href }));
      }
    }
  }
  if (m.manifest) out.push(link({ rel: "manifest", href: asString(m.manifest) }));
  if (isDict(m.icons)) {
    for (const entry of iconEntries(m.icons.icon, "icon")) out.push(link(entry));
    for (const entry of iconEntries(m.icons.shortcut, "shortcut icon")) out.push(link(entry));
    for (const entry of iconEntries(m.icons.apple, "apple-touch-icon")) out.push(link(entry));
    for (const entry of iconEntries(m.icons.other, "icon")) out.push(link(entry));
  } else if (Array.isArray(m.icons)) {
    for (const entry of iconEntries(m.icons, "icon")) out.push(link(entry));
  }
  if (isDict(m.openGraph)) {
    const og = m.openGraph;
    const ogTitle = isDict(og.title) ? absoluteTitle(og.title as TitleSpec) : asString(og.title);
    if (ogTitle) out.push(meta({ property: "og:title", content: ogTitle }));
    if (og.description) out.push(meta({ property: "og:description", content: asString(og.description) }));
    const ogUrl = resolveUrl(og.url, base);
    if (ogUrl) out.push(meta({ property: "og:url", content: ogUrl }));
    if (og.siteName) out.push(meta({ property: "og:site_name", content: asString(og.siteName) }));
    if (og.locale) out.push(meta({ property: "og:locale", content: asString(og.locale) }));
    for (const img of imageEntries(og.images, base)) {
      out.push(meta({ property: "og:image", content: img.url }));
      if (img.width) out.push(meta({ property: "og:image:width", content: img.width }));
      if (img.height) out.push(meta({ property: "og:image:height", content: img.height }));
      if (img.alt) out.push(meta({ property: "og:image:alt", content: img.alt }));
      if (img.type) out.push(meta({ property: "og:image:type", content: img.type }));
    }
    if (og.type) out.push(meta({ property: "og:type", content: asString(og.type) }));
    if (og.publishedTime) out.push(meta({ property: "article:published_time", content: asString(og.publishedTime) }));
    if (og.modifiedTime) out.push(meta({ property: "article:modified_time", content: asString(og.modifiedTime) }));
    for (const author of asList(og.authors as string | ReadonlyArray<string> | undefined)) out.push(meta({ property: "article:author", content: author }));
    for (const tag of asList(og.tags as string | ReadonlyArray<string> | undefined)) out.push(meta({ property: "article:tag", content: tag }));
  }
  if (isDict(m.twitter)) {
    const tw = m.twitter;
    const card = asString(tw.card) ?? (tw.images !== undefined ? "summary_large_image" : "summary");
    out.push(meta({ name: "twitter:card", content: card }));
    if (tw.site) out.push(meta({ name: "twitter:site", content: asString(tw.site) }));
    if (tw.siteId) out.push(meta({ name: "twitter:site:id", content: asString(tw.siteId) }));
    if (tw.creator) out.push(meta({ name: "twitter:creator", content: asString(tw.creator) }));
    if (tw.creatorId) out.push(meta({ name: "twitter:creator:id", content: asString(tw.creatorId) }));
    const twTitle = isDict(tw.title) ? absoluteTitle(tw.title as TitleSpec) : asString(tw.title);
    if (twTitle) out.push(meta({ name: "twitter:title", content: twTitle }));
    if (tw.description) out.push(meta({ name: "twitter:description", content: asString(tw.description) }));
    for (const img of imageEntries(tw.images, base)) {
      out.push(meta({ name: "twitter:image", content: img.url }));
      if (img.alt) out.push(meta({ name: "twitter:image:alt", content: img.alt }));
    }
  }
  if (isDict(m.verification)) {
    const names: Record<string, string> = { google: "google-site-verification", yahoo: "y_key", yandex: "yandex-verification", me: "me" };
    for (const [key, value] of Object.entries(m.verification)) {
      if (key === "other" && isDict(value)) {
        for (const [name, v] of Object.entries(value)) for (const item of asList(v as string | ReadonlyArray<string>)) out.push(meta({ name, content: asString(item) }));
        continue;
      }
      for (const item of asList(value as string | ReadonlyArray<string>)) out.push(meta({ name: names[key] ?? key, content: asString(item) }));
    }
  }
  if (isDict(m.appleWebApp) || m.appleWebApp === true) {
    const app = isDict(m.appleWebApp) ? m.appleWebApp : {};
    if (app.capable !== false) out.push(meta({ name: "mobile-web-app-capable", content: "yes" }));
    if (app.title) out.push(meta({ name: "apple-mobile-web-app-title", content: asString(app.title) }));
    if (app.statusBarStyle) out.push(meta({ name: "apple-mobile-web-app-status-bar-style", content: asString(app.statusBarStyle) }));
    for (const image of asList(app.startupImage as unknown as ReadonlyArray<unknown>)) {
      const href = isDict(image) ? asString(image.url) : asString(image);
      if (href) out.push(link({ rel: "apple-touch-startup-image", href, media: isDict(image) ? asString(image.media) : undefined }));
    }
  }
  if (isDict(m.formatDetection)) {
    const parts: string[] = [];
    for (const key of ["telephone", "date", "address", "email", "url"]) if (m.formatDetection[key] === false) parts.push(`${key}=no`);
    if (parts.length) out.push(meta({ name: "format-detection", content: parts.join(", ") }));
  }
  if (m.category) out.push(meta({ name: "category", content: m.category }));
  if (m.other) {
    for (const [name, value] of Object.entries(m.other)) for (const item of asList(value)) out.push(meta({ name, content: String(item) }));
  }
  return out;
}

const ESCAPE: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

/** Escapes text for HTML attribute and text positions. */
export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (ch) => ESCAPE[ch] ?? ch);
}

const ATTR_NAMES: Record<string, string> = { charSet: "charset", hrefLang: "hreflang" };

/** Renders the `<head>` tags of a resolved metadata + viewport pair as an HTML string. */
export function renderHeadTags(metadata: Metadata, viewport?: Viewport): string {
  return headElements(metadata, viewport)
    .map((el) => {
      const attrs = Object.entries(el.attrs)
        .map(([key, value]) => ` ${ATTR_NAMES[key] ?? key}="${escapeHtml(value)}"`)
        .join("");
      if (el.tag === "title") return `<title>${escapeHtml(el.children ?? "")}</title>`;
      return `<${el.tag}${attrs}>`;
    })
    .join("\n");
}
