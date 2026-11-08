// SPDX-License-Identifier: Apache-2.0
// next/server shim: NextRequest, NextResponse, NextURL, NextFetchEvent, userAgent, after, connection, ImageResponse.

import { after, connection, serializeCookie, userAgent, type CookieOptions, type UserAgent } from "../headers.ts";
import { ImageResponse } from "../og.ts";

/** `after`, `connection`, `userAgent` and `ImageResponse` as exported by `next/server`. */
export { after, connection, userAgent, ImageResponse };
/** `userAgent()` result type. */
export type { UserAgent };

/** Parsed cookie (`name` / `value`). */
export interface RequestCookie {
  readonly name: string;
  readonly value: string;
}

/** Cookie with attributes as stored on a response. */
export interface ResponseCookie extends RequestCookie, CookieOptions {}

function parseCookieHeader(header: string | null): Map<string, string> {
  const out = new Map<string, string>();
  if (!header) {
    return out;
  }
  for (const part of header.split(";")) {
    const separator = part.indexOf("=");
    if (separator === -1) {
      continue;
    }
    const name = part.slice(0, separator).trim();
    if (!name) {
      continue;
    }
    let value = part.slice(separator + 1).trim();
    if (value.startsWith('"') && value.endsWith('"')) {
      value = value.slice(1, -1);
    }
    try {
      value = decodeURIComponent(value);
    } catch {
      // Keep the raw value.
    }
    out.set(name, value);
  }
  return out;
}

/** Request cookies (`NextRequest.cookies`) backed by the request `cookie` header. */
export class RequestCookies implements Iterable<[string, RequestCookie]> {
  readonly #headers: Headers;
  readonly #cookies: Map<string, string>;

  constructor(headers: Headers) {
    this.#headers = headers;
    this.#cookies = parseCookieHeader(headers.get("cookie"));
  }

  /** Number of cookies. */
  get size(): number {
    return this.#cookies.size;
  }

  /** Iterates `[name, cookie]` pairs. */
  *[Symbol.iterator](): Iterator<[string, RequestCookie]> {
    for (const [name, value] of this.#cookies) {
      yield [name, { name, value }];
    }
  }

  /** Cookie by name (or `{ name }`). */
  get(...args: [name: string] | [options: { name: string }]): RequestCookie | undefined {
    const name = typeof args[0] === "string" ? args[0] : args[0].name;
    const value = this.#cookies.get(name);
    return value === undefined ? undefined : { name, value };
  }

  /** All cookies, optionally filtered by name. */
  getAll(...args: [name: string] | [options: { name: string }] | []): RequestCookie[] {
    const all = [...this].map(([, cookie]) => cookie);
    if (args.length === 0) {
      return all;
    }
    const name = typeof args[0] === "string" ? args[0] : args[0].name;
    return all.filter((cookie) => cookie.name === name);
  }

  /** True when the cookie exists. */
  has(name: string): boolean {
    return this.#cookies.has(name);
  }

  /** Sets a cookie and updates the `cookie` header. */
  set(...args: [key: string, value: string] | [options: RequestCookie]): this {
    const [name, value] = args.length === 1 ? [args[0].name, args[0].value] : args;
    this.#cookies.set(name, value);
    this.#sync();
    return this;
  }

