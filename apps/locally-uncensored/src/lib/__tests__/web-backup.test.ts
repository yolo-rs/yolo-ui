/**
 * The store backup of the web build (lib/web-backup.ts): backup_stores and
 * restore_stores without Rust, against the origin private file system, and a
 * downloaded JSON file where the browser has none.
 *
 * The OPFS here is a fake with the three calls the module uses
 * (getFileHandle, getFile().text(), createWritable), so the merge rule and
 * the fallback are tested without a browser. The last block runs the real
 * store-backup.ts and api/backend.ts in a web build.
 *
 * Run: bunx vitest run src/lib/__tests__/web-backup.test.ts
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  downloadWebBackup,
  hasPendingWebBackupDownload,
  mergeBackup,
  readWebBackup,
  resetWebBackupForTests,
  writeWebBackup,
  WEB_BACKUP_FILE,
  type BackupDirectory,
  type WebBackupEnv,
} from '../web-backup'
import { standOn } from '../../test-support/devices'

function fakeOpfs(opts: { writable?: boolean } = {}) {
  const files = new Map<string, string>()
  const dir: BackupDirectory = {
    async getFileHandle(name, options) {
      if (!files.has(name) && !options?.create) throw new DOMException('not found', 'NotFoundError')
      if (!files.has(name)) files.set(name, '')
      return {
        async getFile() { return { text: async () => files.get(name) ?? '' } },
        ...(opts.writable === false ? {} : {
          createWritable: async () => {
            let buffered = ''
            return {
              async write(data: string) { buffered += data },
              async close() { files.set(name, buffered) },
            }
          },
        }),
      }
    },
  }
  return { files, dir }
}

function envWith(dir: BackupDirectory | null): WebBackupEnv & { downloads: Array<{ name: string; text: string }> } {
  const downloads: Array<{ name: string; text: string }> = []
  return { downloads, directory: async () => dir, download: (name, text) => { downloads.push({ name, text }) } }
}

beforeEach(() => resetWebBackupForTests())

describe('mergeBackup, the rule of the Rust command', () => {
  it('carries over keys the incoming snapshot lost or left empty, and lets it win elsewhere', () => {
    const prev = JSON.stringify({ __ts: 'old', chats: 'C1', settings: 'S1', todos: 'T1' })
    const next = JSON.stringify({ __ts: 'new', chats: 'C2', settings: '' })
    expect(JSON.parse(mergeBackup(prev, next))).toEqual({ __ts: 'new', chats: 'C2', settings: 'S1', todos: 'T1' })
  })

  it('ignores a previous file that is not a snapshot', () => {
    expect(JSON.parse(mergeBackup('not json', JSON.stringify({ a: '1' })))).toEqual({ a: '1' })
    expect(JSON.parse(mergeBackup('[1,2]', JSON.stringify({ a: '1' })))).toEqual({ a: '1' })
  })

  it('refuses an incoming body that is not a JSON object', () => {
    expect(() => mergeBackup(null, '[]')).toThrow(/not a JSON object/)
    expect(() => mergeBackup(null, '{')).toThrow(/not a JSON object/)
  })
})

describe('with an origin private file system', () => {
  it('writes the file and reads it back', async () => {
    const opfs = fakeOpfs()
    const env = envWith(opfs.dir)
    expect(await readWebBackup(env)).toBeNull()
    expect(await writeWebBackup(JSON.stringify({ chats: 'C1' }), env)).toBe('opfs')
    expect(JSON.parse((await readWebBackup(env))!)).toEqual({ chats: 'C1' })
    expect(opfs.files.has(WEB_BACKUP_FILE)).toBe(true)
  })

  it('an incomplete snapshot never costs the previous value of a key', async () => {
    const env = envWith(fakeOpfs().dir)
    await writeWebBackup(JSON.stringify({ chats: 'C1', settings: 'S1' }), env)
    // The chat store could not be read this tick: the key is simply absent.
    await writeWebBackup(JSON.stringify({ settings: 'S2' }), env)
    expect(JSON.parse((await readWebBackup(env))!)).toEqual({ chats: 'C1', settings: 'S2' })
  })

  it('downloads nothing: the file is the backup', async () => {
    const env = envWith(fakeOpfs().dir)
    await writeWebBackup(JSON.stringify({ chats: 'C1' }), env)
    expect(hasPendingWebBackupDownload()).toBe(false)
    expect(downloadWebBackup(env)).toBe(false)
    expect(env.downloads).toHaveLength(0)
  })
})

describe('without one', () => {
  it('keeps the merged snapshot and hands it out as a JSON file when asked', async () => {
    const env = envWith(null)
    expect(await writeWebBackup(JSON.stringify({ chats: 'C1', settings: 'S1' }), env)).toBe('kept-for-download')
    expect(await writeWebBackup(JSON.stringify({ chats: 'C2' }), env)).toBe('kept-for-download')
    // Nothing to restore from unasked: the page cannot read a downloaded file.
    expect(await readWebBackup(env)).toBeNull()
    // Writing alone downloads nothing (the five-second tick goes through here).
    expect(env.downloads).toHaveLength(0)

    expect(downloadWebBackup(env, new Date('2026-11-05T14:02:03Z'))).toBe(true)
    expect(env.downloads).toHaveLength(1)
    expect(env.downloads[0].name).toBe('locally-uncensored-backup-2026-11-05-14-02-03.json')
    expect(JSON.parse(env.downloads[0].text)).toEqual({ chats: 'C2', settings: 'S1' })
  })

  it('a file system without createWritable counts as none', async () => {
    const opfs = fakeOpfs({ writable: false })
    const env = envWith(opfs.dir)
    expect(await writeWebBackup(JSON.stringify({ chats: 'C1' }), env)).toBe('kept-for-download')
    expect(hasPendingWebBackupDownload()).toBe(true)
  })

  it('downloads nothing before the first snapshot', () => {
    const env = envWith(null)
    expect(downloadWebBackup(env)).toBe(false)
    expect(env.downloads).toHaveLength(0)
  })
})

// ── Through store-backup.ts and backendCall, in a web build ───────────────

function installLocalStorage(seed: Record<string, string> = {}) {
  const map = new Map<string, string>(Object.entries(seed))
  const ls = {
    getItem: (k: string) => (map.has(k) ? map.get(k)! : null),
    setItem: (k: string, v: string) => { map.set(k, String(v)) },
    removeItem: (k: string) => { map.delete(k) },
    clear: () => { map.clear() },
    key: (i: number) => [...map.keys()][i] ?? null,
    get length() { return map.size },
  }
  vi.stubGlobal('localStorage', ls)
  vi.stubGlobal('window', { localStorage: ls })
  return map
}

function installDocument() {
  const anchors: Array<{ href: string; download: string; click: ReturnType<typeof vi.fn> }> = []
  vi.stubGlobal('document', {
    createElement: () => {
      const a = { href: '', download: '', click: vi.fn() }
      anchors.push(a)
      return a
    },
    body: { appendChild: vi.fn(), removeChild: vi.fn() },
  })
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:lu-backup')
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  return anchors
}

describe('the web build end to end', () => {
  let key = ''

  beforeEach(async () => {
    vi.resetModules()
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    const { STORE_KEYS, IDB_STORE_KEYS, SECRET_BEARING_STORE_KEYS } = await import('../store-backup')
    key = STORE_KEYS.find((k) => !IDB_STORE_KEYS.has(k) && !SECRET_BEARING_STORE_KEYS.includes(k))!
    vi.resetModules()
  })

  afterEach(() => {
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('backup_stores and restore_stores go to the OPFS file', async () => {
    installLocalStorage()
    const opfs = fakeOpfs()
    vi.stubGlobal('navigator', { storage: { getDirectory: async () => opfs.dir } })
    const { backendCall } = await import('../../api/backend')
    await backendCall('backup_stores', { data: JSON.stringify({ [key]: 'V1' }) })
    expect(JSON.parse((await backendCall<string | null>('restore_stores'))!)).toEqual({ [key]: 'V1' })
  })

  it('without OPFS the tick downloads nothing, "back up now" downloads the file', async () => {
    installLocalStorage({ [key]: JSON.stringify({ state: { x: 1 }, version: 0 }) })
    standOn('windows') // a navigator without `storage`
    const anchors = installDocument()
    const { backupStoresIfChanged, backupStoresNow } = await import('../store-backup')

    expect(await backupStoresIfChanged()).toBe('written')
    await new Promise((r) => setTimeout(r, 0))
    expect(anchors).toHaveLength(0)

    expect(await backupStoresNow()).toBe(true)
    expect(anchors).toHaveLength(1)
    expect(anchors[0].download).toMatch(/^locally-uncensored-backup-.+\.json$/)
    expect(anchors[0].click).toHaveBeenCalledTimes(1)
  })
})
