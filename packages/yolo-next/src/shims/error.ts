// SPDX-License-Identifier: Apache-2.0
// next/error compatible error boundary helpers: `catchError(fallback)` HOC that rethrows router control flow.

import { Component, createElement, type ComponentType, type ReactElement, type ReactNode } from "react";
import { isNextRouterError } from "../navigation.ts";

/** Props received by an error fallback (the `error.tsx` contract). */
export interface ErrorFallbackProps {
  readonly error: Error & { digest?: string };
  readonly reset: () => void;
  readonly retry: () => void;
}

/** Props of `YoloErrorBoundary`. */
export interface YoloErrorBoundaryProps {
  readonly fallback: ComponentType<ErrorFallbackProps>;
  readonly onError?: (error: Error) => void;
  readonly children?: ReactNode;
}

interface YoloErrorBoundaryState {
  readonly error: (Error & { digest?: string }) | null;
}

/** Error boundary rendering `fallback` for ordinary errors and rethrowing redirect / notFound control flow. */
export class YoloErrorBoundary extends Component<YoloErrorBoundaryProps, YoloErrorBoundaryState> {
  override state: YoloErrorBoundaryState = { error: null };

  /** Captures ordinary errors; Next.js control-flow errors propagate to the router. */
  static getDerivedStateFromError(error: unknown): YoloErrorBoundaryState {
    if (isNextRouterError(error)) {
      throw error;
    }
    return { error: error instanceof Error ? error : new Error(String(error)) };
  }

  override componentDidCatch(error: Error): void {
    this.props.onError?.(error);
  }

  readonly reset = (): void => {
    this.setState({ error: null });
  };

  override render(): ReactNode {
    const { error } = this.state;
    if (error) {
      return createElement(this.props.fallback, { error, reset: this.reset, retry: this.reset });
    }
    return this.props.children;
  }
}

/** HOC wrapping a component in `YoloErrorBoundary` with `fallback` (an `error.tsx` component). */
export function catchError<P extends object>(
  fallback: ComponentType<ErrorFallbackProps>,
): (Wrapped: ComponentType<P>) => ComponentType<P> {
  return (Wrapped) => {
    function WithErrorBoundary(props: P): ReactElement {
      return createElement(YoloErrorBoundary, { fallback }, createElement(Wrapped, props));
    }
    WithErrorBoundary.displayName = `catchError(${Wrapped.displayName ?? Wrapped.name ?? "Component"})`;
    return WithErrorBoundary;
  };
}

/** Props of the default `next/error` page component. */
export interface ErrorPageProps {
  readonly statusCode: number;
  readonly title?: string;
  readonly withDarkMode?: boolean;
}

const STATUS_TITLES: Readonly<Record<number, string>> = {
  400: "Bad Request",
  401: "Unauthorized",
  403: "Forbidden",
  404: "This page could not be found",
  405: "Method Not Allowed",
  500: "Internal Server Error",
};

/** Default `next/error` page (`<Error statusCode={404} />`). */
export default function ErrorPage(props: ErrorPageProps): ReactElement {
  const title = props.title ?? STATUS_TITLES[props.statusCode] ?? "An unexpected error has occurred";
  return createElement(
    "div",
    { style: { fontFamily: "system-ui, sans-serif", textAlign: "center", padding: "4rem 1rem" } },
    createElement("h1", { style: { display: "inline-block", margin: "0 1rem 0 0", fontSize: "1.5rem", fontWeight: 500 } }, String(props.statusCode)),
    createElement("p", { style: { display: "inline-block", margin: 0, fontSize: "1rem" } }, title),
  );
}
