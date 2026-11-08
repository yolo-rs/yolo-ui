// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/rsc: React Flight encoding/decoding (compatible with the Rust `yolo_react::rsc`
// line protocol), server component tree serialization, Fizz SSR of Flight payloads with inline
// `self.__next_f` injection, and the `--conditions=react-server` render worker client.

import { createElement, Fragment, Suspense, type ReactElement, type ReactNode } from "react";
import * as React from "react";
import { createRequestStore, runWithRequestStore, type RequestStore } from "./headers.ts";

const REACT_ELEMENT = Symbol.for("react.transitional.element");
const REACT_LEGACY_ELEMENT = Symbol.for("react.element");
const REACT_FRAGMENT = Symbol.for("react.fragment");
const REACT_SUSPENSE = Symbol.for("react.suspense");
const REACT_CLIENT_REFERENCE = Symbol.for("react.client.reference");
const REACT_SERVER_REFERENCE = Symbol.for("react.server.reference");
const REACT_MEMO = Symbol.for("react.memo");
const REACT_FORWARD_REF = Symbol.for("react.forward_ref");
const REACT_LAZY = Symbol.for("react.lazy");
const REACT_CONTEXT = Symbol.for("react.context");
const REACT_PROVIDER = Symbol.for("react.provider");
const REACT_CONSUMER = Symbol.for("react.consumer");
const REACT_PROFILER = Symbol.for("react.profiler");
const REACT_STRICT_MODE = Symbol.for("react.strict_mode");
const REACT_ACTIVITY = Symbol.for("react.activity");
const REACT_VIEW_TRANSITION = Symbol.for("react.view_transition");

const TRANSPARENT_TYPES = new Set<symbol>([
  REACT_CONTEXT,
  REACT_PROVIDER,
  REACT_CONSUMER,
  REACT_PROFILER,
  REACT_STRICT_MODE,
  REACT_ACTIVITY,
  REACT_VIEW_TRANSITION,
]);

/** `"use client"` reference object as produced by `registerClientReference` (`$$id = "module#export"`). */
export interface ClientReferenceLike {
  readonly $$typeof: symbol;
  readonly $$id: string;
  readonly $$async?: boolean;
  readonly $$name?: string;
  readonly $$chunks?: readonly string[];
}

/** `"use server"` reference object as produced by `registerServerReference`. */
export interface ServerReferenceLike {
  readonly $$typeof: symbol;
  readonly $$id: string;
  readonly $$bound?: readonly unknown[] | null;
}

/** Entry of a Flight client manifest (`{ id, name, chunks }`). */
export interface ClientModuleEntry {
  readonly id: string;
  readonly name: string;
  readonly chunks: readonly string[];
  readonly async?: boolean;
}

/** Client manifest keyed by `"module#export"` (or `module`), as consumed by the server renderer. */
export type ClientManifest = Record<string, ClientModuleEntry>;

/** SSR consumer module map: `moduleMap[moduleId][exportName]`. */
export type ClientModuleMap = Record<string, Record<string, ClientModuleEntry>>;

/** Element record produced by `decodeFlightModel` (`$$typeof` matches React elements). */
export interface FlightElement {
  readonly $$typeof: symbol;
  readonly type: unknown;
  readonly key: string | null;
  readonly ref: null;
  readonly props: Record<string, unknown>;
}

/** Whether `value` is a React element (React 19 or legacy symbol). */
export function isReactElement(value: unknown): value is ReactElement {
  if (value === null || typeof value !== "object") {
    return false;
  }
  const tag = (value as { $$typeof?: unknown }).$$typeof;
  return tag === REACT_ELEMENT || tag === REACT_LEGACY_ELEMENT;
}

/** Whether `value` is a client reference (function or object tagged `react.client.reference`). */
export function isClientReference(value: unknown): value is ClientReferenceLike {
  return (
    (typeof value === "function" || (typeof value === "object" && value !== null)) &&
    (value as { $$typeof?: unknown }).$$typeof === REACT_CLIENT_REFERENCE
  );
}

/** Whether `value` is a server reference (function tagged `react.server.reference`). */
export function isServerReference(value: unknown): value is ServerReferenceLike {
  return (
    (typeof value === "function" || (typeof value === "object" && value !== null)) &&
    (value as { $$typeof?: unknown }).$$typeof === REACT_SERVER_REFERENCE
  );
}

function isThenable(value: unknown): value is PromiseLike<unknown> {
  return (
    value !== null &&
    (typeof value === "object" || typeof value === "function") &&
    typeof (value as { then?: unknown }).then === "function"
  );
}

function splitClientId(id: string): { module: string; exportName: string } {
  const index = id.lastIndexOf("#");
  if (index === -1) {
    return { module: id, exportName: "default" };
  }
  return { module: id.slice(0, index), exportName: id.slice(index + 1) };
}

// ---------------------------------------------------------------------------
// Inline client component registry (fallback path without a bundler transform)
// ---------------------------------------------------------------------------

type ComponentLike = (props: Record<string, unknown>) => unknown;

const inlineClientById = new Map<string, ComponentLike>();
const inlineClientByFn = new Map<ComponentLike, ClientReferenceLike>();
let inlineCounter = 0;

/** Prefix of module ids registered through `registerInlineClientComponent`. */
export const INLINE_CLIENT_PREFIX = "yolo-inline:";

/** Registers a component as a client reference resolvable in-process (`yolo-inline:<n>#default`). */
export function registerInlineClientComponent<P extends object>(input: (props: P) => unknown, name?: string): ClientReferenceLike {
  const component = input as unknown as ComponentLike;
  const existing = inlineClientByFn.get(component);
  if (existing) {
    return existing;
  }
  inlineCounter += 1;
  const moduleId = `${INLINE_CLIENT_PREFIX}${inlineCounter}:${name ?? component.name ?? "Component"}`;
  const reference: ClientReferenceLike = {
    $$typeof: REACT_CLIENT_REFERENCE,
    $$id: `${moduleId}#default`,
    $$async: false,
    $$name: "default",
    $$chunks: [],
  };
  inlineClientById.set(moduleId, component);
  inlineClientByFn.set(component, reference);
  return reference;
}

/** Resolves a module id registered through `registerInlineClientComponent`. */
export function resolveInlineClientComponent(moduleId: string): ComponentLike | undefined {
  return inlineClientById.get(moduleId);
}

