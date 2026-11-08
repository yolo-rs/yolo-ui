// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/actions: Server Actions wire protocol (Next.js 16 compatible): reference ids,
// registry/manifest loading, body decoding (fetch + no-JS form posts), CSRF, redirects and
// revalidation headers, plus the browser `callServer`/`encodeReply` helpers.

import { deserializeCacheValue } from "./cache.ts";
import {
  createRequestStore,
  mergeStoreHeaders,
  runAfterCallbacks,
  runWithRequestStore,
  type RequestStore,
} from "./headers.ts";
import {
  getAccessFallbackStatus,
  getRedirectStatus,
  getRedirectType,
  getRedirectUrl,
  isHttpAccessFallbackError,
  isRedirectError,
} from "./navigation.ts";
import { decodeFlightModel, encodeFlightModel, loadReactServerDom, resolveFlightModel } from "./rsc.ts";
import { ACTION_FIELD, RSC_HEADERS } from "./types.ts";

const REACT_SERVER_REFERENCE = Symbol.for("react.server.reference");

/** Default Server Actions body limit (1 MiB, Next.js default). */
export const DEFAULT_ACTION_BODY_LIMIT = 1_048_576;

/** `Cache-Control` of every Server Action response. */
export const ACTION_CACHE_CONTROL = "no-cache, no-store, max-age=0, must-revalidate";

/** 40-hex server reference id: `"00" + sha256(salt + module + ":" + export).slice(0, 38)`. */
export function serverReferenceId(moduleId: string, exportName: string, salt = ""): string {
  const digest = new Bun.CryptoHasher("sha256").update(`${salt}${moduleId}:${exportName}`).digest("hex");
  return `00${digest.slice(0, 38)}`;
}

/** Registry entry of one Server Action. */
export interface ActionEntry {
  readonly module: string;
  readonly exportName: string;
  readonly load: () => Promise<Record<string, unknown>>;
  readonly inline: boolean;
  readonly cache?: boolean;
}

/** Server Action registry keyed by reference id. */
export type ActionRegistry = Map<string, ActionEntry>;

/** Input of `createActionRegistry`. */
export interface ActionRegistryInput {
  readonly id?: string;
  readonly module: string;
  readonly exportName: string;
  readonly load: () => Promise<Record<string, unknown>>;
  readonly inline?: boolean;
  readonly cache?: boolean;
}

/** Builds a registry from entries (ids default to `serverReferenceId(module, exportName)`). */
export function createActionRegistry(entries: ReadonlyArray<ActionRegistryInput>): ActionRegistry {
  const registry: ActionRegistry = new Map();
  for (const entry of entries) {
    const id = entry.id ?? serverReferenceId(entry.module, entry.exportName);
    registry.set(id, {
      module: entry.module,
      exportName: entry.exportName,
      load: entry.load,
      inline: entry.inline ?? false,
      ...(entry.cache !== undefined ? { cache: entry.cache } : {}),
    });
  }
  return registry;
}

/** Tags `fn` as a server reference (`$$typeof`, `$$id`, `$$bound`) when not already tagged. */
export function tagServerReference<T extends (...args: never[]) => unknown>(fn: T, id: string): T {
  const tagged = fn as T & { $$typeof?: symbol; $$id?: string };
  if (tagged.$$typeof !== REACT_SERVER_REFERENCE) {
    Object.defineProperties(fn, {
      $$typeof: { value: REACT_SERVER_REFERENCE, configurable: true },
      $$id: { value: id, configurable: true },
      $$bound: { value: null, configurable: true, writable: true },
    });
  }
  return fn;
}

/** Registers an in-process action function and returns its id. */
export function registerServerAction(
  registry: ActionRegistry,
  module: string,
  exportName: string,
  fn: (...args: never[]) => unknown,
): string {
  const id = serverReferenceId(module, exportName);
  tagServerReference(fn, id);
  registry.set(id, {
    module,
    exportName,
    load: async () => ({ [exportName]: fn }),
    inline: false,
  });
  return id;
}

interface ManifestEntry {
  readonly module?: string;
  readonly exportName?: string;
  readonly inline?: boolean;
  readonly cache?: boolean;
}

