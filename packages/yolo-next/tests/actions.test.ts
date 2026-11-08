// SPDX-License-Identifier: Apache-2.0
import { describe, expect, spyOn, test } from "bun:test";
import {
  callServer,
  createActionRegistry,
  decodeActionBody,
  decodeActionResult,
  encodeReply,
  handleServerAction,
  isActionRequest,
  loadActionManifest,
  registerServerAction,
  resolveServerAction,
  serverReferenceId,
  tagServerReference,
  type ActionRegistry,
} from "../src/actions.ts";
import { MemoryCacheHandler, setCacheHandler, updateTag } from "../src/cache.ts";
import { after, cookies, createRequestStore, headers } from "../src/headers.ts";
import { notFound, redirect } from "../src/navigation.ts";

const ORIGIN = "http://localhost:3000";

function fetchAction(id: string, args: unknown[], init: { headers?: Record<string, string>; url?: string } = {}): Request {
  return new Request(init.url ?? `${ORIGIN}/dashboard`, {
    method: "POST",
    headers: {
      "next-action": id,
      accept: "text/x-component",
      "content-type": "text/plain;charset=UTF-8",
      origin: ORIGIN,
      host: "localhost:3000",
      ...(init.headers ?? {}),
    },
    body: JSON.stringify(args),
  });
}

function formAction(id: string, fields: Record<string, string>, init: { headers?: Record<string, string>; bound?: unknown[] } = {}): Request {
  const form = new FormData();
  form.append(`$ACTION_ID_${id}`, "");
  if (init.bound) {
    form.append("$ACTION_REF_1", "");
    form.append("$ACTION_1:0", JSON.stringify({ id, bound: init.bound }));
  }
  for (const [key, value] of Object.entries(fields)) {
    form.append(key, value);
  }
  return new Request(`${ORIGIN}/dashboard?tab=1`, {
    method: "POST",
    headers: { origin: ORIGIN, host: "localhost:3000", ...(init.headers ?? {}) },
    body: form,
  });
}

function registryWith(actions: Record<string, (...args: never[]) => unknown>): { registry: ActionRegistry; ids: Record<string, string> } {
  const registry: ActionRegistry = new Map();
  const ids: Record<string, string> = {};
  for (const [name, fn] of Object.entries(actions)) {
    ids[name] = registerServerAction(registry, "app/actions.ts", name, fn);
  }
  return { registry, ids };
}

describe("serverReferenceId", () => {
  test("is deterministic, 40 hex chars and matches 00 + sha256", () => {
    const id = serverReferenceId("app/actions.ts", "createPost");
    expect(id).toMatch(/^00[0-9a-f]{38}$/);
    expect(serverReferenceId("app/actions.ts", "createPost")).toBe(id);
    const expected = `00${new Bun.CryptoHasher("sha256").update("app/actions.ts:createPost").digest("hex").slice(0, 38)}`;
    expect(id).toBe(expected);
    expect(serverReferenceId("app/actions.ts", "createPost", "salt")).not.toBe(id);
    expect(serverReferenceId("app/actions.ts", "deletePost")).not.toBe(id);
  });

  test("createActionRegistry defaults ids and registerServerAction tags functions", async () => {
    const fn = async (): Promise<string> => "ok";
    const registry = createActionRegistry([{ module: "app/a.ts", exportName: "fn", load: async () => ({ fn }) }]);
    const id = serverReferenceId("app/a.ts", "fn");
    expect(registry.get(id)?.inline).toBe(false);
    expect(await resolveServerAction(registry, id)).toBe(fn);
    expect(await resolveServerAction(registry, "00" + "0".repeat(38))).toBeUndefined();
    const other = async (): Promise<number> => 1;
    const otherId = registerServerAction(registry, "app/b.ts", "other", other);
    expect((other as { $$id?: string }).$$id).toBe(otherId);
    expect((tagServerReference(other, "x") as { $$id?: string }).$$id).toBe(otherId);
  });

  test("loadActionManifest returns an empty registry when the manifest is missing", async () => {
    const registry = await loadActionManifest(`${import.meta.dir}/does-not-exist`);
    expect(registry.size).toBe(0);
  });
});