// ---------------------------------------------------------------------------
// Flight line protocol encoder
// ---------------------------------------------------------------------------

/** Options of `encodeFlightModel`. */
export interface EncodeFlightOptions {
  readonly clientRefs?: ClientManifest;
}

interface EncodeContext {
  readonly rows: string[];
  readonly importIds: Map<string, number>;
  readonly clientRefs: ClientManifest;
  nextId: number;
}

function elementKey(element: ReactElement): string | null {
  return element.key === null || element.key === undefined ? null : String(element.key);
}

function encodeClientReference(reference: ClientReferenceLike, ctx: EncodeContext): string {
  const existing = ctx.importIds.get(reference.$$id);
  if (existing !== undefined) {
    return `$L${existing}`;
  }
  const id = ctx.nextId;
  ctx.nextId += 1;
  ctx.importIds.set(reference.$$id, id);
  const { module, exportName } = splitClientId(reference.$$id);
  const entry = ctx.clientRefs[reference.$$id] ?? ctx.clientRefs[module];
  const payload: unknown[] = entry
    ? [entry.id, [...entry.chunks], ctx.clientRefs[reference.$$id] ? entry.name : exportName]
    : [module, [...(reference.$$chunks ?? [])], exportName];
  if (entry?.async || reference.$$async) {
    payload.push(1);
  }
  ctx.rows.push(`${id}:I${JSON.stringify(payload)}`);
  return `$L${id}`;
}

function encodeElementType(type: unknown, ctx: EncodeContext): string {
  if (typeof type === "string") {
    return type;
  }
  if (isClientReference(type)) {
    return encodeClientReference(type, ctx);
  }
  if (typeof type === "symbol") {
    return `$S${type.description ?? ""}`;
  }
  if (typeof type === "function") {
    throw new Error(
      `encodeFlightModel: function component "${(type as { name?: string }).name ?? "anonymous"}" must be resolved with serializeElementTree() before encoding.`,
    );
  }
  if (type !== null && typeof type === "object") {
    const tag = (type as { $$typeof?: unknown }).$$typeof;
    if (tag === REACT_MEMO || tag === REACT_FORWARD_REF || tag === REACT_LAZY) {
      throw new Error("encodeFlightModel: memo/forwardRef/lazy types must be resolved with serializeElementTree() before encoding.");
    }
  }
  throw new Error(`encodeFlightModel: unsupported element type ${String(type)}.`);
}

function encodeProps(props: Record<string, unknown>, ctx: EncodeContext): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(props)) {
    if (key === "ref") {
      continue;
    }
    out[key] = toFlightJson(value, ctx);
  }
  return out;
}

function toFlightJson(value: unknown, ctx: EncodeContext): unknown {
  if (value === undefined) {
    return "$undefined";
  }
  if (value === null) {
    return null;
  }
  switch (typeof value) {
    case "boolean":
      return value;
    case "number":
      if (Number.isNaN(value)) {
        return "$NaN";
      }
      if (value === Number.POSITIVE_INFINITY) {
        return "$Infinity";
      }
      if (value === Number.NEGATIVE_INFINITY) {
        return "$-Infinity";
      }
      if (Object.is(value, -0)) {
        return "$-0";
      }
      return value;
    case "string":
      return value.startsWith("$") ? `$${value}` : value;
    case "bigint":
      return `$n${value.toString()}`;
    case "symbol":
      return `$S${value.description ?? ""}`;
    case "function": {
      if (isClientReference(value)) {
        return encodeClientReference(value, ctx);
      }
      if (isServerReference(value)) {
        const id = ctx.nextId;
        ctx.nextId += 1;
        ctx.rows.push(`${id}:${JSON.stringify({ id: value.$$id, bound: value.$$bound ? toFlightJson([...value.$$bound], ctx) : null })}`);
        return `$F${id}`;
      }
      throw new Error(
        `Functions cannot be passed directly to Client Components unless you explicitly expose it by marking it with "use server". ("${value.name || "anonymous"}")`,
      );
    }
    default:
      break;
  }
  if (isThenable(value)) {
    throw new Error("encodeFlightModel: pending Promise in model; resolve it first with resolveFlightModel() or serializeElementTree().");
  }
  if (value instanceof Date) {
    return `$D${value.toISOString()}`;
  }
  if (value instanceof Map) {
    const id = ctx.nextId;
    ctx.nextId += 1;
    const entries = [...value.entries()].map(([k, v]) => [toFlightJson(k, ctx), toFlightJson(v, ctx)]);
    ctx.rows.push(`${id}:${JSON.stringify(entries)}`);
    return `$Q${id}`;
  }
  if (value instanceof Set) {
    const id = ctx.nextId;
    ctx.nextId += 1;
    const items = [...value].map((item) => toFlightJson(item, ctx));
    ctx.rows.push(`${id}:${JSON.stringify(items)}`);
    return `$W${id}`;
  }
  if (Array.isArray(value)) {
    return value.map((item) => toFlightJson(item, ctx));
  }
  if (isReactElement(value)) {
    const element = value as ReactElement<Record<string, unknown>>;
    const key = elementKey(element);
    const props = (element.props ?? {}) as Record<string, unknown>;
    const elementType: unknown = element.type;
    if (elementType === REACT_FRAGMENT && key === null) {
      return toFlightJson(props.children, ctx);
    }
    if (typeof elementType === "symbol" && TRANSPARENT_TYPES.has(elementType)) {
      return toFlightJson(props.children, ctx);
    }
    return ["$", encodeElementType(elementType, ctx), key, encodeProps(props, ctx)];
  }
  if (isClientReference(value)) {
    return encodeClientReference(value, ctx);
  }
  if (value instanceof URL) {
    return value.href;
  }
  if (value instanceof URLSearchParams) {
    return Object.fromEntries(value.entries());
  }
  const out: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
    out[key] = toFlightJson(item, ctx);
  }
  return out;
}

/** Encodes a resolved model into Flight rows (`<id>:<json>` lines, root row `0`, imports `<n>:I[...]`). */
export function encodeFlightModel(model: unknown, options: EncodeFlightOptions = {}): string {
  const ctx: EncodeContext = { rows: [], importIds: new Map(), clientRefs: options.clientRefs ?? {}, nextId: 1 };
  const root = JSON.stringify(toFlightJson(model, ctx));
  return `${[...ctx.rows, `0:${root}`].join("\n")}\n`;
}

