// SPDX-License-Identifier: Apache-2.0
// client-only shim: importing this module in the RSC layer (YOLO_RSC_LAYER=1) throws.

if (typeof window === "undefined" && typeof Bun !== "undefined" && Bun.env.YOLO_RSC_LAYER === "1") {
  throw new Error(
    "This module cannot be imported from a Server Component module. It should only be used from a Client Component.",
  );
}

/** Marker constant (the module's only purpose is its import-time check). */
export const clientOnly = true;
