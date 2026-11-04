// SPDX-License-Identifier: MIT OR Apache-2.0
// `rewrites`, `redirects`, `headers`, `basePath` and `trailingSlash` of next.config, compiled once and applied
// by the host around the App Router: redirects and `beforeFiles` rewrites run before the routes, `afterFiles`
// and `fallback` rewrites run when no route or static file matched, headers decorate every response.
import type { HeaderRule, Redirect, ResolvedConfig, Rewrite, RouteHas } from "./next-config";

export interface CompiledSource {
  regex: RegExp;
  /** Parameter names in order; `repeat` for `:name*` / `:name+`. */
  keys: { name: string; repeat: boolean; optional: boolean }[];
}

const escape = (text: string) => text.replace(/[.+*?^${}()|[\]\\]/g, c => `\\${c}`);

/**
 * Compile a Next.js `source` (path-to-regexp v6 subset: `:name`, `:name*`, `:name+`, `:name?`,
 * `:name(regex)`, literal segments) into a RegExp with named groups.
 */
export function compileSource(source: string): CompiledSource {
  const keys: CompiledSource["keys"] = [];
  let out = "";
  let i = 0;
  while (i < source.length) {
    const c = source[i]!;
    if (c === ":") {
      const m = /^:([A-Za-z_]\w*)(\([^)]*\))?([*+?])?/.exec(source.slice(i));
      if (!m) throw new Error(`@yolo-rs/fullstack: invalid source ${source}`);
      const [, name, group, mod] = m;
      const inner = group ? group.slice(1, -1) : "[^/]+";
      const slash = out.endsWith("/");
      if (slash) out = out.slice(0, -1);
      if (mod === "*") out += `(?:/(?<${name}>${inner}(?:/${inner})*))?`;
      else if (mod === "+") out += `/(?<${name}>${inner}(?:/${inner})*)`;
      else if (mod === "?") out += `(?:/(?<${name}>${inner}))?`;
      else out += `${slash ? "/" : ""}(?<${name}>${inner})`;
      if (!slash && (mod === "*" || mod === "+" || mod === "?")) throw new Error(`@yolo-rs/fullstack: modifier ${mod} needs a leading "/" in ${source}`);
      keys.push({ name: name!, repeat: mod === "*" || mod === "+", optional: mod === "*" || mod === "?" });
      i += m[0].length;
      continue;
    }
    if (c === "(") {
      let depth = 0;
      let end = i;
      for (; end < source.length; end++) {
        if (source[end] === "\\") end++;
        else if (source[end] === "(") depth++;
        else if (source[end] === ")" && --depth === 0) break;
      }
      out += `(?:${source.slice(i + 1, end)})`;
      i = end + 1;
      continue;
    }
    out += escape(c);
    i++;
  }
  return { regex: new RegExp(`^${out}/?$`), keys };
}

export type Captures = Record<string, string | undefined>;

/** Match `pathname` against a compiled source; `undefined` when it does not match. */
export function matchSource(compiled: CompiledSource, pathname: string): Captures | undefined {
  const m = compiled.regex.exec(pathname);
  if (!m) return undefined;
  return { ...m.groups };
}

/** Replace `:name` references in a destination with captured values. */
export function fillDestination(destination: string, captures: Captures): string {
  return destination.replace(/:([A-Za-z_]\w*)(\*|\+|\?)?/g, (whole, name: string) => {
    const value = captures[name];
    return value === undefined ? whole : value;
  });
}

interface RequestFacts {
  url: URL;
  headers: Headers;
  cookies: Bun.CookieMap;
}

