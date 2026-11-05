/**
 * The key vault of the web build: API keys sealed with AES-GCM under a key the
 * page itself cannot read.
 *
 * WHY. The desktop app keeps provider keys in the OS credential store (the
 * Rust secret_* commands). A browser has none, and until now the web build
 * fell back to localStorage under base64 of the reversed string
 * (stores/providerStore.ts, `obfuscate`). That is an encoding, not
 * encryption: anything that can read localStorage, a stray extension, a
 * copied profile folder, a support screenshot of devtools, reads the key.
 *
 * WHAT IT IS.
 *  - One AES-GCM 256 key, generated once with `extractable: false` and kept
 *    in IndexedDB. IndexedDB stores the CryptoKey object by structured clone;
 *    the raw key bytes never exist in JavaScript, so no script on the page,
 *    this one included, can export them. What it can do is ask the browser to
 *    encrypt or decrypt with it.
 *  - Each secret sealed with a fresh 96-bit IV, the account name bound in as
 *    additional data, so a sealed value moved to another account does not
 *    open.
 *  - The sealed values live in the same database, never in localStorage.
 *
 * WHAT IT IS NOT. It does not protect a key from code running in this origin
 * (that code can simply ask for the decryption). It protects the key at rest:
 * on disk, in a profile copy, in a storage dump, in a store backup.
 *
 * The interface mirrors the Rust secret_* commands (api/backend.ts calls it
 * there in a web build), so every caller keeps its own migration path: the
 * first read that resolves proves the vault, a null answer moves the old
 * base64 value in (providerStore.hydrateProviderKeys, workflowStore) and the
 * localStorage copy is dropped on the next persist.
 *
 * The storage is a seam (`VaultBackend`) so the tests run the real WebCrypto
 * of Node against an in-memory backend; the browser gets `idbVaultBackend`.
 */

/** The message every caller already treats as "no keychain here". */
export const VAULT_UNAVAILABLE = "keychain unavailable (web build)";

export interface SealedSecret {
  v: 1;
  iv: Uint8Array<ArrayBuffer>;
  data: ArrayBuffer;
}

export interface VaultBackend {
  /** The stored key, or null before the first one was made. */
  loadKey(): Promise<CryptoKey | null>;
  /** Stores `key` unless one is already there, and returns the one that is
   *  stored afterwards, so two tabs making their first key at once end up on
   *  the same one. */
  saveKeyOnce(key: CryptoKey): Promise<CryptoKey>;
  load(account: string): Promise<SealedSecret | null>;
  save(account: string, sealed: SealedSecret): Promise<void>;
  remove(account: string): Promise<void>;
}

export interface WebKeyVault {
  get(account: string): Promise<string | null>;
  set(account: string, value: string): Promise<void>;
  delete(account: string): Promise<void>;
}

const ALGORITHM = { name: "AES-GCM", length: 256 } as const;
const IV_BYTES = 12;

function isSealed(raw: unknown): raw is SealedSecret {
  if (!raw || typeof raw !== "object") return false;
  const r = raw as Partial<SealedSecret>;
  return r.v === 1 && r.iv instanceof Uint8Array && r.iv.byteLength === IV_BYTES && r.data instanceof ArrayBuffer;
}

export function createWebKeyVault(
  backend: VaultBackend,
  crypto: Crypto = globalThis.crypto,
): WebKeyVault {
  const subtle = crypto?.subtle;
  if (!subtle) throw new Error(VAULT_UNAVAILABLE);
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  // One key promise per vault: a burst of first reads (hydrateProviderKeys
  // walks every slot) must not mint a key per call.
  let keyPromise: Promise<CryptoKey> | null = null;
  const key = (): Promise<CryptoKey> => {
    if (!keyPromise) {
      keyPromise = (async () => {
        const stored = await backend.loadKey();
        if (stored) return stored;
        const made = await subtle.generateKey(ALGORITHM, false, ["encrypt", "decrypt"]);
        return backend.saveKeyOnce(made);
      })();
      // A failed attempt (storage refused the write) must not stick.
      keyPromise.catch(() => { keyPromise = null; });
    }
    return keyPromise;
  };

  return {
    async get(account) {
      const sealed = await backend.load(account);
      if (!sealed) return null;
      if (!isSealed(sealed)) return null;
      try {
        const plain = await subtle.decrypt(
          { name: "AES-GCM", iv: sealed.iv, additionalData: encoder.encode(account) },
          await key(),
          sealed.data,
        );
        return decoder.decode(plain);
      } catch {
        // Wrong key (the key store was cleared alone) or a tampered value:
        // there is nothing to give back, and "nothing stored" is the answer
        // the callers' migration path knows.
        return null;
      }
    },
    async set(account, value) {
      const iv = crypto.getRandomValues(new Uint8Array(IV_BYTES));
      const data = await subtle.encrypt(
        { name: "AES-GCM", iv, additionalData: encoder.encode(account) },
        await key(),
        encoder.encode(value),
      );
      await backend.save(account, { v: 1, iv, data });
    },
    async delete(account) {
      await backend.remove(account);
    },
  };
}

