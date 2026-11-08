// SPDX-License-Identifier: Apache-2.0
// next/headers shim.

/** `headers()`, `cookies()` and `draftMode()` (async, Next.js 16 signatures). */
export { headers, cookies, draftMode } from "../headers.ts";
/** Types returned by the `next/headers` functions. */
export type { ReadonlyHeaders, ResponseCookies, DraftMode, RequestCookie, ResponseCookie, CookieOptions } from "../headers.ts";
/** Sync-unwrapped `headers()` type kept for `UnsafeUnwrappedHeaders` compatibility. */
export type UnsafeUnwrappedHeaders = Headers;
/** Sync-unwrapped `cookies()` type kept for `UnsafeUnwrappedCookies` compatibility. */
export type UnsafeUnwrappedCookies = import("../headers.ts").ResponseCookies;
/** Sync-unwrapped `draftMode()` type kept for `UnsafeUnwrappedDraftMode` compatibility. */
export type UnsafeUnwrappedDraftMode = import("../headers.ts").DraftMode;
