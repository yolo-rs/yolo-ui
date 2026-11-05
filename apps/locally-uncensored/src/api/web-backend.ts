/**
 * `backendCall` in the web build (VITE_LU_BUILD_TARGET=web).
 *
 * The desktop routes every command to Rust, a browser next to the dev server
 * routes it to /local-api. The web build has neither, so each command is one
 * of two things here:
 *
 *  - a command with a browser path, answered in the page: the store backup
 *    (OPFS, lib/web-backup.ts), and the ComfyUI address, which the browser
 *    reaches directly and which is kept in localStorage instead of Rust's
 *    config.json;
 *  - everything else, which needs the native app: shell, file system jail,
 *    process control (ComfyUI, the engine, Whisper, Piper, MLX), computer
 *    control, native dialogs, downloads to disk, Remote Access. These answer
 *    NativeAppRequiredError instead of fetching a route that is not there.
 *
 * The Ollama address needs no command: the providers store holds it and
 * AppShell hands it to api/backend.ts (`setOllamaBase`). `get_ollama_host`
 * answers "native only", which is what makes AppShell keep the stored one.
 *
 * Kept free of api/backend.ts: the URL builder is handed in, so this module
 * closes no import circle and its tests need no mock of the backend.
 */

import { NativeAppRequiredError } from "./native-required";
import {
  readWebBackup,
  readWebBackupFile,
  writeWebBackup,
  writeWebBackupFile,
  WEB_RAG_BACKUP_FILE,
} from "../lib/web-backup";

/** The part of api/backend.ts the ComfyUI commands touch. */
export interface ComfyAddress {
  getHost(): string;
  setHost(host: string): void;
  getPort(): number;
  setPort(port: number): void;
}

/** localStorage key of the ComfyUI address in the web build. */
export const WEB_COMFY_KEY = "lu-web-comfyui";

/** The default ComfyUI address of the web build. 127.0.0.1 and not
 *  localhost: a browser may resolve localhost to ::1 first, where ComfyUI's
 *  default `--listen 127.0.0.1` does not answer. */
export const WEB_COMFY_DEFAULT = { host: "127.0.0.1", port: 8188 } as const;

const HOST_RE = /^[a-z0-9.-]{1,253}$|^\[[0-9a-f:]+\]$/i;

function readStoredComfy(): { host?: string; port?: number } {
  try {
    const raw = typeof localStorage !== "undefined" ? localStorage.getItem(WEB_COMFY_KEY) : null;
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    if (!parsed || typeof parsed !== "object") return {};
    const p = parsed as { host?: unknown; port?: unknown };
    return {
      host: typeof p.host === "string" && HOST_RE.test(p.host) ? p.host : undefined,
      port: typeof p.port === "number" && p.port > 0 && p.port <= 65535 ? p.port : undefined,
    };
  } catch {
    return {};
  }
}

function storeComfy(next: { host?: string; port?: number }): void {
  try {
    localStorage.setItem(WEB_COMFY_KEY, JSON.stringify({ ...readStoredComfy(), ...next }));
  } catch { /* quota: the address still applies for this session */ }
}

/** Puts the stored ComfyUI address into the URL builder. Called once at boot
 *  of the web build (and harmless again). */
export function applyStoredComfyAddress(comfy: ComfyAddress): void {
  const stored = readStoredComfy();
  if (stored.host) comfy.setHost(stored.host);
  if (stored.port) comfy.setPort(stored.port);
}

function isLoopback(host: string): boolean {
  const h = host.toLowerCase().replace(/^\[|\]$/g, "");
  return h === "localhost" || h === "127.0.0.1" || h === "::1";
}

async function comfyRunning(comfy: ComfyAddress): Promise<boolean> {
  const host = comfy.getHost();
  const shown = host.includes(":") && !host.startsWith("[") ? `[${host}]` : host;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 3_000);
  try {
    // A CORS refusal lands here as a TypeError, the same as "nothing
    // listens": both mean this page cannot use it. YOLO-INTEGRATION.md names
    // the ComfyUI flag that lets the page in.
    const res = await fetch(`http://${shown}:${comfy.getPort()}/system_stats`, { signal: controller.signal });
    return res.ok;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/** Every command that has a browser path. Exported for the test that keeps
 *  the list honest. */
export const WEB_COMMANDS = [
  "backup_stores",
  "restore_stores",
  "backup_rag_chunks",
  "restore_rag_chunks",
  "comfyui_status",
  "set_comfyui_host",
  "set_comfyui_port",
] as const;

function stringArg(args: Record<string, unknown> | undefined, name: string): string {
  const v = args?.[name];
  if (typeof v !== "string") throw new Error(`${name} is required`);
  return v;
}

export async function webBackendCall<T = unknown>(
  command: string,
  args: Record<string, unknown> | undefined,
  comfy: ComfyAddress,
): Promise<T> {
  switch (command) {
    case "backup_stores":
      await writeWebBackup(stringArg(args, "data"));
      return undefined as T;
    case "restore_stores":
      return (await readWebBackup()) as T;
    case "backup_rag_chunks":
      await writeWebBackupFile(WEB_RAG_BACKUP_FILE, stringArg(args, "data"));
      return undefined as T;
    case "restore_rag_chunks":
      return (await readWebBackupFile(WEB_RAG_BACKUP_FILE)) as T;
    case "comfyui_status": {
      applyStoredComfyAddress(comfy);
      const running = await comfyRunning(comfy);
      // `isLocal: false` is what the field gates in Settings and in
      // comfy-idle-watch: whether LU may start, stop, install or point a path
      // at this ComfyUI. From a web page it may not, wherever it runs, so the
      // panel shows the address fields and the "manage the process yourself"
      // line, and none of the process buttons.
      return {
        running,
        starting: false,
        found: false,
        path: null,
        logs: [],
        processAlive: false,
        ownedByApp: false,
        host: comfy.getHost(),
        port: comfy.getPort(),
        isLocal: false,
        web: true,
        loopback: isLoopback(comfy.getHost()),
      } as T;
    }
    case "set_comfyui_host": {
      const host = stringArg(args, "host").trim();
      if (!HOST_RE.test(host)) throw new Error("Invalid host");
      comfy.setHost(host);
      storeComfy({ host });
      return undefined as T;
    }
    case "set_comfyui_port": {
      const port = Number(args?.port);
      if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("Invalid port");
      comfy.setPort(port);
      storeComfy({ port });
      return undefined as T;
    }
    default:
      throw new NativeAppRequiredError(command);
  }
}
