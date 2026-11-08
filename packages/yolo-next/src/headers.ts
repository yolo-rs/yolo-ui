// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/headers: request-scoped context (`headers()`, `cookies()`, `draftMode()`,
// `connection()`, `after()`, `userAgent()`) backed by AsyncLocalStorage and Bun.CookieMap.

import { AsyncLocalStorage } from "node:async_hooks";
import { getCacheScope } from "./cache.ts";

/** Pipeline phase of the current request store. */
export type RequestPhase = "render" | "action" | "route" | "proxy";

/** Cookie attributes accepted by `ResponseCookies.set` (Next.js `ResponseCookie` shape). */
export interface CookieOptions {
  readonly expires?: number | Date;
  readonly maxAge?: number;
  readonly domain?: string;
  readonly path?: string;
  readonly secure?: boolean;
  readonly httpOnly?: boolean;
  readonly sameSite?: boolean | "lax" | "strict" | "none";
  readonly priority?: "low" | "medium" | "high";
  readonly partitioned?: boolean;
}

/** Parsed request cookie. */
export interface RequestCookie {
  readonly name: string;
  readonly value: string;
}

/** Cookie with its attributes, as passed to `ResponseCookies.set(options)`. */
export interface ResponseCookie extends RequestCookie, CookieOptions {}

/** Revalidation bookkeeping of a request (`x-action-revalidated`). */
export interface RevalidatedState {
  paths: string[];
  tags: string[];
  cookiesChanged: boolean;
  refresh: boolean;
}

/** Per-request state shared by rendering, Route Handlers, Server Actions and the proxy. */
export interface RequestStore {
  request: Request;
  url: URL;
  headers: Headers;
  cookies: ResponseCookies;
  responseHeaders: Headers;
  status: number;
  params: Record<string, string>;
  searchParams: URLSearchParams;
  draft: { enabled: boolean };
  after: Array<() => unknown | Promise<unknown>>;
  revalidated: RevalidatedState;
  deploymentId?: string;
  phase: RequestPhase;
  isPrerender: boolean;
  /** Set when a dynamic API (`headers`, `cookies`, `connection`, `draftMode`) was used. */
  usedDynamic: boolean;
}

/** Hooks wired by the request store into `ResponseCookies`. */
export interface ResponseCookiesHooks {
  readonly canModify?: () => void;
  readonly onSet?: (setCookieHeader: string) => void;
}

/** Name of the draft mode bypass cookie (Next.js compatible). */
export const PRERENDER_BYPASS_COOKIE = "__prerender_bypass";

/** Error thrown when cookies are mutated outside a Server Action or Route Handler. */
export const COOKIES_READ_ONLY_MESSAGE =
  "Cookies can only be modified in a Server Action or Route Handler";

const MAX_AGE_DELETE = new Date(0);

function toCookieInit(name: string, value: string, options: CookieOptions): Bun.CookieInit {
  const init: Bun.CookieInit = { name, value, path: options.path ?? "/" };
  if (options.domain !== undefined) {
    init.domain = options.domain;
  }
  if (options.expires !== undefined) {
    init.expires = typeof options.expires === "number" ? new Date(options.expires) : options.expires;
  }
  if (options.maxAge !== undefined) {
    init.maxAge = options.maxAge;
  }
  if (options.secure !== undefined) {
    init.secure = options.secure;
  }
  if (options.httpOnly !== undefined) {
    init.httpOnly = options.httpOnly;
  }
  if (options.partitioned !== undefined) {
    init.partitioned = options.partitioned;
  }
  const sameSite = options.sameSite;
  if (sameSite === true || sameSite === "strict") {
    init.sameSite = "strict";
  } else if (sameSite === "none") {
    init.sameSite = "none";
  } else {
    init.sameSite = "lax";
  }
  return init;
}

/** Serializes one cookie into a `Set-Cookie` header value via `Bun.Cookie`. */
export function serializeCookie(name: string, value: string, options: CookieOptions = {}): string {
  const init = toCookieInit(name, value, options);
  let serialized = new Bun.Cookie(name, value, init).serialize();
  if (options.priority) {
    const priority = options.priority.charAt(0).toUpperCase() + options.priority.slice(1);
    serialized += `; Priority=${priority}`;
  }
  return serialized;
}

/** Next.js compatible cookie jar over `Bun.CookieMap` (reads the request, records `Set-Cookie`). */
export class ResponseCookies implements Iterable<[string, ResponseCookie]> {
  readonly #map: Bun.CookieMap;
  readonly #pending = new Map<string, ResponseCookie>();
  readonly #hooks: ResponseCookiesHooks;

  constructor(cookieHeader = "", hooks: ResponseCookiesHooks = {}) {
    this.#map = new Bun.CookieMap(cookieHeader);
    this.#hooks = hooks;
  }

