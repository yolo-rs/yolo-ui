// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { createElement, type ReactElement } from "react";
import { renderToString } from "react-dom/server";
import { createRequestStore, runWithRequestStore } from "../src/headers.ts";
import {
  ReadonlyURLSearchParams,
  RedirectType,
  YoloLayoutBoundary,
  YoloRouterProvider,
  forbidden,
  getAccessFallbackStatus,
  getRedirectStatus,
  getRedirectType,
  getRedirectUrl,
  isHttpAccessFallbackError,
  isNextRouterError,
  isRedirectError,
  notFound,
  permanentRedirect,
  rateWebVital,
  redirect,
  unauthorized,
  unstable_rethrow,
  useLinkStatus,
  useParams,
  usePathname,
  useRouter,
  useSearchParams,
  useSelectedLayoutSegment,
  useSelectedLayoutSegments,
} from "../src/navigation.ts";

function capture(fn: () => never): Error & { digest?: string } {
  try {
    fn();
  } catch (error) {
    return error as Error & { digest?: string };
  }
  throw new Error("expected a throw");
}

function decodeHtml(html: string): string {
  return html.replace(/&quot;/g, '"').replace(/&#x27;/g, "'").replace(/&amp;/g, "&").replace(/<!--.*?-->/g, "");
}

function renderJson(element: ReactElement): Record<string, unknown> {
  const html = decodeHtml(renderToString(element));
  const match = /<div>(.*)<\/div>/s.exec(html);
  if (!match || match[1] === undefined) {
    throw new Error(`unexpected markup: ${html}`);
  }
  return JSON.parse(match[1]) as Record<string, unknown>;
}

describe("redirect control-flow errors", () => {
  test("redirect defaults to replace with status 307", () => {
    const error = capture(() => redirect("/login"));
    expect(error.message).toBe("NEXT_REDIRECT");
    expect(error.digest).toBe("NEXT_REDIRECT;replace;/login;307;");
    expect(isRedirectError(error)).toBe(true);
    expect(getRedirectUrl(error as never)).toBe("/login");
    expect(getRedirectStatus(error as never)).toBe(307);
    expect(getRedirectType(error as never)).toBe("replace");
  });

  test("explicit push type and permanentRedirect 308", () => {
    expect(capture(() => redirect("/next", "push")).digest).toBe("NEXT_REDIRECT;push;/next;307;");
    expect(capture(() => permanentRedirect("/moved")).digest).toBe("NEXT_REDIRECT;replace;/moved;308;");
    expect(getRedirectStatus(capture(() => permanentRedirect("/moved")) as never)).toBe(308);
  });

  test("defaults to push inside a Server Action request store", () => {
    const store = createRequestStore(new Request("http://localhost/"), { phase: "action" });
    const error = runWithRequestStore(store, () => capture(() => redirect("/done")));
    expect(error.digest).toBe("NEXT_REDIRECT;push;/done;307;");
  });

  test("destinations containing semicolons round-trip", () => {
    const error = capture(() => redirect("/a;b?x=1"));
    expect(isRedirectError(error)).toBe(true);
    expect(getRedirectUrl(error as never)).toBe("/a;b?x=1");
  });

  test("RedirectType values and non-redirect errors", () => {
    expect(RedirectType).toEqual({ push: "push", replace: "replace" });
    expect(isRedirectError(new Error("NEXT_REDIRECT"))).toBe(false);
    expect(isRedirectError({ digest: "NEXT_REDIRECT;jump;/x;307;" })).toBe(false);
    expect(isRedirectError({ digest: "NEXT_REDIRECT;push;/x;301;" })).toBe(false);
  });
});

describe("HTTP access fallback errors", () => {
  test("notFound / forbidden / unauthorized digests", () => {
    expect(capture(notFound).digest).toBe("NEXT_HTTP_ERROR_FALLBACK;404");
    expect(capture(forbidden).digest).toBe("NEXT_HTTP_ERROR_FALLBACK;403");
    expect(capture(unauthorized).digest).toBe("NEXT_HTTP_ERROR_FALLBACK;401");
  });

  test("helpers classify the errors", () => {
    const error = capture(notFound);
    expect(isHttpAccessFallbackError(error)).toBe(true);
    expect(getAccessFallbackStatus(error as never)).toBe(404);
    expect(getAccessFallbackStatus(capture(forbidden) as never)).toBe(403);
    expect(isNextRouterError(error)).toBe(true);
    expect(isNextRouterError(capture(() => redirect("/x")))).toBe(true);
    expect(isNextRouterError(new Error("boom"))).toBe(false);
    expect(isHttpAccessFallbackError({ digest: "NEXT_HTTP_ERROR_FALLBACK;500" })).toBe(false);
  });
});

describe("unstable_rethrow", () => {
  test("rethrows framework control flow", () => {
    expect(() => unstable_rethrow(capture(() => redirect("/x")))).toThrow();
    expect(() => unstable_rethrow(capture(notFound))).toThrow();
    expect(() => unstable_rethrow(Object.assign(new Error("dynamic"), { digest: "DYNAMIC_SERVER_USAGE" }))).toThrow();
  });

  test("returns for ordinary errors and inspects causes", () => {
    expect(unstable_rethrow(new Error("plain"))).toBeUndefined();
    expect(unstable_rethrow("string")).toBeUndefined();
    const wrapped = new Error("wrapped", { cause: capture(notFound) });
    expect(() => unstable_rethrow(wrapped)).toThrow();
  });
});

describe("ReadonlyURLSearchParams", () => {
  test("reads but refuses mutation", () => {
    const params = new ReadonlyURLSearchParams("?q=1&tag=a&tag=b");
    expect(params.get("q")).toBe("1");
    expect(params.getAll("tag")).toEqual(["a", "b"]);
    expect(params instanceof URLSearchParams).toBe(true);
    // The overrides take no parameters on purpose (Next's type-level guard), so call through the base type.
    const mutable = params as URLSearchParams;
    expect(() => mutable.append("x", "1")).toThrow(/Method unavailable on `ReadonlyURLSearchParams`/);
    expect(() => mutable.set("x", "1")).toThrow(/Method unavailable/);
    expect(() => mutable.delete("q")).toThrow(/Method unavailable/);
    expect(() => mutable.sort()).toThrow(/Method unavailable/);
  });
});

describe("hooks rendered on the server inside YoloRouterProvider", () => {
  function Probe(): ReactElement {
    const pathname = usePathname();
    const params = useParams<{ slug: string }>();
    const search = useSearchParams();
    const segments = useSelectedLayoutSegments();
    const segment = useSelectedLayoutSegment();
    const router = useRouter();
    const status = useLinkStatus();
    return createElement(
      "div",
      null,
      JSON.stringify({
        pathname,
        slug: params.slug,
        q: search.get("q"),
        segments,
        segment,
        pending: status.pending,
        hasPush: typeof router.push === "function",
        bfcacheId: router.bfcacheId,
      }),
    );
  }

  test("reads the initial location, params and segments", () => {
    const value = renderJson(
      createElement(YoloRouterProvider, { initialUrl: "/blog/hello?q=1", params: { slug: "hello" } }, createElement(Probe)),
    );
    expect(value).toEqual({
      pathname: "/blog/hello",
      slug: "hello",
      q: "1",
      segments: ["blog", "hello"],
      segment: "blog",
      pending: false,
      hasPush: true,
      bfcacheId: "",
    });
  });

  test("YoloLayoutBoundary makes segments relative to the layout", () => {
    const value = renderJson(
      createElement(
        YoloRouterProvider,
        { initialUrl: "/blog/hello", params: { slug: "hello" } },
        createElement(YoloLayoutBoundary, null, createElement(Probe)),
      ),
    );
    expect(value.segments).toEqual(["hello"]);
    expect(value.segment).toBe("hello");
  });

  test("parallel route segments are keyed by slot", () => {
    function Slots(): ReactElement {
      return createElement(
        "div",
        null,
        JSON.stringify({ analytics: useSelectedLayoutSegments("analytics"), children: useSelectedLayoutSegments() }),
      );
    }
    const value = renderJson(
      createElement(
        YoloRouterProvider,
        { initialUrl: "/dashboard", segments: { children: ["dashboard"], analytics: ["page"] } },
        createElement(Slots),
      ),
    );
    expect(value).toEqual({ analytics: ["page"], children: ["dashboard"] });
  });

  test("hooks outside a provider fall back to the root location", () => {
    const value = renderJson(createElement(Probe));
    expect(value.pathname).toBe("/");
    expect(value.segments).toEqual([]);
  });
});

describe("web vitals rating", () => {
  test("uses the standard thresholds", () => {
    expect(rateWebVital("LCP", 2000)).toBe("good");
    expect(rateWebVital("LCP", 3000)).toBe("needs-improvement");
    expect(rateWebVital("CLS", 0.3)).toBe("poor");
    expect(rateWebVital("INP", 200)).toBe("good");
  });
});