/** For tests, and a page without IndexedDB that still wants the semantics. */
export function memoryVaultBackend(): VaultBackend & { readonly secrets: Map<string, SealedSecret> } {
  let stored: CryptoKey | null = null;
  const secrets = new Map<string, SealedSecret>();
  return {
    secrets,
    async loadKey() { return stored; },
    async saveKeyOnce(key) { if (!stored) stored = key; return stored; },
    async load(account) { return secrets.get(account) ?? null; },
    async save(account, sealed) { secrets.set(account, sealed); },
    async remove(account) { secrets.delete(account); },
  };
}

// ── IndexedDB ───────────────────────────────────────────────────────────

export const VAULT_DB = "lu-key-vault";
const KEY_STORE = "keys";
const SECRET_STORE = "secrets";
const MASTER = "master";

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error(VAULT_UNAVAILABLE));
  });
}

function done(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onabort = () => reject(tx.error ?? new Error(VAULT_UNAVAILABLE));
    tx.onerror = () => reject(tx.error ?? new Error(VAULT_UNAVAILABLE));
  });
}

export function idbVaultBackend(factory: IDBFactory = indexedDB): VaultBackend {
  let dbPromise: Promise<IDBDatabase> | null = null;
  const db = (): Promise<IDBDatabase> => {
    if (!dbPromise) {
      dbPromise = new Promise<IDBDatabase>((resolve, reject) => {
        const open = factory.open(VAULT_DB, 1);
        open.onupgradeneeded = () => {
          const d = open.result;
          if (!d.objectStoreNames.contains(KEY_STORE)) d.createObjectStore(KEY_STORE);
          if (!d.objectStoreNames.contains(SECRET_STORE)) d.createObjectStore(SECRET_STORE);
        };
        open.onsuccess = () => resolve(open.result);
        open.onerror = () => reject(open.error ?? new Error(VAULT_UNAVAILABLE));
        open.onblocked = () => reject(new Error(VAULT_UNAVAILABLE));
      });
      dbPromise.catch(() => { dbPromise = null; });
    }
    return dbPromise;
  };

  return {
    async loadKey() {
      const d = await db();
      const found = await request(d.transaction(KEY_STORE, "readonly").objectStore(KEY_STORE).get(MASTER));
      return found instanceof CryptoKey ? found : null;
    },
    async saveKeyOnce(key) {
      // Read and write in ONE transaction: IndexedDB serialises readwrite
      // transactions on the same store, so a second tab sees the first key.
      const d = await db();
      const tx = d.transaction(KEY_STORE, "readwrite");
      const store = tx.objectStore(KEY_STORE);
      const existing = await request(store.get(MASTER));
      let winner: CryptoKey = key;
      if (existing instanceof CryptoKey) winner = existing;
      else store.put(key, MASTER);
      await done(tx);
      return winner;
    },
    async load(account) {
      const d = await db();
      const found: unknown = await request(d.transaction(SECRET_STORE, "readonly").objectStore(SECRET_STORE).get(account));
      return isSealed(found) ? found : null;
    },
    async save(account, sealed) {
      const d = await db();
      const tx = d.transaction(SECRET_STORE, "readwrite");
      tx.objectStore(SECRET_STORE).put(sealed, account);
      await done(tx);
    },
    async remove(account) {
      const d = await db();
      const tx = d.transaction(SECRET_STORE, "readwrite");
      tx.objectStore(SECRET_STORE).delete(account);
      await done(tx);
    },
  };
}

/** True when this page can hold the vault at all. */
export function webVaultAvailable(): boolean {
  return typeof indexedDB !== "undefined" && !!globalThis.crypto?.subtle && typeof CryptoKey !== "undefined";
}

let shared: WebKeyVault | null = null;

/** The vault of this page, or a rejection with VAULT_UNAVAILABLE, the same
 *  words the desktop's "no keychain" answer uses, so every caller's fallback
 *  still applies (a private window without IndexedDB, an old engine). Any
 *  storage failure underneath is reported the same way. */
export function webKeyVault(): WebKeyVault {
  if (!webVaultAvailable()) throw new Error(VAULT_UNAVAILABLE);
  if (!shared) {
    const inner = createWebKeyVault(idbVaultBackend());
    const unavailable = <T>(p: Promise<T>): Promise<T> => p.catch(() => { throw new Error(VAULT_UNAVAILABLE); });
    shared = {
      get: (account) => unavailable(inner.get(account)),
      set: (account, value) => unavailable(inner.set(account, value)),
      delete: (account) => unavailable(inner.delete(account)),
    };
  }
  return shared;
}

/** Test seam: forget the shared vault. */
export function resetWebKeyVaultForTests(): void {
  shared = null;
}
