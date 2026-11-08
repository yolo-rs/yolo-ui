// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/cache: "use cache" runtime (cacheLife/cacheTag/cached), the Next.js 16
// `cacheHandlers` API (memory + bun:sqlite) and the `next/cache` revalidation surface.

import { AsyncLocalStorage } from "node:async_hooks";
import { Database, type Statement } from "bun:sqlite";
import { peekRequestStore, type RequestStore } from "./headers.ts";
import { BUILTIN_CACHE_LIFE, type CacheLifeProfile } from "./types.ts";

/** Stored cache record (Next.js 16 `CacheEntry` shape, durations in seconds, timestamp in ms). */
export interface CacheEntry {
  value: Uint8Array | string;
  tags: string[];
  stale: number;
  revalidate: number;
  expire: number;
  timestamp: number;
}

/** Next.js 16 `cacheHandlers` contract. */
export interface CacheHandler {
  get(key: string, softTags?: readonly string[]): Promise<CacheEntry | undefined>;
  set(key: string, entry: Promise<CacheEntry> | CacheEntry): Promise<void>;
  refreshTags(): Promise<void>;
  getExpiration(...tags: string[]): Promise<number>;
  updateTags(tags: readonly string[], durations?: { expire?: number }): Promise<void>;
}

/** Clock injection shared by handlers and `cached` (tests use fake clocks). */
export type Clock = () => number;

/** Options of `MemoryCacheHandler`. */
export interface MemoryCacheHandlerOptions {
  readonly maxBytes?: number;
  readonly now?: Clock;
}

/** Tag expiration record: hard expiry instant plus an optional stale-while-revalidate window. */
interface TagExpiration {
  expiredAt: number;
  staleUntil: number;
}

const DEFAULT_MAX_BYTES = 50 * 1024 * 1024;

function byteSize(value: Uint8Array | string): number {
  return typeof value === "string" ? Buffer.byteLength(value) : value.byteLength;
}

function applyTagExpiration(
  entry: CacheEntry,
  tags: Iterable<string>,
  lookup: (tag: string) => TagExpiration | undefined,
  now: number,
): CacheEntry | undefined {
  let staleUntil = Number.POSITIVE_INFINITY;
  let expired = false;
  for (const tag of tags) {
    const expiration = lookup(tag);
    if (!expiration || expiration.expiredAt < entry.timestamp) {
      continue;
    }
    expired = true;
    staleUntil = Math.min(staleUntil, expiration.staleUntil);
  }
  if (!expired) {
    return entry;
  }
  if (staleUntil <= now) {
    return undefined;
  }
  // Soft expiration: serve stale (revalidate immediately) until the SWR window closes.
  return {
    ...entry,
    stale: 0,
    revalidate: 0,
    expire: Math.max(0, Math.ceil((staleUntil - entry.timestamp) / 1000)),
  };
}

/** In-memory LRU cache handler (byte-bounded, default 50 MiB). */
export class MemoryCacheHandler implements CacheHandler {
  readonly #entries = new Map<string, CacheEntry>();
  readonly #tags = new Map<string, TagExpiration>();
  readonly #maxBytes: number;
  readonly #now: Clock;
  #bytes = 0;

  constructor(options: MemoryCacheHandlerOptions = {}) {
    this.#maxBytes = options.maxBytes ?? DEFAULT_MAX_BYTES;
    this.#now = options.now ?? Date.now;
  }

  /** Number of stored entries. */
  get size(): number {
    return this.#entries.size;
  }

  /** Bytes currently held by values. */
  get bytes(): number {
    return this.#bytes;
  }