/** Deep-awaits Promises inside a model (arrays, plain objects, element props). */
export async function resolveFlightModel(model: unknown): Promise<unknown> {
  let value = model;
  while (isThenable(value)) {
    value = await value;
  }
  if (value === null || typeof value !== "object") {
    return value;
  }
  if (Array.isArray(value)) {
    return Promise.all(value.map((item) => resolveFlightModel(item)));
  }
  if (isReactElement(value)) {
    const element = value as ReactElement<Record<string, unknown>>;
    const props = await resolveFlightModel(element.props ?? {});
    return { ...element, props } as ReactElement;
  }
  if (value instanceof Date || value instanceof Map || value instanceof Set || isClientReference(value)) {
    return value;
  }
  const out: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
    out[key] = await resolveFlightModel(item);
  }
  return out;
}

// ---------------------------------------------------------------------------
// Flight decoder (tests and fallback SSR)
// ---------------------------------------------------------------------------

/** Import row payload (`[moduleId, chunks, exportName, async?]`). */
export interface FlightImport {
  readonly module: string;
  readonly chunks: readonly string[];
  readonly exportName: string;
  readonly async: boolean;
}

/** Result of `decodeFlightModel`. */
export interface DecodedFlight {
  readonly root: unknown;
  readonly rows: ReadonlyMap<number, unknown>;
  readonly imports: ReadonlyMap<number, FlightImport>;
  readonly errors: ReadonlyArray<{ readonly id: number; readonly message: string; readonly digest?: string }>;
}

function parseRows(text: string): { rows: Map<number, unknown>; imports: Map<number, FlightImport>; errors: Array<{ id: number; message: string; digest?: string }> } {
  const rows = new Map<number, unknown>();
  const imports = new Map<number, FlightImport>();
  const errors: Array<{ id: number; message: string; digest?: string }> = [];
  let offset = 0;
  while (offset < text.length) {
    const colon = text.indexOf(":", offset);
    if (colon === -1) {
      break;
    }
    const id = Number.parseInt(text.slice(offset, colon), 16);
    const tag = text.charAt(colon + 1);
    if (tag === "T") {
      // Length-prefixed text row: <id>:T<hexlen>,<text>
      const comma = text.indexOf(",", colon + 2);
      const length = Number.parseInt(text.slice(colon + 2, comma), 16);
      const start = comma + 1;
      rows.set(id, text.slice(start, start + length));
      offset = start + length;
      continue;
    }
    let newline = text.indexOf("\n", colon + 1);
    if (newline === -1) {
      newline = text.length;
    }
    const body = text.slice(colon + 1, newline);
    offset = newline + 1;
    if (/^[A-Za-z]/.test(body)) {
      const payload = body.slice(1);
      switch (tag) {
        case "I": {
          const parsed = JSON.parse(payload) as [string, string[], string, number?];
          imports.set(id, { module: parsed[0], chunks: parsed[1] ?? [], exportName: parsed[2] ?? "default", async: parsed.length === 4 });
          break;
        }
        case "E": {
          const parsed = JSON.parse(payload) as { message?: string; digest?: string };
          const error: { id: number; message: string; digest?: string } = { id, message: parsed.message ?? "Server error" };
          if (parsed.digest !== undefined) {
            error.digest = parsed.digest;
          }
          errors.push(error);
          break;
        }
        default:
          // H (hints), D (debug), W (console) and other rows are ignored.
          break;
      }
      continue;
    }
    rows.set(id, JSON.parse(body));
  }
  return { rows, imports, errors };
}

function makeClientReference(id: number, imports: ReadonlyMap<number, FlightImport>): ClientReferenceLike {
  const entry = imports.get(id);
  if (!entry) {
    throw new Error(`decodeFlightModel: missing import row ${id}.`);
  }
  return {
    $$typeof: REACT_CLIENT_REFERENCE,
    $$id: `${entry.module}#${entry.exportName}`,
    $$async: entry.async,
    $$name: entry.exportName,
    $$chunks: entry.chunks,
  };
}

function makeServerReference(id: number, rows: ReadonlyMap<number, unknown>, resolve: (value: unknown) => unknown): ServerReferenceLike {
  const row = rows.get(id) as { id?: string; bound?: unknown } | undefined;
  if (!row || typeof row.id !== "string") {
    throw new Error(`decodeFlightModel: missing server reference row ${id}.`);
  }
  const bound = row.bound == null ? null : (resolve(row.bound) as unknown[]);
  return { $$typeof: REACT_SERVER_REFERENCE, $$id: row.id, $$bound: bound };
}

function decodeString(value: string, ctx: { rows: ReadonlyMap<number, unknown>; imports: ReadonlyMap<number, FlightImport>; resolve: (value: unknown) => unknown }): unknown {
  if (!value.startsWith("$")) {
    return value;
  }
  if (value.startsWith("$$")) {
    return value.slice(1);
  }
  switch (value) {
    case "$undefined":
      return undefined;
    case "$NaN":
      return Number.NaN;
    case "$Infinity":
      return Number.POSITIVE_INFINITY;
    case "$-Infinity":
      return Number.NEGATIVE_INFINITY;
    case "$-0":
      return -0;
    default:
      break;
  }
  const kind = value.charAt(1);
  const rest = value.slice(2);
  switch (kind) {
    case "D":
      return new Date(rest);
    case "n":
      return BigInt(rest);
    case "S":
      return Symbol.for(rest);
    case "L": {
      // `$L<n>` is a client reference when row n is an import row, otherwise a lazy model row.
      const id = Number.parseInt(rest, 16);
      return ctx.imports.has(id) ? makeClientReference(id, ctx.imports) : ctx.resolve(ctx.rows.get(id));
    }
    case "F":
      return makeServerReference(Number.parseInt(rest, 16), ctx.rows, ctx.resolve);
    case "Q":
      return new Map((ctx.resolve(ctx.rows.get(Number.parseInt(rest, 16))) as Array<[unknown, unknown]>).map(([k, v]) => [k, v]));
    case "W":
      return new Set(ctx.resolve(ctx.rows.get(Number.parseInt(rest, 16))) as unknown[]);
    case "@":
      return ctx.resolve(ctx.rows.get(Number.parseInt(rest, 16)));
    case "T":
      return ctx.resolve(ctx.rows.get(Number.parseInt(rest, 16)));
    default:
      if (/^[0-9a-f]+$/i.test(value.slice(1))) {
        return ctx.resolve(ctx.rows.get(Number.parseInt(value.slice(1), 16)));
      }
      return value;
  }
}

