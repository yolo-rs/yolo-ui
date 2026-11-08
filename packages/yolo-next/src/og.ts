// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/og: next/og compatible ImageResponse rendered by satori (SVG) and @resvg/resvg-js (PNG).

import type { ReactElement, ReactNode } from "react";

/** Font weights accepted by satori. */
export type OgFontWeight = 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900;

/** Font descriptor accepted by `ImageResponse` (satori `Font`). */
export interface ImageResponseFont {
  readonly name: string;
  readonly data: ArrayBuffer | Uint8Array;
  readonly weight?: OgFontWeight;
  readonly style?: "normal" | "italic";
  readonly lang?: string;
}

/** Options of `ImageResponse` (`next/og` `ImageResponseOptions`). */
export interface ImageResponseOptions {
  readonly width?: number;
  readonly height?: number;
  readonly fonts?: ReadonlyArray<ImageResponseFont>;
  readonly emoji?: "twemoji" | "blobmoji" | "noto" | "openmoji" | "fluent" | "fluentFlat";
  readonly debug?: boolean;
  readonly status?: number;
  readonly statusText?: string;
  readonly headers?: HeadersInit;
}

/** Default width of an Open Graph image. */
export const OG_DEFAULT_WIDTH = 1200;
/** Default height of an Open Graph image. */
export const OG_DEFAULT_HEIGHT = 630;
/** Default `cache-control` of `ImageResponse` (Next.js value). */
export const OG_CACHE_CONTROL = "public, immutable, no-transform, max-age=31536000";

const SATORI_PACKAGE = "satori";
const RESVG_PACKAGE = "@resvg/resvg-js";
const DEFAULT_FONT_CANDIDATES: ReadonlyArray<string> = [
  "C:/Windows/Fonts/arial.ttf",
  "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
  "/usr/share/fonts/TTF/DejaVuSans.ttf",
  "/usr/share/fonts/dejavu/DejaVuSans.ttf",
  "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
  "/System/Library/Fonts/Supplemental/Arial.ttf",
];

let satoriResolvable: boolean | undefined;
let resvgResolvable: boolean | undefined;
let defaultFontPromise: Promise<ImageResponseFont | undefined> | undefined;

function canResolve(specifier: string): boolean {
  try {
    Bun.resolveSync(specifier, import.meta.dir);
    return true;
  } catch {
    return false;
  }
}

/** True when `satori` can be imported in this runtime. */
export function satoriAvailable(): boolean {
  satoriResolvable ??= canResolve(SATORI_PACKAGE);
  return satoriResolvable;
}

/** True when `@resvg/resvg-js` can be imported in this runtime. */
export function resvgAvailable(): boolean {
  resvgResolvable ??= canResolve(RESVG_PACKAGE);
  return resvgResolvable;
}

/** Loads the default font: `YOLO_OG_FONT` when set, else the first common system sans-serif found. */
export async function defaultOgFont(): Promise<ImageResponseFont | undefined> {
  defaultFontPromise ??= (async () => {
    const candidates = [Bun.env.YOLO_OG_FONT, ...DEFAULT_FONT_CANDIDATES].filter(
      (path): path is string => typeof path === "string" && path.length > 0,
    );
    for (const path of candidates) {
      try {
        const file = Bun.file(path);
        if (await file.exists()) {
          return { name: "sans-serif", data: await file.arrayBuffer(), weight: 400, style: "normal" };
        }
      } catch {
        // Unreadable candidate: try the next one.
      }
    }
    return undefined;
  })();
  return defaultFontPromise;
}

function collectText(node: ReactNode, out: string[]): void {
  if (node === null || node === undefined || typeof node === "boolean") {
    return;
  }
  if (typeof node === "string" || typeof node === "number") {
    out.push(String(node));
    return;
  }
  if (Array.isArray(node)) {
    for (const child of node) {
      collectText(child as ReactNode, out);
    }
    return;
  }
  if (typeof node === "object" && "props" in node) {
    const props = (node as { props?: { children?: ReactNode } }).props;
    collectText(props?.children, out);
  }
}