  /** Deletes cookies and updates the header; returns whether each existed. */
  delete(names: string | string[]): boolean | boolean[] {
    const result = Array.isArray(names) ? names.map((name) => this.#cookies.delete(name)) : this.#cookies.delete(names);
    this.#sync();
    return result;
  }

  /** Removes every cookie. */
  clear(): this {
    this.#cookies.clear();
    this.#sync();
    return this;
  }

  /** Serialized `cookie` header value. */
  toString(): string {
    return [...this.#cookies].map(([name, value]) => `${name}=${encodeURIComponent(value)}`).join("; ");
  }

  #sync(): void {
    const serialized = this.toString();
    if (serialized) {
      this.#headers.set("cookie", serialized);
    } else {
      this.#headers.delete("cookie");
    }
  }
}

function parseSetCookie(header: string): ResponseCookie | undefined {
  const [pair, ...attributes] = header.split(";");
  if (!pair) {
    return undefined;
  }
  const separator = pair.indexOf("=");
  if (separator === -1) {
    return undefined;
  }
  const name = pair.slice(0, separator).trim();
  let value = pair.slice(separator + 1).trim();
  try {
    value = decodeURIComponent(value);
  } catch {
    // Keep the raw value.
  }
  const cookie: { -readonly [K in keyof ResponseCookie]: ResponseCookie[K] } = { name, value };
  for (const attribute of attributes) {
    const [rawKey = "", ...rest] = attribute.trim().split("=");
    const attributeValue = rest.join("=");
    switch (rawKey.toLowerCase()) {
      case "path":
        cookie.path = attributeValue;
        break;
      case "domain":
        cookie.domain = attributeValue;
        break;
      case "expires": {
        const date = new Date(attributeValue);
        if (!Number.isNaN(date.getTime())) {
          cookie.expires = date;
        }
        break;
      }
      case "max-age":
        cookie.maxAge = Number(attributeValue);
        break;
      case "secure":
        cookie.secure = true;
        break;
      case "httponly":
        cookie.httpOnly = true;
        break;
      case "samesite": {
        const sameSite = attributeValue.toLowerCase();
        if (sameSite === "lax" || sameSite === "strict" || sameSite === "none") {
          cookie.sameSite = sameSite;
        }
        break;
      }
      case "priority": {
        const priority = attributeValue.toLowerCase();
        if (priority === "low" || priority === "medium" || priority === "high") {
          cookie.priority = priority;
        }
        break;
      }
      case "partitioned":
        cookie.partitioned = true;
        break;
      default:
        break;
    }
  }
  return cookie;
}

/** Response cookies (`NextResponse.cookies`) backed by the `Set-Cookie` headers. */
export class ResponseCookies implements Iterable<[string, ResponseCookie]> {
  readonly #headers: Headers;
  readonly #cookies = new Map<string, ResponseCookie>();

  constructor(headers: Headers) {
    this.#headers = headers;
    for (const header of headers.getSetCookie()) {
      const cookie = parseSetCookie(header);
      if (cookie) {
        this.#cookies.set(cookie.name, cookie);
      }
    }
  }

  /** Iterates `[name, cookie]` pairs. */
  [Symbol.iterator](): Iterator<[string, ResponseCookie]> {
    return this.#cookies[Symbol.iterator]();
  }

  /** Cookie by name (or `{ name }`). */
  get(...args: [name: string] | [options: { name: string }]): ResponseCookie | undefined {
    const name = typeof args[0] === "string" ? args[0] : args[0].name;
    return this.#cookies.get(name);
  }

  /** All cookies, optionally filtered by name. */
  getAll(...args: [name: string] | [options: { name: string }] | []): ResponseCookie[] {
    const all = [...this.#cookies.values()];
    if (args.length === 0) {
      return all;
    }
    const name = typeof args[0] === "string" ? args[0] : args[0].name;
    return all.filter((cookie) => cookie.name === name);
  }

  /** True when a cookie with `name` is set on the response. */
  has(name: string): boolean {
    return this.#cookies.has(name);
  }

  /** Sets a cookie (`set(name, value, options)` or `set({ name, value, ...options })`). */
  set(...args: [key: string, value: string, options?: CookieOptions] | [options: ResponseCookie]): this {
    const cookie: ResponseCookie =
      args.length === 1 ? args[0] : { ...(args[2] ?? {}), name: args[0], value: args[1] };
    this.#cookies.set(cookie.name, cookie);
    this.#sync();
    return this;
  }

  /** Deletes a cookie by setting it expired (`delete(name)` or `delete({ name, path, domain })`). */
  delete(...args: [key: string] | [options: { name: string; path?: string; domain?: string }]): this {
    const options = typeof args[0] === "string" ? { name: args[0] } : args[0];
    const expired: ResponseCookie = { name: options.name, value: "", expires: new Date(0), path: options.path ?? "/" };
    if (options.domain !== undefined) {
      this.set({ ...expired, domain: options.domain });
    } else {
      this.set(expired);
    }
    return this;
  }

  /** Serialized `Set-Cookie` headers joined by `, `. */
  toString(): string {
    return [...this.#cookies.values()].map(serializeResponseCookie).join(", ");
  }

  #sync(): void {
    this.#headers.delete("set-cookie");
    for (const cookie of this.#cookies.values()) {
      this.#headers.append("set-cookie", serializeResponseCookie(cookie));
    }
  }
}

function serializeResponseCookie(cookie: ResponseCookie): string {
  const { name, value, ...options } = cookie;
  return serializeCookie(name, value, options);
}

/** i18n configuration subset used by `NextURL`. */
export interface NextURLI18n {
  readonly locales: ReadonlyArray<string>;
  readonly defaultLocale: string;
}

/** `nextConfig` subset accepted by `NextRequest` / `NextURL`. */
export interface NextURLConfig {
  readonly basePath?: string;
  readonly i18n?: NextURLI18n | null;
  readonly trailingSlash?: boolean;
}

/** Options of `NextURL`. */
export interface NextURLOptions extends NextURLConfig {
  readonly headers?: Headers | Record<string, string | string[] | undefined>;
  readonly nextConfig?: NextURLConfig;
}

/** URL wrapper aware of `basePath`, `locale`, `buildId` and `trailingSlash` (Next's `NextURL`). */
export class NextURL {
  #url: URL;
  #basePath: string;
  #trailingSlash: boolean;
  #i18n: NextURLI18n | undefined;
  #locale = "";
  #buildId: string | undefined;

