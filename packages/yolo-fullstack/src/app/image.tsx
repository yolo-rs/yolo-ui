// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/image`: an `<img>` with `srcset` from the configured loader (the `/_next/image` optimizer or a CDN),
// intrinsic size from static imports, lazy loading, `priority`, `fill` and blur placeholders. The images config
// comes from the `__YOLO_IMAGE_CONFIG__` build define (`imageConfigDefine`) or from `globalThis` at runtime.
import type { CSSProperties, ImgHTMLAttributes, Ref, SyntheticEvent } from "react";
import {
  DEFAULT_DEVICE_SIZES,
  DEFAULT_IMAGE_PATH,
  DEFAULT_IMAGE_SIZES,
  DEFAULT_QUALITIES,
} from "../config/next-config";
import type { ImageLoader } from "../config/next-config";
import { generateImgAttrs, imageLoaderUrl } from "../image/loaders";
import type { ImageLoaderFn, ImageLoaderProps } from "../image/loaders";
import customLoader from "./image-loader-default";

export type { ImageLoaderFn, ImageLoaderProps };

export interface StaticImageData {
  src: string;
  width?: number;
  height?: number;
  blurDataURL?: string;
  blurWidth?: number;
  blurHeight?: number;
}

/** `import img from "./a.png"` under some bundlers is `{ default: data }`. */
export type StaticImport = StaticImageData | { default: StaticImageData };

export type PlaceholderValue = "blur" | "empty" | `data:image/${string}`;

export interface ImageProps extends Omit<
  ImgHTMLAttributes<HTMLImageElement>,
  "src" | "srcSet" | "width" | "height" | "loading" | "ref"
> {
  src: string | StaticImport;
  alt: string;
  width?: number | `${number}`;
  height?: number | `${number}`;
  /** Fill the parent (which must be positioned). */
  fill?: boolean;
  /** Load eagerly with high fetch priority (LCP images). */
  priority?: boolean;
  loading?: "lazy" | "eager";
  quality?: number | `${number}`;
  placeholder?: PlaceholderValue;
  blurDataURL?: string;
  unoptimized?: boolean;
  loader?: ImageLoaderFn;
  /** The `src` attribute, when it must differ from the optimized URL (SEO, bots). */
  overrideSrc?: string;
  /** Deprecated Next.js callback, called like `onLoad` with the element. */
  onLoadingComplete?: (img: HTMLImageElement) => void;
  ref?: Ref<HTMLImageElement>;
}

export interface ImageConfig {
  loader: ImageLoader;
  path: string;
  deviceSizes: number[];
  imageSizes: number[];
  qualities: number[];
  unoptimized: boolean;
  loaderFile: boolean;
}

declare const __YOLO_IMAGE_CONFIG__: Partial<ImageConfig> | undefined;

const DEFAULT_CONFIG: ImageConfig = {
  loader: "default",
  path: DEFAULT_IMAGE_PATH,
  deviceSizes: DEFAULT_DEVICE_SIZES,
  imageSizes: DEFAULT_IMAGE_SIZES,
  qualities: DEFAULT_QUALITIES,
  unoptimized: false,
  loaderFile: false,
};

/** The images config of the build (define) or of the process (`globalThis.__YOLO_IMAGE_CONFIG__`), with defaults. */
export function imageConfig(): ImageConfig {
  const defined = typeof __YOLO_IMAGE_CONFIG__ !== "undefined" ? __YOLO_IMAGE_CONFIG__ : undefined;
  const runtime = (globalThis as { __YOLO_IMAGE_CONFIG__?: Partial<ImageConfig> }).__YOLO_IMAGE_CONFIG__;
  const config = defined ?? runtime;
  return config ? { ...DEFAULT_CONFIG, ...config } : DEFAULT_CONFIG;
}

const toNumber = (value: number | `${number}` | undefined): number | undefined => {
  if (value === undefined) return undefined;
  const n = typeof value === "number" ? value : parseInt(value, 10);
  return Number.isFinite(n) ? n : undefined;
};

function staticData(src: string | StaticImport): StaticImageData {
  if (typeof src === "string") return { src };
  if ("default" in src && typeof src.default === "object") return src.default;
  return src as StaticImageData;
}

