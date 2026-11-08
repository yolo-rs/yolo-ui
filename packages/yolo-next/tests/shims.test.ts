// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToString } from "react-dom/server";
import { YoloRouterProvider } from "../src/navigation.ts";
import dynamic from "../src/shims/dynamic.ts";
import ErrorPage, { YoloErrorBoundary, catchError } from "../src/shims/error.ts";
import { GOOGLE_FONT_FAMILIES, Inter, Roboto_Mono, createGoogleFont } from "../src/shims/font-google.ts";
import Form, { formTargetUrl } from "../src/shims/form.ts";
import Head, { collectHead } from "../src/shims/head.ts";
import LegacyImage from "../src/shims/legacy-image.ts";
import Link from "../src/shims/link.ts";
import * as navigationShim from "../src/shims/navigation.ts";
import * as cacheShim from "../src/shims/cache.ts";
import { createRootParamsAccessor, setRootParams, unstable_rootParams } from "../src/shims/root-params.ts";
import Router, { useRouter as usePagesRouter, withRouter } from "../src/shims/router.ts";
import Script from "../src/shims/script.ts";
import { NextFetchEvent, NextRequest, NextResponse, NextURL, URLPattern, userAgentFromString } from "../src/shims/server.ts";
import { GoogleAnalytics, GoogleMapsEmbed, GoogleTagManager, YouTubeEmbed } from "../src/shims/third-parties.ts";

const SHIMS = [
  "font-google",
  "font-local",
  "image",
  "legacy-image",
  "link",
  "navigation",
  "headers",
  "server",
  "cache",
  "script",
  "dynamic",
  "form",
  "og",
  "router",
  "head",
  "root-params",
  "error",
  "server-only",
  "client-only",
  "third-parties",
];

describe("shim modules", () => {
  test("every shim imports on the server without throwing", async () => {
    for (const name of SHIMS) {
      const module = (await import(`../src/shims/${name}.ts`)) as Record<string, unknown>;
      expect(Object.keys(module).length).toBeGreaterThan(0);
    }
  });

  test("key default and named exports exist", async () => {
    expect(typeof (await import("../src/shims/image.ts")).default).toBe("function");
    expect(typeof (await import("../src/shims/font-local.ts")).default).toBe("function");
    expect(typeof (await import("../src/shims/og.ts")).ImageResponse).toBe("function");
    const headers = await import("../src/shims/headers.ts");
    expect(typeof headers.headers).toBe("function");
    expect(typeof headers.cookies).toBe("function");
    expect(typeof headers.draftMode).toBe("function");
    expect(typeof navigationShim.useRouter).toBe("function");
    expect(typeof navigationShim.redirect).toBe("function");
    expect(navigationShim.RedirectType).toEqual({ push: "push", replace: "replace" });
    expect(typeof cacheShim.cacheTag).toBe("function");
    expect(typeof cacheShim.revalidatePath).toBe("function");
    expect(typeof cacheShim.unstable_cache).toBe("function");
    expect(typeof cacheShim.io).toBe("function");
  });

  test("font-google factories", () => {
    expect(GOOGLE_FONT_FAMILIES.length).toBeGreaterThanOrEqual(80);
    const inter = Inter({ subsets: ["latin"], variable: "--font-inter" });
    expect(inter.className).toMatch(/^yolo-font-[0-9a-f]{8}$/);
    expect(inter.plan.css2Url).toBe("https://fonts.googleapis.com/css2?family=Inter&display=swap");
    const mono = Roboto_Mono({ subsets: ["latin"], weight: ["400", "700"], variable: "--font-mono" });
    expect(mono.plan.css2Url).toBe("https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;700&display=swap");
    expect(createGoogleFont("Noto Sans KR")({ weight: "400" }).plan.css2Url).toContain("family=Noto+Sans+KR:wght@400");
  });
});

