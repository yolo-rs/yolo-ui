// SPDX-License-Identifier: MIT OR Apache-2.0
// Server rendering of the `next/dynamic`, `next/script` and `next/config` shims.
import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToReadableStream, renderToString } from "react-dom/server";
import { PHASE_PRODUCTION_BUILD } from "../src/app/constants";
import dynamic from "../src/app/dynamic";
import getConfig, { RUNTIME_CONFIG_GLOBAL, runtimeConfigDefine } from "../src/app/runtime-config";
import Script from "../src/app/script";

const Hello = ({ name }: { name: string }) => createElement("p", null, `hello ${name}`);
const Loading = () => createElement("span", null, "loading");

async function renderAll(element: Parameters<typeof renderToString>[0]): Promise<string> {
  const stream = await renderToReadableStream(element);
  await stream.allReady;
  return new Response(stream).text();
}

describe("dynamic", () => {
  test("ssr true streams the loaded component after the fallback", async () => {
    const Dynamic = dynamic(() => Promise.resolve({ default: Hello }), { loading: Loading });
    const html = await renderAll(createElement(Dynamic, { name: "ssr" }));
    expect(html).toContain("hello ssr");
    expect(html).toContain("<span>loading</span>");
    expect(html).toContain("<!--$?-->");
  });

  test("ssr true renders the fallback synchronously with renderToString", () => {
    const Dynamic = dynamic(() => Promise.resolve({ default: Hello }), { loading: Loading });
    const html = renderToString(createElement(Dynamic, { name: "sync" }));
    expect(html).toContain("<span>loading</span>");
    expect(html).not.toContain("hello sync");
  });

  test("ssr false renders only the loading component on the server", async () => {
    let loaded = false;
    const Dynamic = dynamic(
      () => {
        loaded = true;
        return Promise.resolve(Hello);
      },
      { loading: Loading, ssr: false },
    );
    const html = await renderAll(createElement(Dynamic, { name: "client" }));
    expect(html).toBe("<span>loading</span>");
    expect(loaded).toBe(false);
  });

  test("ssr false without loading renders nothing", () => {
    const Dynamic = dynamic(() => Promise.resolve(Hello), { ssr: false });
    expect(renderToString(createElement(Dynamic, { name: "x" }))).toBe("");
  });

  test("legacy object form and loader resolving to a component", async () => {
    const Dynamic = dynamic<{ name: string }>({ loader: () => Promise.resolve(Hello), loading: Loading });
    const html = await renderAll(createElement(Dynamic, { name: "legacy" }));
    expect(html).toContain("hello legacy");
  });

  test("needs a loader", () => {
    expect(() => dynamic({ loading: Loading })).toThrow(/loader/);
  });
});

describe("Script", () => {
  test("afterInteractive renders an async script with src", () => {
    const html = renderToString(createElement(Script, { src: "/a.js", id: "a", strategy: "afterInteractive" }));
    expect(html).toContain('src="/a.js"');
    expect(html).toContain("async");
    expect(html).toContain('id="a"');
  });

  test("beforeInteractive and worker render like afterInteractive", () => {
    for (const strategy of ["beforeInteractive", "worker"] as const) {
      const html = renderToString(createElement(Script, { src: "/b.js", strategy }));
      expect(html).toContain('<script');
      expect(html).toContain('src="/b.js"');
    }
  });

  test("lazyOnload renders nothing on the server", () => {
    expect(renderToString(createElement(Script, { src: "/lazy.js", strategy: "lazyOnload" }))).toBe("");
  });

  test("inline children and dangerouslySetInnerHTML", () => {
    const inline = renderToString(createElement(Script, { id: "inline" }, "console.log(1)"));
    expect(inline).toBe('<script id="inline">console.log(1)</script>');
    const html = renderToString(createElement(Script, { dangerouslySetInnerHTML: { __html: "window.x = 1" } }));
    expect(html).toBe("<script>window.x = 1</script>");
    expect(renderToString(createElement(Script, {}))).toBe("");
  });
});

describe("getConfig", () => {
  const g = globalThis as Record<string, unknown>;

  test("defaults to empty objects", () => {
    delete g[RUNTIME_CONFIG_GLOBAL];
    expect(getConfig()).toEqual({ serverRuntimeConfig: {}, publicRuntimeConfig: {} });
  });

  test("reads the define global as a JSON string or an object", () => {
    const config = { serverRuntimeConfig: { secret: "s" }, publicRuntimeConfig: { api: "/api" } };
    const define = runtimeConfigDefine(config, "ssr");
    expect(Object.keys(define)).toEqual([RUNTIME_CONFIG_GLOBAL]);
    // The define value is a JS string literal; evaluate it the way the bundler would.
    g[RUNTIME_CONFIG_GLOBAL] = JSON.parse(define[RUNTIME_CONFIG_GLOBAL]!);
    expect(getConfig()).toEqual(config);
    g[RUNTIME_CONFIG_GLOBAL] = config;
    expect(getConfig()).toEqual(config);
    g[RUNTIME_CONFIG_GLOBAL] = "{not json";
    expect(getConfig()).toEqual({ serverRuntimeConfig: {}, publicRuntimeConfig: {} });
    delete g[RUNTIME_CONFIG_GLOBAL];
  });

  test("the client layer only gets publicRuntimeConfig", () => {
    const define = runtimeConfigDefine({ serverRuntimeConfig: { secret: "s" }, publicRuntimeConfig: { api: "/api" } }, "client");
    g[RUNTIME_CONFIG_GLOBAL] = JSON.parse(define[RUNTIME_CONFIG_GLOBAL]!);
    expect(getConfig()).toEqual({ serverRuntimeConfig: {}, publicRuntimeConfig: { api: "/api" } });
    delete g[RUNTIME_CONFIG_GLOBAL];
  });
});

test("constants re-export the phases", () => {
  expect(PHASE_PRODUCTION_BUILD).toBe("phase-production-build");
});