describe("isActionRequest", () => {
  test("detects fetch actions, no-JS forms and ignores other requests", async () => {
    expect(await isActionRequest(fetchAction("00" + "a".repeat(38), []))).toBe(true);
    expect(await isActionRequest(formAction("00" + "a".repeat(38), { title: "x" }))).toBe(true);
    const plain = new FormData();
    plain.append("title", "x");
    expect(await isActionRequest(new Request(ORIGIN, { method: "POST", body: plain }))).toBe(false);
    expect(await isActionRequest(new Request(ORIGIN, { method: "GET" }))).toBe(false);
    expect(await isActionRequest(new Request(ORIGIN, { method: "POST", headers: { "content-type": "application/json" }, body: "{}" }))).toBe(false);
    const encoded = new Request(ORIGIN, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ "$ACTION_ID_abc": "", title: "x" }).toString(),
    });
    expect(await isActionRequest(encoded)).toBe(true);
  });
});

describe("decodeActionBody", () => {
  test("text/plain JSON args with $-escapes", async () => {
    const request = fetchAction("id", ["a", 1, "$$money", "$undefined", "$D2026-01-01T00:00:00.000Z"]);
    const decoded = await decodeActionBody(request, 1024);
    expect(decoded.ok).toBe(true);
    if (decoded.ok) {
      expect(decoded.id).toBe("id");
      expect(decoded.isForm).toBe(false);
      expect(decoded.args.slice(0, 3)).toEqual(["a", 1, "$money"]);
      expect(decoded.args[3]).toBeUndefined();
      expect(decoded.args[4]).toEqual(new Date("2026-01-01T00:00:00.000Z"));
    }
  });

  test("application/json args", async () => {
    const request = new Request(ORIGIN, { method: "POST", headers: { "content-type": "application/json", "next-action": "x" }, body: JSON.stringify([{ a: 1 }]) });
    const decoded = await decodeActionBody(request);
    expect(decoded.ok && decoded.args).toEqual([{ a: 1 }]);
  });

  test("multipart $ACTION_ID_ forms strip action fields and prepend bound args", async () => {
    const id = serverReferenceId("app/actions.ts", "createPost");
    const decoded = await decodeActionBody(formAction(id, { title: "Hello" }, { bound: [{ message: "" }] }));
    expect(decoded.ok).toBe(true);
    if (decoded.ok) {
      expect(decoded.id).toBe(id);
      expect(decoded.isForm).toBe(true);
      expect(decoded.bound).toEqual([{ message: "" }]);
      expect(decoded.args).toHaveLength(2);
      expect(decoded.args[0]).toEqual({ message: "" });
      const form = decoded.args[1] as FormData;
      expect(form.get("title")).toBe("Hello");
      expect([...form.keys()].some((k) => k.startsWith("$ACTION"))).toBe(false);
    }
    const simple = await decodeActionBody(formAction(id, { title: "Solo" }));
    expect(simple.ok && simple.args).toHaveLength(1);
  });

  test("urlencoded forms are converted to FormData", async () => {
    const request = new Request(ORIGIN, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ "$ACTION_ID_abc": "", title: "x" }).toString(),
    });
    const decoded = await decodeActionBody(request);
    expect(decoded.ok && decoded.id).toBe("abc");
    expect(decoded.ok && (decoded.args[0] as FormData).get("title")).toBe("x");
  });

  test("fetch FormData replies (encodeReply format) are revived", async () => {
    const form = new FormData();
    form.append("0", JSON.stringify(["$K1", 2]));
    form.append("1_title", "Hello");
    const request = new Request(ORIGIN, { method: "POST", headers: { "next-action": "x" }, body: form });
    const decoded = await decodeActionBody(request);
    expect(decoded.ok).toBe(true);
    if (decoded.ok) {
      expect(decoded.args[1]).toBe(2);
      expect((decoded.args[0] as FormData).get("title")).toBe("Hello");
    }
  });

  test("413 when the body exceeds the limit (content-length and streamed)", async () => {
    const big = fetchAction("id", ["x".repeat(2048)]);
    const declared = await decodeActionBody(big, 1024);
    expect(declared.ok).toBe(false);
    if (!declared.ok) {
      expect(declared.response.status).toBe(413);
      expect(await declared.response.text()).toContain("Body exceeded 1 kB limit");
    }
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(JSON.stringify(["y".repeat(700)])));
        controller.enqueue(new TextEncoder().encode(" ".repeat(700)));
        controller.close();
      },
    });
    const streamed = new Request(ORIGIN, { method: "POST", headers: { "next-action": "id", "content-type": "text/plain" }, body: stream, duplex: "half" } as RequestInit);
    const result = await decodeActionBody(streamed, 1024);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.response.status).toBe(413);
    }
  });
});

