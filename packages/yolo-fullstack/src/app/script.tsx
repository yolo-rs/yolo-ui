"use client";
// SPDX-License-Identifier: MIT OR Apache-2.0
// `next/script`: `beforeInteractive`, `afterInteractive` and `worker` render `<script async src>` (React 19
// hoists and deduplicates async scripts); `lazyOnload` injects the script after the window `load` event.
// Inline scripts (`children` or `dangerouslySetInnerHTML`) are rendered as is. A `src` is injected once.
import { useEffect, type ReactNode, type ScriptHTMLAttributes, type SyntheticEvent } from "react";

export type ScriptStrategy = "beforeInteractive" | "afterInteractive" | "lazyOnload" | "worker";

export interface ScriptProps extends Omit<ScriptHTMLAttributes<HTMLScriptElement>, "onLoad" | "onError" | "children"> {
  src?: string;
  id?: string;
  strategy?: ScriptStrategy;
  onLoad?: (event: Event | SyntheticEvent<HTMLScriptElement>) => void;
  onError?: (event: Event | SyntheticEvent<HTMLScriptElement>) => void;
  /** Runs after load and on every mount once the script is loaded. */
  onReady?: () => void;
  children?: ReactNode;
}

/** `src` values already injected or loaded in this realm. */
const injected = new Set<string>();

/** Reset the injection registry (tests). */
export function resetScriptRegistry(): void {
  injected.clear();
}

function inlineSource(children: ReactNode, html: string | TrustedHTML | undefined): string | undefined {
  if (html !== undefined) return String(html);
  if (children === undefined || children === null) return undefined;
  const parts = Array.isArray(children) ? children : [children];
  return parts.map(part => (typeof part === "string" || typeof part === "number" ? String(part) : "")).join("");
}

export default function Script({
  src,
  strategy = "afterInteractive",
  onLoad,
  onError,
  onReady,
  id,
  children,
  dangerouslySetInnerHTML,
  ...rest
}: ScriptProps) {
  const inline = inlineSource(children, dangerouslySetInnerHTML?.__html);
  const lazy = strategy === "lazyOnload";

  useEffect(() => {
    if (!src) return;
    if (!lazy) {
      // The rendered `<script>` loads itself; mark it and honour `onReady` for later mounts.
      if (injected.has(src)) onReady?.();
      else injected.add(src);
      return;
    }
    if (injected.has(src)) {
      onReady?.();
      return;
    }
    injected.add(src);
    const run = () => {
      const el = document.createElement("script");
      el.src = src;
      el.async = true;
      if (id) el.id = id;
      for (const [key, value] of Object.entries(rest)) {
        if (value === undefined || value === null || typeof value === "function") continue;
        el.setAttribute(key.toLowerCase() === "classname" ? "class" : key, String(value));
      }
      el.addEventListener("load", event => {
        onLoad?.(event);
        onReady?.();
      });
      el.addEventListener("error", event => onError?.(event));
      document.body.appendChild(el);
    };
    if (document.readyState === "complete") run();
    else {
      window.addEventListener("load", run, { once: true });
      return () => window.removeEventListener("load", run);
    }
    // Keep the deps to the identity of the script: handlers change without re-injecting.
  }, [src, lazy, id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (lazy && src) return null;
  if (src) {
    return (
      <script
        {...rest}
        id={id}
        src={src}
        async
        onLoad={event => {
          onLoad?.(event);
          onReady?.();
        }}
        onError={event => onError?.(event)}
      />
    );
  }
  if (inline !== undefined) return <script {...rest} id={id} dangerouslySetInnerHTML={{ __html: inline }} />;
  return null;
}