  async get(key: string, softTags: readonly string[] = []): Promise<CacheEntry | undefined> {
    const entry = this.#entries.get(key);
    if (!entry) {
      return undefined;
    }
    // LRU touch.
    this.#entries.delete(key);
    this.#entries.set(key, entry);
    const result = applyTagExpiration(entry, [...entry.tags, ...softTags], (tag) => this.#tags.get(tag), this.#now());
    if (!result) {
      this.#remove(key);
    }
    return result;
  }

  async set(key: string, pending: Promise<CacheEntry> | CacheEntry): Promise<void> {
    const entry = await pending;
    this.#remove(key);
    const size = byteSize(entry.value);
    if (size > this.#maxBytes) {
      return;
    }
    this.#entries.set(key, entry);
    this.#bytes += size;
    while (this.#bytes > this.#maxBytes) {
      const oldest = this.#entries.keys().next();
      if (oldest.done) {
        break;
      }
      this.#remove(oldest.value);
    }
  }

  async refreshTags(): Promise<void> {
    // Local handler: nothing to refresh.
  }

  async getExpiration(...tags: string[]): Promise<number> {
    let latest = 0;
    for (const tag of tags) {
      latest = Math.max(latest, this.#tags.get(tag)?.expiredAt ?? 0);
    }
    return latest;
  }

  async updateTags(tags: readonly string[], durations: { expire?: number } = {}): Promise<void> {
    const now = this.#now();
    const staleUntil = now + Math.max(0, durations.expire ?? 0) * 1000;
    for (const tag of tags) {
      this.#tags.set(tag, { expiredAt: now, staleUntil });
    }
  }

  /** Removes every entry and tag expiration. */
  clear(): void {
    this.#entries.clear();
    this.#tags.clear();
    this.#bytes = 0;
  }

  #remove(key: string): void {
    const existing = this.#entries.get(key);
    if (existing) {
      this.#bytes -= byteSize(existing.value);
      this.#entries.delete(key);
    }
  }
}

interface EntryRow {
  key: string;
  value: Uint8Array | string;
  tags: string;
  stale: number;
  revalidate: number;
  expire: number;
  timestamp: number;
}

interface TagRow {
  tag: string;
  expired_at: number;
  stale_until: number;
}

/** Options of `SqliteCacheHandler`. */
export interface SqliteCacheHandlerOptions {
  readonly now?: Clock;
}

/** Persistent cache handler on `bun:sqlite` (WAL, prepared statements). */
export class SqliteCacheHandler implements CacheHandler {
  readonly #db: Database;
  readonly #now: Clock;
  readonly #selectEntry: Statement<EntryRow, [string]>;
  readonly #upsertEntry: Statement<void, [string, Uint8Array | string, string, number, number, number, number]>;
  readonly #deleteEntry: Statement<void, [string]>;
  readonly #selectTag: Statement<TagRow, [string]>;
  readonly #upsertTag: Statement<void, [string, number, number]>;

  constructor(path = ":memory:", options: SqliteCacheHandlerOptions = {}) {
    this.#db = new Database(path, { create: true });
    this.#now = options.now ?? Date.now;
    this.#db.exec("PRAGMA journal_mode = WAL;");
    this.#db.exec(
      "CREATE TABLE IF NOT EXISTS entries (key TEXT PRIMARY KEY, value BLOB, tags TEXT, stale INTEGER, revalidate INTEGER, expire INTEGER, timestamp INTEGER);",
    );
    this.#db.exec(
      "CREATE TABLE IF NOT EXISTS tag_expirations (tag TEXT PRIMARY KEY, expired_at INTEGER, stale_until INTEGER DEFAULT 0);",
    );
    this.#selectEntry = this.#db.query<EntryRow, [string]>(
      "SELECT key, value, tags, stale, revalidate, expire, timestamp FROM entries WHERE key = ?1",
    );
    this.#upsertEntry = this.#db.query(
      "INSERT INTO entries (key, value, tags, stale, revalidate, expire, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(key) DO UPDATE SET value = excluded.value, tags = excluded.tags, stale = excluded.stale, revalidate = excluded.revalidate, expire = excluded.expire, timestamp = excluded.timestamp",
    );
    this.#deleteEntry = this.#db.query("DELETE FROM entries WHERE key = ?1");
    this.#selectTag = this.#db.query<TagRow, [string]>(
      "SELECT tag, expired_at, stale_until FROM tag_expirations WHERE tag = ?1",
    );
    this.#upsertTag = this.#db.query(
      "INSERT INTO tag_expirations (tag, expired_at, stale_until) VALUES (?1, ?2, ?3) ON CONFLICT(tag) DO UPDATE SET expired_at = excluded.expired_at, stale_until = excluded.stale_until",
    );
  }

  async get(key: string, softTags: readonly string[] = []): Promise<CacheEntry | undefined> {
    const row = this.#selectEntry.get(key);
    if (!row) {
      return undefined;
    }
    const entry: CacheEntry = {
      value: row.value,
      tags: JSON.parse(row.tags) as string[],
      stale: row.stale,
      revalidate: row.revalidate,
      expire: row.expire,
      timestamp: row.timestamp,
    };
    const result = applyTagExpiration(
      entry,
      [...entry.tags, ...softTags],
      (tag) => {
        const tagRow = this.#selectTag.get(tag);
        return tagRow ? { expiredAt: tagRow.expired_at, staleUntil: tagRow.stale_until } : undefined;
      },
      this.#now(),
    );
    if (!result) {
      this.#deleteEntry.run(key);
    }
    return result;
  }

  async set(key: string, pending: Promise<CacheEntry> | CacheEntry): Promise<void> {
    const entry = await pending;
    this.#upsertEntry.run(
      key,
      entry.value,
      JSON.stringify(entry.tags),
      entry.stale,
      entry.revalidate,
      entry.expire,
      entry.timestamp,
    );
  }

  async refreshTags(): Promise<void> {
    // Local handler: nothing to refresh.
  }

  async getExpiration(...tags: string[]): Promise<number> {
    let latest = 0;
    for (const tag of tags) {
      latest = Math.max(latest, this.#selectTag.get(tag)?.expired_at ?? 0);
    }
    return latest;
  }

  async updateTags(tags: readonly string[], durations: { expire?: number } = {}): Promise<void> {
    const now = this.#now();
    const staleUntil = now + Math.max(0, durations.expire ?? 0) * 1000;
    const write = this.#db.transaction((list: readonly string[]) => {
      for (const tag of list) {
        this.#upsertTag.run(tag, now, staleUntil);
      }
    });
    write(tags);
  }

  /** Closes the database. */
  close(): void {
    this.#db.close();
  }
}

