// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/segments: static analysis of page/layout/route modules (route segment config,
// directives, generate* exports, metadata literals, hooks, next/* imports). Uses oxc-parser when it
// is installed and a tokenizer/regex fallback otherwise; both paths produce the same SegmentConfig.

import type { Diagnostic, Metadata, ModuleDirective, SegmentConfig, Viewport } from "./types.ts";

// ---------------------------------------------------------------------------
// oxc-parser (optional, loaded synchronously and lazily)
// ---------------------------------------------------------------------------

interface OxcSpan {
  readonly start: number;
  readonly end: number;
}

interface OxcName extends OxcSpan {
  readonly kind: "Name" | "Default" | "None";
  readonly name: string | null;
}

interface OxcStaticExportEntry {
  readonly exportName: OxcName;
  readonly localName: OxcName;
  readonly moduleRequest: { readonly value: string } | null;
}

interface OxcParseResult {
  readonly program: { readonly body: ReadonlyArray<Record<string, unknown>> };
  readonly errors: ReadonlyArray<unknown>;
  readonly module: {
    readonly staticExports: ReadonlyArray<{ readonly entries: ReadonlyArray<OxcStaticExportEntry> }>;
    readonly staticImports: ReadonlyArray<{ readonly moduleRequest: { readonly value: string } }>;
  };
}

type OxcParseSync = (filename: string, source: string) => OxcParseResult;

let oxcLoaded = false;
let oxcParseSync: OxcParseSync | undefined;

/** Returns `oxc-parser`'s `parseSync` when the package is installed, else undefined (cached). */
export function oxcParser(): OxcParseSync | undefined {
  if (oxcLoaded) return oxcParseSync;
  oxcLoaded = true;
  try {
    const mod = import.meta.require("oxc-parser") as { parseSync?: unknown };
    if (typeof mod.parseSync === "function") oxcParseSync = mod.parseSync as OxcParseSync;
  } catch {
    oxcParseSync = undefined;
  }
  return oxcParseSync;
}

function parseWithOxc(source: string, file: string): OxcParseResult | undefined {
  const parse = oxcParser();
  if (!parse) return undefined;
  try {
    const result = parse(file, source);
    return result.errors.length > 0 && result.program.body.length === 0 ? undefined : result;
  } catch {
    return undefined;
  }
}

// ---------------------------------------------------------------------------
// Tokenizer helpers
// ---------------------------------------------------------------------------

const WHITESPACE = /\s/;

/** Index of the first significant character at or after `i` (skips whitespace, comments and a hashbang). */
function skipTrivia(source: string, i: number): number {
  let pos = i;
  if (pos === 0 && source.startsWith("#!")) {
    const nl = source.indexOf("\n");
    pos = nl === -1 ? source.length : nl + 1;
  }
  for (;;) {
    while (pos < source.length && WHITESPACE.test(source.charAt(pos))) pos += 1;
    if (source.startsWith("//", pos)) {
      const nl = source.indexOf("\n", pos);
      pos = nl === -1 ? source.length : nl + 1;
      continue;
    }
    if (source.startsWith("/*", pos)) {
      const end = source.indexOf("*/", pos + 2);
      pos = end === -1 ? source.length : end + 2;
      continue;
    }
    return pos;
  }
}

/** Reads a `"..."`/`'...'` literal starting at `i`; returns its decoded value and end index. */
function readStringLiteral(source: string, i: number): { value: string; end: number } | undefined {
  const quote = source.charAt(i);
  if (quote !== '"' && quote !== "'") return undefined;
  let out = "";
  let pos = i + 1;
  while (pos < source.length) {
    const ch = source.charAt(pos);
    if (ch === "\\") {
      const next = source.charAt(pos + 1);
      out += next === "n" ? "\n" : next === "t" ? "\t" : next;
      pos += 2;
      continue;
    }
    if (ch === quote) return { value: out, end: pos + 1 };
    if (ch === "\n") return undefined;
    out += ch;
    pos += 1;
  }
  return undefined;
}

/** Maps a directive string to a ModuleDirective (`use cache: private` and friends count as `use-cache`). */
export function directiveFromString(value: string): ModuleDirective {
  const text = value.trim();
  if (text === "use client") return "use-client";
  if (text === "use server") return "use-server";
  if (text === "use cache" || text.startsWith("use cache:")) return "use-cache";
  return "none";
}

