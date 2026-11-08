// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/rules: redirects()/rewrites()/headers() sources (path-to-regexp v6 subset) compiled to
// matchers, Bun.serve patterns, axum patterns and the request-time rule engine used by the server.

import type { HeaderRule, ResolvedYoloConfig, RouteCondition, RouteRule } from "./types.ts";

/** Modifier of a named parameter. */
export type ParamModifier = "one" | "zero-or-more" | "one-or-more" | "optional";

/** One compiled segment of a source pattern. */
export type PatternSegment =
  | { readonly kind: "static"; readonly text: string }
  | { readonly kind: "param"; readonly name: string; readonly modifier: ParamModifier; readonly regex?: string }
  | { readonly kind: "wildcard"; readonly name: string; readonly regex: string };

/** A compiled `source` pattern. */
export interface CompiledRule {
  readonly source: string;
  readonly segments: ReadonlyArray<PatternSegment>;
  /** JavaScript regex source with named groups (`(?<slug>[^/]+?)`), anchored and trailing-slash tolerant. */
  readonly regexSource: string;
  readonly regex: RegExp;
  /** `Bun.serve` routes key (`/blog/:slug`, `/docs/*`). */
  readonly bunPattern: string;
  /** axum 0.8 route (`/blog/{slug}`, `/docs/{*path}`). */
  readonly axumPattern: string;
  /** Parameter names in order (unnamed groups are numbered). */
  readonly keys: ReadonlyArray<string>;
  /** Matches a pathname; captured params are percent-decoded, repeated params keep their `/` separators. */
  match(pathname: string): Record<string, string> | undefined;
}

const ESCAPE_REGEX = /[.*+?^${}()|[\]\\]/g;
const escapeRegex = (text: string): string => text.replace(ESCAPE_REGEX, "\\$&");

/** Reads a balanced `(...)` group starting at `i` (which must be `(`); returns the inner text and end index. */
function readGroup(source: string, i: number): { inner: string; end: number } {
  let depth = 0;
  for (let pos = i; pos < source.length; pos += 1) {
    const ch = source.charAt(pos);
    if (ch === "\\") {
      pos += 1;
      continue;
    }
    if (ch === "(") depth += 1;
    else if (ch === ")") {
      depth -= 1;
      if (depth === 0) return { inner: source.slice(i + 1, pos), end: pos + 1 };
    }
  }
  throw new Error(`unbalanced group in route source: ${source}`);
}

/** Tokenizes a path-to-regexp source into segments. */
function tokenize(source: string): PatternSegment[] {
  const segments: PatternSegment[] = [];
  let text = "";
  let unnamed = 0;
  const flush = (): void => {
    if (text) segments.push({ kind: "static", text });
    text = "";
  };
  let i = 0;
  while (i < source.length) {
    const ch = source.charAt(i);
    if (ch === "\\") {
      text += source.charAt(i + 1);
      i += 2;
      continue;
    }
    if (ch === ":") {
      const match = /^[A-Za-z_$][\w$]*/.exec(source.slice(i + 1));
      if (!match) {
        text += ch;
        i += 1;
        continue;
      }
      flush();
      const name = match[0];
      i += 1 + name.length;
      let regex: string | undefined;
      if (source.charAt(i) === "(") {
        const group = readGroup(source, i);
        regex = group.inner;
        i = group.end;
      }
      let modifier: ParamModifier = "one";
      const mod = source.charAt(i);
      if (mod === "*") modifier = "zero-or-more";
      else if (mod === "+") modifier = "one-or-more";
      else if (mod === "?") modifier = "optional";
      if (modifier !== "one") i += 1;
      segments.push({ kind: "param", name, modifier, regex });
      continue;
    }
    if (ch === "(") {
      flush();
      const group = readGroup(source, i);
      i = group.end;
      let suffix = "";
      const mod = source.charAt(i);
      if (mod === "*" || mod === "+" || mod === "?") {
        suffix = mod;
        i += 1;
      }
      const name = String(unnamed);
      unnamed += 1;
      segments.push({ kind: "wildcard", name, regex: suffix ? `(?:${group.inner})${suffix}` : group.inner });
      continue;
    }
    if (ch === "*") {
      flush();
      const name = String(unnamed);
      unnamed += 1;
      segments.push({ kind: "wildcard", name, regex: ".*" });
      i += 1;
      continue;
    }
    text += ch;
    i += 1;
  }
  flush();
  return segments;
}

