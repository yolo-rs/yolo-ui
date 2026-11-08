// SPDX-License-Identifier: Apache-2.0
import { afterAll, beforeEach, describe, expect, spyOn, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  MemoryCacheHandler,
  SqliteCacheHandler,
  cacheKeyFor,
  cacheLife,
  cacheTag,
  cached,
  configureCacheLife,
  createCacheHandler,
  deserializeCacheValue,
  getCacheHandler,
  implicitTagsFor,
  pathTags,
  refresh,
  resetCacheHandlers,
  resetCacheRuntime,
  resolveCacheLife,
  revalidatePath,
  revalidateTag,
  serializeCacheValue,
  setCacheHandler,
  stableStringify,
  unstable_cache,
  unstable_noStore,
  updateTag,
  waitForRevalidations,
  type CacheEntry,
  type CacheHandler,
} from "../src/cache.ts";
import { cookies, createRequestStore, runWithRequestStore } from "../src/headers.ts";

const tmp = mkdtempSync(join(tmpdir(), "yolo-next-cache-"));
const openHandlers: SqliteCacheHandler[] = [];

afterAll(() => {
  for (const handler of openHandlers) {
    try {
      handler.close();
    } catch {
      // already closed
    }
  }
  try {
    rmSync(tmp, { recursive: true, force: true });
  } catch {
    // Windows keeps WAL files briefly locked; the OS temp dir is cleaned later.
  }
});

beforeEach(() => {
  resetCacheHandlers();
  resetCacheRuntime();
});

function entry(value: string, tags: string[] = [], timestamp = 1_000_000): CacheEntry {
  return { value, tags, stale: 1, revalidate: 10, expire: 100, timestamp };
}

async function handlerContract(name: string, make: (now: () => number) => CacheHandler): Promise<void> {
  describe(name, () => {
    test("roundtrips entries", async () => {
      let now = 1_000_000;
      const handler = make(() => now);
      expect(await handler.get("missing")).toBeUndefined();
      await handler.set("k", entry("hello", ["posts"]));
      await handler.set("bytes", Promise.resolve(entry("x")) .then((e) => ({ ...e, value: new TextEncoder().encode("bin") })));
      const hit = await handler.get("k");
      expect(hit?.value).toBe("hello");
      expect(hit?.tags).toEqual(["posts"]);
      expect(hit?.revalidate).toBe(10);
      const bin = await handler.get("bytes");
      expect(new TextDecoder().decode(bin?.value as Uint8Array)).toBe("bin");
      expect(await handler.getExpiration("posts")).toBe(0);
      now += 1;
      await handler.updateTags(["posts"]);
      expect(await handler.getExpiration("posts", "other")).toBe(now);
      // Hard expiration: entry written before the tag update is gone.
      expect(await handler.get("k")).toBeUndefined();
      expect(await handler.get("bytes")).toBeDefined();
    });

    test("soft tags and stale-while-revalidate windows", async () => {
      let now = 2_000_000;
      const handler = make(() => now);
      await handler.set("page", entry("v1", [], now));
      now += 1_000;
      await handler.updateTags(["_N_T_/blog"], { expire: 60 });
      const stale = await handler.get("page", ["_N_T_/blog"]);
      expect(stale?.value).toBe("v1");
      expect(stale?.revalidate).toBe(0);
      expect((stale?.expire ?? 0) * 1000).toBeGreaterThanOrEqual(61_000 - 1000);
      now += 61_000;
      expect(await handler.get("page", ["_N_T_/blog"])).toBeUndefined();
      await handler.refreshTags();
    });
  });
}

await handlerContract("MemoryCacheHandler", (now) => new MemoryCacheHandler({ now }));
await handlerContract("SqliteCacheHandler", (now) => {
  const handler = new SqliteCacheHandler(join(tmp, `cache-${Math.random().toString(36).slice(2)}.sqlite`), { now });
  openHandlers.push(handler);
  return handler;
});

test("memory handler evicts least recently used entries by bytes", async () => {
  const handler = new MemoryCacheHandler({ maxBytes: 10 });
  await handler.set("a", entry("12345"));
  await handler.set("b", entry("12345"));
  expect(handler.size).toBe(2);
  await handler.get("a");
  await handler.set("c", entry("123"));
  expect(await handler.get("b")).toBeUndefined();
  expect(await handler.get("a")).toBeDefined();
  expect(handler.bytes).toBe(8);
  await handler.set("huge", entry("x".repeat(11)));
  expect(await handler.get("huge")).toBeUndefined();
});

