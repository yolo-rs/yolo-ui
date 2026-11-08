// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/image: next/image compatible props builder, loaders, Image component and optimizer route.

import {
  createElement,
  useCallback,
  useState,
  type CSSProperties,
  type ImgHTMLAttributes,
  type ReactElement,
  type Ref,
  type SyntheticEvent,
} from "react";
import * as ReactDOM from "react-dom";
import { mkdir, readdir, unlink } from "node:fs/promises";
import { basename, extname, resolve, sep } from "node:path";
import {
  IMAGES_DEFAULTS,
  type ImageLoader,
  type ImageLoaderKind,
  type ImageLoaderProps,
  type ImageOptimizerPolicy,
  type LocalPattern,
  type RemotePattern,
} from "./types.ts";

/** Default optimizer route of Yolo (Next's `/_next/image` is kept as a compat route). */
export const DEFAULT_IMAGE_ROUTE = "/_yolo/image";
const DUMMY_ORIGIN = "http://yolo.local";
const DEFAULT_QUALITY = 75;
const MAX_URL_LENGTH = 3072;
const UPSTREAM_TIMEOUT_MS = 7000;
const TRANSFORMABLE_FORMATS: ReadonlySet<string> = new Set(["image/avif", "image/webp", "image/jpeg", "image/png"]);
const EXTENSIONS: Readonly<Record<string, string>> = {
  "image/avif": "avif",
  "image/webp": "webp",
  "image/png": "png",
  "image/jpeg": "jpeg",
  "image/gif": "gif",
  "image/svg+xml": "svg",
  "image/x-icon": "ico",
  "image/vnd.microsoft.icon": "ico",
  "image/bmp": "bmp",
  "image/tiff": "tiff",
};
const EXTENSION_TYPES: Readonly<Record<string, string>> = {
  avif: "image/avif",
  webp: "image/webp",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  svg: "image/svg+xml",
  ico: "image/x-icon",
  bmp: "image/bmp",
  tif: "image/tiff",
  tiff: "image/tiff",
};

function isDev(): boolean {
  return Bun.env.NODE_ENV !== "production";
}

function sha256(input: string | Uint8Array): string {
  return new Bun.CryptoHasher("sha256").update(input).digest("hex");
}

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/** Builds an `ImageOptimizerPolicy` from Next's defaults, overridden by `overrides`. */
export function createImagePolicy(overrides: Partial<ImageOptimizerPolicy> = {}): ImageOptimizerPolicy {
  return {
    route: DEFAULT_IMAGE_ROUTE,
    compatRoutes: [IMAGES_DEFAULTS.path],
    loader: "default",
    formats: [...IMAGES_DEFAULTS.formats],
    deviceSizes: [...IMAGES_DEFAULTS.deviceSizes],
    imageSizes: [...IMAGES_DEFAULTS.imageSizes],
    qualities: [...IMAGES_DEFAULTS.qualities],
    minimumCacheTtl: IMAGES_DEFAULTS.minimumCacheTTL,
    cacheControl: "public, max-age={max_age}, must-revalidate",
    allowSvg: IMAGES_DEFAULTS.dangerouslyAllowSVG,
    allowLocalIp: IMAGES_DEFAULTS.dangerouslyAllowLocalIP,
    contentSecurityPolicy: IMAGES_DEFAULTS.contentSecurityPolicy,
    contentDispositionType: IMAGES_DEFAULTS.contentDispositionType,
    remotePatterns: [],
    maximumRedirects: IMAGES_DEFAULTS.maximumRedirects,
    maximumResponseBody: IMAGES_DEFAULTS.maximumResponseBody,
    unoptimized: IMAGES_DEFAULTS.unoptimized,
    ...overrides,
  };
}

/** Runtime image settings shared by `Image` and `getImageProps` when no explicit policy is passed. */
export interface ImageRuntimeOptions {
  readonly deploymentId?: string;
  readonly loader?: ImageLoader;
}

let activePolicy: ImageOptimizerPolicy = createImagePolicy();
let activeRuntime: ImageRuntimeOptions = {};

/** Installs the policy (and optional deployment id / custom loader) used by `Image` without an explicit policy. */
export function configureImages(policy: ImageOptimizerPolicy, runtime: ImageRuntimeOptions = {}): void {
  activePolicy = policy;
  activeRuntime = runtime;
}

