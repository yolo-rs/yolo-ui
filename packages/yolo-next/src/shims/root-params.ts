// SPDX-License-Identifier: Apache-2.0
// next/root-params shim: per-param async accessors reading the request store (or a static fallback).

import { peekRequestStore } from "../headers.ts";

/** Root params record. */
export type RootParams = Record<string, string | string[]>;

const ROOT_PARAMS_GLOBAL = "__YOLO_ROOT_PARAMS__";

/** Sets the root params used outside a request store (static generation, tests). */
export function setRootParams(params: RootParams | undefined): void {
  (globalThis as Record<string, unknown>)[ROOT_PARAMS_GLOBAL] = params;
}

function staticRootParams(): RootParams {
  const params = (globalThis as Record<string, unknown>)[ROOT_PARAMS_GLOBAL];
  return typeof params === "object" && params !== null ? (params as RootParams) : {};
}

/** Creates the accessor exported by `next/root-params` for one root param (`export const lang = createRootParamsAccessor("lang")`). */
export function createRootParamsAccessor(name: string): () => Promise<string | string[] | undefined> {
  return async () => {
    try {
      const value = peekRequestStore()?.params[name];
      if (value !== undefined) {
        return value;
      }
    } catch {
      // Outside a request scope: fall back to the static params.
    }
    return staticRootParams()[name];
  };
}

/** Deprecated `unstable_rootParams()`: every root param of the current request. */
export async function unstable_rootParams(): Promise<RootParams> {
  try {
    const params = peekRequestStore()?.params;
    if (params) {
      return { ...params };
    }
  } catch {
    // Outside a request scope.
  }
  return { ...staticRootParams() };
}