test("sqlite handler persists across instances", async () => {
  const path = join(tmp, "persist.sqlite");
  const first = new SqliteCacheHandler(path);
  await first.set("k", entry("persisted"));
  first.close();
  const second = new SqliteCacheHandler(path);
  expect((await second.get("k"))?.value).toBe("persisted");
  second.close();
}, 20_000);

test("createCacheHandler and the registry", () => {
  const memory = createCacheHandler({ kind: "memory", maxBytes: 1024 });
  const sqlite = createCacheHandler({ kind: "sqlite" });
  expect(memory).toBeInstanceOf(MemoryCacheHandler);
  expect(sqlite).toBeInstanceOf(SqliteCacheHandler);
  setCacheHandler("default", memory);
  setCacheHandler("remote", sqlite);
  expect(getCacheHandler()).toBe(memory);
  expect(getCacheHandler("remote")).toBe(sqlite);
  expect(getCacheHandler("static")).toBe(memory);
  (sqlite as SqliteCacheHandler).close();
});

describe("cached()", () => {
  test("miss, hit, stale-while-revalidate and expiry with an injected clock", async () => {
    let now = 10_000_000;
    const clock = () => now;
    setCacheHandler("default", new MemoryCacheHandler({ now: clock }));
    let calls = 0;
    const getPosts = cached(
      async (prefix: string) => {
        calls += 1;
        return { list: [`${prefix}-${calls}`], at: new Date(now) };
      },
      { id: "posts", life: { stale: 1, revalidate: 10, expire: 100 }, now: clock },
    );
    const first = await getPosts("a");
    expect(first.list).toEqual(["a-1"]);
    expect(first.at).toBeInstanceOf(Date);
    expect(calls).toBe(1);
    expect((await getPosts("a")).list).toEqual(["a-1"]);
    expect(calls).toBe(1);
    expect((await getPosts("b")).list).toEqual(["b-2"]);
    now += 11_000;
    const stale = await getPosts("a");
    expect(stale.list).toEqual(["a-1"]);
    await waitForRevalidations();
    expect(calls).toBe(3);
    expect((await getPosts("a")).list).toEqual(["a-3"]);
    now += 101_000;
    expect((await getPosts("a")).list).toEqual(["a-4"]);
    expect(calls).toBe(4);
  });

  test("dedupes concurrent misses per key", async () => {
    setCacheHandler("default", new MemoryCacheHandler());
    let calls = 0;
    const slow = cached(
      async () => {
        calls += 1;
        await Bun.sleep(5);
        return calls;
      },
      { id: "slow" },
    );
    const results = await Promise.all([slow(), slow(), slow()]);
    expect(results).toEqual([1, 1, 1]);
    expect(calls).toBe(1);
  });

  test("cacheLife/cacheTag inside the scope shape the stored entry", async () => {
    const handler = new MemoryCacheHandler();
    setCacheHandler("default", handler);
    configureCacheLife({ blog: { stale: 3600, revalidate: 900, expire: 86_400 } });
    expect(resolveCacheLife("blog")).toEqual({ stale: 3600, revalidate: 900, expire: 86_400 });
    expect(resolveCacheLife({ revalidate: 60 })).toEqual({ stale: 300, revalidate: 60, expire: 0xfffffffe });
    expect(() => resolveCacheLife("nope")).toThrow('Unknown cacheLife profile "nope"');
    const getPost = cached(
      async (slug: string) => {
        cacheLife("blog");
        cacheLife("hours");
        cacheTag("posts", `post:${slug}`);
        return { slug };
      },
      { id: "post" },
    );
    await getPost("hello");
    const stored = await handler.get(cacheKeyFor("post", ["hello"]));
    expect(stored?.tags).toEqual(["posts", "post:hello"]);
    expect(stored?.revalidate).toBe(900);
    expect(stored?.expire).toBe(86_400);
    expect(stored?.stale).toBe(300);
  });

  test("nested caches propagate tags and lifetimes to the parent", async () => {
    const handler = new MemoryCacheHandler();
    setCacheHandler("default", handler);
    const inner = cached(
      async () => {
        cacheLife("seconds");
        cacheTag("inner");
        return 1;
      },
      { id: "inner" },
    );
    const outer = cached(
      async () => {
        cacheTag("outer");
        return (await inner()) + 1;
      },
      { id: "outer", life: "days" },
    );
    expect(await outer()).toBe(2);
    const stored = await handler.get(cacheKeyFor("outer", []));
    expect(stored?.tags.sort()).toEqual(["inner", "outer"]);
    expect(stored?.revalidate).toBe(1);
    expect(stored?.expire).toBe(60);
  });

  test("cacheTag validates its arguments", () => {
    const warn = spyOn(console, "warn").mockImplementation(() => undefined);
    expect(() => cacheTag(...Array.from({ length: 129 }, (_, i) => `t${i}`))).toThrow("exceeding the maximum of 128");
    expect(() => cacheTag("x".repeat(257))).toThrow("maximum length of 256");
    cacheTag("outside");
    cacheTag("outside-again");
    cacheLife("hours");
    expect(warn).toHaveBeenCalledTimes(2);
    warn.mockRestore();
  });

  test("reading cookies inside a shared cache throws; private caches memoize per request", async () => {
    setCacheHandler("default", new MemoryCacheHandler());
    const shared = cached(async () => (await cookies()).get("a")?.value, { id: "shared-cookies" });
    const store = createRequestStore(new Request("http://localhost/", { headers: { cookie: "a=1" } }));
    await runWithRequestStore(store, async () => {
      const error = await shared().catch((e: unknown) => e);
      expect((error as { digest?: string }).digest).toBe("next-request-in-use-cache");
    });
    let calls = 0;
    const personal = cached(
      async () => {
        calls += 1;
        return (await cookies()).get("a")?.value;
      },
      { id: "private-cookies", variant: "private" },
    );
    await runWithRequestStore(store, async () => {
      expect(await personal()).toBe("1");
      expect(await personal()).toBe("1");
    });
    expect(calls).toBe(1);
    const other = createRequestStore(new Request("http://localhost/", { headers: { cookie: "a=2" } }));
    await runWithRequestStore(other, async () => {
      expect(await personal()).toBe("2");
    });
    expect(calls).toBe(2);
  });
});