/**
 * Detects the leading module directive (`"use client"`, `"use server"`, `"use cache"`), ignoring
 * comments, whitespace, a hashbang and other directives such as `"use strict"`.
 */
export function detectDirective(source: string): ModuleDirective {
  let pos = 0;
  for (;;) {
    pos = skipTrivia(source, pos);
    const literal = readStringLiteral(source, pos);
    if (!literal) return "none";
    const after = skipTrivia(source, literal.end);
    const next = source.charAt(after);
    // A directive is a complete expression statement: `;`, a newline before the next token, or EOF.
    const terminated = next === ";" || next === "" || source.slice(literal.end, after).includes("\n");
    if (!terminated) return "none";
    const directive = directiveFromString(literal.value);
    if (directive !== "none") return directive;
    pos = next === ";" ? after + 1 : after;
  }
}

/** Strips comments while preserving string, template and regex-free code (good enough for export scanning). */
export function stripComments(source: string): string {
  let out = "";
  let i = 0;
  while (i < source.length) {
    const ch = source.charAt(i);
    if (ch === "/" && source.charAt(i + 1) === "/") {
      const nl = source.indexOf("\n", i);
      i = nl === -1 ? source.length : nl;
      continue;
    }
    if (ch === "/" && source.charAt(i + 1) === "*") {
      const end = source.indexOf("*/", i + 2);
      i = end === -1 ? source.length : end + 2;
      out += " ";
      continue;
    }
    if (ch === '"' || ch === "'" || ch === "`") {
      const start = i;
      i += 1;
      while (i < source.length && source.charAt(i) !== ch) {
        if (source.charAt(i) === "\\") i += 1;
        if (ch !== "`" && source.charAt(i) === "\n") break;
        i += 1;
      }
      i += 1;
      out += source.slice(start, i);
      continue;
    }
    out += ch;
    i += 1;
  }
  return out;
}

// ---------------------------------------------------------------------------
// Literal evaluator (JSON5-ish subset of JavaScript expressions)
// ---------------------------------------------------------------------------

class LiteralError extends Error {}

const IDENT_START = /[A-Za-z_$]/;
const IDENT_PART = /[\w$]/;

class LiteralParser {
  pos: number;

  constructor(
    private readonly source: string,
    start: number,
  ) {
    this.pos = start;
  }

  private skip(): void {
    this.pos = skipTrivia(this.source, this.pos);
  }

  private peek(): string {
    this.skip();
    return this.source.charAt(this.pos);
  }

  private expect(ch: string): void {
    if (this.peek() !== ch) throw new LiteralError(`expected ${ch}`);
    this.pos += 1;
  }

  private ident(): string {
    this.skip();
    const start = this.pos;
    if (!IDENT_START.test(this.source.charAt(this.pos))) throw new LiteralError("identifier expected");
    while (this.pos < this.source.length && IDENT_PART.test(this.source.charAt(this.pos))) this.pos += 1;
    return this.source.slice(start, this.pos);
  }

  /** Parses one value and skips trailing `as const` / `satisfies X` annotations. */
  value(): unknown {
    const result = this.primary();
    this.skip();
    if (this.source.startsWith("as ", this.pos) || this.source.startsWith("satisfies ", this.pos)) {
      // Skip the type annotation up to the end of the statement (`;`, newline or EOF).
      while (this.pos < this.source.length && this.source.charAt(this.pos) !== ";" && this.source.charAt(this.pos) !== "\n") this.pos += 1;
    }
    return result;
  }

  private primary(): unknown {
    const ch = this.peek();
    if (ch === "{") return this.object();
    if (ch === "[") return this.array();
    if (ch === '"' || ch === "'") {
      const literal = readStringLiteral(this.source, this.pos);
      if (!literal) throw new LiteralError("unterminated string");
      this.pos = literal.end;
      return literal.value;
    }
    if (ch === "`") return this.template();
    if (ch === "-" || ch === "+" || /\d|\./.test(ch)) return this.number();
    if (ch === "(") {
      this.pos += 1;
      const inner = this.value();
      this.expect(")");
      return inner;
    }
    const word = this.ident();
    switch (word) {
      case "true":
        return true;
      case "false":
        return false;
      case "null":
        return null;
      case "undefined":
        return undefined;
      case "Infinity":
        return Number.POSITIVE_INFINITY;
      case "new":
        return this.construct();
      default:
        throw new LiteralError(`unsupported identifier ${word}`);
    }
  }

