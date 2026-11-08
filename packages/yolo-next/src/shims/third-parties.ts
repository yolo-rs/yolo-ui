// SPDX-License-Identifier: Apache-2.0
// @next/third-parties shim: GoogleAnalytics, GoogleTagManager, YouTubeEmbed, GoogleMapsEmbed + event helpers.

import { Fragment, createElement, type CSSProperties, type ReactElement } from "react";

/** Props of `GoogleAnalytics`. */
export interface GoogleAnalyticsProps {
  readonly gaId: string;
  readonly dataLayerName?: string;
  readonly debugMode?: boolean;
  readonly nonce?: string;
}

let currentDataLayerName = "dataLayer";

/** Google Analytics 4 (gtag.js) loader plus the inline config snippet. */
export function GoogleAnalytics(props: GoogleAnalyticsProps): ReactElement {
  const { gaId, dataLayerName = "dataLayer", debugMode = false, nonce } = props;
  currentDataLayerName = dataLayerName;
  const inline =
    `window['${dataLayerName}'] = window['${dataLayerName}'] || [];` +
    `function gtag(){window['${dataLayerName}'].push(arguments);}` +
    `gtag('js', new Date());` +
    `gtag('config', '${gaId}'${debugMode ? ", { 'debug_mode': true }" : ""});`;
  const query = `id=${encodeURIComponent(gaId)}${dataLayerName !== "dataLayer" ? `&l=${encodeURIComponent(dataLayerName)}` : ""}`;
  return createElement(
    Fragment,
    null,
    createElement("script", { id: "_next-ga-init", nonce, dangerouslySetInnerHTML: { __html: inline } }),
    createElement("script", { id: "_next-ga", async: true, src: `https://www.googletagmanager.com/gtag/js?${query}`, nonce }),
  );
}

/** Props of `GoogleTagManager`. */
export interface GoogleTagManagerProps {
  readonly gtmId: string;
  readonly gtmScriptUrl?: string;
  readonly dataLayer?: Record<string, unknown>;
  readonly dataLayerName?: string;
  readonly auth?: string;
  readonly preview?: string;
  readonly nonce?: string;
}

/** Google Tag Manager container snippet. */
export function GoogleTagManager(props: GoogleTagManagerProps): ReactElement {
  const {
    gtmId,
    gtmScriptUrl = "https://www.googletagmanager.com/gtm.js",
    dataLayer,
    dataLayerName = "dataLayer",
    auth,
    preview,
    nonce,
  } = props;
  currentDataLayerName = dataLayerName;
  const gtmAuth = auth ? `+'&gtm_auth=${auth}'` : "";
  const gtmPreview = preview ? `+'&gtm_preview=${preview}&gtm_cookies_win=x'` : "";
  const inline =
    `(function(w,d,s,l,i){w[l]=w[l]||[];w[l].push({'gtm.start':new Date().getTime(),event:'gtm.js'});` +
    `var f=d.getElementsByTagName(s)[0],j=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';` +
    `j.async=true;j.src='${gtmScriptUrl}?id='+i+dl${gtmAuth}${gtmPreview};f.parentNode.insertBefore(j,f);` +
    `})(window,document,'script','${dataLayerName}','${gtmId}');`;
  const initial = dataLayer
    ? createElement("script", {
        id: "_next-gtm-init",
        nonce,
        dangerouslySetInnerHTML: {
          __html: `window['${dataLayerName}'] = window['${dataLayerName}'] || [];window['${dataLayerName}'].push(${JSON.stringify(dataLayer)});`,
        },
      })
    : null;
  return createElement(
    Fragment,
    null,
    initial,
    createElement("script", { id: "_next-gtm", nonce, dangerouslySetInnerHTML: { __html: inline } }),
  );
}

/** Pushes a gtag command (`sendGAEvent("event", "purchase", { value: 1 })`). */
export function sendGAEvent(...args: unknown[]): void {
  if (typeof window === "undefined") {
    return;
  }
  const layer = ((window as unknown as Record<string, unknown>)[currentDataLayerName] ??= []) as unknown[];
  // gtag.js expects an `arguments` object, so forward through a plain function.
  const push = function (this: unknown): void {
    // eslint-disable-next-line prefer-rest-params
    layer.push(arguments);
  };
  push.apply(null, args as []);
}

/** Pushes an object (or gtag-style array) into the GTM data layer. */
export function sendGTMEvent(data: Record<string, unknown> | unknown[], dataLayerName = currentDataLayerName): void {
  if (typeof window === "undefined") {
    return;
  }
  const layer = ((window as unknown as Record<string, unknown>)[dataLayerName] ??= []) as unknown[];
  layer.push(data);
}

/** Props of `YouTubeEmbed` (lite-youtube custom element). */
export interface YouTubeEmbedProps {
  readonly videoid: string;
  readonly height?: number;
  readonly width?: number;
  readonly playlabel?: string;
  readonly params?: string;
  readonly style?: string;
  readonly js_api?: boolean;
}

const LITE_YOUTUBE_SCRIPT = "https://cdn.jsdelivr.net/npm/@justinribeiro/lite-youtube@1.5.0/lite-youtube.js";

/** Lazy YouTube embed rendered with the `lite-youtube` web component. */
export function YouTubeEmbed(props: YouTubeEmbedProps): ReactElement {
  const { videoid, height = 315, width, playlabel, params, style, js_api } = props;
  const wrapperStyle: CSSProperties = { maxWidth: width === undefined ? "100%" : `${width}px`, height: `${height}px` };
  return createElement(
    "div",
    { style: wrapperStyle, "data-yolo-third-party": "youtube" },
    createElement("lite-youtube", {
      videoid,
      playlabel: playlabel ?? "Play",
      params,
      style,
      ...(js_api ? { "js-api": "" } : {}),
    }),
    createElement("script", { type: "module", async: true, src: LITE_YOUTUBE_SCRIPT }),
  );
}

/** Props of `GoogleMapsEmbed` (Maps Embed API). */
export interface GoogleMapsEmbedProps {
  readonly apiKey: string;
  readonly mode: "place" | "view" | "directions" | "streetview" | "search";
  readonly height?: number | string;
  readonly width?: number | string;
  readonly style?: string;
  readonly allowfullscreen?: boolean;
  readonly loading?: "eager" | "lazy";
  readonly q?: string;
  readonly center?: string;
  readonly zoom?: string;
  readonly maptype?: string;
  readonly language?: string;
  readonly region?: string;
}

/** Google Maps Embed API iframe (`https://www.google.com/maps/embed/v1/<mode>?key=...`). */
export function GoogleMapsEmbed(props: GoogleMapsEmbedProps): ReactElement {
  const { apiKey, mode, height = "auto", width = "auto", style, allowfullscreen = true, loading = "lazy", ...query } = props;
  const params = new URLSearchParams({ key: apiKey });
  for (const [key, value] of Object.entries(query)) {
    if (typeof value === "string" && value.length > 0) {
      params.set(key, value);
    }
  }
  return createElement(
    "div",
    { style: { height, width }, "data-yolo-third-party": "google-maps" },
    createElement("iframe", {
      loading,
      src: `https://www.google.com/maps/embed/v1/${mode}?${params.toString()}`,
      referrerPolicy: "no-referrer-when-downgrade",
      allowFullScreen: allowfullscreen,
      style: { border: 0, height: "100%", width: "100%", ...(style ? { cssText: style } : {}) },
    }),
  );
}
