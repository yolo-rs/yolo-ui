import type { StateStorage } from 'zustand/middleware';

/**
 * zustand `StateStorage` backed by IndexedDB, shared by the desktop webview and the web client.
 *
 * IndexedDB survives larger state than localStorage and does not block the main thread. Values
 * written by earlier builds to localStorage are migrated on first read and then removed. When
 * IndexedDB is unavailable (private mode in some browsers, tests), it falls back to localStorage.
 */
const DB_NAME = 'spacedrive';
const STORE = 'preferences';

let dbPromise: Promise<IDBDatabase> | null = null;

function openDb(): Promise<IDBDatabase> {
	if (!dbPromise) {
		dbPromise = new Promise((resolve, reject) => {
			const request = indexedDB.open(DB_NAME, 1);
			request.onupgradeneeded = () => request.result.createObjectStore(STORE);
			request.onsuccess = () => resolve(request.result);
			request.onerror = () => reject(request.error);
		});
		dbPromise.catch(() => {
			dbPromise = null;
		});
	}
	return dbPromise;
}

function run<T>(mode: IDBTransactionMode, op: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
	return openDb().then(
		(db) =>
			new Promise<T>((resolve, reject) => {
				const request = op(db.transaction(STORE, mode).objectStore(STORE));
				request.onsuccess = () => resolve(request.result);
				request.onerror = () => reject(request.error);
			})
	);
}

const hasIndexedDb = () => typeof indexedDB !== 'undefined';
const hasLocalStorage = () => typeof localStorage !== 'undefined';

export const idbStorage: StateStorage = {
	async getItem(name) {
		if (!hasIndexedDb()) return hasLocalStorage() ? localStorage.getItem(name) : null;
		const stored = await run<string | undefined>('readonly', (s) => s.get(name) as IDBRequest<string | undefined>);
		if (stored !== undefined) return stored;
		const legacy = hasLocalStorage() ? localStorage.getItem(name) : null;
		if (legacy !== null) {
			await run('readwrite', (s) => s.put(legacy, name));
			localStorage.removeItem(name);
		}
		return legacy;
	},
	async setItem(name, value) {
		if (!hasIndexedDb()) {
			if (hasLocalStorage()) localStorage.setItem(name, value);
			return;
		}
		await run('readwrite', (s) => s.put(value, name));
	},
	async removeItem(name) {
		if (hasLocalStorage()) localStorage.removeItem(name);
		if (hasIndexedDb()) await run('readwrite', (s) => s.delete(name));
	},
};