function hasMatches(cond: RouteHas, facts: RequestFacts, captures: Captures): boolean {
  let actual: string | undefined | null;
  if (cond.type === "header") actual = facts.headers.get(cond.key);
  else if (cond.type === "cookie") actual = facts.cookies.get(cond.key);
  else if (cond.type === "query") actual = facts.url.searchParams.get(cond.key);
  else actual = facts.url.host;
  if (actual === null || actual === undefined) return false;
  if (cond.value === undefined) {
    captures[cond.key] = actual;
    return true;
  }
  const m = new RegExp(`^${cond.value}$`).exec(actual);
  if (!m) return false;
  Object.assign(captures, m.groups);
  return true;
}

export function conditionsMatch(rule: { has?: RouteHas[]; missing?: RouteHas[] }, facts: RequestFacts, captures: Captures): boolean {
  for (const cond of rule.has ?? []) if (!hasMatches(cond, facts, captures)) return false;
  for (const cond of rule.missing ?? []) if (hasMatches(cond, facts, { ...captures })) return false;
  return true;
}

interface CompiledRule<T> {
  rule: T;
  source: CompiledSource;
}

export interface RouteRules {
  basePath: string;
  trailingSlash: boolean;
  skipTrailingSlashRedirect: boolean;
  poweredByHeader: boolean;
  redirects: CompiledRule<Redirect>[];
  beforeFiles: CompiledRule<Rewrite>[];
  afterFiles: CompiledRule<Rewrite>[];
  fallback: CompiledRule<Rewrite>[];
  headers: CompiledRule<HeaderRule>[];
}

const compileRules = <T extends { source: string; basePath?: false }>(rules: T[], basePath: string): CompiledRule<T>[] =>
  rules.map(rule => ({ rule, source: compileSource(rule.basePath === false || !basePath ? rule.source : basePath + rule.source) }));

/** Compile the routing rules of a resolved config. */
export function compileRoutes(config: Pick<ResolvedConfig, "basePath" | "trailingSlash" | "skipTrailingSlashRedirect" | "poweredByHeader" | "redirects" | "rewrites" | "headers">): RouteRules {
  const { basePath } = config;
  return {
    basePath,
    trailingSlash: config.trailingSlash,
    skipTrailingSlashRedirect: config.skipTrailingSlashRedirect,
    poweredByHeader: config.poweredByHeader,
    redirects: compileRules(config.redirects, basePath),
    beforeFiles: compileRules(config.rewrites.beforeFiles, basePath),
    afterFiles: compileRules(config.rewrites.afterFiles, basePath),
    fallback: compileRules(config.rewrites.fallback, basePath),
    headers: compileRules(config.headers, basePath),
  };
}

export type RouteDecision =
  | { kind: "response"; response: Response }
  | { kind: "rewrite"; url: URL; external: boolean }
  | { kind: "next" };

const facts = (request: Request, url: URL): RequestFacts => ({ url, headers: request.headers, cookies: new Bun.CookieMap(request.headers.get("cookie") ?? "") });

function destinationUrl(rule: Rewrite, captures: Captures, url: URL, basePath: string): { url: URL; external: boolean } {
  let target = fillDestination(rule.destination, captures);
  const external = /^https?:\/\//.test(target);
  if (!external && basePath && rule.basePath !== false && !target.startsWith(basePath)) target = basePath + target;
  const out = new URL(target, url);
  // Query parameters of the request not consumed by the destination are carried over.
  for (const [key, value] of url.searchParams) if (!out.searchParams.has(key)) out.searchParams.append(key, value);
  return { url: out, external };
}

