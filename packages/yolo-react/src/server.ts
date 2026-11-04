// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/react/server — Native Bun.serve Fullstack React 19 RSC Flight + Streaming SSR + Expo API Server

import {
  createYoloReactConfig,
  createYoloTheme,
  parseYoloPreset,
  type YoloPresetId,
  type YoloReactConfig,
} from "./core.ts";

export interface RscFlightChunk {
  readonly id: number;
  readonly kind: "model" | "client-ref" | "hint";
  readonly payload: unknown;
}

export interface YoloReactServerOptions extends Partial<YoloReactConfig> {
  readonly port?: number;
  readonly hostname?: string;
  readonly apiRoutes?: Record<
    string,
    (req: Request) => Response | Promise<Response>
  >;
}

export function encodeRscFlightStream(
  chunks: readonly RscFlightChunk[],
): string {
  return chunks
    .map((chunk) => {
      const prefix =
        chunk.kind === "client-ref" ? "I" : chunk.kind === "hint" ? "H" : "";
      return `${chunk.id}:${prefix}${JSON.stringify(chunk.payload)}\n`;
    })
    .join("");
}

export function renderYoloSsrHtml(options: {
  title: string;
  preset: YoloPresetId;
  dark: boolean;
  bodyHtml: string;
  rscFlightPayload?: string;
}): string {
  const theme = createYoloTheme(options.preset, options.dark);
  const flightScript = options.rscFlightPayload
    ? `<script id="__YOLO_RSC_FLIGHT__" type="application/x-react-flight">${options.rscFlightPayload.replace(/</g, "\\u003c")}</script>`
    : "";

  return `<!DOCTYPE html>
<html lang="en" class="${options.dark ? "dark" : "light"}" data-yolo-preset="${theme.canonicalId}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${options.title}</title>
  <style id="yolo-theme-vars">${theme.cssText}</style>
</head>
<body style="margin:0;background:var(--yolo-bg-primary);color:var(--yolo-text-primary);font-family:var(--yolo-font-sans);">
  <div id="root">${options.bodyHtml}</div>
  ${flightScript}
</body>
</html>`;
}

export function createYoloReactServer(options: YoloReactServerOptions = {}) {
  const config = createYoloReactConfig(options);

  const handleRequest = async (req: Request): Promise<Response> => {
    const url = new URL(req.url);
    const reqPreset = url.searchParams.get("preset")
      ? parseYoloPreset(url.searchParams.get("preset")!)
      : config.preset;
    const reqDark = url.searchParams.get("dark")
      ? url.searchParams.get("dark") !== "false"
      : config.dark;

    if (url.pathname === config.themeCssEndpoint) {
      const theme = createYoloTheme(reqPreset, reqDark);
      return new Response(theme.cssText, {
        headers: {
          "content-type": "text/css; charset=utf-8",
          "cache-control": "public, max-age=3600",
        },
      });
    }

    if (url.pathname === config.rscEndpoint) {
      const theme = createYoloTheme(reqPreset, reqDark);
      const flight = encodeRscFlightStream([
        {
          id: 0,
          kind: "model",
          payload: [
            "$",
            "main",
            null,
            {
              "data-yolo-preset": theme.canonicalId,
              children: `${config.title} (${theme.displayName})`,
            },
          ],
        },
      ]);
      return new Response(flight, {
        headers: {
          "content-type": "text/x-component; charset=utf-8",
          "x-yolo-engine": "bun+rust-rsc",
        },
      });
    }

    if (url.pathname === "/_yolo/health") {
      return Response.json({
        status: "ok",
        runtime: `bun@${Bun.version}`,
        preset: config.preset,
        rscEndpoint: config.rscEndpoint,
        themeCssEndpoint: config.themeCssEndpoint,
      });
    }

    const customHandler = options.apiRoutes?.[url.pathname];
    if (customHandler) {
      return await customHandler(req);
    }

    const flight = encodeRscFlightStream([
      {
        id: 0,
        kind: "model",
        payload: ["$", "div", null, { children: config.title }],
      },
    ]);
    const html = renderYoloSsrHtml({
      title: config.title,
      preset: reqPreset,
      dark: reqDark,
      bodyHtml: `<main data-yolo-app="${config.appId}"><h1>${config.title}</h1></main>`,
      rscFlightPayload: flight,
    });

    return new Response(html, {
      headers: { "content-type": "text/html; charset=utf-8" },
    });
  };

  const start = () =>
    Bun.serve({
      port: options.port ?? 0,
      hostname: options.hostname ?? "127.0.0.1",
      fetch: handleRequest,
    });

  return {
    config,
    handleRequest,
    start,
  };
}
