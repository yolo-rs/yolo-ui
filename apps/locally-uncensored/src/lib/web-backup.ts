/**
 * The store backup of the web build: the same snapshot the desktop writes to
 * %APPDATA%/store_backup.json, written to a file in the origin private file
 * system (OPFS) instead.
 *
 * WHY. lib/store-backup.ts hands its snapshot to `backup_stores`, and
 * AppShell reads it back with `restore_stores`. Both are Rust commands. In the
 * web build api/backend.ts routes the two here, so the triad, the restore on
 * boot and the chat recovery run unchanged.
 *
 * OPFS is a second storage layer next to localStorage and IndexedDB. It does
 * not survive "Clear site data" (nothing in the origin does), but it does
 * survive what the desktop backup was built for: IndexedDB discarded on its
 * own after a crash mid write, or a localStorage quota failure.
 *
 * THE MERGE. `backup_stores` in Rust does not replace the file, it carries
 * every key over that held a value before and is missing or empty in the
 * incoming snapshot (commands/system.rs, see the comment on
 * unreadableIdbKeys in store-backup.ts). The same rule is applied here, so an
 * incomplete snapshot never costs the previous value of a key.
 *
 * WITHOUT OPFS (an old engine, some private windows) there is nowhere to
 * write silently. The newest merged snapshot is kept for this session, and
 * the fallback is a downloaded JSON file: `downloadWebBackup()`. It is not
 * fired from the five-second tick, which would download a file per chat
 * message; it is fired by the explicit "back up now" paths
 * (store-backup.ts `backupStoresNow`) and by the footer of the web build.
 *
 * No imports from api/: store-backup.ts reaches this module, and its tests
 * mock api/backend with nothing but backendCall.
 */

export const WEB_BACKUP_FILE = "store_backup.json";
export const WEB_RAG_BACKUP_FILE = "rag_backup.json";

/** The part of the OPFS API this module uses. */
export interface BackupDirectory {
  getFileHandle(name: string, options?: { create?: boolean }): Promise<BackupFileHandle>;
}
export interface BackupFileHandle {
  getFile(): Promise<{ text(): Promise<string> }>;
  createWritable?: () => Promise<{ write(data: string): Promise<void>; close(): Promise<void> }>;
}

export interface WebBackupEnv {
  /** The OPFS root, or null where there is none. */
  directory: () => Promise<BackupDirectory | null>;
  /** Hands a file to the user as a download. */
  download: (name: string, text: string) => void;
}

function browserDirectory(): Promise<BackupDirectory | null> {
  const storage = typeof navigator !== "undefined" ? navigator.storage : undefined;
  if (!storage || typeof storage.getDirectory !== "function") return Promise.resolve(null);
  return storage.getDirectory().then(
    (dir) => dir as unknown as BackupDirectory,
    () => null,
  );
}

function browserDownload(name: string, text: string): void {
  if (typeof document === "undefined") return;
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

export const browserBackupEnv: WebBackupEnv = { directory: browserDirectory, download: browserDownload };

/** True when `text` is a JSON object (the only shape a snapshot has). */
function parseObject(text: string | null): Record<string, unknown> | null {
  if (!text) return null;
  try {
    const parsed: unknown = JSON.parse(text);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? (parsed as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

/**
 * The Rust merge: every key of `previous` that held a non-empty string and is
 * missing or empty in `incoming` is carried over. `incoming` wins for every
 * key it has a value for. A `previous` that is not a JSON object is ignored.
 * Throws when `incoming` is not a JSON object, exactly like the Rust command
 * refuses a body it cannot parse.
 */
export function mergeBackup(previous: string | null, incoming: string): string {
  const next = parseObject(incoming);
  if (!next) throw new Error("backup_stores: the snapshot is not a JSON object");
  const prev = parseObject(previous);
  if (prev) {
    for (const [key, value] of Object.entries(prev)) {
      if (typeof value !== "string" || value === "") continue;
      const now = next[key];
      if (now === undefined || now === null || now === "") next[key] = value;
    }
  }
  return JSON.stringify(next);
}

async function readFile(dir: BackupDirectory, name: string): Promise<string | null> {
  try {
    const handle = await dir.getFileHandle(name);
    return await (await handle.getFile()).text();
  } catch {
    return null; // NotFoundError: no backup yet
  }
}

async function writeFile(dir: BackupDirectory, name: string, text: string): Promise<boolean> {
  const handle = await dir.getFileHandle(name, { create: true });
  // createWritable writes to a swap file and swaps it in on close(), so a tab
  // closed mid write leaves the previous file, not half of the new one.
  if (typeof handle.createWritable !== "function") return false;
  const writable = await handle.createWritable();
  await writable.write(text);
  await writable.close();
  return true;
}

/** The newest snapshot of this session, kept for the download fallback. */
let pendingDownload: string | null = null;

export type WebBackupWrite = "opfs" | "kept-for-download";

/**
 * What `backup_stores` does in the web build. Writes the merged snapshot to
 * OPFS. Where there is none, keeps it (merged with the one kept before) for
 * `downloadWebBackup` and says so.
 */
export async function writeWebBackup(data: string, env: WebBackupEnv = browserBackupEnv): Promise<WebBackupWrite> {
  const dir = await env.directory();
  if (dir) {
    const merged = mergeBackup(await readFile(dir, WEB_BACKUP_FILE), data);
    if (await writeFile(dir, WEB_BACKUP_FILE, merged)) {
      pendingDownload = null;
      return "opfs";
    }
  }
  pendingDownload = mergeBackup(pendingDownload, data);
  return "kept-for-download";
}

/** What `restore_stores` answers in the web build: the OPFS file, or null
 *  when there is none (or no OPFS: a downloaded file is the user's to bring
 *  back, the page cannot read it unasked). */
export async function readWebBackup(env: WebBackupEnv = browserBackupEnv): Promise<string | null> {
  const dir = await env.directory();
  if (!dir) return null;
  return readFile(dir, WEB_BACKUP_FILE);
}

/** True when OPFS can take the backup on this page. */
export async function webBackupHasFileSystem(env: WebBackupEnv = browserBackupEnv): Promise<boolean> {
  return (await env.directory()) !== null;
}

/** A plain file of the given name (the RAG backup): no merge, no download
 *  fallback, that backup is a cache of re-buildable embeddings. */
export async function writeWebBackupFile(name: string, data: string, env: WebBackupEnv = browserBackupEnv): Promise<boolean> {
  const dir = await env.directory();
  return dir ? writeFile(dir, name, data) : false;
}

export async function readWebBackupFile(name: string, env: WebBackupEnv = browserBackupEnv): Promise<string | null> {
  const dir = await env.directory();
  return dir ? readFile(dir, name) : null;
}

/**
 * The fallback: hand the newest snapshot of this session to the user as a
 * JSON file. Only when there is no OPFS file to rely on, and only when a
 * snapshot was taken. Returns whether a download was started.
 */
export function downloadWebBackup(env: WebBackupEnv = browserBackupEnv, now: Date = new Date()): boolean {
  if (!pendingDownload) return false;
  const stamp = now.toISOString().slice(0, 19).replace(/[:T]/g, "-");
  env.download(`locally-uncensored-backup-${stamp}.json`, pendingDownload);
  return true;
}

/** True when a snapshot waits for the download fallback. */
export function hasPendingWebBackupDownload(): boolean {
  return pendingDownload !== null;
}

/** Test seam. */
export function resetWebBackupForTests(): void {
  pendingDownload = null;
}
