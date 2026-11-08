// SPDX-License-Identifier: Apache-2.0
// next/dynamic shim: React.lazy + Suspense; `ssr: false` renders the loading state until mounted.

import {
  Suspense,
  createElement,
  lazy,
  useEffect,
  useState,
  type ComponentType,
  type ReactElement,
  type ReactNode,
} from "react";

/** Module shapes accepted by the dynamic loader. */
export type DynamicLoader<P> = () => Promise<ComponentType<P> | { readonly default: ComponentType<P> }>;

/** Props passed to the `loading` component. */
export interface DynamicLoadingProps {
  readonly error?: Error | null;
  readonly isLoading?: boolean;
  readonly pastDelay?: boolean;
  readonly retry?: () => void;
  readonly timedOut?: boolean;
}

/** Options of `dynamic()`. */
export interface DynamicOptions<P> {
  readonly ssr?: boolean;
  readonly loading?: (props: DynamicLoadingProps) => ReactNode;
  readonly loader?: DynamicLoader<P>;
  readonly suspense?: boolean;
}

function normalizeModule<P>(module: ComponentType<P> | { readonly default: ComponentType<P> }): { default: ComponentType<P> } {
  if (typeof module === "object" && module !== null && "default" in module) {
    return { default: module.default };
  }
  return { default: module as ComponentType<P> };
}

/** `next/dynamic`: lazy component wrapped in Suspense; with `ssr: false` the `loading` state renders on the server. */
export default function dynamic<P extends object = Record<string, never>>(
  loader: DynamicLoader<P> | DynamicOptions<P>,
  options: DynamicOptions<P> = {},
): ComponentType<P> {
  const resolvedLoader = typeof loader === "function" ? loader : loader.loader;
  const settings: DynamicOptions<P> = typeof loader === "function" ? options : { ...loader, ...options };
  if (!resolvedLoader) {
    throw new Error("next/dynamic requires a loader: dynamic(() => import('./Component'))");
  }
  const Lazy = lazy(async () => normalizeModule(await resolvedLoader()));
  const Loading = settings.loading;
  const fallback = Loading ? Loading({ isLoading: true, pastDelay: true, error: null }) : null;
  const ssr = settings.ssr !== false;

  function DynamicComponent(props: P): ReactElement {
    const [mounted, setMounted] = useState(ssr);
    useEffect(() => {
      if (!ssr) {
        setMounted(true);
      }
    }, []);
    if (!mounted) {
      return createElement(Suspense, { fallback }, fallback);
    }
    return createElement(Suspense, { fallback }, createElement(Lazy as ComponentType<P>, props));
  }
  DynamicComponent.displayName = "LoadableComponent";
  return DynamicComponent;
}