describe("handleServerAction", () => {
  test("rejects cross-origin requests unless allowed", async () => {
    const { registry, ids } = registryWith({ hello: async () => "hi" });
    const bad = await handleServerAction(fetchAction(ids.hello ?? "", [], { headers: { origin: "https://evil.example" } }), { registry });
    expect(bad.status).toBe(500);
    expect(await bad.text()).toBe("Invalid Server Actions request.");
    expect(bad.headers.get("cache-control")).toBe("no-cache, no-store, max-age=0, must-revalidate");
    const allowed = await handleServerAction(fetchAction(ids.hello ?? "", [], { headers: { origin: "https://app.example.com" } }), {
      registry,
      allowedOrigins: ["*.example.com"],
    });
    expect(allowed.status).toBe(200);
    const forwarded = await handleServerAction(fetchAction(ids.hello ?? "", [], { headers: { origin: "https://proxy.example", "x-forwarded-host": "proxy.example" } }), { registry });
    expect(forwarded.status).toBe(200);
  });

  test("unknown action ids yield 404 with x-nextjs-action-not-found", async () => {
    const response = await handleServerAction(fetchAction("00" + "f".repeat(38), []), { registry: new Map() });
    expect(response.status).toBe(404);
    expect(response.headers.get("x-nextjs-action-not-found")).toBe("1");
    expect(await response.text()).toBe("Server Action unavailable.");
  });

  test("fetch success returns a text/x-component Flight body with revalidation headers", async () => {
    setCacheHandler("default", new MemoryCacheHandler());
    const afterCalls: string[] = [];
    const { registry, ids } = registryWith({
      createPost: async (name: string) => {
        const h = await headers();
        (await cookies()).set("last", name, { httpOnly: true });
        updateTag("posts");
        after(() => {
          afterCalls.push(name);
        });
        return { message: `created ${name} via ${h.get("x-client") ?? "?"}`, at: new Date("2026-01-01T00:00:00.000Z") };
      },
    });
    const response = await handleServerAction(fetchAction(ids.createPost ?? "", ["Bob"], { headers: { "x-client": "test" } }), {
      registry,
      onRevalidate: (revalidated) => {
        expect(revalidated.tags).toEqual(["posts"]);
      },
    });
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("text/x-component");
    expect(response.headers.get("x-action-revalidated")).toBe('[[],["posts"],1]');
    expect(response.headers.getSetCookie()[0]).toContain("last=Bob");
    const text = await response.text();
    expect(text.startsWith("0:")).toBe(true);
    const result = decodeActionResult(text) as { message: string; at: Date };
    expect(result.message).toBe("created Bob via test");
    expect(result.at).toEqual(new Date("2026-01-01T00:00:00.000Z"));
    expect(afterCalls).toEqual(["Bob"]);
  });

  test("redirect() during a fetch action sets x-action-redirect; forms get 303", async () => {
    const { registry, ids } = registryWith({
      go: async () => {
        redirect("/done");
      },
    });
    const response = await handleServerAction(fetchAction(ids.go ?? "", []), { registry });
    expect(response.status).toBe(200);
    expect(response.headers.get("x-action-redirect")).toMatch(/^\/done;(push|replace)$/);
    expect(response.headers.get("content-type")).toBe("text/x-component");
    expect(decodeActionResult(await response.text())).toBeNull();
    const form = await handleServerAction(formAction(ids.go ?? "", {}), { registry });
    expect(form.status).toBe(303);
    expect(form.headers.get("location")).toBe("/done");
  });

  test("notFound() maps to 404 and other errors to 500 Flight errors", async () => {
    const errorSpy = spyOn(console, "error").mockImplementation(() => undefined);
    const { registry, ids } = registryWith({
      missing: async () => {
        notFound();
      },
      broken: async () => {
        throw Object.assign(new Error("kaboom"), { digest: "d1" });
      },
    });
    const missing = await handleServerAction(fetchAction(ids.missing ?? "", []), { registry });
    expect(missing.status).toBe(404);
    const broken = await handleServerAction(fetchAction(ids.broken ?? "", []), { registry });
    expect(broken.status).toBe(500);
    expect(broken.headers.get("content-type")).toBe("text/x-component");
    const text = await broken.text();
    expect(() => decodeActionResult(text)).toThrow("kaboom");
    errorSpy.mockRestore();
  });

  test("no-JS form posts redirect back with the form state cookie", async () => {
    const { registry, ids } = registryWith({
      createPost: async (prev: { message: string }, form: FormData) => ({ message: `${prev.message}${form.get("title")}` }),
    });
    const response = await handleServerAction(formAction(ids.createPost ?? "", { title: "Hello" }, { bound: [{ message: "prev:" }] }), { registry });
    expect(response.status).toBe(303);
    expect(response.headers.get("location")).toBe("/dashboard?tab=1");
    const cookie = response.headers.getSetCookie().find((c) => c.startsWith("__yolo_form_state="));
    expect(cookie).toBeDefined();
    expect(decodeURIComponent((cookie ?? "").split(";")[0]?.slice("__yolo_form_state=".length) ?? "")).toBe('{"message":"prev:Hello"}');
    expect(response.headers.get("x-action-revalidated")).toBe("[[],[],0]");
  });

  test("uses the provided store (phase forced to action)", async () => {
    const { registry, ids } = registryWith({ ping: async () => "pong" });
    const store = createRequestStore(fetchAction(ids.ping ?? "", []), { phase: "render", deploymentId: "dpl_1" });
    const response = await handleServerAction(fetchAction(ids.ping ?? "", []), { registry, store });
    expect(store.phase).toBe("action");
    expect(response.headers.get("x-nextjs-deployment-id")).toBe("dpl_1");
    expect(decodeActionResult(await response.text())).toBe("pong");
  });
});