/** Parses Flight text back into a model (elements become `FlightElement` records). */
export function decodeFlightModel(text: string): DecodedFlight {
  const { rows, imports, errors } = parseRows(text);
  const resolve = (value: unknown): unknown => {
    if (typeof value === "string") {
      return decodeString(value, { rows, imports, resolve });
    }
    if (value === null || typeof value !== "object") {
      return value;
    }
    if (Array.isArray(value)) {
      // Production rows are 4-tuples; development rows append owner/stack/validated debug fields.
      if (value.length >= 4 && value[0] === "$" && (typeof value[1] === "string") && (value[3] === null || typeof value[3] === "object" || typeof value[3] === "string")) {
        const element: FlightElement = {
          $$typeof: REACT_ELEMENT,
          type: resolve(value[1]),
          key: value[2] === null || value[2] === undefined ? null : String(value[2]),
          ref: null,
          props: (resolve(value[3]) as Record<string, unknown>) ?? {},
        };
        return element;
      }
      return value.map(resolve);
    }
    const out: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      out[key] = resolve(item);
    }
    return out;
  };
  const root = rows.has(0) ? resolve(rows.get(0)) : undefined;
  return { root, rows, imports, errors };
}

// ---------------------------------------------------------------------------
// Server component tree serialization (fallback without the react-server renderer)
// ---------------------------------------------------------------------------

const HOOK_ERROR = /Invalid hook call|reading '(use[A-Z]\w*)'|useState|useReducer|useEffect|useContext is not a function|Cannot read properties of null/;

let warnedFunctionProps = false;

function clientReferenceElement(reference: ClientReferenceLike, key: string | null, props: Record<string, unknown>): ReactElement {
  const element: FlightElement = { $$typeof: REACT_ELEMENT, type: reference, key, ref: null, props };
  return element as unknown as ReactElement;
}

async function serializeProps(props: Record<string, unknown>, depth: number, host: boolean): Promise<Record<string, unknown>> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(props)) {
    if (key === "ref") {
      continue;
    }
    if (host && typeof value === "function" && !isServerReference(value) && !isClientReference(value)) {
      if (!warnedFunctionProps) {
        warnedFunctionProps = true;
        console.warn(`[yolo-next] dropping function prop "${key}" on a host element: event handlers must live in "use client" components.`);
      }
      continue;
    }
    out[key] = await serializeNode(value, depth + 1);
  }
  return out;
}

async function serializeNode(node: unknown, depth: number): Promise<unknown> {
  if (depth > 512) {
    throw new Error("serializeElementTree: maximum depth exceeded (cyclic tree?).");
  }
  let value = node;
  while (isThenable(value)) {
    value = await value;
  }
  if (value === null || value === undefined || typeof value !== "object") {
    if (typeof value === "function" && !isClientReference(value) && !isServerReference(value)) {
      return value;
    }
    return value;
  }
  if (Array.isArray(value)) {
    const items: unknown[] = [];
    for (const item of value) {
      items.push(await serializeNode(item, depth + 1));
    }
    return items;
  }
  if (!isReactElement(value)) {
    if (value instanceof Date || value instanceof Map || value instanceof Set || isClientReference(value)) {
      return value;
    }
    const out: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      out[key] = await serializeNode(item, depth + 1);
    }
    return out;
  }
  const element = value as ReactElement<Record<string, unknown>>;
  const props = (element.props ?? {}) as Record<string, unknown>;
  const key = elementKey(element);
  const type: unknown = element.type;
  if (typeof type === "string") {
    const { children, ...rest } = await serializeProps(props, depth, true);
    const hostProps = { ...rest, key: key ?? undefined };
    if (Array.isArray(children)) {
      return createElement(type, hostProps, ...(children as ReactNode[]));
    }
    return children === undefined ? createElement(type, hostProps) : createElement(type, hostProps, children as ReactNode);
  }
  if (isClientReference(type)) {
    return clientReferenceElement(type, key, await serializeProps(props, depth, false));
  }
  if (typeof type === "symbol") {
    if (type === REACT_FRAGMENT) {
      const children = await serializeNode(props.children, depth + 1);
      return key === null ? children : createElement(Fragment, { key }, children as ReactNode);
    }
    if (type === REACT_SUSPENSE) {
      const fallback = await serializeNode(props.fallback, depth + 1);
      const children = await serializeNode(props.children, depth + 1);
      return createElement(Suspense, { key: key ?? undefined, fallback: fallback as ReactNode }, children as ReactNode);
    }
    if (TRANSPARENT_TYPES.has(type)) {
      return serializeNode(props.children, depth + 1);
    }
    return serializeNode(props.children, depth + 1);
  }
  if (type !== null && typeof type === "object") {
    const tag = (type as { $$typeof?: unknown }).$$typeof;
    if (tag === REACT_MEMO) {
      return serializeNode(createElement((type as { type: ComponentLike }).type as never, { ...props, key: key ?? undefined }), depth + 1);
    }
    if (tag === REACT_FORWARD_REF) {
      const render = (type as { render: (props: Record<string, unknown>, ref: null) => unknown }).render;
      return serializeNode(render(props, null), depth + 1);
    }
    if (tag === REACT_LAZY) {
      const lazy = type as { _payload: unknown; _init: (payload: unknown) => unknown };
      let resolved: unknown;
      try {
        resolved = lazy._init(lazy._payload);
      } catch (thrown) {
        if (isThenable(thrown)) {
          await thrown;
          resolved = lazy._init(lazy._payload);
        } else {
          throw thrown;
        }
      }
      return serializeNode(createElement(resolved as never, { ...props, key: key ?? undefined }), depth + 1);
    }
    if (tag === REACT_PROVIDER || tag === REACT_CONTEXT || tag === REACT_CONSUMER) {
      return serializeNode(props.children, depth + 1);
    }
    throw new Error(`serializeElementTree: unsupported element type object ${String(tag)}.`);
  }
  if (typeof type === "function") {
    const component = type as ComponentLike;
    const inline = inlineClientByFn.get(component);
    if (inline) {
      return clientReferenceElement(inline, key, await serializeProps(props, depth, false));
    }
    if ((component as { prototype?: { isReactComponent?: unknown } }).prototype?.isReactComponent) {
      return clientReferenceElement(registerInlineClientComponent(component), key, await serializeProps(props, depth, false));
    }
    let result: unknown;
    try {
      result = component(props);
      while (isThenable(result)) {
        result = await result;
      }
    } catch (error) {
      if (error instanceof Error && HOOK_ERROR.test(error.message) && !isThenable(error)) {
        return clientReferenceElement(registerInlineClientComponent(component), key, await serializeProps(props, depth, false));
      }
      throw error;
    }
    return serializeNode(result, depth + 1);
  }
  throw new Error(`serializeElementTree: unsupported element type ${String(type)}.`);
}