function toFileUrlOrPath(root: string, module: string): string {
  if (/^[A-Za-z]:[\\/]/.test(module) || module.startsWith("/") || module.startsWith("file:")) {
    return module;
  }
  const normalizedRoot = root.replace(/[\\/]+$/, "");
  return `${normalizedRoot}/${module.replace(/^\.\//, "")}`;
}

/** Loads `.yolo/server-reference-manifest.json` (`{ node: { [id]: { module, exportName, inline, cache } } }`). */
export async function loadActionManifest(root: string): Promise<ActionRegistry> {
  const registry: ActionRegistry = new Map();
  const file = Bun.file(`${root.replace(/[\\/]+$/, "")}/.yolo/server-reference-manifest.json`);
  if (!(await file.exists())) {
    return registry;
  }
  let json: unknown;
  try {
    json = await file.json();
  } catch (error) {
    console.error("[yolo-next] invalid server-reference-manifest.json:", error);
    return registry;
  }
  if (!json || typeof json !== "object") {
    return registry;
  }
  const record = json as Record<string, unknown>;
  const groups: Array<Record<string, ManifestEntry>> = [];
  for (const key of ["node", "edge"]) {
    const group = record[key];
    if (group && typeof group === "object") {
      groups.push(group as Record<string, ManifestEntry>);
    }
  }
  if (groups.length === 0) {
    groups.push(record as Record<string, ManifestEntry>);
  }
  for (const group of groups) {
    for (const [id, entry] of Object.entries(group)) {
      if (!entry || typeof entry !== "object" || typeof entry.module !== "string") {
        continue;
      }
      const module = entry.module;
      const exportName = entry.exportName ?? "default";
      const specifier = toFileUrlOrPath(root, module);
      registry.set(id, {
        module,
        exportName,
        inline: entry.inline ?? false,
        ...(entry.cache !== undefined ? { cache: entry.cache } : {}),
        load: () => import(specifier) as Promise<Record<string, unknown>>,
      });
    }
  }
  return registry;
}

/** Resolves the action function for `id` (`undefined` when unknown or not a function). */
export async function resolveServerAction(
  registry: ActionRegistry,
  id: string,
): Promise<((...args: unknown[]) => unknown) | undefined> {
  const entry = registry.get(id);
  if (!entry) {
    return undefined;
  }
  const module = await entry.load();
  const candidate = module[entry.exportName] ?? (entry.exportName === "default" ? module.default : undefined);
  return typeof candidate === "function" ? (candidate as (...args: unknown[]) => unknown) : undefined;
}

function contentTypeOf(request: Request): string {
  return (request.headers.get("content-type") ?? "").toLowerCase();
}

/** Whether `request` is a Server Action (`next-action` header or a `$ACTION_ID_` form field). */
export async function isActionRequest(request: Request): Promise<boolean> {
  if (request.method !== "POST") {
    return false;
  }
  if (request.headers.has(RSC_HEADERS.action)) {
    return true;
  }
  const contentType = contentTypeOf(request);
  if (!contentType.startsWith("multipart/form-data") && !contentType.startsWith("application/x-www-form-urlencoded")) {
    return false;
  }
  const length = Number(request.headers.get("content-length") ?? "0");
  if (length > 16 * 1024 * 1024) {
    return false;
  }
  try {
    if (contentType.startsWith("application/x-www-form-urlencoded")) {
      const text = await request.clone().text();
      return text.includes(encodeURIComponent(ACTION_FIELD.id)) || text.includes(ACTION_FIELD.id);
    }
    const form = await request.clone().formData();
    for (const key of form.keys()) {
      if (key.startsWith(ACTION_FIELD.id)) {
        return true;
      }
    }
  } catch {
    return false;
  }
  return false;
}

/** Decoded Server Action invocation. */
export interface DecodedActionBody {
  readonly ok: true;
  readonly id?: string;
  readonly args: unknown[];
  readonly isForm: boolean;
  readonly formData?: FormData;
  readonly bound?: unknown[];
}

/** Failure of `decodeActionBody` carrying the HTTP response to send. */
export interface DecodedActionFailure {
  readonly ok: false;
  readonly response: Response;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_048_576) {
    const mb = bytes / 1_048_576;
    return `${Number.isInteger(mb) ? mb : mb.toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    const kb = bytes / 1024;
    return `${Number.isInteger(kb) ? kb : kb.toFixed(1)} kB`;
  }
  return `${bytes} B`;
}

function bodyTooLarge(limit: number): DecodedActionFailure {
  return {
    ok: false,
    response: new Response(
      `Body exceeded ${formatBytes(limit)} limit. To configure the body size limit for Server Actions, see: https://nextjs.org/docs/app/api-reference/next-config-js/serverActions#bodysizelimit`,
      { status: 413, headers: { "content-type": "text/plain; charset=utf-8" } },
    ),
  };
}