/** Policy currently installed by `configureImages` (Next defaults otherwise). */
export function getImagePolicy(): ImageOptimizerPolicy {
  return activePolicy;
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

function hostnameMatches(pattern: string, hostname: string): boolean {
  const expected = pattern.toLowerCase();
  const actual = hostname.toLowerCase();
  if (expected.startsWith("**.")) {
    const suffix = expected.slice(3);
    return actual.endsWith(`.${suffix}`) && actual.length > suffix.length + 1;
  }
  if (expected.startsWith("*.")) {
    const suffix = expected.slice(2);
    if (!actual.endsWith(`.${suffix}`)) {
      return false;
    }
    const prefix = actual.slice(0, actual.length - suffix.length - 1);
    return prefix.length > 0 && !prefix.includes(".");
  }
  return expected === actual;
}

function globToRegExp(glob: string): RegExp {
  let source = "";
  for (let index = 0; index < glob.length; index += 1) {
    const char = glob[index] ?? "";
    if (char === "*") {
      if (glob[index + 1] === "*") {
        source += ".*";
        index += 1;
      } else {
        source += "[^/]*";
      }
    } else if (char === "?") {
      source += "[^/]";
    } else {
      source += char.replace(/[.+^${}()|[\]\\]/g, "\\$&");
    }
  }
  return new RegExp(`^${source}$`);
}

/** True when `url` is allowed by `pattern` (Next.js `remotePatterns` semantics). */
export function remotePatternMatches(pattern: RemotePattern, url: string | URL): boolean {
  let parsed: URL;
  try {
    parsed = typeof url === "string" ? new URL(url) : url;
  } catch {
    return false;
  }
  if (pattern.protocol !== undefined && `${pattern.protocol}:` !== parsed.protocol) {
    return false;
  }
  if (pattern.port !== undefined && pattern.port !== parsed.port) {
    return false;
  }
  if (!hostnameMatches(pattern.hostname, parsed.hostname)) {
    return false;
  }
  if (pattern.search !== undefined && pattern.search !== parsed.search) {
    return false;
  }
  if (pattern.pathname !== undefined && !globToRegExp(pattern.pathname).test(parsed.pathname)) {
    return false;
  }
  return true;
}

/** True when a local `src` (`/path?query`) is allowed by `pattern` (Next.js `localPatterns` semantics). */
export function localPatternMatches(pattern: LocalPattern, src: string): boolean {
  const parsed = new URL(src, DUMMY_ORIGIN);
  if (pattern.search !== undefined && pattern.search !== parsed.search) {
    return false;
  }
  if (pattern.pathname !== undefined && !globToRegExp(pattern.pathname).test(parsed.pathname)) {
    return false;
  }
  return true;
}

function isAbsoluteUrl(src: string): boolean {
  return /^https?:\/\//i.test(src);
}

/** True when `src` is allowed by the policy (remote patterns / domains for absolute URLs, local patterns otherwise). */
export function matchesImagePolicy(policy: ImageOptimizerPolicy, src: string): boolean {
  if (isAbsoluteUrl(src)) {
    const domains = (policy as ImageOptimizerPolicy & { readonly domains?: ReadonlyArray<string> }).domains ?? [];
    let parsed: URL;
    try {
      parsed = new URL(src);
    } catch {
      return false;
    }
    return (
      domains.some((domain) => domain.toLowerCase() === parsed.hostname.toLowerCase()) ||
      policy.remotePatterns.some((pattern) => remotePatternMatches(pattern, parsed))
    );
  }
  if (!src.startsWith("/")) {
    return false;
  }
  const patterns = policy.localPatterns;
  return patterns === undefined || patterns.some((pattern) => localPatternMatches(pattern, src));
}

// ---------------------------------------------------------------------------
// Loaders
// ---------------------------------------------------------------------------

/** Options of the default loader (optimizer route and deployment id for skew protection). */
export interface DefaultLoaderOptions {
  readonly route?: string;
  readonly deploymentId?: string;
}

/** Options of the CDN loaders that need a base origin (Cloudflare). */
export interface CdnLoaderOptions {
  readonly origin?: string;
}

function withQuery(src: string, mutate: (params: URLSearchParams) => void): string {
  const absolute = isAbsoluteUrl(src);
  const url = new URL(src, DUMMY_ORIGIN);
  mutate(url.searchParams);
  return absolute ? url.toString() : `${url.pathname}${url.search}${url.hash}`;
}

/** Next.js default loader: `${route}?url=&w=&q=` (`&dpl=` appended for app-relative sources). */
export function defaultLoader({ src, width, quality }: ImageLoaderProps, options: DefaultLoaderOptions = {}): string {
  const route = options.route ?? DEFAULT_IMAGE_ROUTE;
  let url = `${route}?url=${encodeURIComponent(src)}&w=${width}&q=${quality ?? DEFAULT_QUALITY}`;
  if (options.deploymentId && src.startsWith("/")) {
    url += `&dpl=${options.deploymentId}`;
  }
  return url;
}

/** Cloudflare Images loader: `/cdn-cgi/image/width=,quality=,format=auto/<src>`. */
export function cloudflareLoader({ src, width, quality }: ImageLoaderProps, options: CdnLoaderOptions = {}): string {
  const path = src.startsWith("/") ? src.slice(1) : src;
  return `${options.origin ?? ""}/cdn-cgi/image/width=${width},quality=${quality ?? DEFAULT_QUALITY},format=auto/${path}`;
}

/** Cloudinary loader: `f_auto,c_limit,w_<w>,q_<q|auto>` inserted after the origin. */
export function cloudinaryLoader({ src, width, quality }: ImageLoaderProps): string {
  const params = `f_auto,c_limit,w_${width},q_${quality ?? "auto"}`;
  if (isAbsoluteUrl(src)) {
    const url = new URL(src);
    return `${url.origin}/${params}${url.pathname}${url.search}`;
  }
  return `/${params}${src.startsWith("/") ? src : `/${src}`}`;
}

/** Imgix loader: `?auto=format&fit=max&w=&q=` merged with the existing query. */
export function imgixLoader({ src, width, quality }: ImageLoaderProps): string {
  return withQuery(src, (params) => {
    params.set("auto", params.getAll("auto").join(",") || "format");
    params.set("fit", params.get("fit") || "max");
    params.set("w", params.get("w") || String(width));
    if (quality !== undefined) {
      params.set("q", String(quality));
    }
  });
}

/** Akamai Image Manager loader: `?imwidth=`. */
export function akamaiLoader({ src, width }: ImageLoaderProps): string {
  return withQuery(src, (params) => params.set("imwidth", String(width)));
}

/** Fastly Image Optimizer loader: `?auto=webp&width=&quality=`. */
export function fastlyLoader({ src, width, quality }: ImageLoaderProps): string {
  return withQuery(src, (params) => {
    params.set("auto", "webp");
    params.set("width", String(width));
    params.set("quality", String(quality ?? DEFAULT_QUALITY));
  });
}

/** Supabase Storage transformation loader: `?width=&quality=`. */
export function supabaseLoader({ src, width, quality }: ImageLoaderProps): string {
  return withQuery(src, (params) => {
    params.set("width", String(width));
    params.set("quality", String(quality ?? DEFAULT_QUALITY));
  });
}

/** CloudFront (Serverless Image Handler) loader: `?format=auto&width=&quality=`. */
export function cloudfrontLoader({ src, width, quality }: ImageLoaderProps): string {
  return withQuery(src, (params) => {
    params.set("format", "auto");
    params.set("width", String(width));
    params.set("quality", String(quality ?? DEFAULT_QUALITY));
  });
}

/** Resolves a built-in loader by kind (`custom` must go through `loadCustomLoader`). */
export function loaderFor(kind: ImageLoaderKind, options: DefaultLoaderOptions & CdnLoaderOptions = {}): ImageLoader {
  switch (kind) {
    case "default":
      return (props) => defaultLoader(props, options);
    case "cloudflare":
      return (props) => cloudflareLoader(props, options);
    case "cloudinary":
      return cloudinaryLoader;
    case "imgix":
      return imgixLoader;
    case "akamai":
      return akamaiLoader;
    case "fastly":
      return fastlyLoader;
    case "supabase":
      return supabaseLoader;
    case "cloudfront":
      return cloudfrontLoader;
    case "unoptimized":
      return ({ src }) => src;
    case "custom":
      throw new Error('images.loader "custom" requires loadCustomLoader(images.loaderFile)');
  }
}

/** Imports `images.loaderFile` and returns its default export. */
export async function loadCustomLoader(file: string): Promise<ImageLoader> {
  const module = (await import(file)) as { default?: unknown };
  if (typeof module.default !== "function") {
    throw new Error(`Image loader file "${file}" must default-export a function`);
  }
  return module.default as ImageLoader;
}

// ---------------------------------------------------------------------------
// Props (get-img-props.ts port)
// ---------------------------------------------------------------------------

/** Nearest allowed quality (ties go lower); 75 when allowed, else the first configured quality, when omitted. */
export function coerceQuality(policy: ImageOptimizerPolicy, quality?: number | string): number {
  const allowed = policy.qualities.length > 0 ? [...policy.qualities].sort((a, b) => a - b) : [DEFAULT_QUALITY];
  const first = allowed[0] ?? DEFAULT_QUALITY;
  const requested = typeof quality === "string" ? Number.parseInt(quality, 10) : quality;
  if (requested === undefined || Number.isNaN(requested)) {
    return allowed.includes(DEFAULT_QUALITY) ? DEFAULT_QUALITY : first;
  }
  let best = first;
  let bestDistance = Math.abs(first - requested);
  for (const candidate of allowed) {
    const distance = Math.abs(candidate - requested);
    if (distance < bestDistance) {
      best = candidate;
      bestDistance = distance;
    }
  }
  return best;
}

/** Static image import (`import hero from "./hero.png"`). */
export interface StaticImageData {
  readonly src: string;
  readonly height: number;
  readonly width: number;
  readonly blurDataURL?: string;
  readonly blurWidth?: number;
  readonly blurHeight?: number;
}

/** Static import shapes accepted by `src`. */
export type StaticImport = StaticImageData | { readonly default: StaticImageData };

/** Legacy `next/legacy/image` layout values (mapped to `fill` / `sizes` / styles). */
export type LegacyLayout = "fill" | "fixed" | "intrinsic" | "responsive";

/** Props of `Image` (`next/image` `ImageProps`). */
export interface ImageProps
  extends Omit<ImgHTMLAttributes<HTMLImageElement>, "src" | "srcSet" | "ref" | "alt" | "width" | "height" | "loading"> {
  readonly src: string | StaticImport;
  readonly alt: string;
  readonly width?: number | `${number}`;
  readonly height?: number | `${number}`;
  readonly fill?: boolean;
  readonly loader?: ImageLoader;
  readonly quality?: number | `${number}`;
  readonly priority?: boolean;
  readonly preload?: boolean;
  readonly loading?: "eager" | "lazy";
  readonly placeholder?: "blur" | "empty" | `data:image/${string}`;
  readonly blurDataURL?: string;
  readonly unoptimized?: boolean;
  readonly overrideSrc?: string;
  readonly onLoadingComplete?: (img: HTMLImageElement) => void;
  readonly layout?: LegacyLayout;
  readonly objectFit?: CSSProperties["objectFit"];
  readonly objectPosition?: CSSProperties["objectPosition"];
  readonly lazyBoundary?: string;
  readonly lazyRoot?: unknown;
  readonly ref?: Ref<HTMLImageElement>;
}

/** Attributes of the `<img>` produced by `getImageProps`. */
export interface ResolvedImageProps extends Omit<ImgHTMLAttributes<HTMLImageElement>, "src" | "width" | "height"> {
  readonly src: string;
  readonly srcSet?: string;
  readonly sizes?: string;
  readonly width?: number;
  readonly height?: number;
  readonly loading: "eager" | "lazy";
  readonly decoding: "async" | "sync" | "auto";
  readonly fetchPriority?: "high" | "low" | "auto";
  readonly style: CSSProperties;
  readonly alt: string;
  readonly "data-nimg": "1" | "fill";
}

/** Non-attribute information derived while resolving image props. */
export interface ResolvedImageMeta {
  readonly unoptimized: boolean;
  readonly priority: boolean;
  readonly preload: boolean;
  readonly fill: boolean;
  readonly placeholder: string;
  readonly blurDataURL?: string;
}

/** Internal rendering state passed by the `Image` component. */
export interface ImageRenderState {
  readonly blurComplete?: boolean;
  readonly showAltText?: boolean;
}

/** Result of `getImageProps`. */
export interface GetImagePropsResult {
  readonly props: ResolvedImageProps;
  readonly meta: ResolvedImageMeta;
}

interface SizeConfig {
  readonly deviceSizes: ReadonlyArray<number>;
  readonly allSizes: ReadonlyArray<number>;
}

function sizeConfig(policy: ImageOptimizerPolicy): SizeConfig {
  const deviceSizes = [...policy.deviceSizes].sort((a, b) => a - b);
  const allSizes = [...new Set([...policy.deviceSizes, ...policy.imageSizes])].sort((a, b) => a - b);
  return { deviceSizes, allSizes };
}

function getWidths(
  { deviceSizes, allSizes }: SizeConfig,
  width: number | undefined,
  sizes: string | undefined,
): { widths: ReadonlyArray<number>; kind: "w" | "x" } {
  if (sizes) {
    const viewportWidthRe = /(^|\s)(1?\d?\d)vw/g;
    const percentSizes: number[] = [];
    for (let match = viewportWidthRe.exec(sizes); match; match = viewportWidthRe.exec(sizes)) {
      percentSizes.push(Number.parseInt(match[2] ?? "100", 10));
    }
    if (percentSizes.length > 0) {
      const smallestRatio = Math.min(...percentSizes) * 0.01;
      const smallestDevice = deviceSizes[0] ?? 0;
      return { widths: allSizes.filter((size) => size >= smallestDevice * smallestRatio), kind: "w" };
    }
    return { widths: allSizes, kind: "w" };
  }
  if (typeof width !== "number") {
    return { widths: deviceSizes, kind: "w" };
  }
  const last = allSizes[allSizes.length - 1] ?? width;
  const widths = [...new Set([width, width * 2].map((target) => allSizes.find((size) => size >= target) ?? last))];
  return { widths, kind: "x" };
}

function generateImgAttrs(input: {
  readonly config: SizeConfig;
  readonly src: string;
  readonly unoptimized: boolean;
  readonly width: number | undefined;
  readonly quality: number;
  readonly sizes: string | undefined;
  readonly loader: ImageLoader;
}): { src: string; srcSet: string | undefined; sizes: string | undefined } {
  const { config, src, unoptimized, width, quality, sizes, loader } = input;
  if (unoptimized) {
    return { src, srcSet: undefined, sizes: undefined };
  }
  const { widths, kind } = getWidths(config, width, sizes);
  const last = widths[widths.length - 1] ?? width ?? 0;
  return {
    sizes: !sizes && kind === "w" ? "100vw" : sizes,
    srcSet: widths
      .map((candidate, index) => `${loader({ src, quality, width: candidate })} ${kind === "w" ? candidate : index + 1}${kind}`)
      .join(", "),
    src: loader({ src, quality, width: last }),
  };
}

/** Blur placeholder SVG (`feGaussianBlur` over the tiny `blurDataURL`), percent-encoded for a CSS `url()`. */
export function getImageBlurSvg(input: {
  readonly widthInt?: number;
  readonly heightInt?: number;
  readonly blurWidth?: number;
  readonly blurHeight?: number;
  readonly blurDataURL: string;
  readonly objectFit?: string;
}): string {
  const std = 20;
  const svgWidth = input.blurWidth ? input.blurWidth * 40 : input.widthInt;
  const svgHeight = input.blurHeight ? input.blurHeight * 40 : input.heightInt;
  const viewBox = svgWidth && svgHeight ? `viewBox='0 0 ${svgWidth} ${svgHeight}'` : "";
  const preserveAspectRatio = viewBox
    ? "none"
    : input.objectFit === "contain"
      ? "xMidYMid"
      : input.objectFit === "cover"
        ? "xMidYMid slice"
        : "none";
  return `%3Csvg xmlns='http://www.w3.org/2000/svg' ${viewBox}%3E%3Cfilter id='b' color-interpolation-filters='sRGB'%3E%3CfeGaussianBlur stdDeviation='${std}'/%3E%3CfeColorMatrix values='1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 100 -1' result='s'/%3E%3CfeFlood x='0' y='0' width='100%25' height='100%25'/%3E%3CfeComposite operator='out' in='s'/%3E%3CfeComposite in2='SourceGraphic'/%3E%3CfeGaussianBlur stdDeviation='${std}'/%3E%3C/filter%3E%3Cimage width='100%25' height='100%25' x='0' y='0' preserveAspectRatio='${preserveAspectRatio}' style='filter: url(%23b);' href='${input.blurDataURL}'/%3E%3C/svg%3E`;
}

function toInt(value: number | string | undefined): number | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (typeof value === "number") {
    return value;
  }
  const parsed = Number.parseInt(value, 10);
  return Number.isNaN(parsed) ? Number.NaN : parsed;
}