  private construct(): unknown {
    const name = this.ident();
    this.expect("(");
    const args: unknown[] = [];
    while (this.peek() !== ")") {
      args.push(this.value());
      if (this.peek() === ",") this.pos += 1;
    }
    this.expect(")");
    if (name === "URL") {
      const [href, base] = args;
      if (typeof href !== "string") throw new LiteralError("URL needs a string");
      return typeof base === "string" ? new URL(href, base).href : new URL(href).href;
    }
    if (name === "Date") {
      const [first] = args;
      if (typeof first === "string" || typeof first === "number") return new Date(first).toISOString();
      throw new LiteralError("Date needs a literal");
    }
    throw new LiteralError(`unsupported constructor ${name}`);
  }

  private number(): number {
    this.skip();
    const match = /^[+-]?(?:0x[\da-fA-F]+|\d+(?:\.\d+)?(?:e[+-]?\d+)?|\.\d+(?:e[+-]?\d+)?)/.exec(this.source.slice(this.pos));
    if (!match) throw new LiteralError("number expected");
    this.pos += match[0].length;
    const text = match[0].replace(/_/g, "");
    return text.includes("0x") ? Number.parseInt(text, 16) * (text.startsWith("-") ? -1 : 1) : Number(text);
  }

  private template(): string {
    this.expect("`");
    let out = "";
    while (this.pos < this.source.length) {
      const ch = this.source.charAt(this.pos);
      if (ch === "`") {
        this.pos += 1;
        return out;
      }
      if (ch === "$" && this.source.charAt(this.pos + 1) === "{") throw new LiteralError("template expression");
      if (ch === "\\") {
        out += this.source.charAt(this.pos + 1);
        this.pos += 2;
        continue;
      }
      out += ch;
      this.pos += 1;
    }
    throw new LiteralError("unterminated template");
  }

  private array(): unknown[] {
    this.expect("[");
    const out: unknown[] = [];
    for (;;) {
      const ch = this.peek();
      if (ch === "]") {
        this.pos += 1;
        return out;
      }
      if (ch === ",") {
        this.pos += 1;
        continue;
      }
      if (this.source.startsWith("...", this.pos)) throw new LiteralError("spread");
      out.push(this.value());
      const next = this.peek();
      if (next === ",") this.pos += 1;
      else if (next !== "]") throw new LiteralError("expected , or ]");
    }
  }

  private object(): Record<string, unknown> {
    this.expect("{");
    const out: Record<string, unknown> = {};
    for (;;) {
      const ch = this.peek();
      if (ch === "}") {
        this.pos += 1;
        return out;
      }
      if (ch === ",") {
        this.pos += 1;
        continue;
      }
      if (this.source.startsWith("...", this.pos)) throw new LiteralError("spread");
      let key: string;
      if (ch === '"' || ch === "'") {
        const literal = readStringLiteral(this.source, this.pos);
        if (!literal) throw new LiteralError("bad key");
        key = literal.value;
        this.pos = literal.end;
      } else if (ch === "[") {
        throw new LiteralError("computed key");
      } else if (/\d/.test(ch)) {
        key = String(this.number());
      } else {
        key = this.ident();
      }
      const sep = this.peek();
      if (sep === ":") {
        this.pos += 1;
        out[key] = this.value();
      } else if (sep === "," || sep === "}") {
        throw new LiteralError("shorthand property");
      } else {
        throw new LiteralError("expected :");
      }
      const next = this.peek();
      if (next === ",") this.pos += 1;
      else if (next !== "}") throw new LiteralError("expected , or }");
    }
  }
}

/** Evaluates a literal JavaScript expression at `start` (object/array/string/number/boolean/null/`new URL`); undefined when unsupported. */
export function evaluateLiteralAt(source: string, start: number): { value: unknown; end: number } | undefined {
  try {
    const parser = new LiteralParser(source, start);
    const value = parser.value();
    return { value, end: parser.pos };
  } catch {
    return undefined;
  }
}

/** Evaluates a standalone literal source text (`{ title: "x" }`); undefined when unsupported. */
export function evaluateLiteral(text: string): unknown {
  return evaluateLiteralAt(text, 0)?.value;
}

// ---------------------------------------------------------------------------
// Export scanning
// ---------------------------------------------------------------------------