async function readBodyWithin(request: Request, limit: number): Promise<Uint8Array<ArrayBuffer> | DecodedActionFailure> {
  const declared = request.headers.get("content-length");
  if (declared !== null && Number(declared) > limit) {
    return bodyTooLarge(limit);
  }
  if (!request.body) {
    return new Uint8Array(0);
  }
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    total += value.byteLength;
    if (total > limit) {
      await reader.cancel();
      return bodyTooLarge(limit);
    }
    chunks.push(value);
  }
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

function reviveReply(value: unknown, form?: FormData): unknown {
  if (typeof value === "string") {
    if (!value.startsWith("$")) {
      return value;
    }
    if (value.startsWith("$$")) {
      return value.slice(1);
    }
    if (value === "$undefined") {
      return undefined;
    }
    const kind = value.charAt(1);
    const rest = value.slice(2);
    switch (kind) {
      case "D":
        return new Date(rest);
      case "n":
        return BigInt(rest);
      case "K": {
        const prefix = `${rest}_`;
        const data = new FormData();
        if (form) {
          for (const [key, item] of form.entries()) {
            if (key.startsWith(prefix)) {
              data.append(key.slice(prefix.length), item);
            }
          }
        }
        return data;
      }
      case "@":
      case "Q":
      case "W": {
        const raw = form?.get(rest);
        if (typeof raw !== "string") {
          return undefined;
        }
        const parsed = reviveReply(JSON.parse(raw), form);
        if (kind === "Q") {
          return new Map(parsed as Array<[unknown, unknown]>);
        }
        if (kind === "W") {
          return new Set(parsed as unknown[]);
        }
        return parsed;
      }
      default:
        if (/^[0-9a-f]+$/i.test(value.slice(1))) {
          const raw = form?.get(value.slice(1));
          return typeof raw === "string" ? reviveReply(JSON.parse(raw), form) : raw ?? undefined;
        }
        return value;
    }
  }
  if (Array.isArray(value)) {
    return value.map((item) => reviveReply(item, form));
  }
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (typeof record.__yolo_type === "string") {
      return deserializeCacheValue(JSON.stringify(record));
    }
    const out: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(record)) {
      out[key] = reviveReply(item, form);
    }
    return out;
  }
  return value;
}

function parseJsonArgs(text: string): unknown[] {
  if (text.trim().length === 0) {
    return [];
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return [text];
  }
  const revived = reviveReply(parsed);
  return Array.isArray(revived) ? revived : [revived];
}

function stripActionFields(form: FormData): { id?: string; bound?: unknown[]; data: FormData } {
  const data = new FormData();
  let id: string | undefined;
  let bound: unknown[] | undefined;
  const payloads = new Map<string, string>();
  for (const [key, value] of form.entries()) {
    if (key.startsWith(ACTION_FIELD.id)) {
      id = key.slice(ACTION_FIELD.id.length);
      continue;
    }
    if (key.startsWith(ACTION_FIELD.ref)) {
      continue;
    }
    if (key.startsWith(ACTION_FIELD.payload)) {
      if (typeof value === "string") {
        payloads.set(key, value);
      }
      continue;
    }
    data.append(key, value);
  }
  const refField = [...form.keys()].find((key) => key.startsWith(ACTION_FIELD.ref));
  const refIndex = refField ? refField.slice(ACTION_FIELD.ref.length) : undefined;
  const payloadKey = refIndex !== undefined ? `${ACTION_FIELD.payload}${refIndex}:0` : undefined;
  const payload = payloadKey ? payloads.get(payloadKey) : [...payloads.values()][0];
  if (payload) {
    try {
      const parsed = JSON.parse(payload) as { id?: string; bound?: unknown };
      if (typeof parsed.id === "string" && id === undefined) {
        id = parsed.id;
      }
      if (Array.isArray(parsed.bound)) {
        bound = parsed.bound.map((item) => reviveReply(item, form));
      }
    } catch {
      // Ignore malformed payloads: the action id from the field name still applies.
    }
  }
  const result: { id?: string; bound?: unknown[]; data: FormData } = { data };
  if (id !== undefined) {
    result.id = id;
  }
  if (bound !== undefined) {
    result.bound = bound;
  }
  return result;
}

