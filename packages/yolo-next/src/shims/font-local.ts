// SPDX-License-Identifier: Apache-2.0
// next/font/local shim.

import { localFont } from "../fonts.ts";

/** `next/font/local` default export (`localFont({ src, variable })`). */
export default localFont;
/** Named alias of the default export. */
export { localFont };
/** Option types of `localFont`. */
export type { LocalFontOptions, LocalFontSrc, NextFont, NextFontWithVariable } from "../types.ts";
