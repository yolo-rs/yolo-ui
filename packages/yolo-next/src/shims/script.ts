// SPDX-License-Identifier: Apache-2.0
// next/script shim: loading strategies plus onLoad / onReady / onError wiring on the client.

import { createElement, useEffect, type ReactElement, type ScriptHTMLAttributes } from "react";

/** Loading strategy of `next/script`. */
export type ScriptStrategy = "beforeInteractive" | "afterInteractive" | "lazyOnload" | "worker";

/** Props of `Script`. */
export interface ScriptProps extends Omit<ScriptHTMLAttributes<HTMLScriptElement>, "onLoad" | "onError" | "children"> {
  readonly strategy?: ScriptStrategy;
  readonly onLoad?: (event: Event) => void;
  readonly onReady?: () => void;
  readonly onError?: (event: Event | string) => void;
  readonly stylesheets?: ReadonlyArray<string>;
  readonly children?: string;
}

const loadedScripts = new Set<string>();
const injectedScripts = new Set<string>();
let warnedWorker = false;

function scriptKey(props: ScriptProps): string {
  return props.src ?? props.id ?? "";
}

function attachListeners(element: HTMLScriptElement, key: string, props: ScriptProps): void {
  const { onLoad, onReady, onError } = props;
  if (loadedScripts.has(key)) {
    onReady?.();
    return;
  }
  const handleLoad = (event: Event): void => {
    loadedScripts.add(key);
    onLoad?.(event);
    onReady?.();
  };
  element.addEventListener("load", handleLoad, { once: true });
  element.addEventListener(
    "error",
    (event) => {
      onError?.(event);
    },
    { once: true },
  );
}

function injectScript(props: ScriptProps): void {
  const key = scriptKey(props);
  if (!key) {
    return;
  }
  const existing = document.querySelector<HTMLScriptElement>(props.src ? `script[src="${CSS.escape(props.src)}"]` : `script#${CSS.escape(key)}`);
  if (existing) {
    attachListeners(existing, key, props);
    return;
  }
  if (injectedScripts.has(key)) {
    return;
  }
  injectedScripts.add(key);
  const element = document.createElement("script");
  attachListeners(element, key, props);
  const { src, id, children, dangerouslySetInnerHTML, strategy: _strategy, onLoad: _l, onReady: _r, onError: _e, stylesheets, ...rest } = props;
  for (const [name, value] of Object.entries(rest)) {
    if (typeof value === "string" || typeof value === "number") {
      element.setAttribute(name.replace(/[A-Z]/g, (char) => `-${char.toLowerCase()}`), String(value));
    } else if (value === true) {
      element.setAttribute(name, "");
    }
  }
  if (id) {
    element.id = id;
  }
  if (src) {
    element.src = src;
    element.async = true;
  } else {
    element.textContent = dangerouslySetInnerHTML?.__html !== undefined ? String(dangerouslySetInnerHTML.__html) : (children ?? "");
  }
  for (const href of stylesheets ?? []) {
    if (!document.querySelector(`link[href="${CSS.escape(href)}"]`)) {
      const link = document.createElement("link");
      link.rel = "stylesheet";
      link.href = href;
      document.head.appendChild(link);
    }
  }
  document.body.appendChild(element);
}

function scheduleIdle(callback: () => void): void {
  const run = (): void => {
    const idle = (window as Window & { requestIdleCallback?: (cb: () => void) => number }).requestIdleCallback;
    if (typeof idle === "function") {
      idle(callback);
    } else {
      setTimeout(callback, 1);
    }
  };
  if (document.readyState === "complete") {
    run();
  } else {
    window.addEventListener("load", run, { once: true });
  }
}

/** `next/script`: renders or injects a script per `strategy` and fires `onLoad` / `onReady` / `onError` on the client. */
export default function Script(props: ScriptProps): ReactElement | null {
  const { strategy = "afterInteractive", onLoad, onReady, onError, src, children, dangerouslySetInnerHTML, stylesheets: _stylesheets, ...rest } = props;
  const key = scriptKey(props);

  useEffect(() => {
    if (typeof document === "undefined" || !key) {
      return;
    }
    if (strategy === "lazyOnload") {
      scheduleIdle(() => injectScript(props));
      return;
    }
    const element = document.querySelector<HTMLScriptElement>(src ? `script[src="${CSS.escape(src)}"]` : `script#${CSS.escape(key)}`);
    if (element) {
      attachListeners(element, key, props);
    } else {
      injectScript(props);
    }
    // Listeners are attached once per script key.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, strategy]);

  if (strategy === "lazyOnload") {
    return null;
  }
  if (strategy === "worker" && !warnedWorker) {
    warnedWorker = true;
    console.warn("[yolo-next] next/script strategy=\"worker\" (Partytown) is not supported; falling back to afterInteractive.");
  }
  const inline = dangerouslySetInnerHTML ?? (children !== undefined ? { __html: children } : undefined);
  return createElement("script", {
    ...rest,
    src,
    async: strategy === "beforeInteractive" ? undefined : true,
    dangerouslySetInnerHTML: inline,
    "data-nscript": strategy,
  });
}