function urlEncodedToFormData(text: string): FormData {
  const data = new FormData();
  for (const [key, value] of new URLSearchParams(text)) {
    data.append(key, value);
  }
  return data;
}

/** Options of `decodeActionBody`. */
export interface DecodeActionBodyOptions {
  readonly serverManifest?: Record<string, unknown>;
}

/** Decodes a Server Action request body (text/plain JSON, multipart, urlencoded or JSON) within `limitBytes`. */
export async function decodeActionBody(
  request: Request,
  limitBytes = DEFAULT_ACTION_BODY_LIMIT,
  options: DecodeActionBodyOptions = {},
): Promise<DecodedActionBody | DecodedActionFailure> {
  const body = await readBodyWithin(request, limitBytes);
  if (!(body instanceof Uint8Array)) {
    return body;
  }
  const contentType = contentTypeOf(request);
  const headerId = request.headers.get(RSC_HEADERS.action) ?? undefined;
  const server = await loadReactServerDom();
  if (contentType.startsWith("multipart/form-data") || contentType.startsWith("application/x-www-form-urlencoded")) {
    const form = contentType.startsWith("multipart/form-data")
      ? await new Response(body, { headers: { "content-type": request.headers.get("content-type") ?? contentType } }).formData()
      : urlEncodedToFormData(new TextDecoder().decode(body));
    const hasActionId = [...form.keys()].some((key) => key.startsWith(ACTION_FIELD.id));
    if (hasActionId || headerId === undefined) {
      const { id, bound, data } = stripActionFields(form);
      const args = [...(bound ?? []), data];
      const result: DecodedActionBody = { ok: true, args, isForm: true, formData: data, ...(bound ? { bound } : {}) };
      const finalId = id ?? headerId;
      return finalId === undefined ? result : { ...result, id: finalId };
    }
    // fetch() action with a FormData reply (react-server-dom-webpack encodeReply).
    if (server) {
      const args = await server.decodeReply(form, options.serverManifest ?? {});
      return { ok: true, id: headerId, args, isForm: false, formData: form };
    }
    const root = form.get("0");
    const args = typeof root === "string" ? (reviveReply(JSON.parse(root), form) as unknown[]) : [form];
    return { ok: true, id: headerId, args: Array.isArray(args) ? args : [args], isForm: false, formData: form };
  }
  const text = new TextDecoder().decode(body);
  if (contentType.startsWith("application/json")) {
    const args = parseJsonArgs(text);
    return headerId === undefined ? { ok: true, args, isForm: false } : { ok: true, id: headerId, args, isForm: false };
  }
  if (server) {
    try {
      const args = await server.decodeReply(text, options.serverManifest ?? {});
      return headerId === undefined ? { ok: true, args, isForm: false } : { ok: true, id: headerId, args, isForm: false };
    } catch {
      // Fall through to the JSON decoder.
    }
  }
  const args = parseJsonArgs(text);
  return headerId === undefined ? { ok: true, args, isForm: false } : { ok: true, id: headerId, args, isForm: false };
}

/** Encodes an action result model as Flight text (real renderer when available). */
export async function encodeActionResult(model: unknown): Promise<string> {
  const server = await loadReactServerDom();
  if (server) {
    const stream = server.renderToReadableStream(model, {}, {});
    return new Response(stream).text();
  }
  return encodeFlightModel(await resolveFlightModel(model));
}

function originAllowed(origin: string, host: string, allowed: ReadonlyArray<string>): boolean {
  let originHost: string;
  try {
    originHost = new URL(origin).host;
  } catch {
    return false;
  }
  if (originHost === host) {
    return true;
  }
  return allowed.some((pattern) => {
    if (pattern === originHost) {
      return true;
    }
    if (pattern.startsWith("*.")) {
      return originHost.endsWith(pattern.slice(1));
    }
    return false;
  });
}

function revalidatedHeader(store: RequestStore): string {
  return JSON.stringify([store.revalidated.paths, store.revalidated.tags, store.revalidated.cookiesChanged ? 1 : 0]);
}