/** Splits a static text into its leading delimiter-free part and a trailing `/` prefix for the next param. */
function splitPrefix(text: string): { body: string; prefix: string } {
  return text.endsWith("/") ? { body: text.slice(0, -1), prefix: "/" } : { body: text, prefix: "" };
}

function buildRegex(segments: ReadonlyArray<PatternSegment>, named: boolean): string {
  let out = "^";
  for (let i = 0; i < segments.length; i += 1) {
    const seg = segments[i];
    if (!seg) continue;
    if (seg.kind === "static") {
      const next = segments[i + 1];
      if (next && next.kind !== "static") {
        const { body } = splitPrefix(seg.text);
        out += escapeRegex(body);
      } else {
        out += escapeRegex(seg.text);
      }
      continue;
    }
    const prev = segments[i - 1];
    const prefix = prev && prev.kind === "static" ? splitPrefix(prev.text).prefix : "";
    const groupName = /^\d/.test(seg.name) ? `_${seg.name}` : seg.name.replace(/[^\w$]/g, "_");
    const open = named ? `(?<${groupName}>` : "(";
    if (seg.kind === "wildcard") {
      out += `(?:${escapeRegex(prefix)}${open}${seg.regex}))`;
      continue;
    }
    const unit = seg.regex ?? "[^/]+?";
    switch (seg.modifier) {
      case "one":
        out += `(?:${escapeRegex(prefix)}${open}${unit}))`;
        break;
      case "optional":
        out += `(?:${escapeRegex(prefix)}${open}${unit}))?`;
        break;
      case "zero-or-more":
        out += `(?:${escapeRegex(prefix)}${open}(?:${unit})(?:/(?:${unit}))*))?`;
        break;
      case "one-or-more":
        out += `(?:${escapeRegex(prefix)}${open}(?:${unit})(?:/(?:${unit}))*))`;
        break;
    }
  }
  return `${out}(?:/)?$`;
}

