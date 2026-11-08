// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/build: production build in three Bun.build passes (rsc with the `react-server`
// condition, ssr, client), Next.js-shaped manifests under the dist dir, optional static export and
// standalone executable. The generated entries carry a small react-server-dom-webpack runtime
// (Flight render, actions, HTML streaming) used by the `webpack-worker` engine of rsc-engine.ts.

import type { BunPlugin } from "bun";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

import { publicEnvDefine } from "./config.ts";
import { yoloNextPlugin } from "./plugin.ts";
import { appPathsManifestJson, pagesManifestJson, routesManifestJson, scanAppDir } from "./router.ts";
import { detectDirective } from "./segments.ts";
import { yoloBundlerPlugins } from "./turbopack.ts";
import type { Diagnostic, ResolvedYoloConfig, RouteManifest, RouteSegment } from "./types.ts";

/** Options of `buildYoloNext`. */
export interface BuildOptions {
  readonly root: string;
  readonly config: ResolvedYoloConfig;
  /** Static export after the build (`out/`). */
  readonly export?: boolean;
  /** Output directory (default `<root>/<distDir>`). */
  readonly outDir?: string;
  /** Static export directory (default `<root>/out`). */
  readonly exportDir?: string;
  /** Compile a standalone executable (implied by `output: "standalone"`). */
  readonly standalone?: boolean;
  /** Pre-scanned manifest. */
  readonly manifest?: RouteManifest;
  /** Development build (no minification, inline source maps). */
  readonly dev?: boolean;
  readonly log?: (message: string) => void;
}

/** Result of `buildYoloNext`. */
export interface BuildResult {
  readonly success: boolean;
  readonly outputs: string[];
  readonly buildId: string;
  readonly manifests: string[];
  readonly warnings: Diagnostic[];
  readonly durationMs: number;
  readonly dist: string;
  readonly clientModules: string[];
  readonly serverActions: Record<string, { module: string; exportName: string }>;
  readonly rscEntry?: string;
  readonly ssrEntry?: string;
  readonly clientEntry?: string;
  readonly rootMainFiles: string[];
  readonly exported?: string[];
  readonly standaloneBinary?: string;
}

const toPosix = (path: string): string => path.replace(/\\/g, "/");
const SRC_DIR = import.meta.dir;
const SKIP_DIRS = new Set(["node_modules", ".git", ".next", ".yolo", ".turbo", "dist", "build", "out", "coverage", "public"]);

function ensureDir(path: string): void {
  mkdirSync(path, { recursive: true });
}