/** Factory options of `createCacheHandler`. */
export interface CreateCacheHandlerOptions {
  readonly kind: "memory" | "sqlite";
  readonly path?: string;
  readonly maxBytes?: number;
  readonly now?: Clock;
}

/** Creates a memory or sqlite cache handler. */
export function createCacheHandler(options: CreateCacheHandlerOptions): CacheHandler {
  if (options.kind === "sqlite") {
    return new SqliteCacheHandler(options.path ?? ":memory:", options.now ? { now: options.now } : {});
  }
  const memoryOptions: MemoryCacheHandlerOptions = {};
  if (options.maxBytes !== undefined) {
    Object.assign(memoryOptions, { maxBytes: options.maxBytes });
  }
  if (options.now) {
    Object.assign(memoryOptions, { now: options.now });
  }
  return new MemoryCacheHandler(memoryOptions);
}

const handlers = new Map<string, CacheHandler>();

/** Registers a cache handler under `name` (`default`, `remote`, `static` or custom). */
export function setCacheHandler(name: string, handler: CacheHandler): void {
  handlers.set(name, handler);
}

/** Returns the handler for `name`, falling back to `default` (created lazily in memory). */
export function getCacheHandler(name = "default"): CacheHandler {
  const named = handlers.get(name);
  if (named) {
    return named;
  }
  let fallback = handlers.get("default");
  if (!fallback) {
    fallback = new MemoryCacheHandler();
    handlers.set("default", fallback);
  }
  return fallback;
}

/** Removes every registered handler (tests). */
export function resetCacheHandlers(): void {
  handlers.clear();
}

// ---------------------------------------------------------------------------
// Profiles and cache scopes
// ---------------------------------------------------------------------------

const profiles = new Map<string, Required<CacheLifeProfile>>(
  Object.entries(BUILTIN_CACHE_LIFE).map(([name, profile]) => [name, { ...profile }]),
);

/** Merges custom `cacheLife` profiles (next.config `cacheLife`) into the registry. */
export function configureCacheLife(custom: Readonly<Record<string, CacheLifeProfile>>): void {
  for (const [name, profile] of Object.entries(custom)) {
    profiles.set(name, resolveCacheLife(profile));
  }
}

/** Resolves a profile name or partial profile into a complete `{ stale, revalidate, expire }`. */
export function resolveCacheLife(profile: string | CacheLifeProfile): Required<CacheLifeProfile> {
  if (typeof profile === "string") {
    const named = profiles.get(profile);
    if (!named) {
      throw new Error(`Unknown cacheLife profile "${profile}" is referenced in cacheLife("${profile}")`);
    }
    return { ...named };
  }
  const base = profiles.get("default") ?? BUILTIN_CACHE_LIFE.default ?? { stale: 300, revalidate: 900, expire: 0xfffffffe };
  const revalidate = profile.revalidate ?? base.revalidate;
  const expire = profile.expire ?? Math.max(base.expire, revalidate);
  return {
    stale: profile.stale ?? base.stale,
    revalidate,
    expire: Math.max(expire, revalidate),
  };
}