function imageError(src: string, message: string): Error {
  return new Error(`Image with src "${src}" ${message}`);
}

/** Resolves `next/image` props into `<img>` attributes (srcset, sizes, lazy loading, placeholder styles). */
export function getImageProps(
  props: ImageProps,
  policy: ImageOptimizerPolicy = activePolicy,
  options: ImageRuntimeOptions & ImageRenderState = {},
): GetImagePropsResult {
  const {
    src: srcProp,
    alt,
    sizes: sizesProp,
    unoptimized: unoptimizedProp = false,
    priority = false,
    preload = false,
    loading,
    quality,
    width,
    height,
    fill: fillProp = false,
    style: styleProp,
    overrideSrc,
    placeholder = "empty",
    blurDataURL: blurDataURLProp,
    fetchPriority: fetchPriorityProp,
    decoding = "async",
    loader: loaderProp,
    layout,
    objectFit,
    objectPosition,
    lazyBoundary: _lazyBoundary,
    lazyRoot: _lazyRoot,
    onLoadingComplete: _onLoadingComplete,
    ...rest
  } = props;

  let fill = fillProp;
  let sizes = sizesProp;
  let style = styleProp;
  if (layout) {
    if (layout === "fill") {
      fill = true;
    }
    const layoutStyles: Partial<Record<LegacyLayout, CSSProperties>> = {
      intrinsic: { maxWidth: "100%", height: "auto" },
      responsive: { width: "100%", height: "auto" },
    };
    const layoutSizes: Partial<Record<LegacyLayout, string>> = { responsive: "100vw", fill: "100vw" };
    style = { ...layoutStyles[layout], ...style };
    sizes ??= layoutSizes[layout];
  }

  let unoptimized = unoptimizedProp || policy.unoptimized;
  let widthInt = toInt(width);
  let heightInt = toInt(height);
  let blurDataURL = blurDataURLProp;
  let blurWidth: number | undefined;
  let blurHeight: number | undefined;
  let src: string;
  if (typeof srcProp === "object" && srcProp !== null) {
    const data = "default" in srcProp ? srcProp.default : srcProp;
    if (!data.src) {
      throw new Error(
        `An object should only be passed to the image component src parameter if it comes from a static image import. It must include src. Received ${JSON.stringify(data)}`,
      );
    }
    if (!data.height || !data.width) {
      throw new Error(
        `An object should only be passed to the image component src parameter if it comes from a static image import. It must include height and width. Received ${JSON.stringify(data)}`,
      );
    }
    blurWidth = data.blurWidth;
    blurHeight = data.blurHeight;
    blurDataURL ??= data.blurDataURL;
    src = data.src;
    if (!fill) {
      if (widthInt === undefined && heightInt === undefined) {
        widthInt = data.width;
        heightInt = data.height;
      } else if (widthInt !== undefined && heightInt === undefined) {
        heightInt = Math.round((data.height * widthInt) / data.width);
      } else if (widthInt === undefined && heightInt !== undefined) {
        widthInt = Math.round((data.width * heightInt) / data.height);
      }
    }
  } else {
    src = typeof srcProp === "string" ? srcProp : "";
  }

  let isLazy = !priority && !preload && (loading === "lazy" || loading === undefined);
  if (src.startsWith("data:") || src.startsWith("blob:")) {
    unoptimized = true;
    isLazy = false;
  }
  if (!policy.allowSvg && (src.split("?", 1)[0] ?? "").endsWith(".svg")) {
    unoptimized = true;
  }
  const qualityInt = coerceQuality(policy, quality);

  if (isDev()) {
    if (alt === undefined) {
      throw imageError(src, 'is missing required "alt" property. Please add Alternative Text to describe the image for screen readers and search engines.');
    }
    if (!src) {
      unoptimized = true;
    } else {
      if (src.startsWith("//")) {
        throw new Error(
          `Failed to parse src "${src}" on \`next/image\`, protocol-relative URL (//) must be changed to an absolute URL (http:// or https://)`,
        );
      }
      if (!src.startsWith("/") && !src.startsWith("data:") && !src.startsWith("blob:")) {
        try {
          new URL(src);
        } catch {
          throw new Error(
            `Failed to parse src "${src}" on \`next/image\`, if using relative image it must start with a leading slash "/" or be an absolute URL (http:// or https://)`,
          );
        }
      }
      if (fill) {
        if (width !== undefined) {
          throw imageError(src, 'has both "width" and "fill" properties. Only one should be used.');
        }
        if (height !== undefined) {
          throw imageError(src, 'has both "height" and "fill" properties. Only one should be used.');
        }
        if (style?.position && style.position !== "absolute") {
          throw imageError(src, `has both "fill" and "style.position" properties. Images with "fill" always use position absolute - it cannot be modified.`);
        }
      } else {
        if (widthInt === undefined) {
          throw imageError(src, 'is missing required "width" property.');
        }
        if (Number.isNaN(widthInt)) {
          throw imageError(src, `has invalid "width" property. Expected a numeric value in pixels but received "${String(width)}".`);
        }
        if (heightInt === undefined) {
          throw imageError(src, 'is missing required "height" property.');
        }
        if (Number.isNaN(heightInt)) {
          throw imageError(src, `has invalid "height" property. Expected a numeric value in pixels but received "${String(height)}".`);
        }
      }
    }
    if (loading !== undefined && loading !== "lazy" && loading !== "eager") {
      throw imageError(src, `has invalid "loading" property. Provided "${String(loading)}" should be one of lazy, eager.`);
    }
    if (priority && loading === "lazy") {
      throw imageError(src, `has both "priority" and "loading='lazy'" properties. Only one should be used.`);
    }
    if (placeholder !== "empty" && placeholder !== "blur" && !placeholder.startsWith("data:image/")) {
      throw imageError(src, `has invalid "placeholder" property "${placeholder}".`);
    }
    if (placeholder === "blur" && !blurDataURL) {
      throw imageError(src, `has "placeholder='blur'" property but is missing the "blurDataURL" property.`);
    }
  }

  const fetchPriority = priority || preload ? "high" : fetchPriorityProp;
  const fillStyle: CSSProperties = fill
    ? { position: "absolute", height: "100%", width: "100%", inset: 0, objectFit, objectPosition }
    : {};
  const imgStyle: CSSProperties = {
    ...fillStyle,
    ...(options.showAltText === true ? {} : { color: "transparent" }),
    ...(!fill && widthInt && heightInt ? { aspectRatio: `${widthInt} / ${heightInt}` } : {}),
    ...style,
  };
  const backgroundImage =
    options.blurComplete !== true && placeholder !== "empty"
      ? placeholder === "blur"
        ? `url("data:image/svg+xml;charset=utf-8,${getImageBlurSvg({
            widthInt,
            heightInt,
            blurWidth,
            blurHeight,
            blurDataURL: blurDataURL ?? "",
            objectFit: imgStyle.objectFit,
          })}")`
        : `url("${placeholder}")`
      : undefined;
  const placeholderStyle: CSSProperties = backgroundImage
    ? {
        backgroundSize: imgStyle.objectFit ?? "cover",
        backgroundPosition: imgStyle.objectPosition ?? "50% 50%",
        backgroundRepeat: "no-repeat",
        backgroundImage,
      }
    : {};

  const loader: ImageLoader =
    loaderProp ??
    options.loader ??
    activeRuntime.loader ??
    ((loaderProps) =>
      defaultLoader(loaderProps, { route: policy.route, deploymentId: options.deploymentId ?? activeRuntime.deploymentId }));
  const attrs = generateImgAttrs({
    config: sizeConfig(policy),
    src,
    unoptimized,
    width: widthInt,
    quality: qualityInt,
    sizes,
    loader,
  });

  const resolved: ResolvedImageProps = {
    ...rest,
    alt,
    loading: isLazy ? "lazy" : "eager",
    fetchPriority,
    width: fill ? undefined : widthInt,
    height: fill ? undefined : heightInt,
    decoding,
    "data-nimg": fill ? "fill" : "1",
    style: { ...imgStyle, ...placeholderStyle },
    sizes: attrs.sizes,
    srcSet: attrs.srcSet,
    src: overrideSrc ?? attrs.src,
  };
  return {
    props: resolved,
    meta: { unoptimized, priority, preload, fill, placeholder, blurDataURL },
  };
}

