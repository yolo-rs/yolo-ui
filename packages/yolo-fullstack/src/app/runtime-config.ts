// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/config`: `serverRuntimeConfig` / `publicRuntimeConfig` of `next.config`, injected into each bundle
// as the `__YOLO_RUNTIME_CONFIG__` global through Bun.build `define` (the client bundle only receives the
// public half).
import type { Layer } from "./plugin";

export const RUNTIME_CONFIG_GLOBAL = "__YOLO_RUNTIME_CONFIG__";

export interface RuntimeConfig {
  serverRuntimeConfig: Record<string, unknown>;
  publicRuntimeConfig: Record<string, unknown>;
}

export interface RuntimeConfigInput {
  serverRuntimeConfig?: Record<string, unknown>;
  publicRuntimeConfig?: Record<string, unknown>;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** Read the global set by `define` (a JSON string or an object). */
export default function getConfig(): RuntimeConfig {
  let raw: unknown = (globalThis as Record<string, unknown>)[RUNTIME_CONFIG_GLOBAL];
  if (typeof raw === "string") {
    try {
      raw = JSON.parse(raw);
    } catch {
      raw = undefined;
    }
  }
  const value = isRecord(raw) ? raw : {};
  return {
    serverRuntimeConfig: isRecord(value.serverRuntimeConfig) ? value.serverRuntimeConfig : {},
    publicRuntimeConfig: isRecord(value.publicRuntimeConfig) ? value.publicRuntimeConfig : {},
  };
}

/** `define` entry of the global for one bundle layer: the client bundle gets only `publicRuntimeConfig`. */
export function runtimeConfigDefine(config: RuntimeConfigInput, layer: Layer): Record<string, string> {
  const value: RuntimeConfig = {
    serverRuntimeConfig: layer === "client" ? {} : (config.serverRuntimeConfig ?? {}),
    publicRuntimeConfig: config.publicRuntimeConfig ?? {},
  };
  // The define value is a JS expression: a string literal holding the JSON.
  return { [RUNTIME_CONFIG_GLOBAL]: JSON.stringify(JSON.stringify(value)) };
}