describe("revalidation", () => {
  test("pathTags and implicitTagsFor follow the _N_T_ convention", () => {
    expect(pathTags("/blog")).toEqual(["_N_T_/blog"]);
    expect(pathTags("/blog", "layout")).toEqual(["_N_T_/blog", "_N_T_/blog/layout"]);
    expect(pathTags("/", "layout")).toEqual(["_N_T_/", "_N_T_/layout"]);
    expect(implicitTagsFor("/blog/hello")).toEqual(["_N_T_/layout", "_N_T_/blog/layout", "_N_T_/blog/hello/layout", "_N_T_/blog/hello"]);
  });

  test("revalidatePath expires entries rendered under the path through soft tags", async () => {
    let now = 5_000_000;
    setCacheHandler("default", new MemoryCacheHandler({ now: () => now }));
    let calls = 0;
    const data = cached(
      async () => {
        calls += 1;
        return calls;
      },
      { id: "page-data", now: () => now },
    );
    const render = (phase: "render" | "action") => createRequestStore(new Request("http://localhost/blog/hello"), { phase });
    expect(await runWithRequestStore(render("render"), () => data())).toBe(1);
    expect(await runWithRequestStore(render("render"), () => data())).toBe(1);
    const action = render("action");
    now += 1_000;
    runWithRequestStore(action, () => revalidatePath("/blog", "layout"));
    expect(action.revalidated.paths).toEqual(["/blog"]);
    await waitForRevalidations();
    expect(await runWithRequestStore(render("render"), () => data())).toBe(2);
  });

  test("revalidateTag with a profile is soft; updateTag is immediate and action-only", async () => {
    let now = 6_000_000;
    const handler = new MemoryCacheHandler({ now: () => now });
    setCacheHandler("default", handler);
    let calls = 0;
    const posts = cached(
      async () => {
        calls += 1;
        cacheTag("posts");
        return calls;
      },
      { id: "tagged", now: () => now },
    );
    expect(await posts()).toBe(1);
    now += 1_000;
    const warn = spyOn(console, "warn").mockImplementation(() => undefined);
    revalidateTag("posts", "max");
    await waitForRevalidations();
    // Soft: stale value served, refresh in background.
    expect(await posts()).toBe(1);
    await waitForRevalidations();
    expect(await posts()).toBe(2);
    expect(() => updateTag("posts")).toThrow("updateTag() can only be called during a Server Action");
    const renderStore = createRequestStore(new Request("http://localhost/"), { phase: "render" });
    expect(() => runWithRequestStore(renderStore, () => updateTag("posts"))).toThrow();
    expect(() => runWithRequestStore(renderStore, () => refresh())).toThrow("refresh() can only be called during a Server Action");
    const actionStore = createRequestStore(new Request("http://localhost/"), { phase: "action" });
    now += 1_000;
    runWithRequestStore(actionStore, () => {
      updateTag("posts");
      revalidateTag("legacy");
      refresh();
    });
    expect(actionStore.revalidated.tags).toEqual(["posts", "legacy"]);
    expect(actionStore.revalidated.refresh).toBe(true);
    expect(warn).toHaveBeenCalledTimes(1);
    await waitForRevalidations();
    expect(await posts()).toBe(3);
    warn.mockRestore();
  });

  test("unstable_cache wraps cached with the legacy options", async () => {
    const handler = new MemoryCacheHandler();
    setCacheHandler("default", handler);
    let calls = 0;
    const getStats = unstable_cache(
      async (id: number) => {
        calls += 1;
        return { visits: 42 + id };
      },
      ["stats"],
      { revalidate: 60, tags: ["stats"] },
    );
    expect(await getStats(1)).toEqual({ visits: 43 });
    expect(await getStats(1)).toEqual({ visits: 43 });
    expect(await getStats(2)).toEqual({ visits: 44 });
    expect(calls).toBe(2);
    const keys: string[] = [];
    const original = handler.get.bind(handler);
    handler.get = async (key, soft) => {
      keys.push(key);
      return original(key, soft);
    };
    await getStats(1);
    expect(keys[0]).toContain("unstable_cache:");
    expect(keys[0]).toContain(":stats:");
    const stored = await original(keys[0] ?? "");
    expect(stored?.revalidate).toBe(60);
    expect(stored?.tags).toEqual(["stats"]);
  });

  test("unstable_noStore marks dynamic usage without a store requirement", () => {
    unstable_noStore();
    const store = createRequestStore(new Request("http://localhost/"));
    runWithRequestStore(store, () => unstable_noStore());
    expect(store.usedDynamic).toBe(true);
  });
});

