// SPDX-License-Identifier: Apache-2.0
import { afterEach, describe, expect, spyOn, test } from "bun:test";
import { cacheScopeStorage, type CacheScope } from "../src/cache.ts";
import {
  COOKIES_READ_ONLY_MESSAGE,
  after,
  connection,
  cookies,
  createRequestStore,
  draftMode,
  getRequestStore,
  headers,
  mergeStoreHeaders,
  runAfterCallbacks,
  runWithRequestStore,
  serializeCookie,
  userAgent,
  type RequestPhase,
} from "../src/headers.ts";

function storeFor(url: string, init: RequestInit = {}, phase: RequestPhase = "action") {
  return createRequestStore(new Request(url, init), { phase });
}

describe("cookies()", () => {
  test("parses the request cookie header", async () => {
    const store = storeFor("http://localhost/", { headers: { cookie: "a=1; b=hello%20world" } });
    await runWithRequestStore(store, async () => {
      const jar = await cookies();
      expect(jar.get("a")).toEqual({ name: "a", value: "1" });
      expect(jar.get("b")?.value).toBe("hello world");
      expect(jar.get("missing")).toBeUndefined();
      expect(jar.has("a")).toBe(true);
      expect(jar.getAll().map((c) => c.name)).toEqual(["a", "b"]);
      expect(jar.size).toBe(2);
      expect(jar.toString()).toBe("a=1; b=hello%20world");
    });
    expect(store.usedDynamic).toBe(true);
  });

  test("set() serializes Set-Cookie with Bun.Cookie and marks cookiesChanged", async () => {
    const store = storeFor("http://localhost/");
    await runWithRequestStore(store, async () => {
      const jar = await cookies();
      jar.set("sess", "v", { httpOnly: true, sameSite: "lax", maxAge: 60 });
      jar.set({ name: "strict", value: "1", sameSite: true, priority: "high", secure: true });
      expect(jar.get("sess")?.value).toBe("v");
      expect(jar.get("strict")).toMatchObject({ name: "strict", value: "1", priority: "high" });
    });
    const setCookies = store.responseHeaders.getSetCookie();
    expect(setCookies).toHaveLength(2);
    expect(setCookies[0]).toBe("sess=v; Path=/; Max-Age=60; HttpOnly; SameSite=Lax");
    expect(setCookies[1]).toContain("strict=1");
    expect(setCookies[1]).toContain("SameSite=Strict");
    expect(setCookies[1]).toContain("Secure");
    expect(setCookies[1]).toContain("Priority=High");
    expect(store.revalidated.cookiesChanged).toBe(true);
    const merged = mergeStoreHeaders(store, new Headers());
    expect(merged.getSetCookie()).toHaveLength(2);
  });

  test("delete() expires the cookie", async () => {
    const store = storeFor("http://localhost/", { headers: { cookie: "a=1" } });
    await runWithRequestStore(store, async () => {
      const jar = await cookies();
      jar.delete("a");
      expect(jar.has("a")).toBe(false);
    });
    const [expired] = store.responseHeaders.getSetCookie();
    expect(expired).toContain("a=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Max-Age=0");
  });

  test("serializeCookie supports partitioned + none and numeric expires", () => {
    const value = serializeCookie("n", "v", { partitioned: true, sameSite: "none", secure: true, expires: 0, domain: "ex.com" });
    expect(value).toContain("Domain=ex.com");
    expect(value).toContain("Partitioned");
    expect(value).toContain("SameSite=None");
    expect(value).toContain("Expires=Thu, 01 Jan 1970 00:00:00 GMT");
  });

  test("mutations during render throw like Next.js", async () => {
    const store = storeFor("http://localhost/", {}, "render");
    await runWithRequestStore(store, async () => {
      const jar = await cookies();
      expect(() => jar.set("a", "1")).toThrow(COOKIES_READ_ONLY_MESSAGE);
      expect(() => jar.delete("a")).toThrow(COOKIES_READ_ONLY_MESSAGE);
    });
    expect(store.responseHeaders.getSetCookie()).toHaveLength(0);
  });

  test("route handlers and proxy may set cookies", async () => {
    for (const phase of ["route", "proxy"] as const) {
      const store = storeFor("http://localhost/", {}, phase);
      await runWithRequestStore(store, async () => {
        (await cookies()).set("ok", phase);
      });
      expect(store.responseHeaders.getSetCookie()[0]).toContain(`ok=${phase}`);
    }
  });
});

