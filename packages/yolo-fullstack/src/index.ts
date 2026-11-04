// SPDX-License-Identifier: MIT OR Apache-2.0
// `@yolo-rs/fullstack`: a Bun-native replacement for Next.js. The App Router runtime (RSC, server actions,
// streaming SSR, static generation) on Bun.build + Bun.serve, driven by next.config, with next/font, the
// image CDN endpoint, TanStack adapters and the Dioxus bridge.
export * from "./app/index";
export * from "./config/next-config";
export * from "./config/routes";
export {
  BUNDLER_FLAGS,
  LOADER_MAP,
  aliasConditions,
  pickAliasTarget,
  ruleGlobToRegExp,
  ruleLoader,
  stripBundlerFlags,
  svgrModule,
  turbopackPlugin,
} from "./config/turbopack";
export type { RuleLoader, TurbopackMapping, TurbopackPluginOptions } from "./config/turbopack";
export * from "./font/index";
export * from "./image/index";
export * from "./migrate/index";
export * as tanstack from "./tanstack/index";
export * as dioxus from "./dioxus/index";