  /** Number of cookies visible to the request. */
  get size(): number {
    return this.#map.size;
  }

  /** Returns the cookie `{ name, value }` or `undefined`. */
  get(name: string): ResponseCookie | undefined;
  get(cookie: Pick<ResponseCookie, "name">): ResponseCookie | undefined;
  get(nameOrCookie: string | Pick<ResponseCookie, "name">): ResponseCookie | undefined {
    const name = typeof nameOrCookie === "string" ? nameOrCookie : nameOrCookie.name;
    const pending = this.#pending.get(name);
    if (pending) {
      return pending;
    }
    const value = this.#map.get(name);
    return value === null ? undefined : { name, value };
  }

  /** Returns every cookie, optionally filtered by name. */
  getAll(name?: string | Pick<ResponseCookie, "name">): ResponseCookie[] {
    const filter = typeof name === "string" ? name : name?.name;
    const all: ResponseCookie[] = [];
    for (const [key, value] of this.#map.entries()) {
      all.push(this.#pending.get(key) ?? { name: key, value });
    }
    return filter === undefined ? all : all.filter((cookie) => cookie.name === filter);
  }

  /** Whether a cookie exists. */
  has(name: string): boolean {
    return this.#map.has(name);
  }

  /** Sets a cookie and records its `Set-Cookie` header. */
  set(name: string, value: string, options?: CookieOptions): this;
  set(cookie: ResponseCookie): this;
  set(nameOrCookie: string | ResponseCookie, value?: string, options: CookieOptions = {}): this {
    const cookie: ResponseCookie =
      typeof nameOrCookie === "string"
        ? { ...options, name: nameOrCookie, value: value ?? "" }
        : nameOrCookie;
    this.#hooks.canModify?.();
    const { name, value: cookieValue, ...attributes } = cookie;
    this.#map.set(name, cookieValue);
    this.#pending.set(name, cookie);
    this.#hooks.onSet?.(serializeCookie(name, cookieValue, attributes));
    return this;
  }

  /** Expires a cookie (`Max-Age=0`, `Expires=1970`). */
  delete(name: string | Pick<ResponseCookie, "name" | "path" | "domain">): this {
    const cookie = typeof name === "string" ? { name } : name;
    this.#hooks.canModify?.();
    this.#map.delete(cookie.name);
    this.#pending.delete(cookie.name);
    const attributes: CookieOptions = {
      expires: MAX_AGE_DELETE,
      maxAge: 0,
      path: "path" in cookie && cookie.path !== undefined ? cookie.path : "/",
    };
    const domain = "domain" in cookie ? cookie.domain : undefined;
    this.#hooks.onSet?.(serializeCookie(cookie.name, "", domain ? { ...attributes, domain } : attributes));
    return this;
  }

  /** `Cookie` header representation (`a=1; b=2`). */
  toString(): string {
    const pairs: string[] = [];
    for (const [key, value] of this.#map.entries()) {
      pairs.push(`${key}=${encodeURIComponent(value)}`);
    }
    return pairs.join("; ");
  }

  /** Iterates `[name, cookie]` pairs. */
  *[Symbol.iterator](): Iterator<[string, ResponseCookie]> {
    for (const cookie of this.getAll()) {
      yield [cookie.name, cookie];
    }
  }
}

/** Frozen `Headers` view returned by `headers()`; mutators throw like in Next.js. */
export class ReadonlyHeaders extends Headers {
  override set(_name: string, _value: string): never {
    throw new Error("Headers cannot be modified. Read more: https://nextjs.org/docs/app/api-reference/functions/headers");
  }

  override append(_name: string, _value: string): never {
    throw new Error("Headers cannot be modified. Read more: https://nextjs.org/docs/app/api-reference/functions/headers");
  }