/** Active `"use cache"` scope. */
export interface CacheScope {
  readonly id: string;
  readonly variant: string;
  life: Required<CacheLifeProfile>;
  explicitLife: boolean;
  readonly tags: Set<string>;
}

/** AsyncLocalStorage carrying the current cache scope. */
export const cacheScopeStorage = new AsyncLocalStorage<CacheScope>();

/** Current cache scope or `undefined`. */
export function getCacheScope(): CacheScope | undefined {
  return cacheScopeStorage.getStore();
}

/** Throws when called inside a non-private `"use cache"` scope (dynamic data access guard). */
export function assertNotInCacheScope(api = "headers"): void {
  const scope = getCacheScope();
  if (scope && scope.variant !== "private") {
    const error = new Error(
      `Route used "${api}" inside "use cache". Accessing Dynamic data sources inside a cache scope is not supported. Read more: https://nextjs.org/docs/messages/next-request-in-use-cache`,
    ) as Error & { digest: string };
    error.digest = "next-request-in-use-cache";
    throw error;
  }
}

const warned = new Set<string>();

function warnOnce(key: string, message: string): void {
  if (!warned.has(key)) {
    warned.add(key);
    console.warn(message);
  }
}

function minLife(a: Required<CacheLifeProfile>, b: Required<CacheLifeProfile>): Required<CacheLifeProfile> {
  return {
    stale: Math.min(a.stale, b.stale),
    revalidate: Math.min(a.revalidate, b.revalidate),
    expire: Math.min(a.expire, b.expire),
  };
}

/** Sets the lifetime of the current `"use cache"` scope (shortest wins when called repeatedly). */
export function cacheLife(profile: string | CacheLifeProfile): void {
  const scope = getCacheScope();
  const life = resolveCacheLife(profile);
  if (!scope) {
    warnOnce("cacheLife", "[yolo-next] cacheLife() was called outside a \"use cache\" scope; ignored.");
    return;
  }
  scope.life = scope.explicitLife ? minLife(scope.life, life) : life;
  scope.explicitLife = true;
}

const MAX_TAGS = 128;
const MAX_TAG_LENGTH = 256;

function validateTags(tags: readonly unknown[], api: string): string[] {
  if (tags.length > MAX_TAGS) {
    throw new Error(`${api}() received ${tags.length} tags, exceeding the maximum of ${MAX_TAGS} tags per call.`);
  }
  const valid: string[] = [];
  for (const tag of tags) {
    if (typeof tag !== "string") {
      throw new Error(`${api}() expects string tags, received ${typeof tag}.`);
    }
    if (tag.length > MAX_TAG_LENGTH) {
      throw new Error(`${api}() tag "${tag.slice(0, 32)}..." exceeds the maximum length of ${MAX_TAG_LENGTH} characters.`);
    }
    valid.push(tag);
  }
  return valid;
}