  constructor(input: string | URL, baseOrOptions?: string | URL | NextURLOptions, maybeOptions?: NextURLOptions) {
    const base = typeof baseOrOptions === "string" || baseOrOptions instanceof URL ? baseOrOptions : undefined;
    const options = (base === undefined ? (baseOrOptions as NextURLOptions | undefined) : maybeOptions) ?? {};
    this.#url = new URL(String(input), base);
    const config = options.nextConfig;
    this.#basePath = (options.basePath ?? config?.basePath ?? "").replace(/\/+$/, "");
    this.#trailingSlash = options.trailingSlash ?? config?.trailingSlash ?? false;
    this.#i18n = options.i18n ?? config?.i18n ?? undefined;
    this.#analyze();
  }

  #analyze(): void {
    const stripped = this.#stripBasePath(this.#url.pathname);
    const i18n = this.#i18n;
    if (!i18n) {
      return;
    }
    const segment = stripped.split("/")[1] ?? "";
    const match = i18n.locales.find((locale) => locale.toLowerCase() === segment.toLowerCase());
    this.#locale = match ?? i18n.defaultLocale;
  }

  #stripBasePath(pathname: string): string {
    const basePath = this.#basePath;
    if (basePath && (pathname === basePath || pathname.startsWith(`${basePath}/`))) {
      return pathname.slice(basePath.length) || "/";
    }
    return pathname;
  }

