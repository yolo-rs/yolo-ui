// SPDX-License-Identifier: MIT OR Apache-2.0
// `@yolo-rs/fullstack/image`: the image CDN layer (loaders, srcset, the `/_next/image` optimizer) and the
// build-time definition that hands the images config to the `next/image` component.
import type { ResolvedImages } from "../config/next-config";

export { generateImgAttrs, getWidths, imageLoaderUrl } from "./loaders";
export type {
  ImageLoaderFn,
  ImageLoaderProps,
  ImgAttrs,
  ImgAttrsOptions,
  LoaderConfig,
  SizesConfig,
  Widths,
} from "./loaders";
export {
  createImageOptimizer,
  defaultResizer,
  detectContentType,
  fileName,
  getMaxAge,
  isAnimated,
  matchLocalPattern,
  matchRemotePattern,
  negotiate,
  passthroughResizer,
  validateParams,
  YOLO_IMAGE_PATH,
} from "./optimizer";
export type {
  ImageOptimizer,
  ImageOptimizerOptions,
  ResizeFormat,
  ResizeOptions,
  ResizeResult,
  Resizer,
  ValidatedParams,
} from "./optimizer";

/** Global the component reads its config from (a `Bun.build` define, else `globalThis`). */
export const IMAGE_CONFIG_GLOBAL = "__YOLO_IMAGE_CONFIG__";

/** The config the `next/image` component needs at runtime. */
export interface ImageRuntimeConfig {
  loader: ResolvedImages["loader"];
  path: string;
  deviceSizes: number[];
  imageSizes: number[];
  qualities: number[];
  unoptimized: boolean;
  /** Whether `images.loaderFile` is set (the module is aliased to `yolo:image-loader` by the build). */
  loaderFile: boolean;
}

export function imageRuntimeConfig(images: ResolvedImages): ImageRuntimeConfig {
  return {
    loader: images.loader,
    path: images.path,
    deviceSizes: images.deviceSizes,
    imageSizes: images.imageSizes,
    qualities: images.qualities,
    unoptimized: images.unoptimized,
    loaderFile: !!images.loaderFile,
  };
}

/** The `Bun.build` `define` entry of the images config. */
export function imageConfigDefine(images: ResolvedImages): Record<string, string> {
  return { [IMAGE_CONFIG_GLOBAL]: JSON.stringify(imageRuntimeConfig(images)) };
}