/** Resolves a React tree into host elements, client references, Suspense/Fragment and primitives. */
export async function serializeElementTree(element: unknown): Promise<unknown> {
  return serializeNode(element, 0);
}

// ---------------------------------------------------------------------------
// react-server-dom-webpack loaders
// ---------------------------------------------------------------------------

/** Subset of `react-server-dom-webpack/server.edge` used by the runtime. */
export interface ReactServerDomServer {
  renderToReadableStream(
    model: unknown,
    clientManifest: ClientManifest | null,
    options?: { onError?: (error: unknown) => string | undefined | void; signal?: AbortSignal; identifierPrefix?: string; temporaryReferences?: unknown },
  ): ReadableStream<Uint8Array>;
  decodeReply(body: string | FormData, serverManifest?: unknown, options?: unknown): Promise<unknown[]>;
  decodeAction(body: FormData, serverManifest: unknown): Promise<(() => Promise<unknown>) | null>;
  decodeFormState(actionResult: unknown, body: FormData, serverManifest: unknown): Promise<unknown>;
  registerServerReference<T>(reference: T, id: string, exportName: string | null): T;
  registerClientReference<T>(proxy: T, id: string, exportName: string): T;
  createClientModuleProxy(moduleId: string): Record<string, unknown>;
}

/** Subset of `react-server-dom-webpack/client.edge` used by SSR. */
export interface ReactServerDomClient {
  createFromReadableStream<T = ReactNode>(
    stream: ReadableStream<Uint8Array>,
    options?: {
      serverConsumerManifest?: { moduleMap: ClientModuleMap | null; moduleLoading: unknown; serverModuleMap: unknown };
      nonce?: string;
      replayConsoleLogs?: boolean;
    },
  ): PromiseLike<T>;
  createServerReference(id: string, callServer?: (id: string, args: unknown[]) => Promise<unknown>): (...args: unknown[]) => Promise<unknown>;
  encodeReply(value: unknown): Promise<string | FormData>;
}

let serverDomPromise: Promise<ReactServerDomServer | undefined> | undefined;
let clientDomPromise: Promise<ReactServerDomClient | undefined> | undefined;

/** Whether `react` resolved with the `react-server` condition (no client hooks exported). */
export function isReactServerEnvironment(): boolean {
  return typeof (React as unknown as { useState?: unknown }).useState !== "function";
}

/** Loads `react-server-dom-webpack/server.edge`; `undefined` outside a `react-server` environment. */
export function loadReactServerDom(): Promise<ReactServerDomServer | undefined> {
  if (!serverDomPromise) {
    serverDomPromise = (async () => {
      if (!isReactServerEnvironment()) {
        return undefined;
      }
      try {
        const specifier = "react-server-dom-webpack/server.edge";
        const mod = (await import(specifier)) as Partial<ReactServerDomServer> & { default?: Partial<ReactServerDomServer> };
        const api = typeof mod.renderToReadableStream === "function" ? mod : mod.default;
        return api && typeof api.renderToReadableStream === "function" ? (api as ReactServerDomServer) : undefined;
      } catch {
        return undefined;
      }
    })();
  }
  return serverDomPromise;
}

/** Loads `react-server-dom-webpack/client.edge`; `undefined` when the package is missing. */
export function loadReactServerDomClient(): Promise<ReactServerDomClient | undefined> {
  if (!clientDomPromise) {
    clientDomPromise = (async () => {
      try {
        const specifier = "react-server-dom-webpack/client.edge";
        const mod = (await import(specifier)) as Partial<ReactServerDomClient> & { default?: Partial<ReactServerDomClient> };
        const api = typeof mod.createFromReadableStream === "function" ? mod : mod.default;
        return api && typeof api.createFromReadableStream === "function" ? (api as ReactServerDomClient) : undefined;
      } catch {
        return undefined;
      }
    })();
  }
  return clientDomPromise;
}

// ---------------------------------------------------------------------------
// Flight rendering
// ---------------------------------------------------------------------------

/** Options of `renderToFlight`. */
export interface RenderToFlightOptions {
  readonly element: unknown;
  readonly clientManifest?: ClientManifest;
  readonly onError?: (error: unknown) => string | undefined | void;
  readonly signal?: AbortSignal;
}

/** Renders a model/element tree to a Flight stream (real renderer when available, else the fallback encoder). */
export async function renderToFlight(options: RenderToFlightOptions): Promise<ReadableStream<Uint8Array>> {
  const server = await loadReactServerDom();
  if (server) {
    const renderOptions: { onError?: (error: unknown) => string | undefined | void; signal?: AbortSignal } = {};
    if (options.onError) {
      renderOptions.onError = options.onError;
    }
    if (options.signal) {
      renderOptions.signal = options.signal;
    }
    return server.renderToReadableStream(options.element, options.clientManifest ?? {}, renderOptions);
  }
  const resolved = await serializeElementTree(options.element);
  const text = encodeFlightModel(resolved, options.clientManifest ? { clientRefs: options.clientManifest } : {});
  return new Response(text).body ?? new ReadableStream<Uint8Array>({ start: (c) => c.close() });
}

/** Escapes text for inline `<script>` embedding (JSON-safe unicode escapes for `<`, `>`, `&`, U+2028/9). */
export function escapeInlineScript(text: string): string {
  return text
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026")
    .replace(new RegExp("\\u2028", "g"), "\\u2028")
    .replace(new RegExp("\\u2029", "g"), "\\u2029");
}

