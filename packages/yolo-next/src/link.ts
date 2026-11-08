// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/link: next/link compatible Link component built on the Yolo router context.

import * as React from "react";
import {
  createElement,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useTransition,
  type AnchorHTMLAttributes,
  type MouseEvent,
  type ReactElement,
  type ReactNode,
  type Ref,
  type TouchEvent,
} from "react";
import { LinkStatusContext, useLinkStatus, useYoloRouter } from "./navigation.ts";

export { useLinkStatus };

/** Query value accepted inside a `UrlObject.query`. */
export type UrlQueryValue = string | number | boolean | ReadonlyArray<string | number | boolean> | null | undefined;

/** URL object accepted by `href` (Node `UrlObject` subset used by Next.js). */
export interface UrlObject {
  readonly href?: string | null;
  readonly pathname?: string | null;
  readonly query?: Readonly<Record<string, UrlQueryValue>> | string | null;
  readonly search?: string | null;
  readonly hash?: string | null;
}

/** Event passed to `onNavigate`. */
export interface LinkNavigateEvent {
  preventDefault(): void;
}

/** Props of `Link` (`next/link` `LinkProps` plus anchor attributes). */
export interface LinkProps
  extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href" | "onClick" | "onMouseEnter" | "onTouchStart"> {
  readonly href: string | UrlObject;
  readonly as?: string | UrlObject;
  readonly replace?: boolean;
  readonly scroll?: boolean;
  readonly shallow?: boolean;
  readonly passHref?: boolean;
  readonly prefetch?: boolean | "auto" | null;
  readonly locale?: string | false;
  readonly onNavigate?: (event: LinkNavigateEvent) => void;
  readonly transitionTypes?: ReadonlyArray<string>;
  readonly onClick?: (event: MouseEvent<HTMLAnchorElement>) => void;
  readonly onMouseEnter?: (event: MouseEvent<HTMLAnchorElement>) => void;
  readonly onTouchStart?: (event: TouchEvent<HTMLAnchorElement>) => void;
  readonly ref?: Ref<HTMLAnchorElement>;
  readonly children?: ReactNode;
}

/** Formats a `UrlObject` into an href string (`{ pathname, query, hash }` -> `/p?q=1#h`). */
export function formatHref(url: UrlObject): string {
  if (typeof url.href === "string" && url.href.length > 0) {
    return url.href;
  }
  let out = url.pathname ?? "";
  const query = url.query;
  if (typeof query === "string") {
    if (query.length > 0) {
      out += query.startsWith("?") ? query : `?${query}`;
    }
  } else if (query && typeof query === "object") {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      if (value === undefined || value === null) {
        continue;
      }
      if (Array.isArray(value)) {
        for (const item of value as ReadonlyArray<string | number | boolean>) {
          params.append(key, String(item));
        }
      } else {
        params.append(key, String(value));
      }
    }
    const serialized = params.toString();
    if (serialized.length > 0) {
      out += `?${serialized}`;
    }
  } else if (typeof url.search === "string" && url.search.length > 0) {
    out += url.search.startsWith("?") ? url.search : `?${url.search}`;
  }
  if (typeof url.hash === "string" && url.hash.length > 0) {
    out += url.hash.startsWith("#") ? url.hash : `#${url.hash}`;
  }
  return out;
}

/** Resolves a string or `UrlObject` href to a string. */
export function resolveHref(href: string | UrlObject): string {
  return typeof href === "string" ? href : formatHref(href);
}

/** True when `href` targets the current origin (or is relative), so it can be soft-navigated. */
export function isLocalURL(href: string): boolean {
  if (/^(?:[a-z][a-z0-9+.-]*:)?\/\//i.test(href)) {
    if (typeof window === "undefined") {
      return false;
    }
    try {
      return new URL(href, window.location.href).origin === window.location.origin;
    } catch {
      return false;
    }
  }
  return !/^[a-z][a-z0-9+.-]*:/i.test(href);
}