  #stripLocale(pathname: string): string {
    const i18n = this.#i18n;
    if (!i18n) {
      return pathname;
    }
    const segment = pathname.split("/")[1] ?? "";
    const match = i18n.locales.find((locale) => locale.toLowerCase() === segment.toLowerCase());
    return match ? pathname.slice(match.length + 1) || "/" : pathname;
  }

  #compose(pathname: string): string {
    const localePrefix = this.#i18n && this.#locale && this.#locale !== this.#i18n.defaultLocale ? `/${this.#locale}` : "";
    const path = `${this.#basePath}${localePrefix}${pathname.startsWith("/") ? pathname : `/${pathname}`}`;
    return path === "" ? "/" : path;
  }

  /** `basePath` stripped from `pathname`. */
  get basePath(): string {
    return this.#basePath;
  }

  set basePath(value: string) {
    const pathname = this.pathname;
    this.#basePath = value.replace(/\/+$/, "");
    this.pathname = pathname;
  }

  /** Build id of the deployment (set by the runtime; empty otherwise). */
  get buildId(): string | undefined {
    return this.#buildId;
  }

  set buildId(value: string | undefined) {
    this.#buildId = value;
  }

  /** Detected locale (defaults to `defaultLocale`; empty without i18n). */
  get locale(): string {
    return this.#locale;
  }

  set locale(value: string) {
    const i18n = this.#i18n;
    if (!i18n || !i18n.locales.includes(value)) {
      throw new TypeError(`The NextURL configuration includes no locale "${value}"`);
    }
    const pathname = this.pathname;
    this.#locale = value;
    this.pathname = pathname;
  }

  /** Default locale of the i18n configuration. */
  get defaultLocale(): string | undefined {
    return this.#i18n?.defaultLocale;
  }

  /** Domain locale (never detected by Yolo). */
  get domainLocale(): undefined {
    return undefined;
  }

  /** Pathname without `basePath` and locale prefix. */
  get pathname(): string {
    return this.#stripLocale(this.#stripBasePath(this.#url.pathname));
  }

  set pathname(value: string) {
    this.#url.pathname = this.#compose(value);
  }

  /** Full href (with `basePath`, locale and trailing slash applied). */
  get href(): string {
    const url = new URL(this.#url.href);
    if (this.#trailingSlash && !url.pathname.endsWith("/")) {
      url.pathname += "/";
    }
    return url.href;
  }

  set href(value: string) {
    this.#url = new URL(value);
    this.#analyze();
  }

  /** Origin. */
  get origin(): string {
    return this.#url.origin;
  }

  /** Protocol (`https:`). */
  get protocol(): string {
    return this.#url.protocol;
  }

  set protocol(value: string) {
    this.#url.protocol = value;
  }

  /** Host with port. */
  get host(): string {
    return this.#url.host;
  }

  set host(value: string) {
    this.#url.host = value;
  }

  /** Hostname. */
  get hostname(): string {
    return this.#url.hostname;
  }

  set hostname(value: string) {
    this.#url.hostname = value;
  }

  /** Port. */
  get port(): string {
    return this.#url.port;
  }

  set port(value: string) {
    this.#url.port = value;
  }

  /** Query string including `?`. */
  get search(): string {
    return this.#url.search;
  }

  set search(value: string) {
    this.#url.search = value;
  }

  /** Live `URLSearchParams`. */
  get searchParams(): URLSearchParams {
    return this.#url.searchParams;
  }

  /** Fragment including `#`. */
  get hash(): string {
    return this.#url.hash;
  }

  set hash(value: string) {
    this.#url.hash = value;
  }

  /** Username. */
  get username(): string {
    return this.#url.username;
  }

  set username(value: string) {
    this.#url.username = value;
  }

  /** Password. */
  get password(): string {
    return this.#url.password;
  }

  set password(value: string) {
    this.#url.password = value;
  }

  /** Copy of this URL with the same configuration. */
  clone(): NextURL {
    const copy = new NextURL(this.#url.href, {
      basePath: this.#basePath,
      trailingSlash: this.#trailingSlash,
      i18n: this.#i18n ?? null,
    });
    copy.#locale = this.#locale;
    copy.#buildId = this.#buildId;
    return copy;
  }

  /** Same as `href`. */
  toString(): string {
    return this.href;
  }

  /** Same as `href`. */
  toJSON(): string {
    return this.href;
  }
}

/** Geolocation shape of the removed `request.geo` (always `undefined` in Next 16). */
export interface Geo {
  readonly city?: string;
  readonly country?: string;
  readonly region?: string;
  readonly latitude?: string;
  readonly longitude?: string;
}

/** Init of `NextRequest`. */
export interface NextRequestInit extends RequestInit {
  readonly nextConfig?: NextURLConfig;
  readonly ip?: string;
  readonly geo?: Geo;
}

function stripNextInit(init: NextRequestInit): RequestInit {
  const { nextConfig: _config, ip: _ip, geo: _geo, ...rest } = init;
  return rest;
}

/** Request with parsed `cookies` and a `nextUrl` (`next/server` `NextRequest`). */
export class NextRequest extends Request {
  readonly cookies: RequestCookies;
  readonly nextUrl: NextURL;
  readonly ip: string | undefined;
  readonly geo: Geo | undefined;

  constructor(input: URL | RequestInfo, init: NextRequestInit = {}) {
    super(input instanceof Request ? input : String(input), stripNextInit(init));
    this.nextUrl = new NextURL(this.url, { nextConfig: init.nextConfig, headers: this.headers });
    this.cookies = new RequestCookies(this.headers);
    this.ip = init.ip;
    this.geo = init.geo;
  }
}

/** Init accepted by `NextResponse.next` / `NextResponse.rewrite` (`request.headers` overrides). */
export interface MiddlewareResponseInit extends ResponseInit {
  readonly request?: { readonly headers?: Headers };
}

const REDIRECT_STATUSES: ReadonlySet<number> = new Set([301, 302, 303, 307, 308]);

function applyRequestHeaderOverrides(init: MiddlewareResponseInit | undefined, headers: Headers): void {
  const requestHeaders = init?.request?.headers;
  if (!requestHeaders) {
    return;
  }
  if (!(requestHeaders instanceof Headers)) {
    throw new Error("request.headers must be an instance of Headers");
  }
  const keys: string[] = [];
  for (const [key, value] of requestHeaders) {
    headers.set(`x-middleware-request-${key}`, value);
    keys.push(key);
  }
  headers.set("x-middleware-override-headers", keys.join(","));
}