/** `next/image`: optimized `<img>` with lazy loading, srcset generation and blur placeholder. */
export function Image(props: ImageProps): ReactElement {
  const { ref, onLoad, onError, onLoadingComplete, ...restProps } = props;
  const [blurComplete, setBlurComplete] = useState(false);
  const [showAltText, setShowAltText] = useState(false);
  const { props: imgProps, meta } = getImageProps(restProps as ImageProps, activePolicy, { blurComplete, showAltText });
  if ((meta.priority || meta.preload) && typeof ReactDOM.preload === "function") {
    ReactDOM.preload(imgProps.src, {
      as: "image",
      imageSrcSet: imgProps.srcSet,
      imageSizes: imgProps.sizes,
      fetchPriority: imgProps.fetchPriority,
    });
  }
  const handleLoad = useCallback(
    (event: SyntheticEvent<HTMLImageElement>): void => {
      const img = event.currentTarget;
      if (img.src.startsWith("data:")) {
        return;
      }
      setBlurComplete(true);
      onLoad?.(event);
      onLoadingComplete?.(img);
    },
    [onLoad, onLoadingComplete],
  );
  const handleError = useCallback(
    (event: SyntheticEvent<HTMLImageElement>): void => {
      setShowAltText(true);
      setBlurComplete(true);
      onError?.(event);
    },
    [onError],
  );
  return createElement("img", { ...imgProps, ref, onLoad: handleLoad, onError: handleError });
}

