// SPDX-License-Identifier: Apache-2.0
// next/navigation shim: hooks, control-flow helpers, ReadonlyURLSearchParams and RedirectType.

/** Hooks and control-flow functions of `next/navigation`. */
export {
  useRouter,
  usePathname,
  useSearchParams,
  useParams,
  useSelectedLayoutSegment,
  useSelectedLayoutSegments,
  useServerInsertedHTML,
  useLinkStatus,
  useReportWebVitals,
  redirect,
  permanentRedirect,
  notFound,
  forbidden,
  unauthorized,
  unstable_rethrow,
  unstable_isUnrecognizedActionError,
  isRedirectError,
  isHttpAccessFallbackError,
  isNextRouterError,
  getRedirectUrl,
  getRedirectStatus,
  getRedirectType,
  getAccessFallbackStatus,
  ReadonlyURLSearchParams,
  RedirectType,
} from "../navigation.ts";
/** Router types of `next/navigation`. */
export type {
  AppRouterInstance,
  NavigateOptions,
  PrefetchOptions,
  RedirectError,
  HTTPAccessFallbackError,
  RouteParams,
  WebVitalsMetric,
} from "../navigation.ts";