/** Response with `cookies` plus the `json`, `redirect`, `rewrite` and `next` helpers (`next/server` `NextResponse`). */
export class NextResponse<Body = unknown> extends Response {
  readonly cookies: ResponseCookies;

  constructor(body?: BodyInit | null, init: ResponseInit = {}) {
    super(body, init);
    this.cookies = new ResponseCookies(this.headers);
  }

  /** JSON response (`content-type: application/json`). */
  static override json<JsonBody>(body: JsonBody, init?: ResponseInit): NextResponse<JsonBody> {
    const response = Response.json(body, init);
    return new NextResponse<JsonBody>(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  }

  /** Redirect response; `init` may be a status code or a `ResponseInit` (default 307). */
  static override redirect(url: string | URL | NextURL, init?: number | ResponseInit): NextResponse {
    const status = typeof init === "number" ? init : (init?.status ?? 307);
    if (!REDIRECT_STATUSES.has(status)) {
      throw new RangeError('Failed to execute "redirect" on "response": Invalid status code');
    }
    const responseInit = typeof init === "object" ? init : {};
    const headers = new Headers(responseInit.headers);
    headers.set("location", String(url));
    return new NextResponse(null, { ...responseInit, headers, status });
  }

  /** Rewrite marker response (`x-middleware-rewrite: <destination>`). */
  static rewrite(destination: string | URL | NextURL, init?: MiddlewareResponseInit): NextResponse {
    const headers = new Headers(init?.headers);
    headers.set("x-middleware-rewrite", String(destination));
    applyRequestHeaderOverrides(init, headers);
    return new NextResponse(null, { ...init, headers });
  }

  /** Continue marker response (`x-middleware-next: 1`) with optional request header overrides. */
  static next(init?: MiddlewareResponseInit): NextResponse {
    const headers = new Headers(init?.headers);
    headers.set("x-middleware-next", "1");
    applyRequestHeaderOverrides(init, headers);
    return new NextResponse(null, { ...init, headers });
  }
}

/** Event passed to middleware / proxy functions (`event.waitUntil`). */
export class NextFetchEvent {
  readonly sourcePage: string;
  readonly #waitUntil: ((promise: Promise<unknown>) => void) | undefined;

  constructor(params: {
    readonly request: NextRequest;
    readonly page: string;
    readonly context?: { waitUntil(promise: Promise<unknown>): void };
  }) {
    this.sourcePage = params.page;
    this.#waitUntil = params.context ? params.context.waitUntil.bind(params.context) : undefined;
  }

  /** Keeps the request alive until `promise` settles. */
  waitUntil(promise: Promise<unknown>): void {
    if (this.#waitUntil) {
      this.#waitUntil(promise);
    } else {
      void promise.catch(() => undefined);
    }
  }
}

/** Result accepted from a middleware / proxy function. */
export type NextMiddlewareResult = NextResponse | Response | null | undefined;

/** Signature of a `proxy.ts` / `middleware.ts` function. */
export type NextMiddleware = (request: NextRequest, event: NextFetchEvent) => NextMiddlewareResult | Promise<NextMiddlewareResult>;

/** `export const config = { matcher }` shape of a proxy file. */
export interface MiddlewareConfig {
  readonly matcher?: string | ReadonlyArray<string | { source: string; regexp?: string; locale?: false; has?: unknown[]; missing?: unknown[] }>;
  readonly regions?: string | ReadonlyArray<string>;
  readonly unstable_allowDynamic?: string | ReadonlyArray<string>;
}

/** Alias of `MiddlewareConfig` for `proxy.ts`. */
export type ProxyConfig = MiddlewareConfig;

/** Parses a raw user agent string (`userAgentFromString(request.headers.get("user-agent"))`). */
export function userAgentFromString(input: string | undefined | null): UserAgent {
  return userAgent({ headers: { "user-agent": input ?? "" } });
}

/** Minimal `URLPattern` constructor type (native API passthrough). */
export interface URLPatternConstructor {
  new (
    input?: string | Record<string, string>,
    baseURL?: string,
  ): {
    test(input?: string | Record<string, string>, baseURL?: string): boolean;
    exec(input?: string | Record<string, string>, baseURL?: string): unknown;
  };
}

/** Native `URLPattern` (undefined when the runtime lacks it). */
export const URLPattern = (globalThis as { URLPattern?: URLPatternConstructor }).URLPattern;