// ---------------------------------------------------------------------------
// Optimizer route (/_yolo/image, /_next/image)
// ---------------------------------------------------------------------------

/** Options of `createImageOptimizerHandler`. */
export interface ImageOptimizerHandlerOptions {
  readonly root: string;
  readonly publicDir?: string;
  readonly cacheDir?: string;
  readonly fetchImpl?: typeof fetch;
  readonly now?: () => number;
  readonly dev?: boolean;
  readonly upstreamTimeoutMs?: number;
}

/** Transformation backend selected at runtime. */
export type ImageBackend = "bun-image" | "sharp" | "passthrough";

/** Transformation request. */
export interface TransformOptions {
  readonly width: number;
  readonly format: string;
  readonly quality: number;
}

interface BunImageLike {
  resize(width: number, height?: number, options?: { withoutEnlargement?: boolean }): BunImageLike;
  avif(options?: { quality?: number }): BunImageLike;
  webp(options?: { quality?: number }): BunImageLike;
  jpeg(options?: { quality?: number }): BunImageLike;
  png(options?: Record<string, unknown>): BunImageLike;
  bytes(): Promise<Uint8Array>;
}

type BunImageConstructor = new (input: Uint8Array) => BunImageLike;

function bunImageConstructor(): BunImageConstructor | undefined {
  const candidate = (Bun as unknown as { Image?: unknown }).Image;
  return typeof candidate === "function" ? (candidate as BunImageConstructor) : undefined;
}

/** Resizes and re-encodes with `Bun.Image` when available; `undefined` when unsupported or on failure. */
export async function transformWithBunImage(bytes: Uint8Array, options: TransformOptions): Promise<Uint8Array | undefined> {
  const BunImage = bunImageConstructor();
  if (!BunImage) {
    return undefined;
  }
  try {
    const image = new BunImage(bytes).resize(options.width, undefined, { withoutEnlargement: true });
    let encoded: BunImageLike;
    switch (options.format) {
      case "image/avif":
        encoded = image.avif({ quality: options.quality });
        break;
      case "image/webp":
        encoded = image.webp({ quality: options.quality });
        break;
      case "image/jpeg":
        encoded = image.jpeg({ quality: options.quality });
        break;
      case "image/png":
        encoded = image.png();
        break;
      default:
        return undefined;
    }
    return await encoded.bytes();
  } catch {
    return undefined;
  }
}

interface SharpInstance {
  resize(options: { width: number; withoutEnlargement: boolean }): SharpInstance;
  toFormat(format: string, options?: { quality?: number }): SharpInstance;
  toBuffer(): Promise<Uint8Array>;
}

type SharpFactory = (input: Uint8Array) => SharpInstance;
let sharpFactory: Promise<SharpFactory | undefined> | undefined;

function sharpResolvable(): boolean {
  try {
    Bun.resolveSync("sharp", process.cwd());
    return true;
  } catch {
    return false;
  }
}

async function loadSharp(): Promise<SharpFactory | undefined> {
  sharpFactory ??= (async () => {
    if (!sharpResolvable()) {
      return undefined;
    }
    try {
      const specifier: string = "sharp";
      const module = (await import(specifier)) as { default?: unknown };
      const factory = typeof module.default === "function" ? module.default : typeof module === "function" ? module : undefined;
      return factory as SharpFactory | undefined;
    } catch {
      return undefined;
    }
  })();
  return sharpFactory;
}