describe("next/server", () => {
  test("NextRequest exposes nextUrl (basePath aware) and cookies", () => {
    const request = new NextRequest("http://localhost/docs/a?x=1", {
      nextConfig: { basePath: "/docs" },
      headers: { cookie: "a=1; b=2" },
    });
    expect(request.nextUrl.pathname).toBe("/a");
    expect(request.nextUrl.basePath).toBe("/docs");
    expect(request.nextUrl.searchParams.get("x")).toBe("1");
    expect(request.nextUrl.href).toBe("http://localhost/docs/a?x=1");
    expect(request.nextUrl.clone().pathname).toBe("/a");
    expect(request.cookies.get("a")?.value).toBe("1");
    expect(request.cookies.has("b")).toBe(true);
    expect(request.cookies.getAll()).toHaveLength(2);
    request.cookies.set("c", "3");
    expect(request.headers.get("cookie")).toContain("c=3");
    expect(request.cookies.delete("a")).toBe(true);
    expect(request.headers.get("cookie")).not.toContain("a=1");
    expect(request.ip).toBeUndefined();
    expect(request.geo).toBeUndefined();
    expect(request instanceof Request).toBe(true);
  });

  test("NextURL handles locales, pathname writes and trailing slashes", () => {
    const url = new NextURL("http://localhost/fr/about", { nextConfig: { i18n: { locales: ["en", "fr"], defaultLocale: "en" } } });
    expect(url.locale).toBe("fr");
    expect(url.pathname).toBe("/about");
    url.pathname = "/contact";
    expect(url.href).toBe("http://localhost/fr/contact");
    url.locale = "en";
    expect(url.href).toBe("http://localhost/contact");
    expect(() => {
      url.locale = "de";
    }).toThrow();
    const slash = new NextURL("http://localhost/docs/a", { basePath: "/docs", trailingSlash: true });
    expect(slash.href).toBe("http://localhost/docs/a/");
    expect(slash.pathname).toBe("/a");
    expect(JSON.stringify({ url: slash })).toBe('{"url":"http://localhost/docs/a/"}');
    expect(new NextURL("/relative", "http://localhost/base/").href).toBe("http://localhost/relative");
  });

  test("NextResponse helpers", async () => {
    const redirect = NextResponse.redirect("http://localhost/login");
    expect(redirect.status).toBe(307);
    expect(redirect.headers.get("location")).toBe("http://localhost/login");
    expect(NextResponse.redirect(new URL("http://localhost/x"), 308).status).toBe(308);
    expect(NextResponse.redirect("http://localhost/x", { status: 301, headers: { "x-a": "1" } }).headers.get("x-a")).toBe("1");
    expect(() => NextResponse.redirect("http://localhost/x", 200)).toThrow(RangeError);
    const rewrite = NextResponse.rewrite("http://localhost/other");
    expect(rewrite.headers.get("x-middleware-rewrite")).toBe("http://localhost/other");
    const next = NextResponse.next({ request: { headers: new Headers({ "x-user": "42", "x-role": "admin" }) } });
    expect(next.headers.get("x-middleware-next")).toBe("1");
    expect(next.headers.get("x-middleware-override-headers")?.split(",").sort()).toEqual(["x-role", "x-user"]);
    expect(next.headers.get("x-middleware-request-x-user")).toBe("42");
    const json = NextResponse.json({ ok: true }, { status: 201 });
    expect(json.status).toBe(201);
    expect(json.headers.get("content-type")).toContain("application/json");
    expect(await json.json()).toEqual({ ok: true });
    expect(json instanceof Response).toBe(true);
  });

  test("NextResponse.cookies writes Set-Cookie headers", () => {
    const response = NextResponse.json({});
    response.cookies.set("session", "abc", { httpOnly: true, path: "/", sameSite: "lax" });
    const setCookie = response.headers.getSetCookie();
    expect(setCookie).toHaveLength(1);
    expect(setCookie[0]).toContain("session=abc");
    expect(setCookie[0]).toContain("HttpOnly");
    expect(response.cookies.get("session")?.value).toBe("abc");
    expect(response.cookies.get("session")?.httpOnly).toBe(true);
    response.cookies.set({ name: "theme", value: "dark", maxAge: 60 });
    expect(response.headers.getSetCookie()).toHaveLength(2);
    response.cookies.delete("session");
    expect(response.cookies.get("session")?.value).toBe("");
    expect(response.headers.getSetCookie().find((header) => header.startsWith("session="))).toContain("Expires=");
  });

  test("userAgentFromString, NextFetchEvent and URLPattern", async () => {
    const ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
    const parsed = userAgentFromString(ua);
    expect(parsed.ua).toBe(ua);
    expect(parsed.isBot).toBe(false);
    expect(userAgentFromString(undefined).ua).toBe("");
    const kept: Promise<unknown>[] = [];
    const event = new NextFetchEvent({
      request: new NextRequest("http://localhost/"),
      page: "/",
      context: { waitUntil: (promise) => kept.push(promise) },
    });
    event.waitUntil(Promise.resolve(1));
    expect(kept).toHaveLength(1);
    expect(event.sourcePage).toBe("/");
    new NextFetchEvent({ request: new NextRequest("http://localhost/"), page: "/" }).waitUntil(Promise.reject(new Error("ignored")));
    await Promise.resolve();
    if (URLPattern) {
      expect(new URLPattern({ pathname: "/blog/:slug" }).test("http://localhost/blog/a")).toBe(true);
    }
  });
});

