// SPDX-License-Identifier: MIT OR Apache-2.0
// Dioxus 0.8 `#[server]` function addressing and calling from Bun.
//
// The fullstack macro mounts every server function as POST at
// `concatcp!(prefix, "/" + fn_name, hash)` where `hash` is the decimal u64
// `xxh64(concat!(env!("CARGO_MANIFEST_DIR"), ":", module_path!()), 0)` for
// implicit endpoints and the empty string for `#[server(endpoint = "...")]`.

/** xxh64 of a UTF-8 string with a 64-bit seed; matches Rust's `xxhash_rust::const_xxh64::xxh64`. */
export function xxh64(input: string | Uint8Array, seed = 0n): bigint {
  return Bun.hash.xxHash64(input, seed);
}

/** Decimal rendering of a u64, as produced by `concatcp!` on the Rust side. */
export function xxh64Decimal(input: string | Uint8Array, seed = 0n): string {
  return xxh64(input, seed).toString(10);
}

export interface ServerFnPathOptions {
  /** Route prefix passed to `#[server(prefix = ...)]`; defaults to "/api". */
  prefix?: string;
  /** Rust function name (`fn_name` in the macro output). */
  name: string;
  /** Value of `CARGO_MANIFEST_DIR` (or `SERVER_FN_OVERRIDE_KEY`) when the crate was built. */
  manifestDir?: string;
  /** Rust `module_path!()` of the function, for example `my_app::api`. */
  modulePath?: string;
  /** Explicit `#[server(endpoint = "...")]`; disables the hash suffix. */
  endpoint?: string;
}

function normalizePrefix(prefix: string | undefined): string {
  const value = prefix ?? "/api";
  if (value === "" || value === "/") return "";
  return value.endsWith("/") ? value.slice(0, -1) : value;
}

/** Compute the mounted path of a Dioxus server function. */
export function serverFnPath(options: ServerFnPathOptions): string {
  const prefix = normalizePrefix(options.prefix);
  if (options.endpoint !== undefined) {
    const endpoint = options.endpoint.startsWith("/") ? options.endpoint : `/${options.endpoint}`;
    return `${prefix}${endpoint}`;
  }
  const hash =
    options.manifestDir && options.modulePath ? xxh64Decimal(`${options.manifestDir}:${options.modulePath}`, 0n) : "";
  return `${prefix}/${options.name}${hash}`;
}

/** Error thrown when a server function replies with a non-2xx status. */
export class DioxusServerFnError extends Error {
  readonly status: number;
  readonly body: unknown;
  readonly path: string;

  constructor(path: string, status: number, body: unknown) {
    super(`Dioxus server function ${path} failed with status ${status}`);
    this.name = "DioxusServerFnError";
    this.status = status;
    this.body = body;
    this.path = path;
  }
}

export type ServerFnInput = "json" | "url";

export type CallServerFnOptions = {
  /** Origin of the Dioxus server, for example `http://127.0.0.1:8080`. */
  baseUrl: string;
  /** Named arguments; keys must match the Rust parameter names. */
  args?: Record<string, unknown>;
  /** Request encoding; "json" (default) or "url" (form-urlencoded). */
  input?: ServerFnInput;
  headers?: HeadersInit;
  /** Custom fetch, for testing or instrumentation. */
  fetch?: typeof fetch;
  signal?: AbortSignal;
} & ({ path: string } | ServerFnPathOptions);

function encodeArgs(args: Record<string, unknown>, input: ServerFnInput): { body: string; contentType: string } {
  if (input === "url") {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(args)) {
      if (value === undefined) continue;
      params.append(key, typeof value === "string" ? value : JSON.stringify(value));
    }
    return {
      body: params.toString(),
      contentType: "application/x-www-form-urlencoded",
    };
  }
  return { body: JSON.stringify(args), contentType: "application/json" };
}

async function parseBody(response: Response): Promise<unknown> {
  const text = await response.text();
  if (text.length === 0) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

/** Invoke a Dioxus server function over HTTP and return its decoded JSON output. */
export async function callServerFn<T = unknown>(options: CallServerFnOptions): Promise<T> {
  const path = "path" in options ? options.path : serverFnPath(options);
  const url = new URL(path, options.baseUrl.endsWith("/") ? options.baseUrl : `${options.baseUrl}/`);
  const { body, contentType } = encodeArgs(options.args ?? {}, options.input ?? "json");
  const headers = new Headers(options.headers);
  headers.set("content-type", contentType);
  if (!headers.has("accept")) headers.set("accept", "application/json");
  const doFetch = options.fetch ?? fetch;
  const response = await doFetch(url, {
    method: "POST",
    headers,
    body,
    signal: options.signal,
  });
  const parsed = await parseBody(response);
  if (!response.ok) throw new DioxusServerFnError(url.pathname, response.status, parsed);
  return parsed as T;
}