/** Options of `handleServerAction`. */
export interface HandleServerActionOptions {
  readonly registry: ActionRegistry;
  readonly store?: RequestStore;
  readonly allowedOrigins?: ReadonlyArray<string>;
  readonly bodySizeLimit?: number;
  readonly onRevalidate?: (revalidated: RequestStore["revalidated"], store: RequestStore) => void | Promise<void>;
  readonly serverManifest?: Record<string, unknown>;
}

/** Name of the cookie carrying the `useActionState` result across a no-JS form post redirect. */
export const FORM_STATE_COOKIE = "__yolo_form_state";

function finalize(response: Response, store: RequestStore): Response {
  const headers = new Headers(response.headers);
  headers.set("cache-control", ACTION_CACHE_CONTROL);
  headers.set(RSC_HEADERS.actionRevalidated, revalidatedHeader(store));
  if (store.revalidated.refresh) {
    headers.set("x-action-refresh", "1");
  }
  if (store.deploymentId) {
    headers.set("x-nextjs-deployment-id", store.deploymentId);
  }
  mergeStoreHeaders(store, headers);
  return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
}

function resolveLocation(url: string, base: string): string {
  try {
    const target = new URL(url, base);
    const origin = new URL(base).origin;
    return target.origin === origin ? `${target.pathname}${target.search}${target.hash}` : target.href;
  } catch {
    return url;
  }
}

/** Executes a Server Action request end to end and produces the Next.js compatible response. */
export async function handleServerAction(request: Request, options: HandleServerActionOptions): Promise<Response> {
  const store = options.store ?? createRequestStore(request, { phase: "action" });
  store.phase = "action";
  const isFetch = request.headers.has(RSC_HEADERS.action);
  const origin = request.headers.get("origin");
  const host = request.headers.get("x-forwarded-host") ?? request.headers.get("host") ?? new URL(request.url).host;
  if (origin !== null && !originAllowed(origin, host, options.allowedOrigins ?? [])) {
    return finalize(new Response("Invalid Server Actions request.", { status: 500, headers: { "content-type": "text/plain; charset=utf-8" } }), store);
  }
  if (origin === null && request.headers.has("x-forwarded-host")) {
    console.warn("[yolo-next] Missing `origin` header from a forwarded Server Actions request.");
  }
  const decoded = await decodeActionBody(request, options.bodySizeLimit ?? DEFAULT_ACTION_BODY_LIMIT, options.serverManifest ? { serverManifest: options.serverManifest } : {});
  if (!decoded.ok) {
    return finalize(decoded.response, store);
  }
  const id = request.headers.get(RSC_HEADERS.action) ?? decoded.id;
  const action = id ? await resolveServerAction(options.registry, id) : undefined;
  if (!action) {
    return finalize(
      new Response("Server Action unavailable.", {
        status: 404,
        headers: { "content-type": "text/plain; charset=utf-8", "x-nextjs-action-not-found": "1" },
      }),
      store,
    );
  }
  const currentUrl = request.headers.get("x-yolo-original-url") ?? request.url;
  let response: Response;
  try {
    const result = await runWithRequestStore(store, () => action(...decoded.args));
    if (isFetch) {
      response = new Response(await encodeActionResult({ a: result }), {
        status: 200,
        headers: { "content-type": RSC_HEADERS.contentType },
      });
    } else {
      const headers = new Headers({ location: resolveLocation(currentUrl, currentUrl) });
      if (result !== undefined) {
        const state = encodeURIComponent(JSON.stringify(result));
        headers.append("set-cookie", `${FORM_STATE_COOKIE}=${state}; Path=/; Max-Age=60; SameSite=Lax`);
      }
      response = new Response(null, { status: 303, headers });
    }
  } catch (error) {
    if (isRedirectError(error)) {
      const url = getRedirectUrl(error);
      const status = getRedirectStatus(error);
      const type = getRedirectType(error);
      if (isFetch) {
        response = new Response(await encodeActionResult({ a: null }), {
          status: 200,
          headers: {
            "content-type": RSC_HEADERS.contentType,
            [RSC_HEADERS.actionRedirect]: `${url};${type}`,
            "x-action-redirect-status": String(status),
          },
        });
      } else {
        response = new Response(null, { status: 303, headers: { location: resolveLocation(url, currentUrl) } });
      }
    } else if (isHttpAccessFallbackError(error)) {
      const status = getAccessFallbackStatus(error);
      response = isFetch
        ? new Response(await encodeActionResult({ a: null }), { status, headers: { "content-type": RSC_HEADERS.contentType } })
        : new Response(status === 404 ? "Not Found" : status === 403 ? "Forbidden" : "Unauthorized", { status, headers: { "content-type": "text/plain; charset=utf-8" } });
    } else {
      const message = error instanceof Error ? error.message : String(error);
      const digest = error instanceof Error ? (error as { digest?: string }).digest : undefined;
      console.error("[yolo-next] Server Action failed:", error);
      const payload = { error: { message, ...(digest !== undefined ? { digest } : {}) } };
      response = isFetch
        ? new Response(await encodeActionResult(payload), { status: 500, headers: { "content-type": RSC_HEADERS.contentType } })
        : new Response(JSON.stringify(payload), { status: 500, headers: { "content-type": "application/json" } });
    }
  }
  await runAfterCallbacks(store);
  if (options.onRevalidate) {
    await options.onRevalidate(store.revalidated, store);
  }
  return finalize(response, store);
}