/** True for clicks the browser must handle itself (new tab, modifier keys, middle button). */
export function isModifiedEvent(event: MouseEvent<HTMLAnchorElement>): boolean {
  const target = event.currentTarget.getAttribute("target");
  return (
    (target !== null && target !== "" && target !== "_self") ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey ||
    event.button === 1 ||
    (event.nativeEvent as { which?: number } | undefined)?.which === 2
  );
}

type AddTransitionType = (type: string) => void;

function addTransitionTypeFn(): AddTransitionType | undefined {
  const api = React as unknown as { addTransitionType?: AddTransitionType; unstable_addTransitionType?: AddTransitionType };
  return api.addTransitionType ?? api.unstable_addTransitionType;
}

/** `next/link`: client-side navigation anchor with hover and viewport prefetching. */
export function Link(props: LinkProps): ReactElement {
  const {
    href: hrefProp,
    as: asProp,
    replace = false,
    scroll = true,
    prefetch = null,
    onNavigate,
    transitionTypes,
    onClick,
    onMouseEnter,
    onTouchStart,
    children,
    ref,
    shallow: _shallow,
    passHref: _passHref,
    locale: _locale,
    ...anchorProps
  } = props;
  const router = useYoloRouter();
  const href = useMemo(() => resolveHref(hrefProp), [hrefProp]);
  const displayHref = useMemo(() => (asProp === undefined ? href : resolveHref(asProp)), [asProp, href]);
  const [pending, startTransition] = useTransition();
  const anchorRef = useRef<HTMLAnchorElement | null>(null);
  const prefetchEnabled = prefetch !== false;
  const routerPrefetch = router.prefetch;
  const routerNavigate = router.navigate;

  const doPrefetch = useCallback((): void => {
    if (prefetchEnabled && isLocalURL(href)) {
      routerPrefetch(href);
    }
  }, [href, prefetchEnabled, routerPrefetch]);

  useEffect(() => {
    const node = anchorRef.current;
    if (!prefetchEnabled || !node || typeof IntersectionObserver === "undefined") {
      return undefined;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          doPrefetch();
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [doPrefetch, prefetchEnabled]);

  const setRef = useCallback(
    (node: HTMLAnchorElement | null): void => {
      anchorRef.current = node;
      if (typeof ref === "function") {
        ref(node);
      } else if (ref) {
        ref.current = node;
      }
    },
    [ref],
  );

  const handleClick = (event: MouseEvent<HTMLAnchorElement>): void => {
    onClick?.(event);
    if (event.defaultPrevented) {
      return;
    }
    const download = anchorProps.download;
    if (!isLocalURL(href) || isModifiedEvent(event) || (download !== undefined && download !== false)) {
      return;
    }
    event.preventDefault();
    if (onNavigate) {
      let prevented = false;
      onNavigate({
        preventDefault: () => {
          prevented = true;
        },
      });
      if (prevented) {
        return;
      }
    }
    startTransition(async () => {
      const addTransitionType = addTransitionTypeFn();
      if (addTransitionType && transitionTypes) {
        for (const type of transitionTypes) {
          addTransitionType(type);
        }
      }
      await routerNavigate(href, { replace, scroll });
    });
  };

  const status = useMemo(() => ({ pending }), [pending]);
  const anchor = createElement(
    "a",
    {
      ...anchorProps,
      href: displayHref,
      ref: setRef,
      onClick: handleClick,
      onMouseEnter: (event: MouseEvent<HTMLAnchorElement>) => {
        onMouseEnter?.(event);
        doPrefetch();
      },
      onTouchStart: (event: TouchEvent<HTMLAnchorElement>) => {
        onTouchStart?.(event);
        doPrefetch();
      },
    },
    children,
  );
  return createElement(LinkStatusContext.Provider, { value: status }, anchor);
}
