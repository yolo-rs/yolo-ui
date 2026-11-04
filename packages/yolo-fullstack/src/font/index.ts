// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/font` at build time. A module importing `next/font/google` or `next/font/local` is rewritten: each
// `Inter({ ... })` / `localFont({ ... })` call (whose argument must be a literal, as in Next.js) is resolved
// to a font artifact and replaced by the default import of a virtual module `yolo-font:<id>` exporting
// `{ className, style, variable }`; the generated stylesheet joins the client build.
import { dirname, join } from "node:path";
import { type GoogleFontContext, type GoogleFontOptions, familyFromExport, googleFont } from "./google";
import { type LocalFontOptions, localFont } from "./local";
import { type FontArtifact, type FontResult, fallbackArtifact, fontClassNames, fontFamilyList, fontId, writeArtifact } from "./shared";

export * from "./google";
export * from "./local";
export * from "./shared";

export const FONT_NAMESPACE = "yolo-font";
export const FONT_SPECIFIERS = {
  google: ["next/font/google", "@yolo-rs/fullstack/font/google"],
  local: ["next/font/local", "@yolo-rs/fullstack/font/local"],
};

export interface FontBinding {
  /** Local identifier in the module. */
  local: string;
  kind: "google" | "local";
  /** Export name (`Roboto_Mono`), or `default` for `localFont`. */
  imported: string;
  /** Span of the import statement. */
  start: number;
  end: number;
}

const IMPORT_RE = /import\s+([^;'"]+?)\s+from\s+(["'])([^"']+)\2\s*;?/g;

/** The `next/font` imports of a module. */
export function findFontImports(source: string): FontBinding[] {
  const out: FontBinding[] = [];
  for (const m of source.matchAll(IMPORT_RE)) {
    const specifier = m[3]!;
    const kind = FONT_SPECIFIERS.google.includes(specifier) ? "google" : FONT_SPECIFIERS.local.includes(specifier) ? "local" : undefined;
    if (!kind) continue;
    const clause = m[1]!.trim();
    const start = m.index!;
    const end = start + m[0].length;
    const braces = /\{([^}]*)\}/.exec(clause);
    const defaultName = clause.replace(/\{[^}]*\}/, "").replace(/,/g, "").trim();
    if (defaultName && !defaultName.startsWith("*")) out.push({ local: defaultName, kind, imported: "default", start, end });
    for (const part of (braces?.[1] ?? "").split(",")) {
      const item = part.trim();
      if (!item) continue;
      const [imported, local] = item.split(/\s+as\s+/).map(s => s.trim());
      out.push({ local: local ?? imported!, kind, imported: imported!, start, end });
    }
  }
  return out;
}

/** Index after the balanced closing parenthesis starting at `open` (a `(`), string/comment aware. */
export function balancedEnd(source: string, open: number): number {
  let depth = 0;
  for (let i = open; i < source.length; i++) {
    const c = source[i]!;
    if (c === '"' || c === "'" || c === "`") {
      for (i++; i < source.length && source[i] !== c; i++) if (source[i] === "\\") i++;
      continue;
    }
    if (c === "/" && source[i + 1] === "/") {
      i = source.indexOf("\n", i);
      if (i === -1) break;
      continue;
    }
    if (c === "/" && source[i + 1] === "*") {
      i = source.indexOf("*/", i + 2) + 1;
      continue;
    }
    if (c === "(" || c === "{" || c === "[") depth++;
    else if (c === ")" || c === "}" || c === "]") {
      depth--;
      if (depth === 0) return i + 1;
    }
  }
  throw new Error("@yolo-rs/fullstack: unbalanced font call");
}

export interface FontCall {
  binding: FontBinding;
  start: number;
  end: number;
  options: Record<string, unknown>;
}

/** Evaluate the literal argument of a font call (Next.js requires a literal too). */
export function evaluateOptions(text: string, where: string): Record<string, unknown> {
  const trimmed = text.trim();
  if (!trimmed) return {};
  if (!/^[\s\S]*\{[\s\S]*\}[\s\S]*$/.test(trimmed) || /\b(require|import|process|globalThis|fetch)\b/.test(trimmed))
    throw new Error(`@yolo-rs/fullstack: the options of a font call must be an object literal (${where})`);
  try {
    return new Function(`"use strict"; return (${trimmed});`)() as Record<string, unknown>;
  } catch (error) {
    throw new Error(`@yolo-rs/fullstack: cannot evaluate the font options in ${where}: ${String(error)}`);
  }
}

/** The calls of the font bindings in a module. */
export function findFontCalls(source: string, bindings: FontBinding[], where: string): FontCall[] {
  const calls: FontCall[] = [];
  for (const binding of bindings) {
    const re = new RegExp(`(^|[^\\w$.])${binding.local.replace(/\$/g, "\\$")}\\s*\\(`, "g");
    for (const m of source.matchAll(re)) {
      const start = m.index! + m[1]!.length;
      if (start >= binding.start && start < binding.end) continue;
      const open = start + m[0].length - m[1]!.length - 1;
      const end = balancedEnd(source, open);
      calls.push({ binding, start, end, options: evaluateOptions(source.slice(open + 1, end - 1), where) });
    }
  }
  return calls.sort((a, b) => a.start - b.start);
}