describe("component shims rendered on the server", () => {
  test("dynamic() renders the loading state when ssr is false", () => {
    const Lazy = dynamic(() => Promise.resolve({ default: () => createElement("canvas", { "data-chart": true }) }), {
      ssr: false,
      loading: () => createElement("p", null, "chart"),
    });
    const html = renderToString(createElement(Lazy));
    expect(html).toContain("<p>chart</p>");
    expect(html).not.toContain("canvas");
    expect(() => dynamic({ ssr: false })).toThrow(/requires a loader/);
  });

  test("Script renders the src with async for afterInteractive and nothing for lazyOnload", () => {
    const html = renderToString(createElement(Script, { src: "https://analytics.example.com/script.js", strategy: "afterInteractive" }));
    expect(html).toContain('src="https://analytics.example.com/script.js"');
    expect(html).toContain("async");
    expect(html).toContain('data-nscript="afterInteractive"');
    expect(renderToString(createElement(Script, { src: "https://x.example/lazy.js", strategy: "lazyOnload" }))).toBe("");
    const inline = renderToString(createElement(Script, { id: "inline", strategy: "beforeInteractive" }, "console.log(1)"));
    expect(inline).toContain("console.log(1)");
    expect(inline).not.toContain("async");
  });

  test("Form renders a GET form and builds the target URL", () => {
    const html = renderToString(
      createElement(YoloRouterProvider, { initialUrl: "/" }, createElement(Form, { action: "/search" }, createElement("input", { name: "q" }))),
    );
    expect(html).toContain('method="get"');
    expect(html).toContain('action="/search"');
    const data = new FormData();
    data.append("q", "bun");
    data.append("page", "2");
    expect(formTargetUrl("/search?old=1", data)).toBe("/search?q=bun&page=2");
    const actionForm = renderToString(createElement(Form, { action: async () => undefined }, createElement("button", null, "Go")));
    expect(actionForm).toContain("<form");
    expect(actionForm).not.toContain('method="get"');
  });

  test("Link formats object hrefs", () => {
    const html = renderToString(
      createElement(YoloRouterProvider, { initialUrl: "/" }, createElement(Link, { href: { pathname: "/blog", query: { page: 2 }, hash: "top" } }, "Blog")),
    );
    expect(html).toContain('href="/blog?page=2#top"');
    expect(html).toContain(">Blog</a>");
  });

  test("third-party components render the official snippets", () => {
    const ga = renderToString(createElement(GoogleAnalytics, { gaId: "G-TEST" }));
    expect(ga).toContain("https://www.googletagmanager.com/gtag/js?id=G-TEST");
    expect(ga).toContain("gtag('config', 'G-TEST')");
    const gtm = renderToString(createElement(GoogleTagManager, { gtmId: "GTM-XYZ", dataLayer: { env: "test" } }));
    expect(gtm).toContain("'GTM-XYZ'");
    expect(gtm).toContain("gtm.start");
    expect(gtm).toContain('{"env":"test"}');
    const yt = renderToString(createElement(YouTubeEmbed, { videoid: "dQw4w9WgXcQ" }));
    expect(yt).toContain('<lite-youtube videoid="dQw4w9WgXcQ"');
    expect(yt).toContain("lite-youtube.js");
    const maps = renderToString(createElement(GoogleMapsEmbed, { apiKey: "KEY", mode: "place", q: "Paris" }));
    expect(maps).toContain("https://www.google.com/maps/embed/v1/place?key=KEY&amp;q=Paris");
  });

  test("Head collects children on the server", () => {
    collectHead();
    const html = renderToString(createElement(Head, null, createElement("title", null, "Hello")));
    expect(html).toBe("");
    const collected = collectHead();
    expect(collected).toHaveLength(1);
    expect(collectHead()).toHaveLength(0);
  });

  test("legacy Image maps layout fill and objectFit", () => {
    const html = renderToString(createElement(LegacyImage, { src: "/a.png", alt: "a", layout: "fill", objectFit: "cover" }));
    expect(html).toContain('data-nimg="fill"');
    expect(html).toContain("object-fit:cover");
    expect(html).toContain("position:absolute");
    const responsive = renderToString(createElement(LegacyImage, { src: "/a.png", alt: "a", layout: "responsive", width: 100, height: 50 }));
    expect(responsive).toContain('sizes="100vw"');
    expect(responsive).toContain("height:auto");
  });

  test("pages router shim and error helpers", () => {
    function Page(): ReturnType<typeof createElement> {
      const router = usePagesRouter();
      return createElement("div", null, JSON.stringify({ pathname: router.pathname, query: router.query, asPath: router.asPath, ready: router.isReady }));
    }
    const html = renderToString(
      createElement(YoloRouterProvider, { initialUrl: "/blog/hello?q=1", params: { slug: "hello" } }, createElement(Page)),
    ).replace(/&quot;/g, '"');
    expect(html).toContain('"pathname":"/blog/hello"');
    expect(html).toContain('"query":{"q":"1","slug":"hello"}');
    expect(html).toContain('"asPath":"/blog/hello?q=1"');
    expect(html).toContain('"ready":true');
    expect(typeof Router.push).toBe("function");
    expect(Router.pathname).toBe("/");
    const Wrapped = withRouter(({ router }) => createElement("span", null, router.pathname));
    expect(renderToString(createElement(YoloRouterProvider, { initialUrl: "/x" }, createElement(Wrapped)))).toContain("<span>/x</span>");
    const Safe = catchError(() => createElement("p", null, "fallback"))(() => createElement("em", null, "ok"));
    expect(renderToString(createElement(Safe))).toContain("<em>ok</em>");
    expect(renderToString(createElement(YoloErrorBoundary, { fallback: () => null }, createElement("b", null, "child")))).toContain("<b>child</b>");
    expect(renderToString(createElement(ErrorPage, { statusCode: 404 }))).toContain("This page could not be found");
  });

  test("root params accessors", async () => {
    setRootParams({ lang: "fr" });
    expect(await createRootParamsAccessor("lang")()).toBe("fr");
    expect(await createRootParamsAccessor("missing")()).toBeUndefined();
    expect(await unstable_rootParams()).toEqual({ lang: "fr" });
    setRootParams(undefined);
  });
});
