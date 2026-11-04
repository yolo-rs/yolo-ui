// SPDX-License-Identifier: MIT OR Apache-2.0
// Stand-in for the user's `images.loaderFile` (the `yolo:image-loader` specifier). The build plugin aliases this
// module to the configured file; without one the component falls back to the configured loader.
import type { ImageLoaderFn } from "../image/loaders";

const customLoader: ImageLoaderFn | undefined = undefined;

export default customLoader;
