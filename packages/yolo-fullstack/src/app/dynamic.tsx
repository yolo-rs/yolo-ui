"use client";
// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/dynamic`: a lazily loaded component. The loader may resolve to a module (`{ default }`) or to the
// component itself; `loading` is the Suspense fallback; `ssr: false` renders the fallback until the
// component is mounted in the browser, so the server never loads the module.
import { Suspense, lazy, useEffect, useState, type ComponentType, type ReactElement } from "react";

/** Props the `loading` component receives (the subset Next.js documents). */
export interface LoadingProps {
  error?: Error | null;
  isLoading?: boolean;
  pastDelay?: boolean;
}

type ComponentModule<P> = { default: ComponentType<P> } | ComponentType<P>;
export type Loader<P> = () => Promise<ComponentModule<P>>;

export interface DynamicOptions<P = object> {
  loading?: ComponentType<LoadingProps>;
  ssr?: boolean;
  /** Legacy object form: `dynamic({ loader, loading, ssr })`. */
  loader?: Loader<P>;
}

const isModule = <P,>(value: ComponentModule<P>): value is { default: ComponentType<P> } =>
  typeof value === "object" && value !== null && "default" in value;

function useMounted(): boolean {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  return mounted;
}

export default function dynamic<P extends object = object>(
  loaderOrOptions: Loader<P> | DynamicOptions<P>,
  options: DynamicOptions<P> = {},
): ComponentType<P> {
  const merged: DynamicOptions<P> =
    typeof loaderOrOptions === "function"
      ? { ...options, loader: loaderOrOptions }
      : { ...loaderOrOptions, ...options };
  const loader = merged.loader;
  if (!loader) throw new Error("@yolo-rs/fullstack: dynamic() needs a loader");
  const Loading = merged.loading;
  const ssr = merged.ssr !== false;
  const Lazy = lazy(async () => {
    const loaded = await loader();
    return { default: isModule(loaded) ? loaded.default : loaded };
  });
  const fallback = Loading ? <Loading isLoading pastDelay error={null} /> : null;

  function Dynamic(props: P): ReactElement | null {
    // `ssr: false` keeps the server (and the hydration pass) on the fallback; the module loads after mount.
    const mounted = useMounted();
    if (!ssr && !mounted) return fallback;
    return (
      <Suspense fallback={fallback}>
        <Lazy {...props} />
      </Suspense>
    );
  }
  Dynamic.displayName = "Dynamic";
  return Dynamic;
}