/** Exports of a module as found by a light scanner or oxc-parser. */
export interface ModuleExports {
  /** Named exports (runtime values only; `type`/`interface` exports are excluded). */
  readonly names: string[];
  readonly hasDefault: boolean;
  /** Local identifier of the default export when it has one (`export default function Page`). */
  readonly defaultName?: string;
  readonly defaultAsync: boolean;
  /** `export * from` specifiers (names cannot be enumerated statically). */
  readonly reexportAll: string[];
}

const EXPORT_DECL = /\bexport\s+(?:declare\s+)?(async\s+)?(function\s*\*?|class|const\s+enum|enum|const|let|var|abstract\s+class)\s+([A-Za-z_$][\w$]*)/g;
const EXPORT_DESTRUCTURE = /\bexport\s+(?:const|let|var)\s*([{[])/g;
const EXPORT_LIST = /\bexport\s*(type\s+)?\{([^}]*)\}/g;
const EXPORT_DEFAULT = /\bexport\s+default\s+(?:(async)\s+)?(?:(function)\s*\*?\s*([A-Za-z_$][\w$]*)?|(class)\s+([A-Za-z_$][\w$]*)?|([A-Za-z_$][\w$]*)\s*(?=;|\n|$))?/g;
const EXPORT_ALL = /\bexport\s*\*\s*(?:as\s+[A-Za-z_$][\w$]*\s*)?from\s*["']([^"']+)["']/g;

function destructuredNames(text: string): string[] {
  const names: string[] = [];
  // Strip nested defaults `= ...` conservatively and collect identifiers in binding position.
  const cleaned = text.replace(/=\s*[^,}\]]+/g, "");
  for (const match of cleaned.matchAll(/(?:^|[,{[\s])(?:[A-Za-z_$][\w$]*\s*:\s*)?([A-Za-z_$][\w$]*)/g)) {
    const name = match[1];
    if (name && !names.includes(name)) names.push(name);
  }
  return names;
}

function scanExportsWithRegex(source: string): ModuleExports {
  const code = stripComments(source);
  const names: string[] = [];
  const add = (name: string): void => {
    if (name && name !== "default" && !names.includes(name)) names.push(name);
  };
  for (const match of code.matchAll(EXPORT_DECL)) add(match[3] ?? "");
  for (const match of code.matchAll(EXPORT_DESTRUCTURE)) {
    const open = match[1] === "{" ? "{" : "[";
    const close = open === "{" ? "}" : "]";
    let depth = 0;
    let i = match.index + match[0].length - 1;
    const start = i + 1;
    for (; i < code.length; i += 1) {
      const ch = code.charAt(i);
      if (ch === open) depth += 1;
      else if (ch === close) {
        depth -= 1;
        if (depth === 0) break;
      }
    }
    for (const name of destructuredNames(code.slice(start, i))) add(name);
  }
  for (const match of code.matchAll(EXPORT_LIST)) {
    if (match[1]) continue;
    for (const item of (match[2] ?? "").split(",")) {
      const part = item.trim();
      if (!part || part.startsWith("type ")) continue;
      const as = /^(.+?)\s+as\s+([A-Za-z_$][\w$]*|default)$/.exec(part);
      const exported = as ? (as[2] ?? "") : part;
      if (exported === "default") continue;
      add(exported.replace(/^default\s+as\s+/, ""));
    }
  }
  let hasDefault = false;
  let defaultName: string | undefined;
  let defaultAsync = false;
  const def = EXPORT_DEFAULT.exec(code);
  EXPORT_DEFAULT.lastIndex = 0;
  if (def) {
    hasDefault = true;
    defaultAsync = def[1] === "async";
    defaultName = def[3] ?? def[5] ?? def[6];
    if (defaultName && !def[2] && !def[4]) {
      const decl = new RegExp(`\\b(async\\s+)?function\\s*\\*?\\s*${defaultName}\\b`).exec(code);
      if (decl?.[1]) defaultAsync = true;
    }
  }
  if (!hasDefault && /\bexport\s*\{[^}]*\bas\s+default\b[^}]*\}/.test(code)) {
    hasDefault = true;
    const local = /\b([A-Za-z_$][\w$]*)\s+as\s+default\b/.exec(code);
    defaultName = local?.[1];
  }
  const reexportAll: string[] = [];
  for (const match of code.matchAll(EXPORT_ALL)) reexportAll.push(match[1] ?? "");
  return { names, hasDefault, defaultName, defaultAsync, reexportAll };
}