function writeJson(path: string, value: unknown): void {
  ensureDir(dirname(path));
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

/** Every source module referenced by the manifest (pages, layouts, templates, boundaries, metadata generators). */
export function collectRouteModules(manifest: RouteManifest): string[] {
  const files = new Set<string>();
  const add = (file: string | undefined): void => {
    if (file) files.add(file);
  };
  const walk = (segment: RouteSegment | undefined): void => {
    if (!segment) return;
    const f = segment.files;
    for (const file of [f.page, f.layout, f.template, f.loading, f.error, f.globalError, f.notFound, f.forbidden, f.unauthorized, f.default, f.route]) add(file);
    for (const meta of f.metadata) if (meta.generated) add(meta.path);
    for (const child of segment.children) walk(child);
  };
  walk(manifest.tree);
  for (const entry of manifest.entries) {
    add(entry.file);
    entry.layouts.forEach(add);
    entry.templates.forEach(add);
    add(entry.loading);
    add(entry.error);
    add(entry.notFound);
  }
  add(manifest.rootNotFound);
  add(manifest.globalError);
  return [...files].sort();
}

/** `"use server"` modules of the project sources (actions imported only by client modules too). */
export function findServerActionModules(root: string, appDir: string): string[] {
  const found: string[] = [];
  const visit = (dir: string, depth: number): void => {
    let dirents;
    try {
      dirents = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const dirent of dirents) {
      if (dirent.isDirectory()) {
        if (SKIP_DIRS.has(dirent.name) || dirent.name.startsWith(".")) continue;
        if (depth < 12) visit(join(dir, dirent.name), depth + 1);
        continue;
      }
      if (!dirent.isFile() || !/\.(?:tsx|ts|jsx|js|mjs|mts)$/.test(dirent.name)) continue;
      const abs = join(dir, dirent.name);
      let source: string;
      try {
        source = readFileSync(abs, "utf8");
      } catch {
        continue;
      }
      if (source.includes("use server") && detectDirective(source) === "use-server") found.push(toPosix(relative(root, abs)));
    }
  };
  const roots = new Set<string>([appDir, "src", "actions", "lib", "components", "server", "features", "modules"]);
  for (const dir of roots) if (existsSync(join(root, dir))) visit(join(root, dir), 0);
  return [...new Set(found)].sort();
}

function importSpecifier(fromDir: string, root: string, file: string): string {
  const rel = toPosix(relative(fromDir, join(root, file)));
  return rel.startsWith(".") ? rel : `./${rel}`;
}

const RSC_RUNTIME = String.raw`
const registryModules = {};
const actionRegistry = new Map();
const SERVER_REFERENCE = Symbol.for("react.server.reference");
for (const [moduleId, mod] of Object.entries(modules)) {
  for (const [exportName, value] of Object.entries(mod)) {
    if (value && typeof value === "function" && value.$$typeof === SERVER_REFERENCE && typeof value.$$id === "string") {
      const fullId = value.$$id;
      const baseId = fullId.includes("#") ? fullId.slice(0, fullId.indexOf("#")) : fullId;
      actionRegistry.set(fullId, value);
      actionRegistry.set(baseId, value);
      registryModules[baseId] = Object.assign(registryModules[baseId] ?? {}, { [exportName]: value });
      void moduleId;
    }
  }
}
const g = globalThis;
const previousRequire = g.__webpack_require__;
g.__webpack_require__ = (id) => (Object.hasOwn(registryModules, id) ? registryModules[id] : typeof previousRequire === "function" ? previousRequire(id) : undefined);
g.__webpack_chunk_load__ ??= () => Promise.resolve();
const serverManifest = {};
for (const [id, fn] of actionRegistry) {
  if (!id.includes("#")) continue;
  const baseId = id.slice(0, id.indexOf("#"));
  serverManifest[id] = { id: baseId, chunks: [], name: id.slice(id.indexOf("#") + 1) };
  void fn;
}
let clientManifestPromise;
/** Client reference manifest written by the client pass (dist/client-reference-manifest.json). */
export function loadClientManifest() {
  clientManifestPromise ??= (async () => {
    try {
      const data = await Bun.file(new URL("../../client-reference-manifest.json", import.meta.url)).json();
      return data && typeof data === "object" && data.clientModules ? data.clientModules : data ?? {};
    } catch {
      return {};
    }
  })();
  return clientManifestPromise;
}
function searchParamsObject(search) {
  const out = {};
  for (const [key, value] of new URLSearchParams(search ?? "")) {
    const prev = out[key];
    out[key] = prev === undefined ? value : Array.isArray(prev) ? [...prev, value] : [prev, value];
  }
  return out;
}
function paramsObject(params, entry) {
  const out = { ...(params ?? {}) };
  for (const p of entry?.params ?? []) {
    const value = out[p.name];
    if (p.catchAll) out[p.name] = typeof value === "string" ? value.split("/").filter(Boolean) : value;
  }
  return out;
}
function dirOf(file) {
  const index = file.lastIndexOf("/");
  return index === -1 ? "" : file.slice(0, index);
}
function onError(error) {
  const digest = error && typeof error === "object" ? error.digest : undefined;
  if (typeof digest === "string") return digest;
  console.error(error);
  return undefined;
}
function headNodes(metadata, viewport) {
  return headElements(metadata, viewport).map((el, index) => createElement(el.tag, { key: "yolo-head-" + index, ...el.attrs }, el.children));
}
/** Builds the React element tree of a matched entry (layouts, templates, loading boundary, head tags). */
export async function createElementTree(input) {
  const entry = input.entry;
  const params = Promise.resolve(paramsObject(input.params, entry));
  const searchParams = Promise.resolve(searchParamsObject(input.search));
  const props = { params, searchParams };
  const chain = [];
  let viewport;
  const collect = async (mod) => {
    if (!mod) return;
    if (mod.metadata && typeof mod.metadata === "object") chain.push(mod.metadata);
    if (typeof mod.generateMetadata === "function") chain.push((await mod.generateMetadata(props, Promise.resolve(resolveMetadataChain(chain)))) ?? {});
    if (mod.viewport && typeof mod.viewport === "object") viewport = mod.viewport;
    if (typeof mod.generateViewport === "function") viewport = (await mod.generateViewport(props, Promise.resolve(viewport ?? {}))) ?? viewport;
  };
  const layouts = entry ? entry.layouts : (rootLayout ? [rootLayout] : []);
  for (const layout of layouts) await collect(modules[layout]);
  let node;
  if (entry && modules[entry.file]) {
    const page = modules[entry.file];
    await collect(page);
    if (typeof page.default !== "function") throw new Error("page module has no default export: " + entry.file);
    node = createElement(page.default, props);
    if (entry.loading && modules[entry.loading] && typeof modules[entry.loading].default === "function") {
      node = createElement(Suspense, { fallback: createElement(modules[entry.loading].default) }, node);
    }
  } else {
    const notFound = rootNotFound && modules[rootNotFound] ? modules[rootNotFound].default : undefined;
    node = typeof notFound === "function" ? createElement(notFound) : createElement("main", null, createElement("h1", null, "404"), createElement("p", null, "This page could not be found."));
  }
  const head = headNodes(resolveMetadataChain(chain), viewport);
  node = createElement(Fragment, null, ...head, node);
  const templates = entry ? entry.templates : [];
  for (let i = layouts.length - 1; i >= 0; i -= 1) {
    const layoutFile = layouts[i];
    const layout = modules[layoutFile];
    if (!layout || typeof layout.default !== "function") continue;
    for (const template of templates) {
      if (dirOf(template) !== dirOf(layoutFile) || !modules[template] || typeof modules[template].default !== "function") continue;
      node = createElement(modules[template].default, { key: input.pathname }, node);
    }
    const slots = {};
    for (const slot of entry?.parallelSlots ?? []) slots[slot] = null;
    node = createElement(layout.default, { params, children: node, ...slots });
  }
  return node;
}
/** The model streamed as the Flight payload root. */
export async function createModel(input, extra = {}) {
  const root = await createElementTree(input);
  return { root, pathname: input.pathname, search: input.search ?? "", params: input.params ?? {}, buildId, status: input.status ?? 200, ...extra };
}
/** Renders the Flight payload of a matched entry. */
export async function renderFlight(input, options = {}) {
  const model = await createModel(input, options.extra);
  return renderToReadableStream(model, await loadClientManifest(), { onError, temporaryReferences: options.temporaryReferences, signal: options.signal });
}
/** Server action by id (next-action header or encoded form field). */
export function findAction(id) {
  return actionRegistry.get(id);
}
/** Handles a Server Action POST: decodes the reply, runs the action, streams the returnValue and the re-rendered root. */
export async function handleAction(request, input) {
  const actionId = input.actionId ?? request.headers.get("next-action");
  const contentType = request.headers.get("content-type") ?? "";
  const temporaryReferences = createTemporaryReferenceSet();
  const extra = {};
  if (actionId) {
    const action = actionRegistry.get(actionId);
    if (!action) return new Response("Unknown server action " + actionId, { status: 404 });
    const body = contentType.startsWith("multipart/form-data") ? await request.formData() : await request.text();
    const args = await decodeReply(body, serverManifest, { temporaryReferences });
    try {
      extra.returnValue = { ok: true, data: await action(...args) };
    } catch (error) {
      const digest = error && typeof error === "object" && typeof error.digest === "string" ? error.digest : undefined;
      if (digest && digest.startsWith("NEXT_REDIRECT")) extra.redirect = digest.split(";")[2];
      else extra.returnValue = { ok: false, data: error instanceof Error ? error.message : String(error) };
    }
  } else if (contentType.startsWith("multipart/form-data") || contentType.startsWith("application/x-www-form-urlencoded")) {
    const form = await request.formData();
    const action = await decodeAction(form, serverManifest);
    if (action) {
      const result = await action();
      extra.formState = await decodeFormState(result, form, serverManifest);
    }
  }
  const stream = await renderFlight(input, { extra, temporaryReferences });
  return new Response(stream, { status: 200, headers: { "content-type": "text/x-component", vary: "RSC, Next-Action" } });
}
`;

const SSR_RUNTIME = String.raw`
const g = globalThis;
const previousRequire = g.__webpack_require__;
g.__webpack_require__ = (id) => (Object.hasOwn(clientModules, id) ? clientModules[id] : typeof previousRequire === "function" ? previousRequire(id) : undefined);
g.__webpack_chunk_load__ ??= () => Promise.resolve();
const moduleMap = {};
for (const id of Object.keys(clientModules)) moduleMap[id] = { "*": { id, chunks: [], name: "*" } };
const serverConsumerManifest = { moduleMap, moduleLoading: null, serverModuleMap: null };
const encoder = new TextEncoder();
const TAIL = "</body></html>";
function flightScript(text, nonce) {
  const json = JSON.stringify(text).replace(/</g, "\\u003c").replace(new RegExp(String.fromCharCode(0x2028), "g"), "\\u2028").replace(new RegExp(String.fromCharCode(0x2029), "g"), "\\u2029");
  const attr = nonce ? ' nonce="' + nonce.replace(/"/g, "&quot;") + '"' : "";
  return "<script" + attr + ">(self.__yolo_f||(self.__yolo_f=[])).push(" + json + ");self.__yolo_f_done=true</script>";
}
/** Holds back a trailing "</body></html>" and appends the Flight payload script before it. */
function appendFlight(html, flightText, nonce) {
  const decoder = new TextDecoder();
  let carry = "";
  return html.pipeThrough(new TransformStream({
    transform(chunk, controller) {
      const text = carry + decoder.decode(chunk, { stream: true });
      const keep = Math.min(text.length, TAIL.length + 64);
      controller.enqueue(encoder.encode(text.slice(0, text.length - keep)));
      carry = text.slice(text.length - keep);
    },
    async flush(controller) {
      const rest = carry + decoder.decode();
      const index = rest.lastIndexOf(TAIL);
      const script = flightScript(await flightText, nonce);
      if (index !== -1) controller.enqueue(encoder.encode(rest.slice(0, index) + script + rest.slice(index)));
      else controller.enqueue(encoder.encode(rest + script));
    },
  }));
}
function onError(error) {
  const digest = error && typeof error === "object" ? error.digest : undefined;
  if (typeof digest === "string") return digest;
  console.error(error);
  return undefined;
}
/** Renders the HTML document of a Flight stream (React SSR + inline payload for hydration). */
export async function renderHtml(flight, options = {}) {
  const [forReact, forInline] = flight.tee();
  const flightText = new Response(forInline).text();
  const payload = createFromReadableStream(forReact, { serverConsumerManifest });
  const stylesheets = options.stylesheets ?? [];
  function Root() {
    const model = use(payload);
    const root = model && typeof model === "object" && "root" in model ? model.root : model;
    const links = stylesheets.map((href) => createElement("link", { key: href, rel: "stylesheet", href, precedence: "yolo" }));
    return createElement(Fragment, null, ...links, root);
  }
  const stream = await renderToReadableStream(createElement(Root), {
    bootstrapModules: options.bootstrapModules,
    bootstrapScriptContent: options.bootstrapScriptContent,
    formState: options.formState,
    nonce: options.nonce,
    signal: options.signal,
    onError,
  });
  if (options.waitForAll) await stream.allReady;
  return appendFlight(stream, flightText, options.nonce);
}
`;

const CLIENT_RUNTIME = String.raw`
const g = globalThis;
g.__yolo_client_modules__ = clientModules;
const previousRequire = g.__webpack_require__;
g.__webpack_require__ = (id) => (Object.hasOwn(clientModules, id) ? clientModules[id] : typeof previousRequire === "function" ? previousRequire(id) : undefined);
g.__webpack_chunk_load__ = () => Promise.resolve();
let setPayload;
let currentPayload;
function streamFromText(text) {
  return new Response(text).body;
}
/** Calls a server action over POST (next-action header) and swaps in the re-rendered tree. */
export async function callServer(id, args) {
  const body = await encodeReply(args);
  const response = await fetch(location.href, { method: "POST", headers: { accept: "text/x-component", "next-action": id }, body });
  const model = await createFromReadableStream(response.body, { callServer });
  if (model && typeof model === "object" && "root" in model && setPayload) setPayload(Promise.resolve(model));
  const returned = model && typeof model === "object" ? model.returnValue : undefined;
  if (returned && typeof returned === "object" && "ok" in returned) {
    if (returned.ok) return returned.data;
    throw new Error(typeof returned.data === "string" ? returned.data : "server action failed");
  }
  return returned;
}
/** Fetches the Flight payload of a URL and renders it (client navigation). */
export async function navigate(url, options = {}) {
  const response = await fetch(url, { headers: { accept: "text/x-component", rsc: "1" } });
  const model = createFromReadableStream(response.body, { callServer });
  if (!options.replace) history.pushState(null, "", url);
  else history.replaceState(null, "", url);
  if (setPayload) startTransition(() => setPayload(model));
  return model;
}
g.__yolo_router__ = { navigate, refresh: () => navigate(location.href, { replace: true }), callServer };
addEventListener("popstate", () => { void navigate(location.href, { replace: true }); });
function Root({ initial }) {
  const [payload, set] = useState(initial);
  setPayload = set;
  currentPayload = payload;
  const model = use(payload);
  return model && typeof model === "object" && "root" in model ? model.root : model;
}
/** Hydrates the document from the inline Flight payload written by the server. */
export function hydrate() {
  const chunks = Array.isArray(g.__yolo_f) ? g.__yolo_f : [];
  const text = chunks.join("");
  if (!text) return undefined;
  const initial = createFromReadableStream(streamFromText(text), { callServer });
  const root = createElement(Root, { initial });
  startTransition(() => { hydrateRoot(document, strictMode ? createElement(StrictMode, null, root) : root); });
  return initial;
}
if (typeof document !== "undefined" && typeof g.__yolo_hydrate__ === "function") g.__yolo_hydrate__({ hydrate, createElement, clientModules, callServer, navigate });
else if (typeof document !== "undefined") hydrate();
void currentPayload;
`;

function generateRscEntry(entryDir: string, root: string, manifest: RouteManifest, files: ReadonlyArray<string>, buildId: string): string {
  const lines = [
    "// Generated by @yolo-rs/next build (rsc layer, react-server condition). Do not edit.",
    'import { createElement, Fragment, Suspense } from "react";',
    'import { renderToReadableStream, decodeReply, decodeAction, decodeFormState, createTemporaryReferenceSet } from "react-server-dom-webpack/server.edge";',
    `import { headElements, resolveMetadataChain } from ${JSON.stringify(toPosix(join(SRC_DIR, "metadata.ts")))};`,
    ...files.map((file, i) => `import * as m${i} from ${JSON.stringify(importSpecifier(entryDir, root, file))};`),
    "",
    "/** Route modules by root-relative id. */",
    "export const modules = {",
    ...files.map((file, i) => `  ${JSON.stringify(file)}: m${i},`),
    "};",
    `export const buildId = ${JSON.stringify(buildId)};`,
    `export const rootLayout = ${JSON.stringify(manifest.rootLayout ?? null)};`,
    `export const rootNotFound = ${JSON.stringify(manifest.rootNotFound ?? null)};`,
    RSC_RUNTIME,
  ];
  return lines.join("\n");
}

function generateSsrEntry(entryDir: string, root: string, clientModules: ReadonlyArray<string>): string {
  const lines = [
    "// Generated by @yolo-rs/next build (ssr layer). Do not edit.",
    'import { createElement, Fragment, use } from "react";',
    'import { renderToReadableStream } from "react-dom/server";',
    'import { createFromReadableStream } from "react-server-dom-webpack/client.edge";',
    ...clientModules.map((file, i) => `import * as c${i} from ${JSON.stringify(importSpecifier(entryDir, root, file))};`),
    "",
    "/** Client modules by root-relative id (the `__webpack_require__` table of the SSR realm). */",
    "export const clientModules = {",
    ...clientModules.map((file, i) => `  ${JSON.stringify(file)}: c${i},`),
    "};",
    SSR_RUNTIME,
  ];
  return lines.join("\n");
}

function generateClientEntry(entryDir: string, root: string, clientModules: ReadonlyArray<string>, strictMode: boolean): string {
  const lines = [
    "// Generated by @yolo-rs/next build (browser runtime). Do not edit.",
    'import { createElement, startTransition, StrictMode, use, useState } from "react";',
    'import { hydrateRoot } from "react-dom/client";',
    'import { createFromReadableStream, encodeReply } from "react-server-dom-webpack/client";',
    ...clientModules.map((file, i) => `import * as c${i} from ${JSON.stringify(importSpecifier(entryDir, root, file))};`),
    "",
    "/** Client modules by root-relative id (the `__webpack_require__` table of the browser). */",
    "export const clientModules = {",
    ...clientModules.map((file, i) => `  ${JSON.stringify(file)}: c${i},`),
    "};",
    `const strictMode = ${strictMode ? "true" : "false"};`,
    CLIENT_RUNTIME,
  ];
  return lines.join("\n");
}

function logsToDiagnostics(logs: ReadonlyArray<unknown>, pass: string): Diagnostic[] {
  return logs.map((log) => {
    const message = typeof log === "object" && log !== null && "message" in log ? String((log as { message: unknown }).message) : String(log);
    const level = typeof log === "object" && log !== null && "level" in log ? String((log as { level: unknown }).level) : "error";
    return { severity: level === "warning" ? "warning" : level === "info" || level === "verbose" || level === "debug" ? "info" : "error", code: `build-${pass}`, message };
  });
}

function randomHex(bytes: number): string {
  const buffer = new Uint8Array(bytes);
  crypto.getRandomValues(buffer);
  return Array.from(buffer, (b) => b.toString(16).padStart(2, "0")).join("");
}

function copyDir(from: string, to: string): void {
  if (!existsSync(from)) return;
  cpSync(from, to, { recursive: true, force: true });
}

function compileTarget(): string {
  const platform = process.platform === "win32" ? "windows" : process.platform;
  return `bun-${platform}-${process.arch}`;
}

interface MetafileLike {
  outputs?: Record<string, { inputs?: Record<string, unknown>; entryPoint?: string }>;
}

function chunksForModule(metafile: MetafileLike | undefined, outDir: string, staticDir: string, id: string): string[] {
  if (!metafile?.outputs) return [];
  const chunks: string[] = [];
  for (const [output, info] of Object.entries(metafile.outputs)) {
    const inputs = Object.keys(info.inputs ?? {}).map(toPosix);
    if (!inputs.some((input) => input === id || input.endsWith(`/${id}`))) continue;
    const abs = resolve(output);
    const rel = toPosix(relative(staticDir, existsSync(abs) ? abs : join(outDir, output)));
    if (!rel.startsWith("..")) chunks.push(rel);
  }
  return chunks;
}

/**
 * Builds a Yolo Next project: rsc pass (react-server condition, collects client modules and server
 * actions), ssr pass (client modules + HTML runtime), client pass (browser runtime, code splitting),
 * manifests (`BUILD_ID`, build/app-build/client-reference/server-reference/routes/app-paths/prerender),
 * `public/` copy, optional static export and standalone executable. Never throws for a missing
 * react-server-dom-webpack: the rsc pass is skipped with a warning.
 */
export async function buildYoloNext(options: BuildOptions): Promise<BuildResult> {
  const started = performance.now();
  const root = resolve(options.root);
  const { config } = options;
  const log = options.log ?? (() => {});
  const warnings: Diagnostic[] = [];
  const manifest = options.manifest ?? scanAppDir(root, { appDir: config.appDir, pagesDir: config.pagesDir, pageExtensions: config.pageExtensions });
  warnings.push(...manifest.diagnostics);
  const dist = resolve(options.outDir ?? join(root, config.distDir));
  const entryDir = join(root, config.distDir);
  ensureDir(entryDir);
  ensureDir(dist);
  for (const sub of ["server/rsc", "server/ssr", "static/chunks"]) ensureDir(join(dist, sub));
  const buildId = config.cdn.deploymentId ?? randomHex(8);
  const outputs: string[] = [];
  const manifests: string[] = [];
  const clientModules = new Set<string>();
  const serverActions: Record<string, { module: string; exportName: string }> = {};
  const define: Record<string, string> = {
    "process.env.NODE_ENV": JSON.stringify(options.dev ? "development" : "production"),
    ...config.bundler.define,
    ...publicEnvDefine(config.publicEnvPrefixes),
  };
  const bundlerPlugins = yoloBundlerPlugins({ ...config.bundler, root: config.bundler.root ?? root });
  const routeModules = collectRouteModules(manifest);
  const actionModules = findServerActionModules(root, config.appDir).filter((file) => !routeModules.includes(file));
  const rscModules = [...routeModules, ...actionModules];
  let success = true;

  // Pass 1: rsc.
  const rscEntry = join(entryDir, "entry-rsc.ts");
  writeFileSync(rscEntry, generateRscEntry(entryDir, root, manifest, rscModules, buildId));
  let rscAvailable = true;
  try {
    Bun.resolveSync("react-server-dom-webpack/server.edge", root);
  } catch {
    try {
      Bun.resolveSync("react-server-dom-webpack/server.edge", SRC_DIR);
    } catch {
      rscAvailable = false;
    }
  }
  let rscOutput: string | undefined;
  if (!rscAvailable) {
    warnings.push({ severity: "warning", code: "build-rsc-skipped", message: "react-server-dom-webpack is not installed; the rsc pass was skipped (pages render through the inline engine)", hint: "bun add react-server-dom-webpack@19.3.1" });
  } else {
    try {
      const onServerAction = (id: string, exportName: string): void => {
        serverActions[`${id}#${exportName}`] = { module: id, exportName };
      };
      const result = await Bun.build({
        entrypoints: [rscEntry],
        target: "bun",
        conditions: ["react-server"],
        outdir: join(dist, "server", "rsc"),
        naming: "[dir]/[name].[ext]",
        splitting: false,
        format: "esm",
        plugins: [yoloNextPlugin({ root, config, env: "rsc", onClientModule: (id) => clientModules.add(id), onServerAction }), ...bundlerPlugins],
        define,
        external: [...config.serverExternalPackages],
        sourcemap: "external",
        minify: false,
        packages: "bundle",
        loader: config.bundler.loaders,
        throw: false,
      });
      warnings.push(...logsToDiagnostics(result.logs, "rsc").filter((d) => d.severity !== "error" || !result.success));
      if (!result.success) {
        success = false;
        warnings.push({ severity: "error", code: "build-rsc-failed", message: "the rsc pass failed; see the build logs" });
      } else {
        for (const artifact of result.outputs) outputs.push(artifact.path);
        rscOutput = result.outputs.find((a) => a.kind === "entry-point")?.path;
      }
    } catch (error) {
      success = false;
      warnings.push({ severity: "error", code: "build-rsc-failed", message: error instanceof Error ? error.message : String(error) });
    }
  }
  log(`rsc pass: ${rscOutput ? "ok" : "skipped"} (${clientModules.size} client modules, ${Object.keys(serverActions).length} actions)`);

  // Pass 2: ssr.
  const clientList = [...clientModules].sort();
  const ssrEntry = join(entryDir, "entry-ssr.ts");
  writeFileSync(ssrEntry, generateSsrEntry(entryDir, root, clientList));
  let ssrOutput: string | undefined;
  try {
    const result = await Bun.build({
      entrypoints: [ssrEntry],
      target: "bun",
      outdir: join(dist, "server", "ssr"),
      naming: "[dir]/[name].[ext]",
      splitting: false,
      format: "esm",
      plugins: [yoloNextPlugin({ root, config, env: "server", onServerAction: (id, exportName) => { serverActions[`${id}#${exportName}`] = { module: id, exportName }; } }), ...bundlerPlugins],
      define,
      external: [...config.serverExternalPackages],
      sourcemap: "external",
      minify: false,
      packages: "bundle",
      loader: config.bundler.loaders,
      throw: false,
    });
    warnings.push(...logsToDiagnostics(result.logs, "ssr").filter((d) => d.severity !== "error" || !result.success));
    if (!result.success) {
      success = false;
      warnings.push({ severity: "error", code: "build-ssr-failed", message: "the ssr pass failed; see the build logs" });
    } else {
      for (const artifact of result.outputs) outputs.push(artifact.path);
      ssrOutput = result.outputs.find((a) => a.kind === "entry-point")?.path;
    }
  } catch (error) {
    success = false;
    warnings.push({ severity: "error", code: "build-ssr-failed", message: error instanceof Error ? error.message : String(error) });
  }

  // Pass 3: client.
  const clientEntry = join(entryDir, "entry-client.tsx");
  writeFileSync(clientEntry, generateClientEntry(entryDir, root, clientList, config.react.strictMode));
  const staticDir = join(dist, "static");
  const rootMainFiles: string[] = [];
  let clientOutput: string | undefined;
  let metafile: MetafileLike | undefined;
  try {
    const publicPath = `${config.cdn.assetPrefix}${config.cdn.staticRoute}/chunks/`;
    const result = await Bun.build({
      entrypoints: [clientEntry],
      target: "browser",
      splitting: true,
      format: "esm",
      outdir: join(staticDir, "chunks"),
      naming: { entry: "[name]-[hash].[ext]", chunk: "chunk-[hash].[ext]", asset: "media/[name]-[hash].[ext]" },
      publicPath,
      minify: !options.dev,
      sourcemap: options.dev ? "inline" : config.next.productionBrowserSourceMaps ? "linked" : "none",
      define,
      plugins: [yoloNextPlugin({ root, config, env: "client", onServerAction: (id, exportName) => { serverActions[`${id}#${exportName}`] = { module: id, exportName }; } }), ...bundlerPlugins],
      env: `${config.publicEnvPrefixes[0] ?? "NEXT_PUBLIC_"}*`,
      reactCompiler: config.react.compiler,
      loader: config.bundler.loaders,
      metafile: true,
      throw: false,
    });
    warnings.push(...logsToDiagnostics(result.logs, "client").filter((d) => d.severity !== "error" || !result.success));
    if (!result.success) {
      success = false;
      warnings.push({ severity: "error", code: "build-client-failed", message: "the client pass failed; see the build logs" });
    } else {
      metafile = (result as { metafile?: MetafileLike }).metafile;
      for (const artifact of result.outputs) {
        outputs.push(artifact.path);
        const rel = toPosix(relative(staticDir, artifact.path));
        if (artifact.kind === "entry-point" || (artifact.kind === "asset" && rel.endsWith(".css"))) rootMainFiles.push(rel);
      }
      clientOutput = result.outputs.find((a) => a.kind === "entry-point")?.path;
    }
  } catch (error) {
    success = false;
    warnings.push({ severity: "error", code: "build-client-failed", message: error instanceof Error ? error.message : String(error) });
  }

  // Manifests.
  const hasher = new Bun.CryptoHasher("sha256");
  for (const output of outputs.sort()) {
    hasher.update(toPosix(relative(dist, output)));
    try {
      hasher.update(readFileSync(output));
    } catch {
      // ignore unreadable outputs
    }
  }
  const finalBuildId = config.cdn.deploymentId ?? hasher.digest("hex").slice(0, 16);
  const write = (name: string, value: unknown): void => {
    const path = join(dist, name);
    writeJson(path, value);
    manifests.push(path);
  };
  writeFileSync(join(dist, "BUILD_ID"), finalBuildId);
  manifests.push(join(dist, "BUILD_ID"));
  const pageChunks: Record<string, string[]> = {};
  for (const entry of manifest.entries) if (entry.kind === "page") pageChunks[entry.appPath === "/" ? "/page" : `${entry.appPath}/page`] = [...rootMainFiles];
  write("build-manifest.json", { version: 1, buildId: finalBuildId, rootMainFiles, pages: { "/_app": [], ...pageChunks }, polyfillFiles: [], lowPriorityFiles: [] });
  write("app-build-manifest.json", { pages: pageChunks });
  const clientReferenceManifest: Record<string, { id: string; name: string; chunks: string[]; async: boolean }> = {};
  for (const id of clientList) {
    const chunks = chunksForModule(metafile, join(staticDir, "chunks"), staticDir, id);
    clientReferenceManifest[id] = { id, name: "*", chunks: chunks.length ? chunks : rootMainFiles.filter((f) => f.endsWith(".js")), async: false };
  }
  write("client-reference-manifest.json", { version: 1, clientModules: clientReferenceManifest, entryCSSFiles: rootMainFiles.filter((f) => f.endsWith(".css")) });
  write("server-reference-manifest.json", { version: 1, actions: serverActions, encryptionKey: Bun.env.NEXT_SERVER_ACTIONS_ENCRYPTION_KEY ?? randomHex(32) });
  write("routes-manifest.json", routesManifestJson(manifest, config.cdn.basePath, { redirects: config.redirects, rewrites: config.rewrites, headers: config.headers }));
  write("app-paths-manifest.json", appPathsManifestJson(manifest));
  write("pages-manifest.json", pagesManifestJson(manifest));
  write("prerender-manifest.json", { version: 4, routes: {}, dynamicRoutes: {}, notFoundRoutes: [], preview: { previewModeId: randomHex(16), previewModeEncryptionKey: randomHex(32), previewModeSigningKey: randomHex(32) } });
  write("route-manifest.json", manifest);
  copyDir(join(root, config.cdn.publicDir), join(dist, "public"));

  // Static export.
  let exported: string[] | undefined;
  if (options.export || config.output === "export") {
    try {
      exported = await exportStatic({ root, config, manifest, dist, exportDir: resolve(options.exportDir ?? join(root, "out")), warnings, log });
    } catch (error) {
      warnings.push({ severity: "warning", code: "export-failed", message: error instanceof Error ? error.message : String(error) });
    }
  }

  // Standalone executable.
  let standaloneBinary: string | undefined;
  if (options.standalone || config.output === "standalone") {
    const serverEntry = [join(root, "src", "server.ts"), join(root, "server.ts")].find((file) => existsSync(file));
    if (!serverEntry) {
      warnings.push({ severity: "info", code: "standalone-skipped", message: "no src/server.ts to compile into a standalone executable" });
    } else {
      try {
        const outfile = join(dist, "standalone", process.platform === "win32" ? "server.exe" : "server");
        ensureDir(dirname(outfile));
        const result = await Bun.build({ entrypoints: [serverEntry], target: "bun", compile: { target: compileTarget() as never, outfile }, define, throw: false });
        if (result.success) standaloneBinary = outfile;
        else warnings.push({ severity: "warning", code: "standalone-failed", message: "the standalone compile failed; see the build logs" });
      } catch (error) {
        warnings.push({ severity: "warning", code: "standalone-failed", message: error instanceof Error ? error.message : String(error) });
      }
    }
  }

  return {
    success,
    outputs,
    buildId: finalBuildId,
    manifests,
    warnings,
    durationMs: Math.round(performance.now() - started),
    dist,
    clientModules: clientList,
    serverActions,
    rscEntry: rscOutput,
    ssrEntry: ssrOutput,
    clientEntry: clientOutput,
    rootMainFiles,
    exported,
    standaloneBinary,
  };
}

interface ExportContext {
  readonly root: string;
  readonly config: ResolvedYoloConfig;
  readonly manifest: RouteManifest;
  readonly dist: string;
  readonly exportDir: string;
  readonly warnings: Diagnostic[];
  readonly log: (message: string) => void;
}

interface AppLike {
  fetch(request: Request): Promise<Response> | Response;
}

async function exportStatic(ctx: ExportContext): Promise<string[]> {
  const { root, config, manifest, dist, exportDir } = ctx;
  rmSync(exportDir, { recursive: true, force: true });
  ensureDir(exportDir);
  let app: AppLike | undefined;
  const serverModule = join(SRC_DIR, "server.ts");
  if (existsSync(serverModule)) {
    try {
      const mod = (await import(Bun.pathToFileURL(serverModule).href)) as { createYoloNextApp?: (opts: { root: string; config: ResolvedYoloConfig }) => AppLike };
      if (typeof mod.createYoloNextApp === "function") app = mod.createYoloNextApp({ root, config });
    } catch (error) {
      ctx.warnings.push({ severity: "warning", code: "export-server-unavailable", message: `server module not usable for export: ${error instanceof Error ? error.message : String(error)}` });
    }
  }
  if (!app) {
    const { resolveRscEngine } = await import("./rsc-engine.ts");
    const engine = await resolveRscEngine({ root, config, manifest });
    app = {
      async fetch(request: Request): Promise<Response> {
        const stream = await engine.renderHtml({ request, manifest, config });
        return new Response(stream, { headers: { "content-type": "text/html; charset=utf-8" } });
      },
    };
  }
  const routes = new Set<string>();
  for (const entry of manifest.entries) {
    if (entry.kind !== "page" || entry.slot) continue;
    if (!entry.params.length) routes.add(entry.urlPattern);
  }
  const rscEntryFile = join(dist, "server", "rsc", "entry-rsc.js");
  if (existsSync(rscEntryFile)) {
    try {
      const rsc = (await import(Bun.pathToFileURL(rscEntryFile).href)) as { modules?: Record<string, Record<string, unknown>> };
      for (const entry of manifest.entries) {
        if (entry.kind !== "page" || entry.slot || !entry.params.length) continue;
        const mod = rsc.modules?.[entry.file];
        const generate = mod?.generateStaticParams;
        if (typeof generate !== "function") continue;
        const list = (await (generate as (ctx: { params: Record<string, string> }) => Promise<unknown>)({ params: {} })) as Array<Record<string, string | string[]>>;
        for (const params of list) {
          let url = entry.urlPattern;
          for (const param of entry.params) {
            const value = params[param.name];
            const text = Array.isArray(value) ? value.join("/") : (value ?? "");
            url = url.replace(param.optional ? `/[[...${param.name}]]` : param.catchAll ? `[...${param.name}]` : `[${param.name}]`, param.optional && text ? `/${text}` : text);
          }
          routes.add(url || "/");
        }
      }
    } catch (error) {
      ctx.warnings.push({ severity: "warning", code: "export-static-params-failed", message: error instanceof Error ? error.message : String(error) });
    }
  }
  const written: string[] = [];
  const writeHtml = async (route: string, request: Request, fileName?: string): Promise<void> => {
    const response = await app.fetch(request);
    const html = await response.text();
    const target = fileName ? join(exportDir, fileName) : route === "/" || config.trailingSlash ? join(exportDir, route, "index.html") : join(exportDir, `${route}.html`);
    ensureDir(dirname(target));
    writeFileSync(target, html);
    written.push(target);
  };
  for (const route of [...routes].sort()) {
    try {
      await writeHtml(route, new Request(`http://localhost${route}`));
      ctx.log(`exported ${route}`);
    } catch (error) {
      ctx.warnings.push({ severity: "warning", code: "export-route-failed", message: `${route}: ${error instanceof Error ? error.message : String(error)}` });
    }
  }
  try {
    await writeHtml("/__yolo_404__", new Request("http://localhost/__yolo_not_found__"), "404.html");
  } catch (error) {
    ctx.warnings.push({ severity: "warning", code: "export-404-failed", message: error instanceof Error ? error.message : String(error) });
  }
  copyDir(join(dist, "static"), join(exportDir, config.cdn.staticRoute.replace(/^\//, "")));
  copyDir(join(root, config.cdn.publicDir), exportDir);
  return written;
}

/** True when a dist directory holds a usable build (BUILD_ID + ssr entry). */
export function hasBuild(dist: string): boolean {
  try {
    return statSync(join(dist, "BUILD_ID")).isFile() && existsSync(join(dist, "server", "ssr", "entry-ssr.js"));
  } catch {
    return false;
  }
}

/** Plugins used by every pass for a config (exposed for custom build scripts). */
export function buildPlugins(root: string, config: ResolvedYoloConfig, env: "rsc" | "server" | "client"): BunPlugin[] {
  return [yoloNextPlugin({ root, config, env }), ...yoloBundlerPlugins({ ...config.bundler, root: config.bundler.root ?? root })];
}
