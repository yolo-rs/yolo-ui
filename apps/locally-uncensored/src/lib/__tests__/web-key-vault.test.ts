/**
 * The key vault of the web build (lib/web-key-vault.ts).
 *
 * Before it, the web build kept provider keys in localStorage as base64 of
 * the reversed string, which anyone with a storage dump reads back in one
 * line. The vault seals them with AES-GCM under a non-extractable key.
 *
 * The cryptography is the real WebCrypto of Node; only the storage is the
 * in-memory backend (vitest runs in `node`, which has no IndexedDB). The
 * second half runs the real api/backend.ts and the real providerStore in a
 * web build, with that vault behind them, to show the old base64 value moving
 * in on the first read and leaving localStorage.
 *
 * Run: bunx vitest run src/lib/__tests__/web-key-vault.test.ts
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  createWebKeyVault,
  memoryVaultBackend,
  VAULT_UNAVAILABLE,
  type VaultBackend,
  type WebKeyVault,
} from '../web-key-vault'

const latin1 = (buf: ArrayBuffer) => Array.from(new Uint8Array(buf), (b) => String.fromCharCode(b)).join('')
const hex = (bytes: Uint8Array) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')

describe('createWebKeyVault', () => {
  it('gives back what was put in, and stores no plaintext', async () => {
    const backend = memoryVaultBackend()
    const vault = createWebKeyVault(backend)
    await vault.set('key:anthropic', 'sk-ant-secret-123')

    expect(await vault.get('key:anthropic')).toBe('sk-ant-secret-123')
    const sealed = backend.secrets.get('key:anthropic')!
    expect(sealed.v).toBe(1)
    expect(sealed.iv.byteLength).toBe(12)
    expect(latin1(sealed.data)).not.toContain('sk-ant-secret-123')
    // Not the old encoding either.
    expect(latin1(sealed.data)).not.toContain(btoa('sk-ant-secret-123'.split('').reverse().join('')))
  })

  it('answers null for an account that holds nothing, like secret_get', async () => {
    const vault = createWebKeyVault(memoryVaultBackend())
    expect(await vault.get('key:openai')).toBeNull()
  })

  it('the key is non-extractable: the page cannot export it', async () => {
    const backend = memoryVaultBackend()
    await createWebKeyVault(backend).set('key:openai', 'sk-x')
    const key = (await backend.loadKey())!
    expect(key.extractable).toBe(false)
    expect(key.algorithm).toMatchObject({ name: 'AES-GCM', length: 256 })
    await expect(globalThis.crypto.subtle.exportKey('raw', key)).rejects.toThrow()
  })

  it('makes the key once, also for a burst of first calls', async () => {
    const inner = memoryVaultBackend()
    let saves = 0
    const backend: VaultBackend = { ...inner, saveKeyOnce: (k) => { saves++; return inner.saveKeyOnce(k) } }
    const vault = createWebKeyVault(backend)
    await Promise.all([vault.set('key:a', '1'), vault.set('key:b', '2'), vault.get('key:c'), vault.set('key:d', '4')])
    // get() of an empty account needs no key, the three sets share one.
    expect(saves).toBe(1)
    expect(await vault.get('key:b')).toBe('2')
  })

  it('two tabs making their first key at once end up on the same key', async () => {
    const backend = memoryVaultBackend()
    const tabA = createWebKeyVault(backend)
    const tabB = createWebKeyVault(backend)
    await Promise.all([tabA.set('key:a', 'from-a'), tabB.set('key:b', 'from-b')])
    expect(await tabA.get('key:b')).toBe('from-b')
    expect(await tabB.get('key:a')).toBe('from-a')
  })

  it('a fresh IV for every write', async () => {
    const backend = memoryVaultBackend()
    const vault = createWebKeyVault(backend)
    await vault.set('key:a', 'same')
    const first = hex(backend.secrets.get('key:a')!.iv)
    await vault.set('key:a', 'same')
    const second = hex(backend.secrets.get('key:a')!.iv)
    expect(first).not.toBe(second)
  })

  it('a sealed value moved to another account does not open there', async () => {
    const backend = memoryVaultBackend()
    const vault = createWebKeyVault(backend)
    await vault.set('key:anthropic', 'sk-ant')
    backend.secrets.set('key:openai', backend.secrets.get('key:anthropic')!)
    expect(await vault.get('key:openai')).toBeNull()
  })

  it('a tampered value answers null instead of garbage', async () => {
    const backend = memoryVaultBackend()
    const vault = createWebKeyVault(backend)
    await vault.set('key:a', 'sk-a')
    const sealed = backend.secrets.get('key:a')!
    const bytes = new Uint8Array(sealed.data.slice(0))
    bytes[0] ^= 0xff
    backend.secrets.set('key:a', { ...sealed, data: bytes.buffer })
    expect(await vault.get('key:a')).toBeNull()
  })

  it('delete removes the account', async () => {
    const backend = memoryVaultBackend()
    const vault = createWebKeyVault(backend)
    await vault.set('key:a', 'sk-a')
    await vault.delete('key:a')
    expect(await vault.get('key:a')).toBeNull()
    expect(backend.secrets.has('key:a')).toBe(false)
  })

  it('refuses to exist without WebCrypto, with the "no keychain" words', () => {
    expect(() => createWebKeyVault(memoryVaultBackend(), {} as Crypto)).toThrow(VAULT_UNAVAILABLE)
  })
})

// ── The web build end to end: api/backend.ts and providerStore ────────────

const shared = vi.hoisted(() => ({
  backend: null as null | ReturnType<typeof import('../web-key-vault').memoryVaultBackend>,
  vault: null as null | WebKeyVault,
}))

vi.mock('../web-key-vault', async (orig) => {
  const real = await orig<typeof import('../web-key-vault')>()
  return {
    ...real,
    // The browser's vault, on the in-memory backend: same code, no IndexedDB.
    webKeyVault: () => {
      if (!shared.backend) throw new Error(real.VAULT_UNAVAILABLE)
      if (!shared.vault) shared.vault = real.createWebKeyVault(shared.backend)
      return shared.vault
    },
  }
})
vi.mock('../../api/providers/client-cache', () => ({ clearProviderCache: vi.fn() }))

const obf = (k: string) => btoa(k.split('').reverse().join(''))

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

describe('the web build keeps keys in the vault', () => {
  beforeEach(() => {
    vi.resetModules()
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    shared.backend = memoryVaultBackend()
    shared.vault = null
  })

  afterEach(() => {
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
  })

  it('secret_* and secret_park_* go to the vault, in separate namespaces', async () => {
    installLocalStorage()
    const backend = await import('../../api/backend')
    await backend.secretSet('anthropic', 'sk-slot')
    await backend.secretParkSet('lm-studio', 'sk-parked')

    expect(await backend.secretGet('anthropic')).toBe('sk-slot')
    expect(await backend.secretParkGet('lm-studio')).toBe('sk-parked')
    expect([...shared.backend!.secrets.keys()].sort()).toEqual(['key:anthropic', 'parked-key:lm-studio'])

    await backend.secretDelete('anthropic')
    expect(await backend.secretGet('anthropic')).toBeNull()
  })

  it('a parked id outside [a-z0-9-]{1,64} is refused like Rust refuses it', async () => {
    installLocalStorage()
    const backend = await import('../../api/backend')
    await expect(backend.secretParkSet('BAD ID', 'sk')).rejects.toThrow(/^refused:/)
    expect(shared.backend!.secrets.size).toBe(0)
  })

  it('without a vault the old "keychain unavailable" answer stands, so the fallback still applies', async () => {
    installLocalStorage()
    shared.backend = null
    const backend = await import('../../api/backend')
    await expect(backend.secretGet('anthropic')).rejects.toThrow(/keychain unavailable/)
  })

  it('migrates an old base64 key on the first read and drops it from localStorage', async () => {
    const storage = installLocalStorage({
      'lu-providers': JSON.stringify({
        state: {
          providers: {
            anthropic: { id: 'anthropic', name: 'Anthropic', enabled: true, baseUrl: 'https://api.anthropic.com', apiKey: obf('sk-ant-legacy'), isLocal: false },
          },
          openaiEntries: [],
        },
        version: 2,
      }),
    })
    const { useProviderStore } = await import('../../stores/providerStore')
    await useProviderStore.persist.rehydrate()
    expect(useProviderStore.getState().getProviderApiKey('anthropic')).toBe('sk-ant-legacy')

    await useProviderStore.getState().hydrateProviderKeys()

    // Now sealed in the vault ...
    const sealed = shared.backend!.secrets.get('key:anthropic')
    expect(sealed).toBeDefined()
    expect(latin1(sealed!.data)).not.toContain('sk-ant-legacy')
    expect(await shared.vault!.get('key:anthropic')).toBe('sk-ant-legacy')
    // ... still usable ...
    expect(useProviderStore.getState().getProviderApiKey('anthropic')).toBe('sk-ant-legacy')
    // ... and gone from localStorage, in both spellings.
    const raw = storage.get('lu-providers') || ''
    expect(raw).not.toContain('sk-ant-legacy')
    expect(raw).not.toContain(obf('sk-ant-legacy'))
  })

  it('COUNTER-CHECK: browser dev mode (no web build) still has no vault and touches none', async () => {
    vi.unstubAllEnvs()
    installLocalStorage()
    const backend = await import('../../api/backend')
    await expect(backend.secretSet('anthropic', 'sk')).rejects.toThrow(/keychain unavailable/)
    expect(shared.backend!.secrets.size).toBe(0)
  })
})
