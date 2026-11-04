// SPDX-License-Identifier: MIT OR Apache-2.0
// Dioxus 0.8 bridge: server function calls, API proxy, dx web build serving.

import { dioxusProxy, type DioxusProxy } from "./proxy";
import { dioxusWebRoutes, type RouteHandler } from "./web";

export {
  callServerFn,
  DioxusServerFnError,
  serverFnPath,
  xxh64,
  xxh64Decimal,
  type CallServerFnOptions,
  type ServerFnInput,
  type ServerFnPathOptions,
} from "./server-fn";
export {
  dioxusProxy,
  forwardedHeaders,
  type DioxusProxy,
  type DioxusProxyOptions,
} from "./proxy";
export {
  contentTypeFor,
  dioxusWebRoutes,
  isUnsafeRelativePath,
  normalizeWebPrefix,
  resolvePublicFile,
  type DioxusWebRoutesOptions,
  type RouteHandler,
} from "./web";
export {
  dxBuild,
  findDxPublicDir,
  type DxBuildOptions,
  type DxBuildResult,
} from "./dx-build";

export interface DioxusBridgeOptions {
  /** Dioxus fullstack server origin receiving server function calls. */
  target: string;
  /** dx web public directory served as the frontend. */
  dir: string;
  /** Server function prefix; defaults to "/api". */
  apiPrefix?: string;
  /** Frontend mount prefix; defaults to "/". */
  webPrefix?: string;
  spa?: boolean;
  fetch?: typeof fetch;
}

export interface DioxusBridge {
  routes: Record<string, RouteHandler>;
  /** Fallback for Bun.serve `fetch`: proxies API paths, otherwise 404. */
  fetch: (request: Request) => Promise<Response>;
  proxy: DioxusProxy;
}

/** Combine the API proxy and the static web routes into one Bun.serve configuration. */
export function dioxusBridge(options: DioxusBridgeOptions): DioxusBridge {
  const proxy = dioxusProxy({
    target: options.target,
    prefix: options.apiPrefix,
    fetch: options.fetch,
  });
  const routes: Record<string, RouteHandler> = {
    [proxy.prefix === "/" ? "/*" : `${proxy.prefix}/*`]: proxy,
    ...dioxusWebRoutes({
      dir: options.dir,
      prefix: options.webPrefix,
      spa: options.spa,
    }),
  };
  const fallback = async (request: Request): Promise<Response> => {
    const pathname = new URL(request.url).pathname;
    if (proxy.matches(pathname)) return proxy(request);
    return new Response("Not Found", { status: 404 });
  };
  return { routes, fetch: fallback, proxy };
}