/** Adds tags to the current `"use cache"` scope. */
export function cacheTag(...tags: string[]): void {
  const valid = validateTags(tags, "cacheTag");
  const scope = getCacheScope();
  if (!scope) {
    warnOnce("cacheTag", "[yolo-next] cacheTag() was called outside a \"use cache\" scope; ignored.");
    return;
  }
  for (const tag of valid) {
    scope.tags.add(tag);
  }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

const TYPE_KEY = "__yolo_type";

type Tagged =
  | { __yolo_type: "date"; value: string }
  | { __yolo_type: "map"; value: unknown[] }
  | { __yolo_type: "set"; value: unknown[] }
  | { __yolo_type: "bigint"; value: string }
  | { __yolo_type: "bytes"; value: string }
  | { __yolo_type: "undefined" };

function encodeValue(value: unknown, seen: WeakSet<object>): unknown {
  if (value === undefined) {
    return { [TYPE_KEY]: "undefined" } satisfies Tagged;
  }
  if (value === null || typeof value !== "object") {
    if (typeof value === "bigint") {
      return { [TYPE_KEY]: "bigint", value: value.toString() } satisfies Tagged;
    }
    if (typeof value === "function" || typeof value === "symbol") {
      throw new Error(`Cannot cache a ${typeof value} value; "use cache" results must be serializable.`);
    }
    return value;
  }
  if (value instanceof Date) {
    return { [TYPE_KEY]: "date", value: value.toISOString() } satisfies Tagged;
  }
  if (value instanceof Uint8Array) {
    return { [TYPE_KEY]: "bytes", value: Buffer.from(value).toString("base64") } satisfies Tagged;
  }
  if (seen.has(value)) {
    throw new Error("Cannot cache a cyclic value.");
  }
  seen.add(value);
  if (value instanceof Map) {
    return { [TYPE_KEY]: "map", value: [...value.entries()].map(([k, v]) => [encodeValue(k, seen), encodeValue(v, seen)]) } satisfies Tagged;
  }
  if (value instanceof Set) {
    return { [TYPE_KEY]: "set", value: [...value].map((item) => encodeValue(item, seen)) } satisfies Tagged;
  }
  if (Array.isArray(value)) {
    return value.map((item) => encodeValue(item, seen));
  }
  const out: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    if (item !== undefined) {
      out[key] = encodeValue(item, seen);
    }
  }
  return out;
}

function reviveValue(value: unknown): unknown {
  if (value === null || typeof value !== "object") {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map(reviveValue);
  }
  const record = value as Record<string, unknown>;
  const tag = record[TYPE_KEY];
  if (typeof tag === "string") {
    switch (tag) {
      case "date":
        return new Date(String(record.value));
      case "bigint":
        return BigInt(String(record.value));
      case "bytes":
        return new Uint8Array(Buffer.from(String(record.value), "base64"));
      case "undefined":
        return undefined;
      case "map":
        return new Map(
          (record.value as unknown[][]).map((pair) => [reviveValue(pair[0]), reviveValue(pair[1])] as [unknown, unknown]),
        );
      case "set":
        return new Set((record.value as unknown[]).map(reviveValue));
      default:
        break;
    }
  }
  const out: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(record)) {
    out[key] = reviveValue(item);
  }
  return out;
}

/** Serializes a cache result to JSON with `__yolo_type` tags for Date/Map/Set/BigInt/undefined/bytes. */
export function serializeCacheValue(value: unknown): string {
  return JSON.stringify(encodeValue(value, new WeakSet()));
}

/** Inverse of `serializeCacheValue`. */
export function deserializeCacheValue<T = unknown>(text: string | Uint8Array): T {
  const json = typeof text === "string" ? text : new TextDecoder().decode(text);
  return reviveValue(JSON.parse(json)) as T;
}

/** Deterministic JSON (sorted keys) used for cache keys. */
export function stableStringify(value: unknown): string {
  const seen = new WeakSet<object>();
  const walk = (input: unknown): string => {
    if (input === undefined) {
      return "null";
    }
    if (input === null || typeof input === "number" || typeof input === "boolean") {
      return JSON.stringify(input);
    }
    if (typeof input === "string") {
      return JSON.stringify(input);
    }
    if (typeof input === "bigint") {
      return `"${input.toString()}n"`;
    }
    if (typeof input === "function") {
      const id = (input as { $$id?: unknown }).$$id;
      return JSON.stringify(typeof id === "string" ? `$F${id}` : `[Function ${input.name}]`);
    }
    if (typeof input === "symbol") {
      return JSON.stringify(input.toString());
    }
    if (input instanceof Date) {
      return JSON.stringify(`$D${input.toISOString()}`);
    }
    if (seen.has(input)) {
      throw new Error("stableStringify: cyclic value");
    }
    seen.add(input);
    if (Array.isArray(input)) {
      return `[${input.map(walk).join(",")}]`;
    }
    if (input instanceof Map) {
      return `{"$map":[${[...input.entries()].map(([k, v]) => `[${walk(k)},${walk(v)}]`).join(",")}]}`;
    }
    if (input instanceof Set) {
      return `{"$set":[${[...input].map(walk).join(",")}]}`;
    }
    if (input instanceof URLSearchParams) {
      return walk(Object.fromEntries([...input.entries()].sort()));
    }
    const keys = Object.keys(input as Record<string, unknown>).sort();
    const parts: string[] = [];
    for (const key of keys) {
      const item = (input as Record<string, unknown>)[key];
      if (item !== undefined) {
        parts.push(`${JSON.stringify(key)}:${walk(item)}`);
      }
    }
    return `{${parts.join(",")}}`;
  };
  return walk(value);
}

