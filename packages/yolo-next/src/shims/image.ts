// SPDX-License-Identifier: Apache-2.0
// next/image shim.

import { Image, getImageProps } from "../image.ts";

/** `next/image` default export. */
export default Image;
/** `getImageProps` (and its pre-14 name `unstable_getImgProps`). */
export { getImageProps, getImageProps as unstable_getImgProps };
/** Prop and loader types of `next/image`. */
export type { ImageProps, StaticImageData, StaticImport, GetImagePropsResult } from "../image.ts";
/** Loader types shared with `next.config` (`images.loader`). */
export type { ImageLoader, ImageLoaderProps } from "../types.ts";