// ---------------------------------------------------------------------------
// Browser helpers (bundled into the client runtime)
// ---------------------------------------------------------------------------

/** Encodes action arguments for `callServer` (JSON string, or FormData passthrough). */
export function encodeReply(args: unknown): string | FormData {
  if (args instanceof FormData) {
    return args;
  }
  const list = Array.isArray(args) ? args : [args];
  if (list.length === 1 && list[0] instanceof FormData) {
    return list[0];
  }
  return JSON.stringify(list, (_key, value: unknown) => {
    if (value === undefined) {
      return "$undefined";
    }
    if (typeof value === "bigint") {
      return `$n${value.toString()}`;
    }
    if (typeof value === "string" && value.startsWith("$")) {
      return `$${value}`;
    }
    return value;
  });
}

interface BrowserLike {
  location?: { href: string };
  history?: { pushState(data: unknown, unused: string, url?: string): void; replaceState(data: unknown, unused: string, url?: string): void };
  dispatchEvent?: (event: Event) => boolean;
  __YOLO_DEPLOYMENT_ID?: string;
}

/** Reads the `a` field of an action Flight payload (JSON line protocol). */
export function decodeActionResult(text: string): unknown {
  const decoded = decodeFlightModel(text);
  const root = decoded.root as { a?: unknown; error?: { message?: string; digest?: string } } | undefined;
  if (root && typeof root === "object" && root.error) {
    const error = new Error(root.error.message ?? "Server Action failed") as Error & { digest?: string };
    if (root.error.digest !== undefined) {
      error.digest = root.error.digest;
    }
    throw error;
  }
  return root?.a;
}

/** Browser `callServer(id, args)`: POSTs to the current URL with the Next.js action headers. */
export async function callServer(id: string, args: unknown[], fetchImpl: typeof fetch = fetch): Promise<unknown> {
  const browser = globalThis as unknown as BrowserLike;
  const href = browser.location?.href ?? "/";
  const body = encodeReply(args);
  const headers: Record<string, string> = {
    [RSC_HEADERS.action]: id,
    accept: RSC_HEADERS.contentType,
    [RSC_HEADERS.routerStateTree]: "%5B%5D",
    [RSC_HEADERS.deploymentId]: browser.__YOLO_DEPLOYMENT_ID ?? "",
  };
  if (typeof body === "string") {
    headers["content-type"] = "text/plain;charset=UTF-8";
  }
  const response = await fetchImpl(href, { method: "POST", headers, body });
  const redirect = response.headers.get(RSC_HEADERS.actionRedirect);
  if (redirect) {
    const [url, type] = redirect.split(";");
    if (url && browser.history) {
      if (type === "replace") {
        browser.history.replaceState({}, "", url);
      } else {
        browser.history.pushState({}, "", url);
      }
      browser.dispatchEvent?.(new CustomEvent("yolo:navigate", { detail: { url, type: type ?? "push" } }));
    }
  }
  const text = await response.text();
  if (!response.ok && !text) {
    throw new Error(`Server Action failed with status ${response.status}`);
  }
  return decodeActionResult(text);
}