function scanExportsWithOxc(result: OxcParseResult, source: string): ModuleExports {
  const names: string[] = [];
  let hasDefault = false;
  let defaultName: string | undefined;
  let defaultAsync = false;
  const reexportAll: string[] = [];
  for (const record of result.module.staticExports) {
    for (const entry of record.entries) {
      if (entry.exportName.kind === "Name" && entry.exportName.name) {
        if (!names.includes(entry.exportName.name)) names.push(entry.exportName.name);
      } else if (entry.exportName.kind === "Default") {
        hasDefault = true;
        if (entry.localName.kind === "Name" && entry.localName.name) defaultName = entry.localName.name;
      } else if (entry.exportName.kind === "None" && entry.moduleRequest) {
        reexportAll.push(entry.moduleRequest.value);
      }
    }
  }
  for (const stmt of result.program.body) {
    if (stmt.type === "ExportDefaultDeclaration") {
      const decl = stmt.declaration as Record<string, unknown> | undefined;
      if (decl?.type === "FunctionDeclaration" || decl?.type === "TSDeclareFunction") {
        defaultAsync = decl.async === true;
        const id = decl.id as { name?: string } | null;
        if (id?.name) defaultName = id.name;
      } else if (decl?.type === "ClassDeclaration") {
        const id = decl.id as { name?: string } | null;
        if (id?.name) defaultName = id.name;
      } else if (decl?.type === "Identifier") {
        defaultName = decl.name as string;
      } else if (decl?.type === "ArrowFunctionExpression" || decl?.type === "FunctionExpression") {
        defaultAsync = decl.async === true;
      }
    }
  }
  if (defaultName && !defaultAsync) {
    const decl = new RegExp(`\\basync\\s+function\\s*\\*?\\s*${defaultName}\\b`).exec(source);
    if (decl) defaultAsync = true;
  }
  const typeOnly = new Set<string>();
  for (const stmt of result.program.body) {
    if (stmt.type !== "ExportNamedDeclaration") continue;
    const decl = stmt.declaration as Record<string, unknown> | null | undefined;
    if (decl && (decl.type === "TSTypeAliasDeclaration" || decl.type === "TSInterfaceDeclaration")) {
      const id = decl.id as { name?: string } | undefined;
      if (id?.name) typeOnly.add(id.name);
    }
    if (stmt.exportKind === "type") {
      for (const spec of (stmt.specifiers as ReadonlyArray<Record<string, unknown>> | undefined) ?? []) {
        const exported = spec.exported as { name?: string; value?: string } | undefined;
        const name = exported?.name ?? exported?.value;
        if (name) typeOnly.add(name);
      }
    }
  }
  return { names: names.filter((n) => !typeOnly.has(n)), hasDefault, defaultName, defaultAsync, reexportAll };
}

/** Lists the exports of a module (oxc-parser when available, otherwise a light scanner). */
export function scanModuleExports(source: string, file = "module.tsx"): ModuleExports {
  const parsed = parseWithOxc(source, file);
  return parsed ? scanExportsWithOxc(parsed, source) : scanExportsWithRegex(source);
}

// ---------------------------------------------------------------------------
// Segment analysis
// ---------------------------------------------------------------------------

const SEGMENT_KEYS = [
  "dynamic",
  "dynamicParams",
  "revalidate",
  "fetchCache",
  "runtime",
  "preferredRegion",
  "maxDuration",
  "ensureStatic",
  "prefetch",
  "instant",
  "experimental_ppr",
  "metadata",
  "viewport",
] as const;

type SegmentKey = (typeof SEGMENT_KEYS)[number];

