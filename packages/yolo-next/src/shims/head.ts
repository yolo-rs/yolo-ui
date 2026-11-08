// SPDX-License-Identifier: Apache-2.0
// next/head shim: portals children into document.head on the client; collects them for SSR.

import type { ReactElement, ReactNode } from "react";
import { createPortal } from "react-dom";

const collected: ReactNode[] = [];

/** `next/head`: renders children into `document.head` on the client; on the server they are collected for `collectHead()`. */
export default function Head(props: { readonly children?: ReactNode }): ReactElement | null {
  if (typeof document === "undefined") {
    collected.push(props.children);
    return null;
  }
  return createPortal(props.children, document.head);
}

/** Returns and clears the head nodes collected while rendering on the server. */
export function collectHead(): ReactNode[] {
  const nodes = [...collected];
  collected.length = 0;
  return nodes;
}
