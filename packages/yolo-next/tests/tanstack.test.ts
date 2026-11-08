// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToString } from "react-dom/server";
import {
  HydrationScript,
  YoloQueryProvider,
  createRequestQueryClient,
  createServerFn,
  dehydrateForHtml,
  escapeInlineJson,
  invokeServerFn,
  loaderFromPage,
  registerTanStackRouter,
  serverFnId,
  setupRouterSsrQueryIntegration,
  unregisterTanStackRouter,
  useTanStackRouter,
} from "../src/tanstack.ts";

describe("query client hydration", () => {
  test("createRequestQueryClient sets a 60 s staleTime", () => {
    const client = createRequestQueryClient();
    expect(client.getDefaultOptions().queries?.staleTime).toBe(60_000);
  });

  test("dehydrateForHtml escapes script-breaking characters", () => {
    const client = createRequestQueryClient();
    client.setQueryData(["post"], { html: "<script>alert(1)</script> & \u2028" });
    const json = dehydrateForHtml(client);
    expect(json).not.toContain("<");
    expect(json).not.toContain(">");
    expect(json).toContain("\\u003cscript\\u003e");
    expect(json).toContain("\\u2028");
    const parsed = JSON.parse(json) as { queries: Array<{ queryKey: string[] }> };
    expect(parsed.queries[0]?.queryKey).toEqual(["post"]);
    expect(escapeInlineJson('{"a":"</script>"}')).toBe('{"a":"\\u003c/script\\u003e"}');
  });

  test("HydrationScript and YoloQueryProvider render on the server", () => {
    const client = createRequestQueryClient();
    client.setQueryData(["k"], 1);
    const script = renderToString(createElement(HydrationScript, { state: dehydrateForHtml(client), nonce: "n1" }));
    expect(script).toContain("window.__YOLO_QUERY_STATE__=");
    expect(script).toContain('nonce="n1"');
    const html = renderToString(createElement(YoloQueryProvider, { client }, createElement("p", null, "child")));
    expect(html).toContain("<p>child</p>");
  });
});

describe("TanStack router registry and ssr-query integration", () => {
  test("registerTanStackRouter round-trips through useTanStackRouter", () => {
    expect(useTanStackRouter()).toBeUndefined();
    const router = { navigate: () => undefined };
    registerTanStackRouter(router);
    expect(useTanStackRouter()).toBe(router);
    unregisterTanStackRouter();
    expect(useTanStackRouter()).toBeUndefined();
  });

  test("setupRouterSsrQueryIntegration resolves a boolean without throwing", async () => {
    const result = await setupRouterSsrQueryIntegration({ router: {}, queryClient: createRequestQueryClient() });
    expect(typeof result).toBe("boolean");
  });
});

describe("createServerFn", () => {
  test("validator + handler run directly on the server", async () => {
    const add = createServerFn({ id: "test/add", method: "POST" })
      .validator((input) => input as { a: number; b: number })
      .handler(async ({ data, method, id }) => ({ sum: data.a + data.b, method, id }));
    expect(await add({ data: { a: 1, b: 2 } })).toEqual({ sum: 3, method: "POST", id: "test/add" });
    expect(add.id).toBe("test/add");
    expect(add.method).toBe("POST");
    expect(await invokeServerFn("test/add", { a: 2, b: 3 })).toEqual({ sum: 5, method: "POST", id: "test/add" });
    await expect(invokeServerFn("missing", {})).rejects.toThrow(/Unknown server function/);
  });

  test("serverFnId delegates to serverReferenceId", () => {
    const id = serverFnId("app/actions.ts", "createPost");
    expect(typeof id).toBe("string");
    expect(id.length).toBeGreaterThan(0);
    expect(serverFnId("app/actions.ts", "createPost")).toBe(id);
  });
});

describe("loaderFromPage", () => {
  test("resolves params, searchParams and metadata", async () => {
    const loader = loaderFromPage({
      default: async () => null,
      generateMetadata: async (props) => ({ title: `Post ${(await props.params).slug}` }),
    });
    const result = await loader({ params: { slug: "hello", skipped: undefined }, location: { searchStr: "?q=1&q=2&x=y" } });
    expect(result.params).toEqual({ slug: "hello" });
    expect(result.searchParams).toEqual({ q: ["1", "2"], x: "y" });
    expect(await result.props.params).toEqual({ slug: "hello" });
    expect(result.metadata).toEqual({ title: "Post hello" });
    const objectSearch = await loaderFromPage({ default: () => null })({ location: { search: { page: 2, tags: ["a", "b"] } } });
    expect(objectSearch.searchParams).toEqual({ page: "2", tags: ["a", "b"] });
    expect(objectSearch.metadata).toBeUndefined();
  });
});