describe("browser helpers", () => {
  test("encodeReply produces JSON or passes FormData through", () => {
    expect(encodeReply(["a", 1, undefined, "$x"])).toBe('["a",1,"$undefined","$$x"]');
    const form = new FormData();
    expect(encodeReply(form)).toBe(form);
    expect(encodeReply([form])).toBe(form);
  });

  test("callServer posts with Next headers and decodes the result", async () => {
    const { registry, ids } = registryWith({ hello: async (name: string) => `hi ${name}` });
    let seen: Request | undefined;
    const fetchImpl = (async (input: string | URL | Request, init?: RequestInit) => {
      seen = new Request(typeof input === "string" ? `${ORIGIN}${input}` : input, init);
      const request = new Request(`${ORIGIN}/`, { method: "POST", headers: { ...Object.fromEntries(seen.headers), origin: ORIGIN, host: "localhost:3000" }, body: await seen.text() });
      return handleServerAction(request, { registry });
    }) as typeof fetch;
    const result = await callServer(ids.hello ?? "", ["Ada"], fetchImpl);
    expect(result).toBe("hi Ada");
    expect(seen?.headers.get("next-action")).toBe(ids.hello ?? "");
    expect(seen?.headers.get("accept")).toBe("text/x-component");
    expect(seen?.headers.get("next-router-state-tree")).toBe("%5B%5D");
    expect(seen?.headers.get("content-type")).toBe("text/plain;charset=UTF-8");
  });
});
