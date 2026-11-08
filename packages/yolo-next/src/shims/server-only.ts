// SPDX-License-Identifier: Apache-2.0
// server-only shim: importing this module from a browser bundle throws.

if (typeof window !== "undefined") {
  throw new Error(
    "This module cannot be imported from a Client Component module. It should only be used from a Server Component.",
  );
}

/** Marker constant (the module's only purpose is its import-time check). */
export const serverOnly = true;