const HTTP_METHODS = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"] as const;
const NEXT_IMPORT = /(?:from|import|require\()\s*["']((?:next(?:\/[^"']*)?)|server-only|client-only)["']/g;
const HOOK_CALL = /\b(use[A-Z][\w$]*)\s*(?:<[^>()]*>)?\s*\(/g;

/** Finds the initializer offsets of `export const <name> =` declarations for the segment keys. */
function findInitializers(source: string, parsed: OxcParseResult | undefined): Map<SegmentKey, number> {
  const out = new Map<SegmentKey, number>();
  const wanted = new Set<string>(SEGMENT_KEYS);
  if (parsed) {
    for (const stmt of parsed.program.body) {
      let decl: Record<string, unknown> | undefined;
      if (stmt.type === "ExportNamedDeclaration") decl = stmt.declaration as Record<string, unknown> | undefined;
      else if (stmt.type === "VariableDeclaration") decl = stmt;
      if (!decl || decl.type !== "VariableDeclaration") continue;
      for (const d of (decl.declarations as ReadonlyArray<Record<string, unknown>> | undefined) ?? []) {
        const id = d.id as { type?: string; name?: string } | undefined;
        const init = d.init as (OxcSpan & { type?: string }) | null | undefined;
        if (id?.type !== "Identifier" || !id.name || !wanted.has(id.name) || !init) continue;
        if (stmt.type !== "ExportNamedDeclaration" && !new RegExp(`\\bexport\\s*\\{[^}]*\\b${id.name}\\b[^}]*\\}`).test(source)) continue;
        out.set(id.name as SegmentKey, init.start);
      }
    }
    return out;
  }
  const code = source;
  const pattern = /\bexport\s+(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::\s*[^=]+?)?=(?!=)/g;
  for (const match of code.matchAll(pattern)) {
    const name = match[1] ?? "";
    if (!wanted.has(name)) continue;
    out.set(name as SegmentKey, match.index + match[0].length);
  }
  return out;
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : undefined;
}

/**
 * Analyzes a page/layout/route module: directive, route segment config exports, `generate*`
 * functions, static `metadata`/`viewport` literals, default export, hooks and `next/*` imports.
 */
export function analyzeSegmentModule(source: string, file: string): SegmentConfig {
  const parsed = parseWithOxc(source, file);
  const exports = parsed ? scanExportsWithOxc(parsed, source) : scanExportsWithRegex(source);
  const inits = findInitializers(source, parsed);
  const values = new Map<SegmentKey, unknown>();
  for (const [key, offset] of inits) {
    const evaluated = evaluateLiteralAt(source, offset);
    if (evaluated) values.set(key, evaluated.value);
  }
  const diagnostics: Diagnostic[] = [];
  const names = new Set(exports.names);
  const str = (key: SegmentKey): string | undefined => {
    const v = values.get(key);
    return typeof v === "string" ? v : undefined;
  };
  const bool = (key: SegmentKey): boolean | undefined => {
    const v = values.get(key);
    return typeof v === "boolean" ? v : undefined;
  };
  const dynamic = str("dynamic");
  const runtime = str("runtime");
  const prefetch = str("prefetch");
  const revalidateRaw = values.get("revalidate");
  const revalidate = revalidateRaw === false || typeof revalidateRaw === "number" ? revalidateRaw : undefined;
  const maxDurationRaw = values.get("maxDuration");
  const maxDuration = typeof maxDurationRaw === "number" ? maxDurationRaw : undefined;
  const experimentalPpr = bool("experimental_ppr");
  if (names.has("experimental_ppr")) {
    diagnostics.push({
      severity: "warning",
      code: "segment-config-experimental-ppr-removed",
      message: "`export const experimental_ppr` was removed in Next.js 16; Partial Prerendering is enabled with `cacheComponents: true`",
      file,
      hint: "Delete the export and enable `cacheComponents` in the config",
    });
  }
  if (runtime === "edge") {
    diagnostics.push({
      severity: "warning",
      code: "segment-config-edge-runtime-deprecated",
      message: "`export const runtime = \"edge\"` is deprecated in Next.js 16; the segment runs on Bun",
      file,
      hint: "Remove the export; Bun serves every segment with Web APIs",
    });
  }
  if (names.has("preferredRegion")) {
    diagnostics.push({
      severity: "info",
      code: "segment-config-preferred-region-deprecated",
      message: "`export const preferredRegion` only applies to Vercel Edge; ignored",
      file,
    });
  }
  const hooks: string[] = [];
  for (const match of stripComments(source).matchAll(HOOK_CALL)) {
    const hook = match[1] ?? "";
    if (!hooks.includes(hook)) hooks.push(hook);
  }
  const nextImports: string[] = [];
  if (parsed) {
    for (const imp of parsed.module.staticImports) {
      const spec = imp.moduleRequest.value;
      if ((spec === "next" || spec.startsWith("next/") || spec === "server-only" || spec === "client-only") && !nextImports.includes(spec)) nextImports.push(spec);
    }
  }
  for (const match of source.matchAll(NEXT_IMPORT)) {
    const spec = match[1] ?? "";
    if (!nextImports.includes(spec)) nextImports.push(spec);
  }
  const metadata = asRecord(values.get("metadata")) as Metadata | undefined;
  const viewport = asRecord(values.get("viewport")) as Viewport | undefined;
  const config: SegmentConfig = {
    dynamic: dynamic === "auto" || dynamic === "force-dynamic" || dynamic === "error" || dynamic === "force-static" ? dynamic : undefined,
    dynamicParams: bool("dynamicParams"),
    revalidate,
    fetchCache: str("fetchCache"),
    runtime: runtime === "nodejs" || runtime === "edge" ? runtime : undefined,
    preferredRegion: values.get("preferredRegion"),
    maxDuration,
    ensureStatic: values.get("ensureStatic"),
    prefetch: prefetch === "auto" || prefetch === "partial" || prefetch === "force-disabled" ? prefetch : undefined,
    instant: bool("instant"),
    experimentalPpr,
    hasGenerateStaticParams: names.has("generateStaticParams"),
    hasGenerateMetadata: names.has("generateMetadata"),
    hasGenerateViewport: names.has("generateViewport"),
    hasGenerateImageMetadata: names.has("generateImageMetadata"),
    hasGenerateSitemaps: names.has("generateSitemaps"),
    staticMetadata: metadata,
    staticViewport: viewport,
    directive: detectDirective(source),
    defaultExport: exports.hasDefault ? (exports.defaultName ?? "default") : undefined,
    namedExports: exports.names,
    defaultExportAsync: exports.defaultAsync,
    hooksUsed: hooks,
    nextImports,
    diagnostics,
  };
  return stripUndefined(config);
}

function stripUndefined<T extends object>(value: T): T {
  const out: Record<string, unknown> = {};
  for (const [key, v] of Object.entries(value)) if (v !== undefined) out[key] = v;
  return out as T;
}

/** HTTP methods exported by a route handler, in canonical order (GET HEAD POST PUT PATCH DELETE OPTIONS). */
export function routeHandlerMethods(source: string, file = "route.ts"): string[] {
  const exports = scanModuleExports(source, file);
  const names = new Set(exports.names);
  return HTTP_METHODS.filter((method) => names.has(method));
}

/** Function-level directives of a module (`"use server"` / `"use cache"` inside function bodies), by enclosing function name. */
export function inlineDirectives(source: string): { useServer: string[]; useCache: string[] } {
  const code = stripComments(source);
  const out = { useServer: [] as string[], useCache: [] as string[] };
  const keywords = new Set(["if", "for", "while", "switch", "catch", "else", "do", "try", "return"]);
  for (const match of code.matchAll(/["']use (server|cache)(?::[^"']*)?["']\s*;?/g)) {
    const before = code.slice(0, match.index).trim();
    if (/^(?:["'][^"'\n]*["']\s*;?\s*)*$/.test(before)) continue;
    const window = code.slice(Math.max(0, match.index - 400), match.index);
    const brace = window.lastIndexOf("{");
    const header = brace === -1 ? window : window.slice(0, brace);
    let best: { index: number; name: string } | undefined;
    const consider = (index: number, name: string | undefined): void => {
      if (!name || keywords.has(name)) return;
      if (!best || index > best.index) best = { index, name };
    };
    for (const m of header.matchAll(/function\s*\*?\s*([A-Za-z_$][\w$]*)\s*[(<]/g)) consider(m.index, m[1]);
    for (const m of header.matchAll(/(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*(?:async\b)?/g)) consider(m.index, m[1]);
    for (const m of header.matchAll(/(?:^|[\s,{;])([A-Za-z_$][\w$]*)\s*\([^()]*\)\s*(?::[^{;]+)?\s*$/g)) consider(m.index, m[1]);
    const name = best?.name ?? "<anonymous>";
    (match[1] === "server" ? out.useServer : out.useCache).push(name);
  }
  return out;
}

/** Legacy Pages Router data functions exported by a module (`getServerSideProps`, `getStaticProps`, `getStaticPaths`, `getInitialProps`). */
export function legacyDataFunctions(source: string, file = "page.tsx"): string[] {
  const exports = scanModuleExports(source, file);
  const names = new Set(exports.names);
  const out: string[] = [];
  for (const fn of ["getServerSideProps", "getStaticProps", "getStaticPaths"]) if (names.has(fn)) out.push(fn);
  if (names.has("getInitialProps") || /\.getInitialProps\s*=/.test(source)) out.push("getInitialProps");
  return out;
}