/** The attributes of the `<img>` `Image` renders. */
export function getImageProps(props: ImageProps): {
  props: ImgHTMLAttributes<HTMLImageElement> & { ref?: Ref<HTMLImageElement> };
} {
  const {
    src: rawSrc,
    alt,
    width: widthProp,
    height: heightProp,
    fill = false,
    priority = false,
    loading,
    quality: qualityProp,
    placeholder = "empty",
    blurDataURL: blurProp,
    unoptimized: unoptimizedProp,
    loader: loaderProp,
    overrideSrc,
    onLoadingComplete,
    onLoad,
    style,
    sizes,
    decoding = "async",
    fetchPriority,
    ...rest
  } = props;
  if (!rawSrc) throw new Error('@yolo-rs/fullstack: next/image requires a "src"');
  const config = imageConfig();
  const data = staticData(rawSrc);
  const src = data.src;
  if (typeof src !== "string")
    throw new Error('@yolo-rs/fullstack: next/image "src" must be a string or a static image import');
  if (src.startsWith("//"))
    throw new Error(`@yolo-rs/fullstack: next/image src "${src}" cannot be protocol-relative; use an absolute URL`);

  let width = toNumber(widthProp);
  let height = toNumber(heightProp);
  const blurDataURL = blurProp ?? data.blurDataURL;
  if (!fill) {
    if (width === undefined && height === undefined) {
      width = data.width;
      height = data.height;
    } else if (data.width && data.height) {
      // One dimension given: keep the aspect ratio of the static import.
      if (width !== undefined && height === undefined) height = Math.round((width * data.height) / data.width);
      else if (height !== undefined && width === undefined) width = Math.round((height * data.width) / data.height);
    }
    if (width === undefined || height === undefined)
      throw new Error(
        `@yolo-rs/fullstack: next/image with src "${src}" must have "width" and "height" properties or use "fill"`,
      );
  } else if (widthProp !== undefined || heightProp !== undefined) {
    throw new Error(`@yolo-rs/fullstack: next/image with src "${src}" has both "fill" and "width"/"height"`);
  }

  const quality = toNumber(qualityProp);
  const unoptimized =
    unoptimizedProp === true ||
    config.unoptimized ||
    src.startsWith("data:") ||
    src.startsWith("blob:") ||
    // SVG is served as built: the optimizer does not rasterize it.
    /\.svg($|\?)/i.test(src);
  const loader: ImageLoaderFn | undefined =
    loaderProp ??
    customLoader ??
    (config.loader === "custom" ? undefined : loaderProps => imageLoaderUrl(config, loaderProps));
  if (!unoptimized && !loader)
    throw new Error(`@yolo-rs/fullstack: images.loader is "custom" but no loader is configured for "${src}"`);

  const attrs = generateImgAttrs({
    config,
    src,
    width,
    quality,
    sizes,
    unoptimized,
    loader,
  });

  const placeholderStyle: CSSProperties = {};
  const blur = placeholder === "blur" ? blurDataURL : placeholder.startsWith("data:") ? placeholder : undefined;
  if (blur) {
    placeholderStyle.backgroundSize = style?.objectFit ?? "cover";
    placeholderStyle.backgroundPosition = style?.objectPosition ?? "50% 50%";
    placeholderStyle.backgroundRepeat = "no-repeat";
    placeholderStyle.backgroundImage = `url("${blur}")`;
  }
  const merged: CSSProperties = {
    ...(fill
      ? {
          position: "absolute",
          height: "100%",
          width: "100%",
          left: 0,
          top: 0,
          right: 0,
          bottom: 0,
          objectFit: "cover",
        }
      : undefined),
    color: "transparent",
    ...placeholderStyle,
    ...style,
  };

  // Handlers are only attached when the caller passes one: a Server Component cannot serialize functions, and a
  // callback given there is already the caller's problem. The blur placeholder is removed on load in that case.
  const handleLoad =
    onLoad || onLoadingComplete
      ? (event: SyntheticEvent<HTMLImageElement>) => {
          const img = event.currentTarget;
          if (blur && img.style) {
            img.style.backgroundImage = "none";
            img.style.backgroundSize = "";
            img.style.backgroundPosition = "";
            img.style.backgroundRepeat = "";
          }
          onLoad?.(event);
          onLoadingComplete?.(img);
        }
      : undefined;

  const imgProps: ImgHTMLAttributes<HTMLImageElement> & {
    ref?: Ref<HTMLImageElement>;
  } = {
    ...rest,
    alt,
    src: overrideSrc ?? attrs.src,
    srcSet: attrs.srcSet,
    sizes: attrs.sizes,
    width: fill ? undefined : width,
    height: fill ? undefined : height,
    decoding,
    loading: priority ? "eager" : (loading ?? "lazy"),
    fetchPriority: priority ? "high" : fetchPriority,
    style: merged,
  };
  if (handleLoad) imgProps.onLoad = handleLoad;
  return { props: imgProps };
}

export default function Image(props: ImageProps) {
  const { props: imgProps } = getImageProps(props);
  return <img {...imgProps} />;
}