// ---------------------------------------------------------------------------
// cached(): the "use cache" wrapper
// ---------------------------------------------------------------------------

/** Options of `cached`. */
export interface CachedOptions {
  readonly id: string;
  readonly variant?: "" | "private" | "remote" | string;
  readonly life?: string | CacheLifeProfile;
  readonly tags?: readonly string[];
  readonly handler?: string;
  readonly now?: Clock;
}

const inflight = new Map<string, Promise<unknown>>();
const background = new Set<Promise<unknown>>();
const privateMemo = new WeakMap<RequestStore, Map<string, Promise<unknown>>>();
const pendingTagWrites = new Set<Promise<void>>();

/** Build identifier used as cache key prefix (`YOLO_BUILD_ID`, default `dev`). */
export function cacheBuildId(): string {
  const build = Bun.env.YOLO_BUILD_ID ?? "dev";
  const deployment = peekRequestStore()?.deploymentId ?? Bun.env.YOLO_DEPLOYMENT_ID;
  return deployment ? `${build}:${deployment}` : build;
}

/** Computes the cache key of `id` for `args`. */
export function cacheKeyFor(id: string, args: readonly unknown[]): string {
  const serialized = stableStringify(args);
  const suffix = serialized.length > 256 ? Bun.hash(serialized).toString(16) : serialized;
  return `${cacheBuildId()}:${id}:${suffix}`;
}

/** Implicit soft tags of a pathname (`_N_T_/layout`, `_N_T_/a/layout`, `_N_T_/a/b`). */
export function implicitTagsFor(pathname: string): string[] {
  const tags = ["_N_T_/layout"];
  const segments = pathname.split("/").filter(Boolean);
  let prefix = "";
  for (const segment of segments) {
    prefix += `/${segment}`;
    tags.push(`_N_T_${prefix}/layout`);
  }
  tags.push(`_N_T_${pathname === "" ? "/" : pathname}`);
  return tags;
}

/** Awaits pending tag writes and in-flight background revalidations (tests and graceful shutdown). */
export async function waitForRevalidations(): Promise<void> {
  await Promise.allSettled([...pendingTagWrites]);
  await Promise.allSettled([...background]);
}

function trackTagWrite(promise: Promise<void>): Promise<void> {
  pendingTagWrites.add(promise);
  const done = promise.catch((error: unknown) => {
    console.error("[yolo-next] cache tag update failed:", error);
  });
  void done.finally(() => pendingTagWrites.delete(promise));
  return done;
}

async function computeEntry<T>(
  fn: () => Promise<T>,
  scope: CacheScope,
  now: number,
): Promise<{ value: T; entry: CacheEntry }> {
  const value = await cacheScopeStorage.run(scope, fn);
  const tags = [...scope.tags];
  const entry: CacheEntry = {
    value: serializeCacheValue(value),
    tags,
    stale: scope.life.stale,
    revalidate: scope.life.revalidate,
    expire: scope.life.expire,
    timestamp: now,
  };
  return { value, entry };
}

function propagateToParent(scope: CacheScope): void {
  const parent = getCacheScope();
  if (!parent || parent === scope) {
    return;
  }
  parent.life = parent.explicitLife ? minLife(parent.life, scope.life) : scope.life;
  parent.explicitLife = parent.explicitLife || scope.explicitLife;
  for (const tag of scope.tags) {
    parent.tags.add(tag);
  }
}

