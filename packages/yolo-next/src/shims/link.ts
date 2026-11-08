// SPDX-License-Identifier: Apache-2.0
// next/link shim.

import { Link, useLinkStatus } from "../link.ts";

/** `next/link` default export. */
export default Link;
/** `useLinkStatus()` (pending state of the nearest Link). */
export { useLinkStatus };
/** Prop types of `Link`. */
export type { LinkProps, UrlObject, LinkNavigateEvent } from "../link.ts";
