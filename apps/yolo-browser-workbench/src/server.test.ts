import { afterEach, describe, expect, test } from "bun:test";
import { startWorkbenchUi } from "./server.ts";

describe("@yolo-rs/browser-workbench", () => {
  const servers: Array<{ stop: (force?: boolean) => void }> = [];
  afterEach(() => {
    for (const server of servers.splice(0)) server.stop(true);
  });

  test("renders the UI in yolo-ui and proxies actions and media to the yolo-web API", async () => {
    const requests: Array<{ path: string; body?: unknown }> = [];
    const backend = Bun.serve({
      hostname: "127.0.0.1",
      port: 0,
      async fetch(request) {
        const url = new URL(request.url);
        if (url.pathname === "/api/status") {
          requests.push({ path: url.pathname });
          return Response.json({
            browserConnected: true,
            browserVersion: { Browser: "Chrome/159.0.0.0" },
          });
        }
        if (url.pathname === "/api/navigate" || url.pathname === "/api/eval") {
          requests.push({ path: url.pathname, body: await request.json() });
          return Response.json({
            ok: true,
            navigatedUrl: "https://example.com/",
            value: 42,
          });
        }
        if (url.pathname === "/stream") {
          requests.push({ path: url.pathname });
          return new Response("frame", {
            headers: {
              "Content-Type": "multipart/x-mixed-replace; boundary=yolo-frame",
            },
          });
        }
        return new Response("Not Found", { status: 404 });
      },
    });
    servers.push(backend);
    const ui = startWorkbenchUi({
      backendUrl: `http://127.0.0.1:${backend.port}`,
      httpPort: 0,
      cdpPort: 9222,
      braveShields: true,
    });
    servers.push(ui);
    const baseUrl = `http://127.0.0.1:${ui.port}`;

    const home = await fetch(baseUrl);
    const html = await home.text();
    expect(home.status).toBe(200);
    expect(html).toContain("YOLO-WEB");
    expect(html).toContain("Connected browser viewport");
    expect(html).toContain("Chrome/159.0.0.0");

    const navigate = await fetch(`${baseUrl}/navigate`, {
      method: "POST",
      body: new URLSearchParams({
        url: "https://example.com",
        theme: "yolo-neutral",
      }),
    });
    expect(navigate.status).toBe(200);
    expect(await navigate.text()).toContain(
      "Navigated to https://example.com/",
    );

    const invalid = await fetch(`${baseUrl}/navigate`, {
      method: "POST",
      body: new URLSearchParams({ url: "javascript:alert(1)" }),
    });
    expect(invalid.status).toBe(400);

    const evaluate = await fetch(`${baseUrl}/evaluate`, {
      method: "POST",
      body: new URLSearchParams({ expression: "6 * 7" }),
    });
    expect(evaluate.status).toBe(200);
    expect(await evaluate.text()).toContain("&quot;value&quot;: 42");

    const stream = await fetch(`${baseUrl}/stream`);
    expect(stream.headers.get("content-type")).toContain(
      "multipart/x-mixed-replace",
    );
    expect(await stream.text()).toBe("frame");
    expect(requests).toContainEqual({
      path: "/api/navigate",
      body: { url: "https://example.com" },
    });
    expect(requests).toContainEqual({
      path: "/api/eval",
      body: { expression: "6 * 7" },
    });
    expect(requests.some((request) => request.path === "/stream")).toBe(true);
  });
});