/** Wraps an async function with `"use cache"` semantics (keyed by `id` + args, SWR, tags). */
export function cached<A extends unknown[], T>(
  fn: (...args: A) => Promise<T>,
  options: CachedOptions,
): (...args: A) => Promise<T> {
  const variant = options.variant ?? "";
  const now = options.now ?? Date.now;
  const handlerName = options.handler ?? (variant === "remote" ? "remote" : "default");
  const baseLife = resolveCacheLife(options.life ?? "default");
  const explicitLife = options.life !== undefined;
  const newScope = (): CacheScope => ({
    id: options.id,
    variant,
    life: { ...baseLife },
    explicitLife,
    tags: new Set(options.tags ?? []),
  });

  const runPrivate = async (args: A): Promise<T> => {
    const store = peekRequestStore();
    const key = cacheKeyFor(options.id, args);
    const scope = newScope();
    const execute = async (): Promise<T> => {
      const value = await cacheScopeStorage.run(scope, () => fn(...args));
      propagateToParent(scope);
      return value;
    };
    if (!store) {
      return execute();
    }
    let memo = privateMemo.get(store);
    if (!memo) {
      memo = new Map();
      privateMemo.set(store, memo);
    }
    const existing = memo.get(key);
    if (existing) {
      return existing as Promise<T>;
    }
    const promise = execute();
    memo.set(key, promise);
    promise.catch(() => memo?.delete(key));
    return promise;
  };

  const runShared = async (args: A): Promise<T> => {
    const handler = getCacheHandler(handlerName);
    const key = cacheKeyFor(options.id, args);
    if (pendingTagWrites.size > 0) {
      await Promise.allSettled([...pendingTagWrites]);
    }
    const store = peekRequestStore();
    const softTags = store ? implicitTagsFor(store.url.pathname) : [];
    const current = now();
    const entry = await handler.get(key, softTags);
    const compute = async (): Promise<T> => {
      const scope = newScope();
      const computed = await computeEntry(() => fn(...args), scope, now());
      await handler.set(key, computed.entry);
      propagateToParent(scope);
      return computed.value;
    };
    if (entry && current - entry.timestamp < entry.expire * 1000) {
      const value = deserializeCacheValue<T>(entry.value);
      if (current - entry.timestamp > entry.revalidate * 1000 && !inflight.has(key)) {
        const refresh = compute()
          .catch((error: unknown) => {
            console.error(`[yolo-next] background revalidation of "${options.id}" failed:`, error);
          })
          .finally(() => {
            inflight.delete(key);
            background.delete(refresh);
          });
        inflight.set(key, refresh);
        background.add(refresh);
      }
      // Re-apply tags/lifetime of the cached entry to the enclosing scope.
      const parent = getCacheScope();
      if (parent) {
        for (const tag of entry.tags) {
          parent.tags.add(tag);
        }
        const life = { stale: entry.stale, revalidate: entry.revalidate, expire: entry.expire };
        parent.life = parent.explicitLife ? minLife(parent.life, life) : life;
        parent.explicitLife = true;
      }
      return value;
    }
    const pending = inflight.get(key);
    if (pending) {
      return pending as Promise<T>;
    }
    const promise = compute().finally(() => {
      if (inflight.get(key) === promise) {
        inflight.delete(key);
      }
    });
    inflight.set(key, promise);
    return promise;
  };

  return (...args: A): Promise<T> => (variant === "private" ? runPrivate(args) : runShared(args));
}

// ---------------------------------------------------------------------------
// next/cache surface
// ---------------------------------------------------------------------------

function allHandlers(): CacheHandler[] {
  const list = [...new Set([getCacheHandler("default"), ...handlers.values()])];
  return list;
}

function expireTags(tags: readonly string[], durations?: { expire?: number }): Promise<void> {
  const writes = allHandlers().map((handler) => handler.updateTags(tags, durations));
  return trackTagWrite(Promise.all(writes).then(() => undefined));
}

/** Soft (stale-while-revalidate) tag revalidation; `profile` bounds how long stale data may be served. */
export function revalidateTag(tag: string, profile?: string | { expire?: number }): void {
  const [valid] = validateTags([tag], "revalidateTag");
  if (valid === undefined) {
    return;
  }
  let expire = 0;
  if (profile === undefined) {
    warnOnce(
      "revalidateTag",
      "[yolo-next] revalidateTag(tag) without a cacheLife profile is deprecated; use revalidateTag(tag, \"max\") or updateTag(tag).",
    );
  } else if (typeof profile === "string") {
    expire = resolveCacheLife(profile).expire;
  } else {
    expire = profile.expire ?? 0;
  }
  const store = peekRequestStore();
  if (store && !store.revalidated.tags.includes(valid)) {
    store.revalidated.tags.push(valid);
  }
  void expireTags([valid], { expire });
}