  override delete(_name: string): never {
    throw new Error("Headers cannot be modified. Read more: https://nextjs.org/docs/app/api-reference/functions/headers");
  }
}

/** AsyncLocalStorage carrying the current `RequestStore`. */
export const requestStorage = new AsyncLocalStorage<RequestStore>();

/** Optional overrides accepted by `createRequestStore`. */
export interface RequestStoreInit {
  readonly params?: Record<string, string>;
  readonly phase?: RequestPhase;
  readonly isPrerender?: boolean;
  readonly deploymentId?: string;
}

/** Builds a `RequestStore` for `request` (cookies parsed from the `cookie` header). */
export function createRequestStore(request: Request, extra: RequestStoreInit = {}): RequestStore {
  const url = new URL(request.url);
  const headers = new Headers(request.headers);
  const cookieHeader = headers.get("cookie") ?? "";
  const store: RequestStore = {
    request,
    url,
    headers,
    cookies: new ResponseCookies(cookieHeader),
    responseHeaders: new Headers(),
    status: 200,
    params: extra.params ?? {},
    searchParams: url.searchParams,
    draft: { enabled: false },
    after: [],
    revalidated: { paths: [], tags: [], cookiesChanged: false, refresh: false },
    phase: extra.phase ?? "render",
    isPrerender: extra.isPrerender ?? false,
    usedDynamic: false,
  };
  if (extra.deploymentId !== undefined) {
    store.deploymentId = extra.deploymentId;
  }
  store.cookies = new ResponseCookies(cookieHeader, {
    canModify: () => {
      if (store.phase === "render") {
        throw new Error(COOKIES_READ_ONLY_MESSAGE);
      }
    },
    onSet: (setCookie) => {
      store.responseHeaders.append("set-cookie", setCookie);
      store.revalidated.cookiesChanged = true;
    },
  });
  store.draft.enabled = store.cookies.has(PRERENDER_BYPASS_COOKIE);
  return store;
}

/** Runs `fn` with `store` as the current request store. */
export function runWithRequestStore<T>(store: RequestStore, fn: () => T): T {
  return requestStorage.run(store, fn);
}

/** Current request store or `undefined` outside a request. */
export function peekRequestStore(): RequestStore | undefined {
  return requestStorage.getStore();
}

function outsideScopeError(api: string): Error {
  return new Error(
    `\`${api}\` was called outside a request scope. Read more: https://nextjs.org/docs/messages/next-dynamic-api-wrong-context`,
  );
}

/** Current request store; throws Next's "called outside a request scope" error otherwise. */
export function getRequestStore(api = "headers"): RequestStore {
  const store = requestStorage.getStore();
  if (!store) {
    throw outsideScopeError(api);
  }
  return store;
}

function assertNotInsideCache(api: string): void {
  const scope = getCacheScope();
  if (scope && scope.variant !== "private") {
    const error = new Error(
      `Route used "${api}" inside "use cache". Accessing Dynamic data sources inside a cache scope is not supported. Read more: https://nextjs.org/docs/messages/next-request-in-use-cache`,
    ) as Error & { digest: string };
    error.digest = "next-request-in-use-cache";
    throw error;
  }
}

function useDynamic(store: RequestStore, api: string): void {
  if (store.isPrerender) {
    const error = new Error(
      `Route used ${api} during prerendering, which opts the route into dynamic rendering. Read more: https://nextjs.org/docs/messages/dynamic-server-error`,
    ) as Error & { digest: string };
    error.digest = "DYNAMIC_SERVER_USAGE";
    throw error;
  }
  store.usedDynamic = true;
}

/** Read-only request headers (Next.js 16 async signature). */
export async function headers(): Promise<ReadonlyHeaders> {
  assertNotInsideCache("headers");
  const store = getRequestStore("headers");
  store.usedDynamic = true;
  return new ReadonlyHeaders(store.headers);
}

/** Request/response cookie jar (Next.js 16 async signature). */
export async function cookies(): Promise<ResponseCookies> {
  assertNotInsideCache("cookies");
  const store = getRequestStore("cookies");
  store.usedDynamic = true;
  return store.cookies;
}

/** Draft mode controls backed by the `__prerender_bypass` cookie. */
export interface DraftMode {
  readonly isEnabled: boolean;
  enable(): void;
  disable(): void;
}

/** Draft mode state of the current request (Next.js 16 async signature). */
export async function draftMode(): Promise<DraftMode> {
  assertNotInsideCache("draftMode");
  const store = getRequestStore("draftMode");
  store.usedDynamic = true;
  const previewId = Bun.env.YOLO_PREVIEW_MODE_ID ?? "yolo";
  return {
    get isEnabled(): boolean {
      return store.draft.enabled;
    },
    enable(): void {
      store.cookies.set(PRERENDER_BYPASS_COOKIE, previewId, {
        httpOnly: true,
        sameSite: "none",
        secure: true,
        path: "/",
      });
      store.draft.enabled = true;
    },
    disable(): void {
      store.cookies.delete(PRERENDER_BYPASS_COOKIE);
      store.draft.enabled = false;
    },
  };
}

/** Opts the current render into dynamic rendering (throws with `DYNAMIC_SERVER_USAGE` during prerender). */
export async function connection(): Promise<void> {
  assertNotInsideCache("connection");
  const store = getRequestStore("connection");
  useDynamic(store, "connection()");
}

/** Schedules `callback` to run after the response is sent (Next.js `after`). */
export function after(callback: () => unknown | Promise<unknown>): void {
  const store = requestStorage.getStore();
  if (!store) {
    throw outsideScopeError("after");
  }
  store.after.push(callback);
}

/** Awaits all `after` callbacks of `store`; failures are logged, never thrown. */
export async function runAfterCallbacks(store: RequestStore): Promise<void> {
  const callbacks = store.after.splice(0, store.after.length);
  const results = await Promise.allSettled(callbacks.map(async (callback) => callback()));
  for (const result of results) {
    if (result.status === "rejected") {
      console.error("[yolo-next] after() callback failed:", result.reason);
    }
  }
}

/** Copies the store's response headers (`Set-Cookie`) into `target`. */
export function mergeStoreHeaders(store: RequestStore, target: Headers): Headers {
  for (const [key, value] of store.responseHeaders) {
    if (key === "set-cookie") {
      target.append("set-cookie", value);
    } else if (!target.has(key)) {
      target.set(key, value);
    }
  }
  return target;
}

/** Result of `userAgent()` (ua-parser-js compatible subset). */
export interface UserAgent {
  readonly isBot: boolean;
  readonly ua: string;
  readonly browser: { name?: string; version?: string };
  readonly device: { type?: string; vendor?: string; model?: string };
  readonly engine: { name?: string; version?: string };
  readonly os: { name?: string; version?: string };
  readonly cpu: { architecture?: string };
}

const BOT_PATTERN = /bot|crawl|spider|slurp|facebookexternalhit|preview|lighthouse/i;

const BROWSERS: ReadonlyArray<readonly [string, RegExp]> = [
  ["Edge", /\bEdg(?:e|A|iOS)?\/([\d.]+)/],
  ["Opera", /\bOPR\/([\d.]+)/],
  ["Firefox", /\bFirefox\/([\d.]+)/],
  ["Chrome", /\bChrome\/([\d.]+)/],
  ["Safari", /\bVersion\/([\d.]+).*\bSafari\b/],
];

function parseOs(ua: string): { name?: string; version?: string } {
  const windows = /Windows NT ([\d.]+)/.exec(ua);
  if (windows) {
    const nt = windows[1] ?? "";
    const version = nt === "10.0" ? "10" : nt === "6.3" ? "8.1" : nt === "6.2" ? "8" : nt === "6.1" ? "7" : nt;
    return { name: "Windows", version };
  }
  const android = /Android ([\d.]+)/.exec(ua);
  if (android) {
    return { name: "Android", version: android[1] ?? "" };
  }
  const ios = /(?:iPhone|CPU) OS ([\d_]+)/.exec(ua);
  if (ios) {
    return { name: "iOS", version: (ios[1] ?? "").replace(/_/g, ".") };
  }
  const mac = /Mac OS X ([\d_.]+)/.exec(ua);
  if (mac) {
    return { name: "Mac OS", version: (mac[1] ?? "").replace(/_/g, ".") };
  }
  if (/\bLinux\b/.test(ua)) {
    return { name: "Linux" };
  }
  return {};
}

function parseDevice(ua: string): { type?: string; vendor?: string; model?: string } {
  const device: { type?: string; vendor?: string; model?: string } = {};
  if (/Tablet|iPad/i.test(ua)) {
    device.type = "tablet";
  } else if (/Mobi/i.test(ua)) {
    device.type = "mobile";
  }
  if (/iPhone/.test(ua)) {
    device.vendor = "Apple";
    device.model = "iPhone";
  } else if (/iPad/.test(ua)) {
    device.vendor = "Apple";
    device.model = "iPad";
  } else if (/Macintosh/.test(ua)) {
    device.vendor = "Apple";
    device.model = "Macintosh";
  }
  return device;
}

/** Parses the `user-agent` header into a compact `UserAgent` descriptor. */
export function userAgent(input: { headers: Headers | Record<string, string> }): UserAgent {
  const ua =
    input.headers instanceof Headers
      ? (input.headers.get("user-agent") ?? "")
      : (input.headers["user-agent"] ?? input.headers["User-Agent"] ?? "");
  const browser: { name?: string; version?: string } = {};
  for (const [name, pattern] of BROWSERS) {
    const match = pattern.exec(ua);
    if (match) {
      browser.name = name;
      browser.version = match[1] ?? "";
      break;
    }
  }
  const engine: { name?: string; version?: string } = {};
  if (browser.name === "Firefox") {
    const gecko = /\brv:([\d.]+)/.exec(ua);
    engine.name = "Gecko";
    if (gecko) {
      engine.version = gecko[1] ?? "";
    }
  } else if (browser.name === "Safari") {
    const webkit = /AppleWebKit\/([\d.]+)/.exec(ua);
    engine.name = "WebKit";
    if (webkit) {
      engine.version = webkit[1] ?? "";
    }
  } else if (browser.name) {
    engine.name = "Blink";
    if (browser.version) {
      engine.version = browser.version;
    }
  }
  return {
    isBot: BOT_PATTERN.test(ua),
    ua,
    browser,
    device: parseDevice(ua),
    engine,
    os: parseOs(ua),
    cpu: {},
  };
}
