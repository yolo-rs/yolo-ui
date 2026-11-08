// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeAll, describe, expect, spyOn, test } from "bun:test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { createElement, Fragment, Suspense, useState, type ReactNode } from "react";
import {
  buildRouteElement,
  decodeFlightModel,
  encodeFlightModel,
  escapeInlineScript,
  flightInlineScript,
  isReactServerEnvironment,
  loadReactServerDom,
  loadReactServerDomClient,
  modelToElement,
  registerInlineClientComponent,
  renderFlightToHtml,
  renderToFlight,
  resolveFlightModel,
  serializeElementTree,
  startRscWorker,
  type ClientReferenceLike,
  type FlightElement,
} from "../src/rsc.ts";

const CLIENT_REF = Symbol.for("react.client.reference");
const ELEMENT = Symbol.for("react.transitional.element");

function clientRef(id: string): ClientReferenceLike {
  return { $$typeof: CLIENT_REF, $$id: id, $$async: false };
}

function elementOf(type: unknown, props: Record<string, unknown>, key: string | null = null): FlightElement {
  return { $$typeof: ELEMENT, type, key, ref: null, props };
}

async function text(stream: ReadableStream<Uint8Array>): Promise<string> {
  return new Response(stream).text();
}

describe("encodeFlightModel / decodeFlightModel", () => {
  test("host elements use the Rust-compatible row format", () => {
    const tree = createElement("main", { className: "x" }, createElement("h1", null, "Hi"), "text");
    const flight = encodeFlightModel(tree);
    expect(flight).toBe('0:["$","main",null,{"className":"x","children":[["$","h1",null,{"children":"Hi"}],"text"]}]\n');
    const decoded = decodeFlightModel(flight);
    const root = decoded.root as FlightElement;
    expect(root.type).toBe("main");
    const children = root.props.children as [FlightElement, string];
    expect(children[0].type).toBe("h1");
    expect(children[0].props.children).toBe("Hi");
    expect(children[1]).toBe("text");
  });

  test("client references emit import rows and $L placeholders", () => {
    const tree = elementOf("section", { children: elementOf(clientRef("./Counter.tsx#Counter"), { initial: 2 }, "k1") });
    const flight = encodeFlightModel(tree);
    expect(flight).toBe('1:I["./Counter.tsx",[],"Counter"]\n0:["$","section",null,{"children":["$","$L1","k1",{"initial":2}]}]\n');
    const decoded = decodeFlightModel(flight);
    const section = decoded.root as FlightElement;
    const counter = section.props.children as FlightElement;
    expect((counter.type as ClientReferenceLike).$$id).toBe("./Counter.tsx#Counter");
    expect(counter.key).toBe("k1");
    expect(decoded.imports.get(1)?.exportName).toBe("Counter");
    const withManifest = encodeFlightModel(tree, { clientRefs: { "./Counter.tsx#Counter": { id: "chunk-abc", name: "Counter", chunks: ["static/chunks/counter.js"] } } });
    expect(withManifest).toContain('1:I["chunk-abc",["static/chunks/counter.js"],"Counter"]');
  });

  test("suspense, keyed fragments, dates, bigint, undefined, maps and sets", () => {
    const tree = createElement(
      "div",
      null,
      createElement(Suspense, { fallback: createElement("p", null, "Loading") }, createElement("b", null, "ok")),
      createElement(Fragment, { key: "frag" }, "a", "b"),
      createElement(Fragment, null, "flat"),
    );
    const flight = encodeFlightModel(tree);
    expect(flight).toContain('["$","$Sreact.suspense",null,{"fallback":["$","p",null,{"children":"Loading"}],"children":["$","b",null,{"children":"ok"}]}]');
    expect(flight).toContain('["$","$Sreact.fragment","frag",{"children":["a","b"]}]');
    expect(flight).toContain('"flat"');
    const decoded = decodeFlightModel(flight) as { root: FlightElement };
    const [suspense, fragment] = decoded.root.props.children as FlightElement[];
    expect(suspense?.type).toBe(Symbol.for("react.suspense"));
    expect(fragment?.type).toBe(Symbol.for("react.fragment"));

    const model = { when: new Date("2026-01-01T00:00:00.000Z"), n: 10n, u: undefined, s: "$money", m: new Map([["k", 1]]), set: new Set([1, 2]), nan: Number.NaN, neg: -0 };
    const encoded = encodeFlightModel(model);
    expect(encoded).toContain('"when":"$D2026-01-01T00:00:00.000Z"');
    expect(encoded).toContain('"n":"$n10"');
    expect(encoded).toContain('"u":"$undefined"');
    expect(encoded).toContain('"s":"$$money"');
    expect(encoded).toContain('"m":"$Q1"');
    expect(encoded).toContain('"set":"$W2"');
    expect(encoded).toContain('"nan":"$NaN"');
    const back = decodeFlightModel(encoded).root as typeof model;
    expect(back.when).toEqual(model.when);
    expect(back.n).toBe(10n);
    expect(back.u).toBeUndefined();
    expect(back.s).toBe("$money");
    expect(back.m.get("k")).toBe(1);
    expect(back.set.has(2)).toBe(true);
    expect(Number.isNaN(back.nan)).toBe(true);
    expect(Object.is(back.neg, -0)).toBe(true);
  });

  test("server references and error rows", () => {
    const action = Object.assign(async () => undefined, { $$typeof: Symbol.for("react.server.reference"), $$id: "00abc", $$bound: null });
    const flight = encodeFlightModel({ a: action });
    expect(flight).toBe('1:{"id":"00abc","bound":null}\n0:{"a":"$F1"}\n');
    const decoded = decodeFlightModel(flight).root as { a: { $$id: string } };
    expect(decoded.a.$$id).toBe("00abc");
    const withError = decodeFlightModel('1:E{"message":"boom","digest":"d"}\n0:{"ok":true}\n');
    expect(withError.errors[0]).toEqual({ id: 1, message: "boom", digest: "d" });
    expect(() => encodeFlightModel({ fn: () => 1 })).toThrow("Functions cannot be passed directly to Client Components");
    expect(() => encodeFlightModel(Promise.resolve(1))).toThrow("pending Promise");
  });

  test("resolveFlightModel awaits nested promises", async () => {
    const resolved = (await resolveFlightModel({ a: Promise.resolve([1, Promise.resolve(2)]) })) as { a: number[] };
    expect(resolved.a).toEqual([1, 2]);
  });
});