/** Trailing-slash and basePath normalization: a redirect, or the pathname to route internally. */
export function normalizePath(rules: RouteRules, url: URL): { redirect?: Response; pathname: string } {
  let pathname = url.pathname;
  if (rules.basePath) {
    if (pathname === rules.basePath) pathname = "/";
    else if (pathname.startsWith(`${rules.basePath}/`)) pathname = pathname.slice(rules.basePath.length);
    else return { pathname };
  }
  if (!rules.skipTrailingSlashRedirect && pathname !== "/" && !pathname.startsWith("/_yolo/") && !/\.[a-z0-9]+$/i.test(pathname)) {
    const hasSlash = pathname.endsWith("/");
    if (rules.trailingSlash && !hasSlash) {
      const target = new URL(url);
      target.pathname = `${url.pathname}/`;
      return { redirect: new Response(null, { status: 308, headers: { location: target.pathname + target.search } }), pathname };
    }
    if (!rules.trailingSlash && hasSlash) {
      const target = new URL(url);
      target.pathname = url.pathname.replace(/\/+$/, "");
      return { redirect: new Response(null, { status: 308, headers: { location: target.pathname + target.search } }), pathname };
    }
  }
  if (pathname.length > 1 && pathname.endsWith("/")) pathname = pathname.replace(/\/+$/, "");
  return { pathname };
}

/** Redirects then `beforeFiles` rewrites, evaluated before the routes. */
export function beforeRoutes(rules: RouteRules, request: Request, url: URL): RouteDecision {
  const f = facts(request, url);
  for (const { rule, source } of rules.redirects) {
    const captures = matchSource(source, url.pathname);
    if (!captures || !conditionsMatch(rule, f, captures)) continue;
    const { url: target } = destinationUrl(rule, captures, url, rules.basePath);
    const status = rule.statusCode ?? (rule.permanent ? 308 : 307);
    const location = target.origin === url.origin ? target.pathname + target.search + target.hash : target.href;
    return { kind: "response", response: new Response(null, { status, headers: { location } }) };
  }
  return applyRewrites(rules.beforeFiles, rules, request, url, f);
}

/** `afterFiles` then `fallback` rewrites, evaluated when nothing matched. */
export function afterRoutes(rules: RouteRules, request: Request, url: URL, stage: "afterFiles" | "fallback"): RouteDecision {
  return applyRewrites(rules[stage], rules, request, url, facts(request, url));
}

function applyRewrites(list: CompiledRule<Rewrite>[], rules: RouteRules, _request: Request, url: URL, f: RequestFacts): RouteDecision {
  for (const { rule, source } of list) {
    const captures = matchSource(source, url.pathname);
    if (!captures || !conditionsMatch(rule, f, captures)) continue;
    const target = destinationUrl(rule, captures, url, rules.basePath);
    return { kind: "rewrite", ...target };
  }
  return { kind: "next" };
}

/** Headers of the config rules matching `url`, plus `x-powered-by`. */
export function responseHeaders(rules: RouteRules, request: Request, url: URL): [string, string][] {
  const out: [string, string][] = [];
  const f = facts(request, url);
  for (const { rule, source } of rules.headers) {
    const captures = matchSource(source, url.pathname);
    if (!captures || !conditionsMatch(rule, f, captures)) continue;
    for (const header of rule.headers) out.push([header.key, fillDestination(header.value, captures)]);
  }
  if (rules.poweredByHeader) out.push(["x-powered-by", "@yolo-rs/fullstack"]);
  return out;
}

const HOP_BY_HOP = new Set(["connection", "keep-alive", "proxy-authenticate", "proxy-authorization", "te", "trailer", "transfer-encoding", "upgrade", "host"]);

/** Proxy an external rewrite (streams the upstream response). */
export async function proxyRewrite(request: Request, target: URL): Promise<Response> {
  const headers = new Headers();
  for (const [key, value] of request.headers) if (!HOP_BY_HOP.has(key)) headers.set(key, value);
  headers.set("x-forwarded-host", new URL(request.url).host);
  headers.set("x-forwarded-proto", new URL(request.url).protocol.replace(":", ""));
  const upstream = await fetch(target, {
    method: request.method,
    headers,
    body: request.method === "GET" || request.method === "HEAD" ? undefined : request.body,
    redirect: "manual",
  });
  const out = new Headers(upstream.headers);
  for (const key of HOP_BY_HOP) out.delete(key);
  return new Response(upstream.body, { status: upstream.status, statusText: upstream.statusText, headers: out });
}
