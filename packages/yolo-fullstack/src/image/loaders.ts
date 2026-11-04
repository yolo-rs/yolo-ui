// SPDX-License-Identifier: MIT OR Apache-2.0
// Image URL loaders of `next/image` (default optimizer, Vercel, imgix, Cloudinary, Akamai, Cloudflare, Fastly)
// and the srcset algorithm of the component: candidate widths from `deviceSizes` / `imageSizes` and `sizes`.
import type { ImageLoader } from "../config/next-config";

export interface ImageLoaderProps {
  src: string;
  width: number;
  quality?: number;
}

/** The part of the images config a loader needs. */
export interface LoaderConfig {
  loader: ImageLoader;
  path: string;
}

/** The part of the images config `getWidths` needs. */
export interface SizesConfig {
  deviceSizes: number[];
  imageSizes: number[];
}

export type ImageLoaderFn = (props: ImageLoaderProps) => string;

/** CDN loaders expect `path` to end with `/`; a leading `/` of `src` is dropped so the two join cleanly. */
const normalizeSrc = (src: string) => (src.startsWith("/") ? src.slice(1) : src);

/** The URL of `src` at `width` for the configured loader. */
export function imageLoaderUrl(config: LoaderConfig, { src, width, quality }: ImageLoaderProps): string {
  const q = quality ?? 75;
  switch (config.loader) {
    case "default":
      return `${config.path}?url=${encodeURIComponent(src)}&w=${width}&q=${q}`;
    case "vercel":
      return `/_vercel/image?url=${encodeURIComponent(src)}&w=${width}&q=${q}`;
    case "imgix": {
      const url = new URL(`${config.path}${normalizeSrc(src)}`);
      const params = url.searchParams;
      params.set("auto", params.getAll("auto").join(",") || "format");
      params.set("fit", params.get("fit") || "max");
      params.set("w", params.get("w") || String(width));
      params.set("q", String(q));
      return url.href;
    }
    case "cloudinary":
      return `${config.path}f_auto,c_limit,w_${width},q_${quality || "auto"}/${normalizeSrc(src)}`;
    case "akamai":
      return `${config.path}${normalizeSrc(src)}?imwidth=${width}`;
    case "cloudflare":
      return `${config.path}/cdn-cgi/image/width=${width},quality=${q},format=auto/${normalizeSrc(src)}`;
    case "fastly": {
      const url = new URL(`${config.path}${normalizeSrc(src)}`);
      url.searchParams.set("auto", "webp");
      url.searchParams.set("width", String(width));
      url.searchParams.set("quality", String(q));
      return url.href;
    }
    case "custom":
      throw new Error(
        `@yolo-rs/fullstack: image with src "${src}" is missing a loader: set images.loaderFile or pass a \`loader\` prop`,
      );
    default:
      throw new Error(`@yolo-rs/fullstack: unknown image loader "${String(config.loader)}"`);
  }
}

export interface Widths {
  widths: number[];
  /** `w` descriptors (viewport based) or `x` descriptors (fixed width). */
  kind: "w" | "x";
}

const VIEWPORT_WIDTH = /(^|\s)(1?\d?\d)vw/g;

/** Candidate widths of an image, the way `next/image` picks them. */
export function getWidths(config: SizesConfig, width: number | undefined, sizes?: string): Widths {
  const allSizes = [...config.deviceSizes, ...config.imageSizes].sort((a, b) => a - b);
  const deviceSizes = [...config.deviceSizes].sort((a, b) => a - b);
  if (sizes) {
    const percentSizes: number[] = [];
    for (let match = VIEWPORT_WIDTH.exec(sizes); match; match = VIEWPORT_WIDTH.exec(sizes))
      percentSizes.push(parseInt(match[2]!, 10));
    VIEWPORT_WIDTH.lastIndex = 0;
    if (percentSizes.length) {
      const smallestRatio = Math.min(...percentSizes) * 0.01;
      return { widths: allSizes.filter(size => size >= (deviceSizes[0] ?? 0) * smallestRatio), kind: "w" };
    }
    return { widths: allSizes, kind: "w" };
  }
  if (typeof width !== "number") return { widths: deviceSizes, kind: "w" };
  const last = allSizes[allSizes.length - 1];
  const widths = [...new Set([width, width * 2].map(w => allSizes.find(p => p >= w) ?? last))].filter(
    (w): w is number => w !== undefined,
  );
  return { widths, kind: "x" };
}

export interface ImgAttrsOptions {
  config: LoaderConfig & SizesConfig;
  src: string;
  width?: number;
  quality?: number;
  sizes?: string;
  unoptimized?: boolean;
  /** Overrides the configured loader (a `loader` prop or `images.loaderFile`). */
  loader?: ImageLoaderFn;
}

export interface ImgAttrs {
  src: string;
  srcSet: string | undefined;
  sizes: string | undefined;
}

/** `src`, `srcset` and `sizes` of an `<img>`. */
export function generateImgAttrs({ config, src, width, quality, sizes, unoptimized, loader }: ImgAttrsOptions): ImgAttrs {
  if (unoptimized) return { src, srcSet: undefined, sizes: undefined };
  const load = loader ?? ((props: ImageLoaderProps) => imageLoaderUrl(config, props));
  const { widths, kind } = getWidths(config, width, sizes);
  const last = widths.length - 1;
  return {
    sizes: !sizes && kind === "w" ? "100vw" : sizes,
    srcSet: widths.map((w, i) => `${load({ src, quality, width: w })} ${kind === "w" ? w : i + 1}${kind}`).join(", "),
    src: load({ src, quality, width: widths[last]! }),
  };
}