function safeDecode(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

/** Compiles a Next.js `source` (`/old/:slug`, `/docs/:path*`, `/:id(\\d+)`, `/(.*)`) into a matcher. */
export function compileSource(source: string): CompiledRule {
  const segments = tokenize(source);
  const keys: string[] = [];
  for (const seg of segments) if (seg.kind !== "static") keys.push(seg.name);
  const regexSource = buildRegex(segments, true);
  const regex = new RegExp(buildRegex(segments, false), "i");
  let bun = "";
  let axum = "";
  segments.forEach((seg, index) => {
    const last = index === segments.length - 1;
    if (seg.kind === "static") {
      bun += seg.text;
      axum += seg.text;
      return;
    }
    if (seg.kind === "wildcard" || seg.modifier === "zero-or-more" || seg.modifier === "one-or-more") {
      bun += last ? "*" : `:${seg.name}`;
      axum += last ? `{*${seg.kind === "wildcard" ? `rest${seg.name}` : seg.name}}` : `{${seg.name}}`;
      return;
    }
    bun += `:${seg.name}`;
    axum += `{${seg.name}}`;
  });
  return {
    source,
    segments,
    regexSource,
    regex,
    bunPattern: bun || "/",
    axumPattern: axum || "/",
    keys,
    match(pathname: string): Record<string, string> | undefined {
      const path = pathname.length > 1 && pathname.endsWith("/") ? pathname.slice(0, -1) : pathname;
      const result = regex.exec(path);
      if (!result) return undefined;
      const params: Record<string, string> = {};
      keys.forEach((key, index) => {
        const value = result[index + 1];
        if (value !== undefined) params[key] = safeDecode(value);
      });
      return params;
    },
  };
}

/** Substitutes `:param`, `:param*` and `:0` tokens of a destination (absolute URLs preserved). */
export function applyDestination(destination: string, params: Record<string, string>): string {
  return destination.replace(/:([A-Za-z_$][\w$]*|\d+)([*+?])?/g, (whole, name: string, _mod: string | undefined, offset: number) => {
    // Keep `https://` and `http://` intact (the `:` of the scheme is followed by `//`).
    if (destination.slice(offset + 1 + name.length).startsWith("//")) return whole;
    const value = params[name];
    return value === undefined ? "" : value;
  });
}

/** Status code of a redirect rule: `statusCode` > `permanent` (308) > 307. */
export function redirectStatus(rule: RouteRule): number {
  if (rule.statusCode) return rule.statusCode;
  return rule.permanent ? 308 : 307;
}

/** Request facts evaluated by `has`/`missing` conditions. */
export interface RequestFacts {
  readonly headers: Headers;
  readonly cookies: Record<string, string>;
  readonly query: URLSearchParams;
  readonly host: string;
}

function valueMatches(expected: string | undefined, actual: string, params?: Record<string, string>): boolean {
  if (expected === undefined) return true;
  if (expected === actual) return true;
  let regex: RegExp;
  try {
    regex = new RegExp(`^(?:${expected})$`);
  } catch {
    return false;
  }
  const match = regex.exec(actual);
  if (!match) return false;
  if (params && match.groups) for (const [key, value] of Object.entries(match.groups)) if (value !== undefined) params[key] = value;
  return true;
}

/** Evaluates one `has`/`missing` condition; named groups of regex values are written to `params`. */
export function conditionMatches(cond: RouteCondition, facts: RequestFacts, params?: Record<string, string>): boolean {
  switch (cond.type) {
    case "header": {
      const value = cond.key ? facts.headers.get(cond.key) : null;
      return value !== null && valueMatches(cond.value, value, params);
    }
    case "cookie": {
      const value = cond.key ? facts.cookies[cond.key] : undefined;
      return value !== undefined && valueMatches(cond.value, value, params);
    }
    case "query": {
      const value = cond.key ? facts.query.get(cond.key) : null;
      return value !== null && valueMatches(cond.value, value, params);
    }
    case "host": {
      const host = facts.host.split(":")[0] ?? "";
      if (!valueMatches(cond.value, host, params)) return false;
      if (params && cond.value !== undefined && !params.host) params.host = host;
      return true;
    }
    default:
      return false;
  }
}

/** Parses a `Cookie` header into a name -> value record. */
export function parseCookies(header: string | null): Record<string, string> {
  const out: Record<string, string> = {};
  if (!header) return out;
  for (const part of header.split(";")) {
    const index = part.indexOf("=");
    if (index === -1) continue;
    const name = part.slice(0, index).trim();
    if (!name || name in out) continue;
    out[name] = safeDecode(part.slice(index + 1).trim());
  }
  return out;
}

/** Builds the condition facts of a request. */
export function requestFacts(request: Request, url = new URL(request.url)): RequestFacts {
  return {
    headers: request.headers,
    cookies: parseCookies(request.headers.get("cookie")),
    query: url.searchParams,
    host: request.headers.get("host") ?? url.host,
  };
}

const compiled = new Map<string, CompiledRule>();

/** Memoized `compileSource`. */
export function compiledRule(source: string): CompiledRule {
  let rule = compiled.get(source);
  if (!rule) {
    rule = compileSource(source);
    compiled.set(source, rule);
  }
  return rule;
}

interface RuleLike {
  readonly source: string;
  readonly basePath?: false;
  readonly has?: ReadonlyArray<RouteCondition>;
  readonly missing?: ReadonlyArray<RouteCondition>;
}

/** Matches one rule (source + has/missing) against a pathname and request facts; returns the params. */
export function matchRule(rule: RuleLike, pathname: string, facts: RequestFacts, basePath = ""): Record<string, string> | undefined {
  const source = basePath && rule.basePath !== false ? basePath + rule.source : rule.source;
  const params = compiledRule(source).match(pathname);
  if (!params) return undefined;
  for (const cond of rule.has ?? []) if (!conditionMatches(cond, facts, params)) return undefined;
  for (const cond of rule.missing ?? []) if (conditionMatches(cond, facts)) return undefined;
  return params;
}

/** Phase of the rule engine. */
export type RulePhase = "redirects" | "beforeFiles" | "afterFiles" | "fallback" | "headers";

/** Result of `applyRules`: a terminal response (redirect), a rewritten URL, or headers to merge. */
export interface RulesResult {
  readonly response?: Response;
  readonly rewrittenUrl?: URL;
  readonly headers?: Headers;
  /** The rule that matched (first match wins for redirects and rewrites). */
  readonly rule?: RouteRule | HeaderRule;
}

function destinationUrl(destination: string, params: Record<string, string>, url: URL): URL {
  const target = applyDestination(destination, params);
  const absolute = /^[a-z][a-z0-9+.-]*:\/\//i.test(target);
  const out = absolute ? new URL(target) : new URL(target, url.origin);
  // Query values of the incoming URL are forwarded unless the destination names them.
  for (const [key, value] of url.searchParams) if (!out.searchParams.has(key)) out.searchParams.append(key, value);
  return out;
}

/**
 * Applies the rules of one phase to a request. `redirects` yields a `response` (Location + 307/308),
 * `beforeFiles`/`afterFiles`/`fallback` yield a `rewrittenUrl` (absolute for external rewrites),
 * `headers` yields the merged response `headers` of every matching rule.
 */
export function applyRules(request: Request, config: ResolvedYoloConfig, phase: RulePhase): RulesResult {
  const url = new URL(request.url);
  const facts = requestFacts(request, url);
  const basePath = config.cdn.basePath;
  const pathname = url.pathname;
  if (phase === "headers") {
    const headers = new Headers();
    let matched: HeaderRule | undefined;
    for (const rule of config.headers) {
      const params = matchRule(rule, pathname, facts, basePath);
      if (!params) continue;
      matched ??= rule;
      for (const kv of rule.headers) headers.set(kv.key, applyDestination(kv.value, params));
    }
    return matched ? { headers, rule: matched } : {};
  }
  if (phase === "redirects") {
    for (const rule of config.redirects) {
      const params = matchRule(rule, pathname, facts, basePath);
      if (!params || rule.destination === undefined) continue;
      const target = destinationUrl(rule.destination, params, url);
      const location = target.origin === url.origin ? target.pathname + target.search + target.hash : target.href;
      if (location === pathname + url.search) continue;
      return { response: new Response(null, { status: redirectStatus(rule), headers: { location } }), rule };
    }
    return {};
  }
  const rules = config.rewrites[phase] ?? [];
  for (const rule of rules) {
    const params = matchRule(rule, pathname, facts, basePath);
    if (!params || rule.destination === undefined) continue;
    return { rewrittenUrl: destinationUrl(rule.destination, params, url), rule };
  }
  return {};
}

/** `routes-manifest.json`-shaped JSON for redirects, rewrites and headers. */
export function rulesManifestJson(config: Pick<ResolvedYoloConfig, "redirects" | "rewrites" | "headers">): Record<string, unknown> {
  const withRegex = <T extends RuleLike>(rule: T): T & { regex: string } => ({ ...rule, regex: compiledRule(rule.source).regexSource });
  return {
    redirects: config.redirects.map(withRegex),
    rewrites: {
      beforeFiles: (config.rewrites.beforeFiles ?? []).map(withRegex),
      afterFiles: (config.rewrites.afterFiles ?? []).map(withRegex),
      fallback: (config.rewrites.fallback ?? []).map(withRegex),
    },
    headers: config.headers.map(withRegex),
  };
}