describe("serialization", () => {
  test("serializeCacheValue roundtrips tagged types", () => {
    const value = {
      when: new Date("2026-01-01T00:00:00.000Z"),
      map: new Map([["k", 1]]),
      set: new Set(["a"]),
      big: 10n,
      nested: [undefined, { deep: undefined }],
      bytes: new Uint8Array([1, 2, 3]),
    };
    const text = serializeCacheValue(value);
    expect(text).toContain('"__yolo_type":"date"');
    const revived = deserializeCacheValue<typeof value>(text);
    expect(revived.when).toEqual(value.when);
    expect(revived.map.get("k")).toBe(1);
    expect(revived.set.has("a")).toBe(true);
    expect(revived.big).toBe(10n);
    expect(revived.nested[0]).toBeUndefined();
    expect(revived.bytes).toEqual(new Uint8Array([1, 2, 3]));
    expect(deserializeCacheValue(serializeCacheValue(undefined))).toBeUndefined();
    expect(() => serializeCacheValue(() => 1)).toThrow("must be serializable");
  });

  test("stableStringify sorts keys and tags special values", () => {
    expect(stableStringify({ b: 1, a: [2, { d: 1, c: 2 }] })).toBe('{"a":[2,{"c":2,"d":1}],"b":1}');
    expect(stableStringify([new Date(0), 1n, undefined])).toBe('["$D1970-01-01T00:00:00.000Z","1n",null]');
    expect(cacheKeyFor("id", ["a"])).toBe('dev:id:["a"]');
  });
});