/** Resizes and re-encodes with `sharp` when installed; `undefined` otherwise or on failure. */
export async function transformWithSharp(bytes: Uint8Array, options: TransformOptions): Promise<Uint8Array | undefined> {
  const sharp = await loadSharp();
  if (!sharp) {
    return undefined;
  }
  const format = EXTENSIONS[options.format];
  if (!format || !TRANSFORMABLE_FORMATS.has(options.format)) {
    return undefined;
  }
  try {
    const output = await sharp(bytes)
      .resize({ width: options.width, withoutEnlargement: true })
      .toFormat(format, { quality: options.quality })
      .toBuffer();
    return new Uint8Array(output);
  } catch {
    return undefined;
  }
}

/** Reports which transformation backend the optimizer uses: `Bun.Image`, `sharp`, or passthrough. */
export function describeImageBackend(): ImageBackend {
  if (bunImageConstructor()) {
    return "bun-image";
  }
  return sharpResolvable() ? "sharp" : "passthrough";
}

/** Detects the MIME type of image bytes from their magic numbers. */
export function detectImageType(bytes: Uint8Array): string | undefined {
  const at = (index: number): number => bytes[index] ?? -1;
  if (at(0) === 0x89 && at(1) === 0x50 && at(2) === 0x4e && at(3) === 0x47) {
    return "image/png";
  }
  if (at(0) === 0xff && at(1) === 0xd8 && at(2) === 0xff) {
    return "image/jpeg";
  }
  if (at(0) === 0x47 && at(1) === 0x49 && at(2) === 0x46 && at(3) === 0x38) {
    return "image/gif";
  }
  if (at(0) === 0x52 && at(1) === 0x49 && at(2) === 0x46 && at(3) === 0x46 && at(8) === 0x57 && at(9) === 0x45 && at(10) === 0x42 && at(11) === 0x50) {
    return "image/webp";
  }
  if (at(4) === 0x66 && at(5) === 0x74 && at(6) === 0x79 && at(7) === 0x70 && at(8) === 0x61 && at(9) === 0x76 && at(10) === 0x69) {
    return "image/avif";
  }
  if (at(0) === 0x00 && at(1) === 0x00 && at(2) === 0x01 && at(3) === 0x00) {
    return "image/x-icon";
  }
  if (at(0) === 0x42 && at(1) === 0x4d) {
    return "image/bmp";
  }
  if ((at(0) === 0x49 && at(1) === 0x49 && at(2) === 0x2a && at(3) === 0x00) || (at(0) === 0x4d && at(1) === 0x4d && at(2) === 0x00 && at(3) === 0x2a)) {
    return "image/tiff";
  }
  const head = new TextDecoder().decode(bytes.subarray(0, 512)).trimStart();
  if (head.startsWith("<svg") || (head.startsWith("<?xml") && head.includes("<svg"))) {
    return "image/svg+xml";
  }
  return undefined;
}

function indexOfSequence(bytes: Uint8Array, sequence: ReadonlyArray<number>, from = 0): number {
  const first = sequence[0];
  if (first === undefined) {
    return -1;
  }
  for (let index = from; index <= bytes.length - sequence.length; index += 1) {
    if (bytes[index] !== first) {
      continue;
    }
    let matched = true;
    for (let offset = 1; offset < sequence.length; offset += 1) {
      if (bytes[index + offset] !== sequence[offset]) {
        matched = false;
        break;
      }
    }
    if (matched) {
      return index;
    }
  }
  return -1;
}

/** True for animated GIF, PNG (`acTL`) and WebP (`ANIM`) images, which are served untouched. */
export function isAnimatedImage(bytes: Uint8Array, type: string): boolean {
  if (type === "image/gif") {
    let frames = 0;
    let index = indexOfSequence(bytes, [0x00, 0x21, 0xf9, 0x04]);
    while (index !== -1 && frames < 2) {
      frames += 1;
      index = indexOfSequence(bytes, [0x00, 0x21, 0xf9, 0x04], index + 1);
    }
    return frames >= 2;
  }
  if (type === "image/png") {
    const idat = indexOfSequence(bytes, [0x49, 0x44, 0x41, 0x54]);
    const actl = indexOfSequence(bytes, [0x61, 0x63, 0x54, 0x4c]);
    return actl !== -1 && (idat === -1 || actl < idat);
  }
  if (type === "image/webp") {
    return indexOfSequence(bytes.subarray(0, 64), [0x41, 0x4e, 0x49, 0x4d]) !== -1;
  }
  return false;
}

function parseIpv4(hostname: string): number[] | undefined {
  const parts = hostname.split(".");
  if (parts.length !== 4) {
    return undefined;
  }
  const octets = parts.map((part) => (/^\d{1,3}$/.test(part) ? Number(part) : Number.NaN));
  return octets.every((octet) => octet >= 0 && octet <= 255) ? octets : undefined;
}

/** True for loopback, link-local, unspecified and private (RFC 1918 / ULA) hosts. */
export function isPrivateHost(hostname: string): boolean {
  let host = hostname.toLowerCase();
  if (host.startsWith("[") && host.endsWith("]")) {
    host = host.slice(1, -1);
  }
  if (host === "localhost" || host.endsWith(".localhost")) {
    return true;
  }
  const mapped = /^::ffff:(\d+\.\d+\.\d+\.\d+)$/.exec(host);
  if (mapped) {
    host = mapped[1] ?? host;
  }
  const v4 = parseIpv4(host);
  if (v4) {
    const [a = 0, b = 0] = v4;
    return a === 10 || a === 127 || a === 0 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168) || (a === 169 && b === 254);
  }
  if (host.includes(":")) {
    if (host === "::1" || host === "::") {
      return true;
    }
    const prefix = host.split(":")[0] ?? "";
    return /^f[cd][0-9a-f]{0,2}$/.test(prefix) || /^fe[89ab][0-9a-f]?$/.test(prefix);
  }
  return false;
}

interface AcceptEntry {
  readonly type: string;
  readonly q: number;
}

function parseAccept(header: string | null): AcceptEntry[] {
  if (header === null || header.trim() === "") {
    return [{ type: "*/*", q: 1 }];
  }
  return header.split(",").map((part) => {
    const [type = "", ...params] = part.trim().split(";");
    let q = 1;
    for (const param of params) {
      const [key, value] = param.trim().split("=");
      if (key === "q" && value !== undefined) {
        const parsed = Number.parseFloat(value);
        q = Number.isNaN(parsed) ? 0 : parsed;
      }
    }
    return { type: type.trim().toLowerCase(), q };
  });
}

function acceptedFormats(policy: ImageOptimizerPolicy, accept: string | null): string[] {
  if (accept === null) {
    return [];
  }
  const entries = parseAccept(accept);
  return policy.formats.filter((format) => entries.some((entry) => entry.q > 0 && entry.type === format));
}

/** First policy format explicitly listed in the `Accept` header (Next's rule), or `undefined` to keep the upstream type. */
export function negotiateImageFormat(policy: ImageOptimizerPolicy, accept: string | null): string | undefined {
  return acceptedFormats(policy, accept)[0];
}