/** `<script>` tag pushing one Flight chunk into `self.__next_f` (Next.js compatible bootstrap). */
export function flightInlineScript(chunk: string, nonce?: string): string {
  const attr = nonce ? ` nonce="${nonce.replace(/"/g, "&quot;")}"` : "";
  return `<script${attr}>(self.__next_f=self.__next_f||[]).push([1,${escapeInlineScript(JSON.stringify(chunk))}])</script>`;
}

/** Bootstrap `<script>` initializing `self.__next_f` (`push([0])`). */
export function flightBootstrapScript(nonce?: string): string {
  const attr = nonce ? ` nonce="${nonce.replace(/"/g, "&quot;")}"` : "";
  return `<script${attr}>(self.__next_f=self.__next_f||[]).push([0])</script>`;
}

type ModuleNamespace = Record<string, unknown>;

const moduleRegistry = new Map<string, ModuleNamespace>();

declare global {
  // eslint-disable-next-line no-var
  var __webpack_require__: ((id: string) => unknown) | undefined;
  // eslint-disable-next-line no-var
  var __webpack_chunk_load__: ((id: string) => Promise<unknown>) | undefined;
}

function installModuleShims(): void {
  if (typeof globalThis.__webpack_require__ !== "function") {
    globalThis.__webpack_require__ = (id: string): unknown => {
      const inline = inlineClientById.get(id);
      if (inline) {
        return { default: inline, __esModule: true };
      }
      const mod = moduleRegistry.get(id);
      if (!mod) {
        throw new Error(`Client module "${id}" was not preloaded; list it in the client module map.`);
      }
      return mod;
    };
  }
  if (typeof globalThis.__webpack_chunk_load__ !== "function") {
    globalThis.__webpack_chunk_load__ = async (): Promise<void> => undefined;
  }
}

/** Imports every module of `map` so `__webpack_require__` can resolve them synchronously. */
export async function preloadClientModules(map: ClientModuleMap): Promise<void> {
  installModuleShims();
  const ids = new Set<string>();
  for (const exportsMap of Object.values(map)) {
    for (const entry of Object.values(exportsMap)) {
      ids.add(entry.id);
    }
  }
  await Promise.all(
    [...ids].map(async (id) => {
      if (moduleRegistry.has(id) || id.startsWith(INLINE_CLIENT_PREFIX)) {
        return;
      }
      moduleRegistry.set(id, (await import(id)) as ModuleNamespace);
    }),
  );
}

/** Registers an already-imported module namespace under `id` (tests, bundled builds). */
export function registerClientModule(id: string, namespace: ModuleNamespace): void {
  installModuleShims();
  moduleRegistry.set(id, namespace);
}

async function resolveClientComponent(reference: ClientReferenceLike, map: ClientModuleMap): Promise<unknown> {
  const { module, exportName } = splitClientId(reference.$$id);
  const inline = inlineClientById.get(module);
  if (inline) {
    return inline;
  }
  const entry = map[module]?.[exportName] ?? map[module]?.["*"];
  const specifier = entry?.id ?? module;
  const name = entry ? (entry.name === "*" ? exportName : entry.name) : exportName;
  let namespace = moduleRegistry.get(specifier);
  if (!namespace) {
    namespace = (await import(specifier)) as ModuleNamespace;
    moduleRegistry.set(specifier, namespace);
  }
  if (name === "" || name === "default") {
    return namespace.default ?? namespace;
  }
  return namespace[name];
}

function serverReferenceProxy(reference: ServerReferenceLike): (...args: unknown[]) => Promise<unknown> {
  const proxy = async (): Promise<unknown> => {
    throw new Error("Server Functions cannot be called during initial render. This would create a fetch waterfall.");
  };
  Object.defineProperties(proxy, {
    $$typeof: { value: REACT_SERVER_REFERENCE },
    $$id: { value: reference.$$id },
    $$bound: { value: reference.$$bound ?? null },
    $$FORM_ACTION: {
      value: (prefix: string) => {
        const data = new FormData();
        const bound = reference.$$bound ?? [];
        bound.forEach((value, index) => {
          data.append(`${prefix}${index}`, typeof value === "string" ? value : JSON.stringify(value));
        });
        return { name: `$ACTION_ID_${reference.$$id}`, method: "POST", encType: "multipart/form-data", data };
      },
    },
  });
  return proxy;
}

/** Rebuilds a React element tree from a decoded Flight model (client references resolved via `map`). */
export async function modelToElement(model: unknown, map: ClientModuleMap = {}): Promise<ReactNode> {
  const walk = async (value: unknown): Promise<unknown> => {
    if (value === null || typeof value !== "object") {
      if (isServerReference(value)) {
        return serverReferenceProxy(value);
      }
      return value;
    }
    if (Array.isArray(value)) {
      const items: unknown[] = [];
      for (const item of value) {
        items.push(await walk(item));
      }
      return items;
    }
    if (isClientReference(value)) {
      return resolveClientComponent(value, map);
    }
    if (isServerReference(value)) {
      return serverReferenceProxy(value);
    }
    if (value instanceof Date || value instanceof Map || value instanceof Set) {
      return value;
    }
    if (isReactElement(value)) {
      const element = value as unknown as FlightElement;
      const rawProps = element.props ?? {};
      const props: Record<string, unknown> = {};
      for (const [key, item] of Object.entries(rawProps)) {
        props[key] = await walk(item);
      }
      let type: unknown = element.type;
      if (isClientReference(type)) {
        type = await resolveClientComponent(type, map);
      }
      if (typeof type === "string" || typeof type === "function" || typeof type === "symbol") {
        const { children, ...rest } = props;
        const finalProps = element.key === null ? rest : { ...rest, key: element.key };
        return children === undefined
          ? createElement(type as string, finalProps)
          : createElement(type as string, finalProps, ...(Array.isArray(children) ? (children as ReactNode[]) : [children as ReactNode]));
      }
      throw new Error(`modelToElement: cannot render element type ${String(type)}.`);
    }
    const out: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      out[key] = await walk(item);
    }
    return out;
  };
  return (await walk(model)) as ReactNode;
}

