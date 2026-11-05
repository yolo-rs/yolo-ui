// Which bundle this is, asked by the bundle itself. A leaf: no imports, so
// lib/capabilities.ts, api/backend.ts and lib/store-backup.ts may all ask
// without pulling each other along, and a test that mocks api/backend does
// not have to know that a web build exists.
//
// Three bundles come out of the same source:
//  - the desktop package (no VITE_LU_BUILD_TARGET), run inside Tauri, or in a
//    plain browser against the Vite dev server and its /local-api routes;
//  - the phone bundle (VITE_LU_BUILD_TARGET=mobile, lib/mobile-host.ts);
//  - the web bundle (VITE_LU_BUILD_TARGET=web): static files on any host,
//    no Tauri, no dev server. Everything native answers "requires the native
//    app" (api/native-required.ts), local model servers are reached by their
//    absolute address, keys go to the WebCrypto vault (lib/web-key-vault.ts)
//    and the store backup to the origin private file system
//    (lib/web-backup.ts).
//
// Read at call time, not once at import: Vite replaces the expression with a
// constant in a build, so the dead branch still falls out of the bundle, and
// vitest can switch it per test with vi.stubEnv.

export type BuildTarget = "desktop" | "mobile" | "web";

/** The bundle this code was built into. */
export function buildTarget(): BuildTarget {
  const named = import.meta.env.VITE_LU_BUILD_TARGET;
  if (named === "web") return "web";
  if (named === "mobile") return "mobile";
  return "desktop";
}

/** True in the static web bundle (VITE_LU_BUILD_TARGET=web). */
export function isWebBuild(): boolean {
  return import.meta.env.VITE_LU_BUILD_TARGET === "web";
}

/** Where the corresponding source of this build can be had (AGPL-3.0
 *  section 13). A build constant; the Yolo fork when nobody set one. */
export const DEFAULT_SOURCE_URL = "https://github.com/yolo-rs/locally-uncensored";

export function sourceUrl(): string {
  const named = import.meta.env.VITE_LU_SOURCE_URL;
  return typeof named === "string" && /^https?:\/\//i.test(named.trim()) ? named.trim() : DEFAULT_SOURCE_URL;
}

/** The commit the build was made from, or null when the build did not say. */
export function sourceCommit(): string | null {
  const named = import.meta.env.VITE_LU_SOURCE_COMMIT;
  if (typeof named !== "string") return null;
  const commit = named.trim();
  return /^[0-9a-f]{7,64}$/i.test(commit) ? commit : null;
}

/** The address of the exact source of this build: the commit's tree when the
 *  build named one (the `/tree/<commit>` form GitHub, GitLab, Gitea and
 *  Forgejo all answer), the repository otherwise. */
export function sourceLink(): string {
  const commit = sourceCommit();
  const base = sourceUrl().replace(/\/+$/, "").replace(/\.git$/i, "");
  return commit ? `${base}/tree/${commit}` : base;
}