/** Index where imports may be inserted: after the directive prologue and shebang. */
export function prologueEnd(source: string): number {
  let i = 0;
  const n = source.length;
  if (source.startsWith("#!")) i = source.indexOf("\n") + 1 || n;
  for (;;) {
    while (i < n && /\s/.test(source[i]!)) i++;
    if (source.startsWith("//", i)) {
      const end = source.indexOf("\n", i);
      i = end === -1 ? n : end + 1;
      continue;
    }
    if (source.startsWith("/*", i)) {
      const end = source.indexOf("*/", i + 2);
      i = end === -1 ? n : end + 2;
      continue;
    }
    const q = source[i];
    if (q !== '"' && q !== "'") return i;
    const end = source.indexOf(q, i + 1);
    if (end === -1) return i;
    i = end + 1;
    while (i < n && (source[i] === " " || source[i] === "\t")) i++;
    if (source[i] === ";") i++;
    if (source[i] === "\n") i++;
  }
}

export interface FontRewriteContext {
  cacheDir: string;
  /** `dev` falls back to the system stack when Google is unreachable; `build` fails unless `YOLO_FONT_FALLBACK=1`. */
  mode: "dev" | "build";
  offline?: boolean;
  fetch?: typeof fetch;
  env?: Record<string, string | undefined>;
  /** Artifacts by id, shared by the layers of one build. */
  registry: Map<string, FontArtifact>;
  warn?: (message: string) => void;
}

export interface FontRewrite {
  code: string;
  artifacts: FontArtifact[];
}

async function resolveCall(call: FontCall, path: string, ctx: FontRewriteContext): Promise<FontArtifact> {
  if (call.binding.kind === "local") return localFont(call.options as unknown as LocalFontOptions, { cacheDir: ctx.cacheDir, importerDir: dirname(path) });
  const options = call.options as GoogleFontOptions;
  const google: GoogleFontContext = { cacheDir: ctx.cacheDir, fetch: ctx.fetch, offline: ctx.offline };
  try {
    return await googleFont(call.binding.imported, options, google);
  } catch (error) {
    const allow = ctx.mode === "dev" || (ctx.env ?? process.env).YOLO_FONT_FALLBACK === "1";
    if (!allow) throw error;
    const family = familyFromExport(call.binding.imported);
    const id = fontId("google-fallback", family, options);
    const names = fontClassNames(id, options.variable);
    const artifact = fallbackArtifact({
      id,
      kind: "google",
      family,
      cssFile: join(ctx.cacheDir, id, "font.css"),
      className: names.className,
      variableClassName: names.variableClassName,
      variable: options.variable,
      style: { fontFamily: fontFamilyList(family, options.fallback ?? ["system-ui", "sans-serif"]) },
    });
    await writeArtifact(artifact, "", options.display ?? "swap");
    (ctx.warn ?? console.warn)(`@yolo-rs/fullstack: ${family} could not be downloaded (${error instanceof Error ? error.message : String(error)}); using the fallback stack`);
    return artifact;
  }
}

/** Rewrite the font calls of a module; `undefined` when it has none. */
export async function rewriteFonts(source: string, path: string, ctx: FontRewriteContext): Promise<FontRewrite | undefined> {
  if (!source.includes("next/font") && !source.includes("@yolo-rs/fullstack/font")) return undefined;
  const bindings = findFontImports(source);
  if (!bindings.length) return undefined;
  const calls = findFontCalls(source, bindings, path);
  const artifacts: FontArtifact[] = [];
  const imports: string[] = [];
  let code = "";
  let cursor = 0;
  // Drop the import statements and replace the calls, left to right.
  const edits: { start: number; end: number; text: string }[] = [];
  const seen = new Set<string>();
  for (const b of bindings) if (!seen.has(`${b.start}`)) {
    seen.add(`${b.start}`);
    edits.push({ start: b.start, end: b.end, text: "" });
  }
  for (const [i, call] of calls.entries()) {
    const artifact = await resolveCall(call, path, ctx);
    ctx.registry.set(artifact.id, artifact);
    artifacts.push(artifact);
    imports.push(`import __yolo_font_${i} from ${JSON.stringify(`${FONT_NAMESPACE}:${artifact.id}`)};`);
    edits.push({ start: call.start, end: call.end, text: `__yolo_font_${i}` });
  }
  edits.sort((a, b) => a.start - b.start);
  for (const edit of edits) {
    code += source.slice(cursor, edit.start) + edit.text;
    cursor = edit.end;
  }
  code += source.slice(cursor);
  const at = prologueEnd(code);
  code = `${code.slice(0, at)}${imports.join("\n")}\n${code.slice(at)}`;
  return { code, artifacts };
}

/** The module exported for a font in one layer (the client layer imports the stylesheet). */
export function fontVirtualModule(artifact: FontArtifact, layer: "rsc" | "ssr" | "client"): string {
  const result: FontResult = {
    className: [artifact.className, artifact.variableClassName].filter(Boolean).join(" "),
    style: artifact.style,
    ...(artifact.variableClassName ? { variable: artifact.variableClassName } : {}),
  };
  const lines = [`export default ${JSON.stringify(result)};`];
  if (layer === "client") lines.unshift(`import ${JSON.stringify(artifact.cssFile.replaceAll("\\", "/"))};`);
  return lines.join("\n");
}

/** `<link rel="preload" as="font">` hrefs of the registry, relative to the client asset prefix. */
export function fontPreloads(registry: Map<string, FontArtifact>): { id: string; files: string[] }[] {
  return [...registry.values()].filter(a => a.preload.length).map(a => ({ id: a.id, files: a.preload }));
}