/** Options of `renderFlightToHtml`. */
export interface RenderFlightToHtmlOptions {
  readonly flightStream: ReadableStream<Uint8Array>;
  readonly clientModuleMap?: ClientModuleMap;
  readonly bootstrapScripts?: readonly string[];
  readonly bootstrapScriptContent?: string;
  readonly headHtml?: string;
  readonly nonce?: string;
  readonly onError?: (error: unknown) => string | undefined | void;
  readonly signal?: AbortSignal;
  /** Force the decode-and-rebuild path even when `react-server-dom-webpack/client.edge` is available. */
  readonly fallback?: boolean;
  /** Skip the inline `self.__next_f` payload (pure HTML output). */
  readonly inlineFlight?: boolean;
}

interface ReactDomServerModule {
  renderToReadableStream(
    children: ReactNode,
    options?: {
      bootstrapScripts?: string[];
      bootstrapScriptContent?: string;
      nonce?: string;
      onError?: (error: unknown) => string | undefined | void;
      signal?: AbortSignal;
    },
  ): Promise<ReadableStream<Uint8Array> & { allReady: Promise<void> }>;
}

async function loadReactDomServer(): Promise<ReactDomServerModule> {
  const specifier = "react-dom/server";
  const mod = (await import(specifier)) as ReactDomServerModule & { default?: ReactDomServerModule };
  return typeof mod.renderToReadableStream === "function" ? mod : (mod.default as ReactDomServerModule);
}

function injectHead(html: string, headHtml: string): { html: string; done: boolean } {
  const headOpen = /<head(\s[^>]*)?>/i.exec(html);
  if (headOpen) {
    const at = headOpen.index + headOpen[0].length;
    return { html: `${html.slice(0, at)}${headHtml}${html.slice(at)}`, done: true };
  }
  const bodyOpen = /<body(\s[^>]*)?>/i.exec(html);
  if (bodyOpen) {
    return { html: `${html.slice(0, bodyOpen.index)}<head>${headHtml}</head>${html.slice(bodyOpen.index)}`, done: true };
  }
  const htmlOpen = /<html(\s[^>]*)?>/i.exec(html);
  if (htmlOpen) {
    const at = htmlOpen.index + htmlOpen[0].length;
    return { html: `${html.slice(0, at)}<head>${headHtml}</head>${html.slice(at)}`, done: true };
  }
  return { html, done: false };
}

/** Renders a Flight stream to an HTML stream with the Flight payload inlined as `self.__next_f` pushes. */
export async function renderFlightToHtml(options: RenderFlightToHtmlOptions): Promise<ReadableStream<Uint8Array>> {
  const map = options.clientModuleMap ?? {};
  const inline = options.inlineFlight ?? true;
  const [forRender, forInline] = inline ? options.flightStream.tee() : [options.flightStream, undefined];
  const client = options.fallback ? undefined : await loadReactServerDomClient();
  let element: ReactNode;
  if (client && !isReactServerEnvironment()) {
    await preloadClientModules(map);
    const hasMap = Object.keys(map).length > 0;
    const root = client.createFromReadableStream<ReactNode>(forRender, {
      serverConsumerManifest: { moduleMap: hasMap ? map : null, moduleLoading: null, serverModuleMap: null },
      ...(options.nonce ? { nonce: options.nonce } : {}),
    });
    const use = (React as unknown as { use: <T>(thenable: PromiseLike<T>) => T }).use;
    const ServerRoot = (): ReactNode => use(root);
    element = createElement(ServerRoot);
  } else {
    const text = await new Response(forRender).text();
    const decoded = decodeFlightModel(text);
    const firstError = decoded.errors[0];
    if (decoded.root === undefined && firstError) {
      const error = new Error(firstError.message) as Error & { digest?: string };
      if (firstError.digest !== undefined) {
        error.digest = firstError.digest;
      }
      throw error;
    }
    element = await modelToElement(decoded.root, map);
  }
  const dom = await loadReactDomServer();
  const renderOptions: Parameters<ReactDomServerModule["renderToReadableStream"]>[1] = {};
  if (options.bootstrapScripts && options.bootstrapScripts.length > 0) {
    renderOptions.bootstrapScripts = [...options.bootstrapScripts];
  }
  if (options.bootstrapScriptContent) {
    renderOptions.bootstrapScriptContent = options.bootstrapScriptContent;
  }
  if (options.nonce) {
    renderOptions.nonce = options.nonce;
  }
  if (options.onError) {
    renderOptions.onError = options.onError;
  }
  if (options.signal) {
    renderOptions.signal = options.signal;
  }
  const htmlStream = await dom.renderToReadableStream(element, renderOptions);
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  const headHtml = options.headHtml ?? "";
  const nonce = options.nonce;
  const pendingScripts: string[] = [];
  let flightDone: Promise<void> = Promise.resolve();
  if (forInline) {
    pendingScripts.push(flightBootstrapScript(nonce));
    flightDone = (async () => {
      const reader = forInline.getReader();
      const textDecoder = new TextDecoder();
      for (;;) {
        const { done, value } = await reader.read();
        if (done) {
          break;
        }
        pendingScripts.push(flightInlineScript(textDecoder.decode(value, { stream: true }), nonce));
      }
    })();
  }
  let headInjected = headHtml.length === 0;
  let carry = "";
  const transform = new TransformStream<Uint8Array, Uint8Array>({
    transform(chunk, controller) {
      let text = carry + decoder.decode(chunk, { stream: true });
      carry = "";
      if (!headInjected) {
        const injected = injectHead(text, headHtml);
        text = injected.html;
        headInjected = injected.done;
        if (!headInjected && text.length < 4096) {
          carry = text;
          return;
        }
      }
      if (pendingScripts.length > 0) {
        text += pendingScripts.splice(0, pendingScripts.length).join("");
      }
      controller.enqueue(encoder.encode(text));
    },
    async flush(controller) {
      await flightDone;
      let text = carry;
      if (!headInjected && headHtml) {
        text = `${headHtml}${text}`;
      }
      if (pendingScripts.length > 0) {
        text += pendingScripts.splice(0, pendingScripts.length).join("");
      }
      if (text.length > 0) {
        controller.enqueue(encoder.encode(text));
      }
    },
  });
  return htmlStream.pipeThrough(transform);
}

// ---------------------------------------------------------------------------
// Route element building (shared by the worker and the inline renderer)
// ---------------------------------------------------------------------------

/** Imported route module (page/layout/template/loading). */
export interface RouteModule {
  readonly default?: unknown;
  readonly [key: string]: unknown;
}

