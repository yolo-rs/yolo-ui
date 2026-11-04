// SPDX-License-Identifier: MIT OR Apache-2.0
// react-server-dom-parcel ships no types; the entry points the runtime uses.
declare module "react-server-dom-parcel/server.node" {
  export function renderToReadableStream(model: unknown, options?: Record<string, unknown>): ReadableStream<Uint8Array>;
  export function decodeReply(body: string | FormData, options?: Record<string, unknown>): Promise<unknown>;
  export function decodeAction(body: FormData): Promise<(() => unknown) | null>;
  export function decodeFormState(result: unknown, body: FormData): Promise<unknown>;
  export function createTemporaryReferenceSet(): unknown;
  export function loadServerAction(id: string): Promise<unknown>;
  export function registerServerActions(manifest: Record<string, string[]>): void;
  export function registerServerReference<T>(reference: T, id: string, name: string): T;
  export function createClientReference(id: string, name: string, bundles: string[]): unknown;
}
declare module "react-server-dom-parcel/client.browser" {
  export function createFromFetch<T>(promise: Promise<Response>, options?: Record<string, unknown>): Promise<T>;
  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: Record<string, unknown>,
  ): Promise<T>;
  export function createTemporaryReferenceSet(): unknown;
  export function encodeReply(value: unknown, options?: Record<string, unknown>): Promise<string | FormData>;
  export function setServerCallback(callback: (id: string, args: unknown[]) => Promise<unknown>): void;
  export function createServerReference(id: string, name: string): unknown;
}
declare module "react-server-dom-parcel/client.edge" {
  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: Record<string, unknown>,
  ): Promise<T>;
  export function createServerReference(id: string, name: string): unknown;
}