/** Parses `max-age` / `s-maxage` (seconds) from a `Cache-Control` header. */
export function parseMaxAge(cacheControl: string | null): number | undefined {
  if (!cacheControl) {
    return undefined;
  }
  const sMaxAge = /s-maxage=(\d+)/i.exec(cacheControl);
  const maxAge = /max-age=(\d+)/i.exec(cacheControl);
  const value = sMaxAge?.[1] ?? maxAge?.[1];
  return value === undefined ? undefined : Number.parseInt(value, 10);
}

function cacheControlHeader(policy: ImageOptimizerPolicy, maxAge: number): string {
  const template = policy.cacheControl;
  if (template.includes("{max_age}") || template.includes("{maxAge}")) {
    return template.replace("{max_age}", String(maxAge)).replace("{maxAge}", String(maxAge));
  }
  return `public, max-age=${maxAge}, must-revalidate`;
}

function fileNameFor(url: string, contentType: string): string {
  const pathname = new URL(url, DUMMY_ORIGIN).pathname;
  const name = basename(pathname, extname(pathname)) || "image";
  return `${name}.${EXTENSIONS[contentType] ?? "bin"}`;
}

function textResponse(status: number, message: string): Response {
  return new Response(message, { status, headers: { "content-type": "text/plain; charset=utf-8" } });
}

class ImageError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

interface UpstreamImage {
  readonly bytes: Uint8Array;
  readonly contentType: string;
  readonly maxAge: number | undefined;
}

async function readLimited(response: Response, limit: number): Promise<Uint8Array> {
  const declared = Number.parseInt(response.headers.get("content-length") ?? "", 10);
  if (!Number.isNaN(declared) && declared > limit) {
    throw new ImageError(413, '"url" parameter is valid but upstream response body exceeds maximumResponseBody');
  }
  const body = response.body;
  if (!body) {
    return new Uint8Array(0);
  }
  const reader = body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    total += value.byteLength;
    if (total > limit) {
      await reader.cancel();
      throw new ImageError(413, '"url" parameter is valid but upstream response body exceeds maximumResponseBody');
    }
    chunks.push(value);
  }
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

function isTimeoutError(error: unknown): boolean {
  return error instanceof Error && (error.name === "TimeoutError" || error.name === "AbortError");
}

async function fetchRemote(
  href: string,
  policy: ImageOptimizerPolicy,
  fetchImpl: typeof fetch,
  timeoutMs: number,
): Promise<UpstreamImage> {
  let current = href;
  for (let redirects = 0; ; redirects += 1) {
    const url = new URL(current);
    if (!policy.allowLocalIp && isPrivateHost(url.hostname)) {
      throw new ImageError(400, '"url" parameter is not allowed');
    }
    let response: Response;
    try {
      response = await fetchImpl(url.href, {
        redirect: "manual",
        signal: AbortSignal.timeout(timeoutMs),
        headers: { accept: "image/*,*/*;q=0.8", "user-agent": "YoloImageOptimizer/1.0" },
      });
    } catch (error) {
      if (isTimeoutError(error)) {
        throw new ImageError(504, '"url" parameter is valid but upstream response timed out');
      }
      throw new ImageError(500, '"url" parameter is valid but upstream response is invalid');
    }
    const location = response.headers.get("location");
    if (response.status >= 300 && response.status < 400 && location) {
      if (redirects + 1 > policy.maximumRedirects) {
        throw new ImageError(508, '"url" parameter is valid but upstream response exceeded maximumRedirects');
      }
      current = new URL(location, url).href;
      continue;
    }
    if (!response.ok) {
      throw new ImageError(response.status, '"url" parameter is valid but upstream response is invalid');
    }
    const bytes = await readLimited(response, policy.maximumResponseBody);
    const headerType = (response.headers.get("content-type") ?? "").split(";")[0]?.trim().toLowerCase();
    const contentType = headerType && headerType.startsWith("image/") ? headerType : detectImageType(bytes);
    if (!contentType) {
      throw new ImageError(400, "The requested resource isn't a valid image.");
    }
    return { bytes, contentType, maxAge: parseMaxAge(response.headers.get("cache-control")) };
  }
}

async function readLocal(src: string, publicRoot: string): Promise<UpstreamImage> {
  const pathname = new URL(src, DUMMY_ORIGIN).pathname;
  let decoded: string;
  try {
    decoded = decodeURIComponent(pathname);
  } catch {
    throw new ImageError(400, '"url" parameter is invalid');
  }
  const filePath = resolve(publicRoot, `.${decoded}`);
  if (filePath !== publicRoot && !filePath.startsWith(`${publicRoot}${sep}`)) {
    throw new ImageError(400, '"url" parameter is invalid');
  }
  const file = Bun.file(filePath);
  if (!(await file.exists())) {
    throw new ImageError(404, '"url" parameter is valid but image not found');
  }
  const bytes = new Uint8Array(await file.arrayBuffer());
  const contentType = detectImageType(bytes) ?? EXTENSION_TYPES[extname(filePath).slice(1).toLowerCase()];
  if (!contentType) {
    throw new ImageError(400, "The requested resource isn't a valid image.");
  }
  return { bytes, contentType, maxAge: undefined };
}

async function optimize(
  upstream: UpstreamImage,
  width: number,
  quality: number,
  formats: ReadonlyArray<string>,
): Promise<{ bytes: Uint8Array; contentType: string }> {
  if (
    upstream.contentType === "image/svg+xml" ||
    upstream.contentType === "image/x-icon" ||
    upstream.contentType === "image/vnd.microsoft.icon" ||
    isAnimatedImage(upstream.bytes, upstream.contentType)
  ) {
    return { bytes: upstream.bytes, contentType: upstream.contentType };
  }
  const candidates = [...new Set([...formats, upstream.contentType])].filter((format) => TRANSFORMABLE_FORMATS.has(format));
  for (const format of candidates) {
    const options: TransformOptions = { width, format, quality };
    const output = (await transformWithBunImage(upstream.bytes, options)) ?? (await transformWithSharp(upstream.bytes, options));
    if (output) {
      return { bytes: output, contentType: format };
    }
  }
  return { bytes: upstream.bytes, contentType: upstream.contentType };
}

interface CachedImage {
  readonly bytes: Uint8Array;
  readonly contentType: string;
  readonly maxAge: number;
  readonly expireAt: number;
  readonly etag: string;
  readonly path: string;
}

async function readCache(dir: string): Promise<CachedImage | undefined> {
  let entries: string[];
  try {
    entries = await readdir(dir);
  } catch {
    return undefined;
  }
  for (const entry of entries) {
    const [maxAge, expireAt, etag, extension] = entry.split(".");
    const contentType = extension ? EXTENSION_TYPES[extension] : undefined;
    if (!maxAge || !expireAt || !etag || !contentType) {
      continue;
    }
    const path = resolve(dir, entry);
    try {
      const bytes = new Uint8Array(await Bun.file(path).arrayBuffer());
      return { bytes, contentType, maxAge: Number(maxAge), expireAt: Number(expireAt), etag, path };
    } catch {
      return undefined;
    }
  }
  return undefined;
}