/** Inputs of `buildRouteElement`. */
export interface RouteElementInput {
  readonly page: RouteModule;
  readonly layouts?: ReadonlyArray<RouteModule>;
  readonly templates?: ReadonlyArray<RouteModule>;
  readonly loading?: RouteModule;
  readonly params?: Record<string, string>;
  readonly searchParams?: Record<string, string | string[]>;
  readonly slots?: Record<string, ReactNode>;
  readonly head?: ReactNode;
}

function componentOf(module: RouteModule, file: string): ComponentLike {
  const component = module.default;
  if (typeof component !== "function" && !isClientReference(component)) {
    throw new Error(`Route module "${file}" has no default export component.`);
  }
  return component as ComponentLike;
}

/** Builds `layouts[0](...layouts[n](template(Suspense(loading, page))))` with params/searchParams as Promises. */
export function buildRouteElement(input: RouteElementInput): ReactElement {
  const params = Promise.resolve({ ...(input.params ?? {}) });
  const searchParams = Promise.resolve({ ...(input.searchParams ?? {}) });
  const Page = componentOf(input.page, "page");
  let node: ReactNode = createElement(Page as never, { params, searchParams });
  if (input.loading) {
    const Loading = componentOf(input.loading, "loading");
    node = createElement(Suspense, { fallback: createElement(Loading as never, {}) }, node);
  }
  for (const template of [...(input.templates ?? [])].reverse()) {
    const Template = componentOf(template, "template");
    node = createElement(Template as never, { params }, node);
  }
  const layouts = [...(input.layouts ?? [])].reverse();
  layouts.forEach((layout, index) => {
    const Layout = componentOf(layout, "layout");
    const isRoot = index === layouts.length - 1;
    const slotProps = isRoot ? { ...(input.slots ?? {}) } : {};
    const children = isRoot && input.head ? createElement(Fragment, null, input.head, node) : node;
    node = createElement(Layout as never, { params, ...slotProps }, children);
  });
  return node as ReactElement;
}

// ---------------------------------------------------------------------------
// RSC worker client
// ---------------------------------------------------------------------------

/** Render request sent to the `react-server` worker. */
export interface RscRenderRequest {
  readonly pathname: string;
  readonly search: string;
  readonly headers: Record<string, string>;
  readonly routeFile: string;
  readonly layoutFiles: ReadonlyArray<string>;
  readonly templateFiles?: ReadonlyArray<string>;
  readonly loadingFile?: string;
  readonly params: Record<string, string>;
  readonly isPrerender?: boolean;
}

/** Handle on a running RSC worker. */
export interface RscWorker {
  readonly port: number;
  readonly url: string;
  render(request: RscRenderRequest): Promise<Response>;
  action(request: Request): Promise<Response>;
  stop(): void;
}

/** Options of `startRscWorker`. */
export interface StartRscWorkerOptions {
  readonly root: string;
  readonly entry?: string;
  readonly preload?: ReadonlyArray<string>;
  readonly env?: Record<string, string>;
  readonly timeoutMs?: number;
}

/** Spawns `bun --conditions=react-server rsc-worker.ts` and resolves once it reports its port. */
export async function startRscWorker(options: StartRscWorkerOptions): Promise<RscWorker> {
  const entry = options.entry ?? new URL("./rsc-worker.ts", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
  const args = ["bun", "--conditions=react-server"];
  for (const preload of options.preload ?? []) {
    args.push("--preload", preload);
  }
  args.push(entry);
  let resolvePort: (port: number) => void = () => undefined;
  let rejectPort: (error: Error) => void = () => undefined;
  const portPromise = new Promise<number>((resolve, reject) => {
    resolvePort = resolve;
    rejectPort = reject;
  });
  const proc = Bun.spawn(args, {
    cwd: options.root,
    env: { ...Bun.env, ...(options.env ?? {}), YOLO_RSC_ROOT: options.root, YOLO_RSC_PORT: "0" },
    stdout: "pipe",
    stderr: "inherit",
    ipc(message: unknown) {
      if (message && typeof message === "object" && typeof (message as { port?: unknown }).port === "number") {
        resolvePort((message as { port: number }).port);
      }
    },
    serialization: "json",
    onExit(_proc, exitCode) {
      rejectPort(new Error(`RSC worker exited with code ${exitCode ?? "unknown"} before reporting its port.`));
    },
  });
  void (async () => {
    if (!proc.stdout) {
      return;
    }
    const reader = proc.stdout.getReader();
    const textDecoder = new TextDecoder();
    let buffer = "";
    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      buffer += textDecoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";
      for (const line of lines) {
        const match = /^YOLO_RSC_PORT=(\d+)/.exec(line.trim());
        if (match) {
          resolvePort(Number(match[1]));
        } else if (line.trim().length > 0) {
          console.log(`[rsc-worker] ${line}`);
        }
      }
    }
  })();
  const timeout = setTimeout(() => {
    rejectPort(new Error("RSC worker did not report its port in time."));
  }, options.timeoutMs ?? 15_000);
  let port: number;
  try {
    port = await portPromise;
  } catch (error) {
    proc.kill();
    throw error;
  } finally {
    clearTimeout(timeout);
  }
  const url = `http://127.0.0.1:${port}`;
  return {
    port,
    url,
    render(request) {
      return fetch(`${url}/render`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(request),
      });
    },
    action(request) {
      const headers = new Headers(request.headers);
      const original = new URL(request.url);
      if (!headers.has("x-forwarded-host")) {
        headers.set("x-forwarded-host", original.host);
      }
      headers.set("x-yolo-original-url", request.url);
      return fetch(`${url}/action`, { method: "POST", headers, body: request.body, duplex: "half" } as RequestInit);
    },
    stop() {
      proc.kill();
    },
  };
}

/** Request store helper for worker/inline rendering (phase `render`). */
export function createRenderStore(request: Request, params: Record<string, string>, isPrerender = false): RequestStore {
  return createRequestStore(request, { params, phase: "render", isPrerender });
}

/** Runs `fn` inside a fresh render store for `request`. */
export function withRenderStore<T>(request: Request, params: Record<string, string>, fn: (store: RequestStore) => T): T {
  const store = createRenderStore(request, params);
  return runWithRequestStore(store, () => fn(store));
}
