// SPDX-License-Identifier: MIT OR Apache-2.0
// Reverse proxy from Bun.serve to a Dioxus fullstack (axum) server.

const HOP_BY_HOP = new Set([
  "connection",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "proxy-connection",
  "te",
  "trailer",
  "transfer-encoding",
  "upgrade",
  "host",
]);

export interface DioxusProxyOptions {
  /** Upstream origin, for example `http://127.0.0.1:8080`. */
  target: string;
  /** Only pathnames under this prefix are proxied; defaults to "/api". */
  prefix?: string;
  fetch?: typeof fetch;
}

export interface DioxusProxy {
  (request: Request): Promise<Response>;
  matches(pathname: string): boolean;
  readonly prefix: string;
  readonly target: string;
}

function normalizePrefix(prefix: string | undefined): string {
  const value = prefix ?? "/api";
  if (value === "" || value === "/") return "/";
  return value.endsWith("/") ? value.slice(0, -1) : value;
}

/** Copy request headers minus hop-by-hop ones and host; add forwarding headers. */
export function forwardedHeaders(request: Request, requestUrl: URL): Headers {
  const headers = new Headers();
  const connectionTokens = new Set(
    (request.headers.get("connection") ?? "")
      .split(",")
      .map(token => token.trim().toLowerCase())
      .filter(token => token.length > 0),
  );
  request.headers.forEach((value, key) => {
    const name = key.toLowerCase();
    if (HOP_BY_HOP.has(name) || connectionTokens.has(name)) return;
    headers.append(key, value);
  });
  headers.set("x-forwarded-host", requestUrl.host);
  headers.set("x-forwarded-proto", requestUrl.protocol.replace(":", ""));
  return headers;
}

/** Strip hop-by-hop headers from the upstream response. */
function responseHeaders(upstream: Response): Headers {
  const headers = new Headers();
  upstream.headers.forEach((value, key) => {
    if (HOP_BY_HOP.has(key.toLowerCase())) return;
    headers.append(key, value);
  });
  return headers;
}

/** Build a request handler that streams matching requests to the Dioxus server. */
export function dioxusProxy(options: DioxusProxyOptions): DioxusProxy {
  const target = new URL(options.target);
  const prefix = normalizePrefix(options.prefix);
  const doFetch = options.fetch ?? fetch;

  const matches = (pathname: string): boolean =>
    prefix === "/" || pathname === prefix || pathname.startsWith(`${prefix}/`);

  const handler = async (request: Request): Promise<Response> => {
    const requestUrl = new URL(request.url);
    const upstreamUrl = new URL(requestUrl.pathname + requestUrl.search, target);
    const method = request.method.toUpperCase();
    const hasBody = method !== "GET" && method !== "HEAD" && request.body !== null;
    const init: RequestInit & { duplex?: "half" } = {
      method,
      headers: forwardedHeaders(request, requestUrl),
      redirect: "manual",
      signal: request.signal,
    };
    if (hasBody) {
      init.body = request.body;
      init.duplex = "half";
    }
    const upstream = await doFetch(upstreamUrl, init);
    return new Response(upstream.body, {
      status: upstream.status,
      statusText: upstream.statusText,
      headers: responseHeaders(upstream),
    });
  };

  return Object.assign(handler, {
    matches,
    prefix,
    target: target.origin,
  }) as DioxusProxy;
}
