#!/usr/bin/env bun
import { Hono } from "hono";
import type { ContentfulStatusCode } from "hono/utils/http-status";
import { renderHumanBrowserHtml, resolveThemePreset } from "./ui.tsx";

export type WorkbenchUiConfig = {
  backendUrl: string;
  httpPort: number;
  hostname?: string;
  cdpPort: number;
  braveShields: boolean;
  themePreset?: string;
};

export function validateBrowserNavigationUrl(rawUrl: string): URL | null {
  try {
    const url = new URL(rawUrl);
    if (url.protocol === "http:" || url.protocol === "https:") return url;
    if (url.href === "about:blank") return url;
    if (url.protocol === "data:") return url;
  } catch {
    // Invalid absolute URL.
  }
  return null;
}

function apiURL(config: WorkbenchUiConfig, path: string): string {
  return new URL(
    path,
    config.backendUrl.endsWith("/")
      ? config.backendUrl
      : `${config.backendUrl}/`,
  ).href;
}

async function readBackendJson(
  config: WorkbenchUiConfig,
  path: string,
  init?: RequestInit,
) {
  const response = await fetch(apiURL(config, path), {
    ...init,
    signal: AbortSignal.timeout(10_000),
  });
  const payload = (await response.json().catch(() => ({}))) as Record<
    string,
    unknown
  >;
  return { response, payload };
}

export function createWorkbenchUi(config: WorkbenchUiConfig) {
  const app = new Hono();

  app.get("/healthz", (c) =>
    c.json({ ok: true, backendUrl: config.backendUrl }),
  );

  app.get("/", async (c) => {
    let notice: string | undefined;
    let telemetry: string | undefined;
    try {
      const { response, payload } = await readBackendJson(
        config,
        "/api/status",
      );
      if (!response.ok)
        throw new Error(`YOLO-Web API returned HTTP ${response.status}`);
      telemetry = JSON.stringify(payload, null, 2);
      if (!payload.browserConnected)
        notice = String(
          payload.browserIntegrationMessage ??
            "Waiting for the YOLO-Web CDP service.",
        );
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
    const themePreset = resolveThemePreset(
      c.req.query("theme") ?? config.themePreset,
    );
    return c.html(
      renderHumanBrowserHtml(
        {
          cdpPort: config.cdpPort,
          braveShields: config.braveShields,
          themePreset,
        },
        { themePreset, notice, telemetry },
      ),
    );
  });

  app.post("/navigate", async (c) => {
    const form = await c.req.formData();
    const themePreset = resolveThemePreset(
      String(form.get("theme") ?? config.themePreset),
    );
    const rawUrl = String(form.get("url") ?? "").trim();
    if (!rawUrl || !validateBrowserNavigationUrl(rawUrl)) {
      return c.html(
        renderHumanBrowserHtml(
          {
            cdpPort: config.cdpPort,
            braveShields: config.braveShields,
            themePreset,
          },
          {
            themePreset,
            notice: "Enter a valid http, https, data, or about:blank address.",
          },
        ),
        400,
      );
    }
    try {
      const { response, payload } = await readBackendJson(
        config,
        "/api/navigate",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: rawUrl }),
        },
      );
      const notice = response.ok
        ? String(
            payload.navigatedUrl
              ? `Navigated to ${payload.navigatedUrl}`
              : "Navigation complete.",
          )
        : String(
            payload.error ?? `YOLO-Web API returned HTTP ${response.status}`,
          );
      return c.html(
        renderHumanBrowserHtml(
          {
            cdpPort: config.cdpPort,
            braveShields: config.braveShields,
            themePreset,
          },
          { themePreset, notice },
        ),
        response.status as ContentfulStatusCode,
      );
    } catch (error) {
      const notice = error instanceof Error ? error.message : String(error);
      return c.html(
        renderHumanBrowserHtml(
          {
            cdpPort: config.cdpPort,
            braveShields: config.braveShields,
            themePreset,
          },
          { themePreset, notice },
        ),
        503,
      );
    }
  });

  app.post("/evaluate", async (c) => {
    const form = await c.req.formData();
    const themePreset = resolveThemePreset(
      String(form.get("theme") ?? config.themePreset),
    );
    const expression =
      String(form.get("expression") ?? "document.title").trim() ||
      "document.title";
    try {
      const { response, payload } = await readBackendJson(config, "/api/eval", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ expression }),
      });
      const notice = response.ok
        ? `Evaluated in the connected browser: ${expression}`
        : String(
            payload.error ?? `YOLO-Web API returned HTTP ${response.status}`,
          );
      return c.html(
        renderHumanBrowserHtml(
          {
            cdpPort: config.cdpPort,
            braveShields: config.braveShields,
            themePreset,
          },
          { themePreset, notice, evaluation: JSON.stringify(payload, null, 2) },
        ),
        response.status as ContentfulStatusCode,
      );
    } catch (error) {
      const notice = error instanceof Error ? error.message : String(error);
      return c.html(
        renderHumanBrowserHtml(
          {
            cdpPort: config.cdpPort,
            braveShields: config.braveShields,
            themePreset,
          },
          {
            themePreset,
            notice,
            evaluation: JSON.stringify({ expression, error: notice }, null, 2),
          },
        ),
        503,
      );
    }
  });

  app.get("/stream", async (c) => {
    const controller = new AbortController();
    const timeout = setTimeout(
      () =>
        controller.abort(
          new Error("Timed out connecting to YOLO-Web media stream."),
        ),
      10_000,
    );
    try {
      const response = await fetch(apiURL(config, "/stream"), {
        signal: controller.signal,
      });
      clearTimeout(timeout);
      return new Response(response.body, {
        status: response.status,
        headers: response.headers,
      });
    } catch (error) {
      clearTimeout(timeout);
      return c.text(
        error instanceof Error ? error.message : String(error),
        503,
      );
    }
  });

  return app;
}

export function startWorkbenchUi(config: WorkbenchUiConfig) {
  const app = createWorkbenchUi(config);
  return Bun.serve({
    port: config.httpPort,
    hostname: config.hostname ?? "127.0.0.1",
    fetch: app.fetch,
  });
}

if (import.meta.main) {
  const backendUrl = process.env.YOLO_WEB_API_URL ?? "http://127.0.0.1:8080";
  const port = Number(Bun.argv[2] ?? process.env.YOLO_UI_HTTP_PORT ?? 8081);
  const cdpPort = Number(process.env.YOLO_CDP_PORT ?? 9222);
  const server = startWorkbenchUi({
    backendUrl,
    httpPort: port,
    cdpPort,
    braveShields: true,
  });
  console.log(
    `Yolo UI browser workbench at http://127.0.0.1:${server.port} (YOLO-Web API ${backendUrl})`,
  );
}
