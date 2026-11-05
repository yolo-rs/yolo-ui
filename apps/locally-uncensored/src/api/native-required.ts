/**
 * The answer of the web build to a command only the desktop app can run.
 *
 * In the desktop package a browser outside Tauri reaches the Vite dev server,
 * whose /local-api routes stand in for the Rust commands. The static web
 * build has no such server: a fetch of /local-api would come back as the
 * host's 404 page (or worse, its index.html with status 200), which every
 * caller would read as a broken backend. So the web build does not ask; it
 * says what is true, in one shape callers can test for.
 *
 * A leaf without imports, like lib/build-target.ts.
 */

export const NATIVE_REQUIRED = "native_required" as const;

export class NativeAppRequiredError extends Error {
  readonly code = NATIVE_REQUIRED;
  readonly command: string;

  constructor(command: string) {
    super(`${command} requires the native app (desktop build of Locally Uncensored); the web build has no browser path for it.`);
    this.name = "NativeAppRequiredError";
    this.command = command;
  }
}

/** True for the error above, also across a module copy (vi.resetModules):
 *  asked by its code, not by its class. */
export function isNativeAppRequired(err: unknown): err is NativeAppRequiredError {
  return !!err && typeof err === "object" && (err as { code?: unknown }).code === NATIVE_REQUIRED;
}