describe("headers()", () => {
  test("returns a frozen view of the request headers", async () => {
    const store = storeFor("http://localhost/", { headers: { "x-test": "1" } });
    await runWithRequestStore(store, async () => {
      const h = await headers();
      expect(h.get("x-test")).toBe("1");
      expect(() => h.set("x", "y")).toThrow("Headers cannot be modified");
      expect(() => h.append("x", "y")).toThrow();
      expect(() => h.delete("x-test")).toThrow();
    });
  });

  test("throws outside a request scope", async () => {
    await expect(headers()).rejects.toThrow("`headers` was called outside a request scope");
    await expect(cookies()).rejects.toThrow("`cookies` was called outside a request scope");
    expect(() => getRequestStore("draftMode")).toThrow("`draftMode` was called outside a request scope");
    expect(() => after(() => undefined)).toThrow("`after` was called outside a request scope");
  });

  test("throws inside a non-private use cache scope", async () => {
    const store = storeFor("http://localhost/");
    const scope: CacheScope = { id: "x", variant: "", life: { stale: 1, revalidate: 1, expire: 1 }, explicitLife: false, tags: new Set() };
    await runWithRequestStore(store, async () => {
      const error = await cacheScopeStorage.run(scope, () => cookies()).catch((e: unknown) => e);
      expect(error).toBeInstanceOf(Error);
      expect((error as { digest?: string }).digest).toBe("next-request-in-use-cache");
      const privateScope: CacheScope = { ...scope, variant: "private" };
      const jar = await cacheScopeStorage.run(privateScope, () => cookies());
      expect(jar.size).toBe(0);
    });
  });
});

describe("draftMode()", () => {
  test("toggles the __prerender_bypass cookie", async () => {
    const store = storeFor("http://localhost/", {}, "route");
    await runWithRequestStore(store, async () => {
      const draft = await draftMode();
      expect(draft.isEnabled).toBe(false);
      draft.enable();
      expect(draft.isEnabled).toBe(true);
      expect(store.draft.enabled).toBe(true);
      draft.disable();
      expect(draft.isEnabled).toBe(false);
    });
    const [enabled, disabled] = store.responseHeaders.getSetCookie();
    expect(enabled).toContain("__prerender_bypass=");
    expect(enabled).toContain("HttpOnly");
    expect(disabled).toContain("__prerender_bypass=; Path=/; Expires=Thu, 01 Jan 1970");
  });

  test("reads an existing bypass cookie", async () => {
    const store = storeFor("http://localhost/", { headers: { cookie: "__prerender_bypass=abc" } }, "render");
    expect(store.draft.enabled).toBe(true);
    await runWithRequestStore(store, async () => {
      expect((await draftMode()).isEnabled).toBe(true);
    });
  });
});

describe("connection() and after()", () => {
  test("connection marks the store dynamic and throws during prerender", async () => {
    const store = storeFor("http://localhost/", {}, "render");
    await runWithRequestStore(store, () => connection());
    expect(store.usedDynamic).toBe(true);
    const prerender = createRequestStore(new Request("http://localhost/"), { isPrerender: true });
    const error = await runWithRequestStore(prerender, () => connection()).catch((e: unknown) => e);
    expect((error as { digest?: string }).digest).toBe("DYNAMIC_SERVER_USAGE");
  });

  test("after callbacks run and failures are logged", async () => {
    const store = storeFor("http://localhost/");
    const calls: string[] = [];
    const errorSpy = spyOn(console, "error").mockImplementation(() => undefined);
    runWithRequestStore(store, () => {
      after(() => {
        calls.push("sync");
      });
      after(async () => {
        await Bun.sleep(1);
        calls.push("async");
      });
      after(() => {
        throw new Error("boom");
      });
    });
    expect(store.after).toHaveLength(3);
    await runAfterCallbacks(store);
    expect(calls.sort()).toEqual(["async", "sync"]);
    expect(store.after).toHaveLength(0);
    expect(errorSpy).toHaveBeenCalledTimes(1);
    errorSpy.mockRestore();
  });
});

describe("userAgent()", () => {
  afterEach(() => undefined);

  const table: Array<[string, Partial<ReturnType<typeof userAgent>>]> = [
    [
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
      { isBot: false, browser: { name: "Chrome", version: "131.0.0.0" }, os: { name: "Windows", version: "10" }, device: {} },
    ],
    [
      "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
      { browser: { name: "Safari", version: "17.5" }, os: { name: "iOS", version: "17.5" }, device: { type: "mobile", vendor: "Apple", model: "iPhone" } },
    ],
    [
      "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
      { isBot: true },
    ],
    [
      "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
      { browser: { name: "Firefox", version: "128.0" }, os: { name: "Linux" }, engine: { name: "Gecko", version: "128.0" } },
    ],
    [
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.2849.56",
      { browser: { name: "Edge", version: "130.0.2849.56" } },
    ],
    [
      "Mozilla/5.0 (iPad; CPU OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1",
      { device: { type: "tablet", vendor: "Apple", model: "iPad" }, os: { name: "iOS", version: "16.6" } },
    ],
    [
      "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Mobile Safari/537.36",
      { os: { name: "Android", version: "14" }, device: { type: "mobile" }, browser: { name: "Chrome", version: "129.0.0.0" } },
    ],
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36 OPR/106.0.0.0", { browser: { name: "Opera", version: "106.0.0.0" }, os: { name: "Mac OS", version: "10.15.7" } }],
  ];

  test.each(table)("parses %s", (ua, expected) => {
    const parsed = userAgent({ headers: new Headers({ "user-agent": ua }) });
    expect(parsed.ua).toBe(ua);
    expect(parsed).toMatchObject(expected);
    expect(parsed.cpu).toEqual({});
  });

  test("accepts plain header records and empty input", () => {
    expect(userAgent({ headers: { "user-agent": "Lighthouse" } }).isBot).toBe(true);
    expect(userAgent({ headers: {} })).toMatchObject({ ua: "", isBot: false, browser: {}, os: {}, device: {} });
  });
});