function escapeXml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function rootStyle(element: ReactElement): { background: string; color: string; fontSize: number } {
  const style = (element.props as { style?: Record<string, unknown> } | null)?.style ?? {};
  const background = style.background ?? style.backgroundColor;
  const color = style.color;
  const fontSize = style.fontSize;
  return {
    background: typeof background === "string" ? background : "#000000",
    color: typeof color === "string" ? color : "#ffffff",
    fontSize: typeof fontSize === "number" ? fontSize : 64,
  };
}

/** Plain `<text>` SVG (system fonts) used when satori or an embeddable font is unavailable. */
export function fallbackOgSvg(element: ReactElement, width: number, height: number): string {
  const texts: string[] = [];
  collectText(element, texts);
  const style = rootStyle(element);
  const label = escapeXml(texts.join(" ").trim());
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">` +
    `<rect width="100%" height="100%" fill="${escapeXml(style.background)}"/>` +
    `<text x="50%" y="50%" dominant-baseline="middle" text-anchor="middle" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="${style.fontSize}" fill="${escapeXml(style.color)}">${label}</text>` +
    `</svg>`
  );
}

function toArrayBuffer(data: ArrayBuffer | Uint8Array): ArrayBuffer {
  if (data instanceof ArrayBuffer) {
    return data;
  }
  return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
}

/** Renders `element` to SVG with satori; falls back to `fallbackOgSvg` when satori or fonts are unavailable. */
export async function renderOgSvg(
  element: ReactElement,
  options: ImageResponseOptions = {},
): Promise<{ svg: string; viaSatori: boolean }> {
  const width = options.width ?? OG_DEFAULT_WIDTH;
  const height = options.height ?? OG_DEFAULT_HEIGHT;
  if (satoriAvailable()) {
    try {
      const { default: satori } = await import("satori");
      let fonts = [...(options.fonts ?? [])];
      if (fonts.length === 0) {
        const fallback = await defaultOgFont();
        if (fallback) {
          fonts = [fallback];
        }
      }
      const svg = await satori(element, {
        width,
        height,
        debug: options.debug === true,
        embedFont: true,
        fonts: fonts.map((font) => ({
          name: font.name,
          data: toArrayBuffer(font.data),
          weight: font.weight,
          style: font.style,
          lang: font.lang,
        })),
      });
      return { svg, viaSatori: true };
    } catch {
      // Missing fonts or unsupported element: use the system-font fallback below.
    }
  }
  return { svg: fallbackOgSvg(element, width, height), viaSatori: false };
}

/** Rasterizes an SVG to PNG with resvg; `undefined` when resvg is unavailable or fails. */
export async function renderOgPng(svg: string, width: number): Promise<Uint8Array | undefined> {
  if (!resvgAvailable()) {
    return undefined;
  }
  try {
    const { Resvg } = await import("@resvg/resvg-js");
    const renderer = new Resvg(svg, { fitTo: { mode: "width", value: width } });
    return new Uint8Array(renderer.render().asPng());
  } catch {
    resvgResolvable = false;
    return undefined;
  }
}

/** `next/og` ImageResponse: a `Response` streaming the rendered PNG (SVG when resvg is unavailable). */
export class ImageResponse extends Response {
  constructor(element: ReactElement, options: ImageResponseOptions = {}) {
    const width = options.width ?? OG_DEFAULT_WIDTH;
    const height = options.height ?? OG_DEFAULT_HEIGHT;
    const png = resvgAvailable();
    const headers = new Headers(options.headers);
    if (!headers.has("content-type")) {
      headers.set("content-type", png ? "image/png" : "image/svg+xml");
    }
    if (!headers.has("cache-control")) {
      headers.set("cache-control", Bun.env.NODE_ENV === "development" ? "no-cache, no-store" : OG_CACHE_CONTROL);
    }
    const body = new ReadableStream<Uint8Array>({
      async start(controller) {
        try {
          const { svg } = await renderOgSvg(element, { ...options, width, height });
          const bytes = png ? await renderOgPng(svg, width) : undefined;
          controller.enqueue(bytes ?? new TextEncoder().encode(svg));
          controller.close();
        } catch (error) {
          controller.error(error);
        }
      },
    });
    super(body, { status: options.status ?? 200, statusText: options.statusText, headers });
  }
}