describe("serializeElementTree", () => {
  function Layout({ children }: { children: ReactNode }) {
    return createElement("html", { lang: "en" }, createElement("body", null, children));
  }
  function Page() {
    return createElement("main", null, "Hello");
  }
  async function AsyncPage({ params }: { params: Promise<{ slug: string }> }) {
    const { slug } = await params;
    return createElement("article", null, `slug:${slug}`);
  }
  function Counter({ initial }: { initial: number }) {
    const [n] = useState(initial);
    return createElement("b", null, n);
  }

  test("invokes sync and async server components and strips function props on host elements", async () => {
    const warn = spyOn(console, "warn").mockImplementation(() => undefined);
    const tree = createElement(Layout, null, createElement(Page), createElement(AsyncPage, { params: Promise.resolve({ slug: "hello" }) }), createElement("button", { onClick: () => undefined }, "x"));
    const resolved = await serializeElementTree(tree);
    const flight = encodeFlightModel(resolved);
    expect(flight).toContain('["$","main",null,{"children":"Hello"}]');
    expect(flight).toContain('["$","article",null,{"children":"slug:hello"}]');
    expect(flight).toContain('["$","button",null,{"children":"x"}]');
    expect(flight).not.toContain("onClick");
    warn.mockRestore();
  });

  test("hook-using components become inline client references", async () => {
    const resolved = (await serializeElementTree(createElement("div", null, createElement(Counter, { initial: 3 })))) as FlightElement;
    const child = resolved.props.children as FlightElement;
    expect((child.type as ClientReferenceLike).$$id).toMatch(/^yolo-inline:\d+:Counter#default$/);
    expect(child.props).toEqual({ initial: 3 });
    const flight = encodeFlightModel(resolved);
    expect(flight).toMatch(/1:I\["yolo-inline:\d+:Counter",\[\],"default"\]/);
    const element = await modelToElement(decodeFlightModel(flight).root);
    const { renderToReadableStream } = await import("react-dom/server");
    const html = await text(await renderToReadableStream(element));
    expect(html).toContain("<b>3</b>");
  });

  test("registered inline components are reused", () => {
    const first = registerInlineClientComponent(Counter);
    expect(registerInlineClientComponent(Counter)).toBe(first);
  });

  test("buildRouteElement nests layouts, templates, loading and the page", async () => {
    const calls: string[] = [];
    const modules = {
      page: { default: async ({ params }: { params: Promise<{ slug: string }> }) => createElement("p", null, (await params).slug) },
      layouts: [
        { default: ({ children }: { children: ReactNode }) => createElement("html", null, createElement("body", null, children)) },
        { default: ({ children }: { children: ReactNode }) => createElement("section", { "data-blog": true }, children) },
      ],
      templates: [{ default: ({ children }: { children: ReactNode }) => createElement("div", { "data-template": true }, children) }],
      loading: { default: () => createElement("p", { role: "status" }, "Loading") },
      params: { slug: "hello" },
      searchParams: { q: "1" },
    };
    const element = buildRouteElement(modules);
    calls.push(String(element.type === modules.layouts[0]?.default));
    const flight = encodeFlightModel(await serializeElementTree(element));
    expect(calls).toEqual(["true"]);
    expect(flight).toContain('["$","html",null,{"children":["$","body",null,{"children":["$","section",null,{"data-blog":true,"children":["$","div",null,{"data-template":true,"children":["$","$Sreact.suspense",null,{"fallback":["$","p",null,{"role":"status","children":"Loading"}],"children":["$","p",null,{"children":"hello"}]}]}]}]}]}]');
  });
});

describe("renderFlightToHtml", () => {
  function Layout({ children }: { children: ReactNode }) {
    return createElement("html", { lang: "en" }, createElement("head", null, createElement("title", null, "T")), createElement("body", null, children));
  }
  function Page() {
    return createElement("main", { id: "app" }, "Hello </script> world");
  }

  test("escapes inline script payloads", () => {
    const script = flightInlineScript('0:["$","script",null,{"children":"</script><!--"}]');
    expect(script.startsWith("<script>(self.__next_f=self.__next_f||[]).push([1,")).toBe(true);
    expect(script.endsWith("])</script>")).toBe(true);
    expect(script.split("</script>")).toHaveLength(2);
    expect(script).toContain("\\u003c/script");
    expect(escapeInlineScript("<!--")).toBe("\\u003c!--");
    expect(flightInlineScript("x", "abc")).toContain('<script nonce="abc">');
  });

  test("fallback path rebuilds the tree and inlines the Flight payload", async () => {
    const flightText = encodeFlightModel(await serializeElementTree(createElement(Layout, null, createElement(Page))));
    const flightStream = new Response(flightText).body as ReadableStream<Uint8Array>;
    const html = await text(await renderFlightToHtml({ flightStream, fallback: true, headHtml: '<meta name="x" content="1"/>', nonce: "n1" }));
    expect(html).toContain('<main id="app">Hello &lt;/script&gt; world</main>');
    expect(html).toContain('<head><meta name="x" content="1"/>');
    expect(html).toContain('<script nonce="n1">(self.__next_f=self.__next_f||[]).push([0])</script>');
    expect(html).toContain('<script nonce="n1">(self.__next_f=self.__next_f||[]).push([1,"0:[');
    expect(html.indexOf("<main")).toBeLessThan(html.indexOf("__next_f=self.__next_f||[]).push([1"));
  });

  test("real client.edge path consumes the fallback encoder output", async () => {
    const client = await loadReactServerDomClient();
    expect(client).toBeDefined();
    const flightText = encodeFlightModel(await serializeElementTree(createElement(Layout, null, createElement(Page))));
    const flightStream = new Response(flightText).body as ReadableStream<Uint8Array>;
    const html = await text(await renderFlightToHtml({ flightStream, inlineFlight: false }));
    expect(html).toContain('<main id="app">');
    expect(html).not.toContain("__next_f");
  });

  test("renderToFlight falls back to the encoder outside react-server", async () => {
    expect(isReactServerEnvironment()).toBe(false);
    expect(await loadReactServerDom()).toBeUndefined();
    const stream = await renderToFlight({ element: createElement("p", null, "flight") });
    expect(await text(stream)).toBe('0:["$","p",null,{"children":"flight"}]\n');
  });
});

describe("rsc worker", () => {
  const root = `${import.meta.dir}/.tmp-rsc-${process.pid}`;

  beforeAll(() => {
    mkdirSync(`${root}/app/blog/[slug]`, { recursive: true });
    writeFileSync(
      `${root}/app/layout.tsx`,
      'export default function Layout({ children }: { children: React.ReactNode }) { return <html lang="en"><body><nav>nav</nav>{children}</body></html>; }\n',
    );
    writeFileSync(
      `${root}/app/blog/[slug]/page.tsx`,
      'export default async function Page({ params }: { params: Promise<{ slug: string }> }) { const { slug } = await params; if (slug === "missing") { const e = new Error("NEXT_HTTP_ERROR_FALLBACK"); (e as Error & { digest: string }).digest = "NEXT_HTTP_ERROR_FALLBACK;404"; throw e; } return <article><h1>{slug}</h1></article>; }\n',
    );
    writeFileSync(`${root}/app/blog/[slug]/loading.tsx`, "export default function Loading() { return <p>loading</p>; }\n");
  });

  afterAll(async () => {
    // The killed worker may still hold its cwd for a moment on Windows.
    for (let attempt = 0; attempt < 10; attempt += 1) {
      try {
        rmSync(root, { recursive: true, force: true });
        return;
      } catch {
        await Bun.sleep(100);
      }
    }
  });

  test("renders Flight with the real react-server renderer and reports router signals", async () => {
    const worker = await startRscWorker({ root, timeoutMs: 30_000 });
    try {
      expect(worker.port).toBeGreaterThan(0);
      const health = await fetch(`${worker.url}/health`).then((r) => r.json() as Promise<{ reactServer: boolean }>);
      expect(health.reactServer).toBe(true);
      const response = await worker.render({
        pathname: "/blog/hello",
        search: "?q=1",
        headers: { host: "localhost:3000" },
        routeFile: `${root}/app/blog/[slug]/page.tsx`,
        layoutFiles: [`${root}/app/layout.tsx`],
        loadingFile: `${root}/app/blog/[slug]/loading.tsx`,
        params: { slug: "hello" },
      });
      expect(response.status).toBe(200);
      expect(response.headers.get("content-type")).toBe("text/x-component");
      const flight = await response.text();
      expect(flight).toContain('"$Sreact.suspense"');
      expect(flight).toContain('["$","h1",null,{"children":"hello"}');
      expect(flight).toContain('["$","nav",null,{"children":"nav"}');
      const html = await text(await renderFlightToHtml({ flightStream: new Response(flight).body as ReadableStream<Uint8Array>, fallback: true }));
      expect(html).toContain("<nav>nav</nav>");
      expect(html).toContain("<h1>hello</h1>");
      const missing = await worker.render({
        pathname: "/blog/missing",
        search: "",
        headers: { host: "localhost:3000" },
        routeFile: `${root}/app/blog/[slug]/page.tsx`,
        layoutFiles: [`${root}/app/layout.tsx`],
        params: { slug: "missing" },
      });
      expect(missing.headers.get("x-yolo-status")).toBe("404");
    } finally {
      worker.stop();
    }
  }, 40_000);
});