/** Immediate tag expiration; only allowed inside a Server Action (read-your-own-writes). */
export function updateTag(tag: string): void {
  const store = peekRequestStore();
  if (!store || store.phase !== "action") {
    throw new Error(
      "updateTag() can only be called during a Server Action. Use revalidateTag(tag, profile) elsewhere. Read more: https://nextjs.org/docs/app/api-reference/functions/updateTag",
    );
  }
  const [valid] = validateTags([tag], "updateTag");
  if (valid === undefined) {
    return;
  }
  if (!store.revalidated.tags.includes(valid)) {
    store.revalidated.tags.push(valid);
  }
  void expireTags([valid]);
}

/** Normalizes `revalidatePath` arguments into implicit `_N_T_` tags. */
export function pathTags(path: string, type?: "page" | "layout"): string[] {
  const normalized = path.startsWith("/") ? path : `/${path}`;
  const tags = [`_N_T_${normalized}`];
  if (type) {
    tags.push(`_N_T_${normalized.endsWith("/") ? normalized.slice(0, -1) : normalized}/${type}`);
  }
  return tags;
}

/** Expires every cache entry rendered under `path` (`type: "layout"` also expires nested segments). */
export function revalidatePath(path: string, type?: "page" | "layout"): void {
  if (path.length > 1024) {
    throw new Error(`revalidatePath received "${path.slice(0, 32)}..." which exceeded max length of 1024.`);
  }
  const store = peekRequestStore();
  if (store && !store.revalidated.paths.includes(path)) {
    store.revalidated.paths.push(path);
  }
  void expireTags(pathTags(path, type), { expire: 0 });
}

/** Requests a client router refresh after the current Server Action. */
export function refresh(): void {
  const store = peekRequestStore();
  if (!store || store.phase !== "action") {
    throw new Error("refresh() can only be called during a Server Action. Read more: https://nextjs.org/docs/app/api-reference/functions/refresh");
  }
  store.revalidated.refresh = true;
}

/** Immediate expiration of tags (alias kept for Next 15 compatibility). */
export function unstable_expireTag(...tags: string[]): void {
  const valid = validateTags(tags, "unstable_expireTag");
  const store = peekRequestStore();
  if (store) {
    for (const tag of valid) {
      if (!store.revalidated.tags.includes(tag)) {
        store.revalidated.tags.push(tag);
      }
    }
  }
  void expireTags(valid);
}

/** Immediate expiration of a path (alias of `revalidatePath`). */
export function unstable_expirePath(path: string, type?: "page" | "layout"): void {
  revalidatePath(path, type);
}

/** Options of `unstable_cache`. */
export interface UnstableCacheOptions {
  readonly revalidate?: number | false;
  readonly tags?: readonly string[];
}

/** Legacy `unstable_cache(fn, keyParts, options)` implemented on top of `cached`. */
export function unstable_cache<A extends unknown[], T>(
  fn: (...args: A) => Promise<T>,
  keyParts: readonly string[] = [],
  options: UnstableCacheOptions = {},
): (...args: A) => Promise<T> {
  const source = Bun.hash(fn.toString()).toString(16);
  const id = `unstable_cache:${source}:${keyParts.join(",")}`;
  const revalidate = typeof options.revalidate === "number" ? options.revalidate : 0xfffffffe;
  const life: CacheLifeProfile = { stale: 300, revalidate, expire: 0xfffffffe };
  return cached(fn, { id, life, tags: options.tags ?? [] });
}

/** Opts the current render out of static caching (no-op outside a request). */
export function unstable_noStore(): void {
  const store = peekRequestStore();
  if (!store) {
    return;
  }
  if (store.isPrerender) {
    const error = new Error(
      "Route used unstable_noStore() during prerendering, which opts the route into dynamic rendering. Read more: https://nextjs.org/docs/messages/dynamic-server-error",
    ) as Error & { digest: string };
    error.digest = "DYNAMIC_SERVER_USAGE";
    throw error;
  }
  store.usedDynamic = true;
}

/** Marks an I/O boundary inside cached scopes (Next 16 `io()`); no-op here. */
export async function io(): Promise<void> {
  // Intentionally empty: the Bun runtime has no prerender/io split.
}

/** Clears in-flight state and the warn-once registry (tests). */
export function resetCacheRuntime(): void {
  inflight.clear();
  background.clear();
  pendingTagWrites.clear();
  warned.clear();
  profiles.clear();
  for (const [name, profile] of Object.entries(BUILTIN_CACHE_LIFE)) {
    profiles.set(name, { ...profile });
  }
}