async function writeCache(dir: string, image: Omit<CachedImage, "path">, stale?: string): Promise<void> {
  try {
    await mkdir(dir, { recursive: true });
    const extension = EXTENSIONS[image.contentType] ?? "bin";
    await Bun.write(resolve(dir, `${image.maxAge}.${image.expireAt}.${image.etag}.${extension}`), image.bytes);
    if (stale) {
      await unlink(stale).catch(() => undefined);
    }
  } catch {
    // Cache writes are best effort.
  }
}

function etagMatches(ifNoneMatch: string | null, etag: string): boolean {
  if (!ifNoneMatch) {
    return false;
  }
  return ifNoneMatch
    .split(",")
    .map((tag) => tag.trim().replace(/^W\//, "").replace(/^"|"$/g, ""))
    .some((tag) => tag === "*" || tag === etag);
}

/** Creates the `/_yolo/image` (and `/_next/image`) request handler for `policy`. */
export function createImageOptimizerHandler(
  policy: ImageOptimizerPolicy,
  options: ImageOptimizerHandlerOptions,
): (request: Request) => Promise<Response> {
  const publicRoot = resolve(options.root, options.publicDir ?? "public");
  const cacheRoot = resolve(options.root, options.cacheDir ?? ".yolo/cache/images");
  const fetchImpl = options.fetchImpl ?? fetch;
  const now = options.now ?? (() => Date.now());
  const dev = options.dev ?? isDev();
  const timeoutMs = options.upstreamTimeoutMs ?? UPSTREAM_TIMEOUT_MS;
  const allowedSizes = new Set([...policy.deviceSizes, ...policy.imageSizes]);
  const allowedQualities = new Set(policy.qualities.length > 0 ? policy.qualities : [DEFAULT_QUALITY]);
  const optimizerRoutes = [policy.route, ...policy.compatRoutes];

  const produce = async (url: string, width: number, quality: number, formats: ReadonlyArray<string>) => {
    const upstream = isAbsoluteUrl(url) ? await fetchRemote(url, policy, fetchImpl, timeoutMs) : await readLocal(url, publicRoot);
    if (upstream.contentType === "image/svg+xml" && !policy.allowSvg) {
      throw new ImageError(400, `The requested resource "${url}" has type "image/svg+xml" but dangerouslyAllowSVG is disabled`);
    }
    const optimized = await optimize(upstream, width, quality, formats);
    const maxAge = Math.max(policy.minimumCacheTtl, upstream.maxAge ?? 0);
    return {
      bytes: optimized.bytes,
      contentType: optimized.contentType,
      maxAge,
      expireAt: now() + maxAge * 1000,
      etag: sha256(optimized.bytes).slice(0, 32),
    };
  };

  const respond = (request: Request, url: string, image: Omit<CachedImage, "path">, cacheStatus: string): Response => {
    const headers = new Headers({
      "content-type": image.contentType,
      "cache-control": cacheControlHeader(policy, dev ? 0 : image.maxAge),
      vary: "Accept",
      etag: `"${image.etag}"`,
      "content-disposition": `${policy.contentDispositionType}; filename="${fileNameFor(url, image.contentType)}"`,
      "content-security-policy": policy.contentSecurityPolicy,
      "x-nextjs-cache": cacheStatus,
    });
    if (etagMatches(request.headers.get("if-none-match"), image.etag)) {
      return new Response(null, { status: 304, headers });
    }
    headers.set("content-length", String(image.bytes.byteLength));
    return new Response(image.bytes as BodyInit, { status: 200, headers });
  };

  return async (request: Request): Promise<Response> => {
    const params = new URL(request.url).searchParams;
    const url = params.get("url");
    const w = params.get("w");
    const q = params.get("q");
    if (url === null || url === "") {
      return textResponse(400, '"url" parameter is required');
    }
    if (params.getAll("url").length > 1) {
      return textResponse(400, '"url" parameter cannot be an array');
    }
    if (url.length > MAX_URL_LENGTH) {
      return textResponse(400, '"url" parameter is too long');
    }
    if (url.startsWith("//")) {
      return textResponse(400, '"url" parameter cannot be a protocol-relative URL (//)');
    }
    if (isAbsoluteUrl(url)) {
      let parsed: URL;
      try {
        parsed = new URL(url);
      } catch {
        return textResponse(400, '"url" parameter is invalid');
      }
      if (!matchesImagePolicy(policy, parsed.href)) {
        return textResponse(400, '"url" parameter is not allowed');
      }
      if (!policy.allowLocalIp && isPrivateHost(parsed.hostname)) {
        return textResponse(400, '"url" parameter is not allowed');
      }
    } else if (url.startsWith("/")) {
      if (optimizerRoutes.some((route) => url === route || url.startsWith(`${route}?`) || url.startsWith(`${route}/`))) {
        return textResponse(400, '"url" parameter cannot be recursive');
      }
      if (!matchesImagePolicy(policy, url)) {
        return textResponse(400, '"url" parameter is not allowed');
      }
    } else {
      return textResponse(400, '"url" parameter is invalid');
    }
    if (w === null || w === "") {
      return textResponse(400, '"w" parameter (width) is required');
    }
    const width = Number.parseInt(w, 10);
    if (Number.isNaN(width) || width <= 0) {
      return textResponse(400, '"w" parameter (width) must be a number greater than 0');
    }
    if (!allowedSizes.has(width)) {
      return textResponse(400, `"w" parameter (width) of ${width} is not allowed`);
    }
    if (q === null || q === "") {
      return textResponse(400, '"q" parameter (quality) is required');
    }
    const quality = Number.parseInt(q, 10);
    if (Number.isNaN(quality) || quality < 1 || quality > 100) {
      return textResponse(400, '"q" parameter (quality) must be an integer between 1 and 100');
    }
    if (!allowedQualities.has(quality)) {
      return textResponse(400, `"q" parameter (quality) of ${quality} is not allowed`);
    }
    const formats = acceptedFormats(policy, request.headers.get("accept"));
    const cacheKey = sha256(`${url}|${width}|${quality}|${formats[0] ?? ""}`);
    const cacheDir = resolve(cacheRoot, cacheKey);
    const cached = await readCache(cacheDir);
    if (cached) {
      if (cached.expireAt > now()) {
        return respond(request, url, cached, "HIT");
      }
      void produce(url, width, quality, formats)
        .then((fresh) => writeCache(cacheDir, fresh, cached.path))
        .catch(() => undefined);
      return respond(request, url, cached, "STALE");
    }
    try {
      const fresh = await produce(url, width, quality, formats);
      await writeCache(cacheDir, fresh);
      return respond(request, url, fresh, "MISS");
    } catch (error) {
      if (error instanceof ImageError) {
        return textResponse(error.status, error.message);
      }
      return textResponse(500, "Internal Server Error");
    }
  };
}
