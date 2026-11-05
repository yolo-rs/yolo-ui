/**
 * The web build (VITE_LU_BUILD_TARGET=web) turns off what only the native
 * app can do, and says so instead of fetching a route that is not there.
 *
 * A static host has no Rust behind the page and no Vite dev server, so a
 * fetch of /local-api would come back as the host's 404 page, or its
 * index.html with status 200. Every capability below has a desktop twin that
 * must answer exactly what it answered before.
 *
 * Run: bunx vitest run src/lib/__tests__/web-build-capabilities.test.ts
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { DESKTOPS, standOn } from '../../test-support/devices'

function installLocalStorage() {
  const map = new Map<string, string>()
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

async function fresh<T>(load: () => Promise<T>): Promise<T> {
  vi.resetModules()
  return load()
}

const capabilities = () => fresh(() => import('../capabilities'))
const backend = () => fresh(() => import('../../api/backend'))

beforeEach(() => {
  installLocalStorage()
})

afterEach(() => {
  vi.unstubAllEnvs()
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('build target', () => {
  it('names the web build only when the variable says web', async () => {
    const { buildTarget, isWebBuild } = await import('../build-target')
    expect(buildTarget()).toBe('desktop')
    expect(isWebBuild()).toBe(false)
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    expect(buildTarget()).toBe('web')
    expect(isWebBuild()).toBe(true)
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'mobile')
    expect(buildTarget()).toBe('mobile')
    expect(isWebBuild()).toBe(false)
  })
})

describe('capabilities in the web build', () => {
  for (const desktop of DESKTOPS) {
    it(`${desktop}: nothing that spawns, dials or moves the mouse`, async () => {
      vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
      standOn(desktop)
      const c = await capabilities()
      expect(c.hasShell()).toBe(false)
      expect(c.hasComputerControl()).toBe(false)
      expect(c.hasFileDialog()).toBe(false)
      expect(c.hasProcessSpawn()).toBe(false)
      expect(c.hasLocalVoice()).toBe(false)
      expect(c.hasComfyProcessControl()).toBe(false)
      expect(c.hasMlx()).toBe(false)
      // What a page can do stays: talk to local servers over HTTP, keep keys.
      expect(c.hasForeignBackends()).toBe(true)
      expect(c.hasKeychain()).toBe(true)
      // A ComfyUI somebody else started is still usable where the desktop
      // build would use one.
      expect(c.hasComfy()).toBe(desktop !== 'mac')
    })
  }

  for (const desktop of DESKTOPS) {
    it(`COUNTER-CHECK ${desktop}: the desktop build answers as before`, async () => {
      standOn(desktop)
      const c = await capabilities()
      expect(c.hasShell()).toBe(true)
      expect(c.hasComputerControl()).toBe(true)
      expect(c.hasFileDialog()).toBe(true)
      expect(c.hasProcessSpawn()).toBe(true)
      expect(c.hasLocalVoice()).toBe(true)
      expect(c.hasComfyProcessControl()).toBe(desktop !== 'mac')
      expect(c.hasMlx()).toBe(desktop === 'mac')
    })
  }
})

describe('backendCall in the web build', () => {
  const NATIVE_ONLY = [
    'shell_execute', 'execute_code', 'fs_read', 'fs_write', 'process_list',
    'computer_screen', 'computer_action', 'start_comfyui', 'stop_comfyui',
    'install_comfyui', 'pick_gguf_file', 'install_whisper', 'mlx_generate',
    'start_remote_server', 'download_model', 'get_ollama_host', 'a_command_nobody_mapped',
  ]

  it('rejects every native-only command with the typed error and fetches nothing', async () => {
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    standOn('windows')
    const fetchSpy = vi.fn(async () => new Response('<!doctype html>', { status: 200 }))
    vi.stubGlobal('fetch', fetchSpy)
    const { backendCall, isNativeAppRequired, NativeAppRequiredError } = await backend()
    for (const command of NATIVE_ONLY) {
      const err = await backendCall(command).then(() => null, (e: unknown) => e)
      expect(isNativeAppRequired(err), command).toBe(true)
      expect(err, command).toBeInstanceOf(NativeAppRequiredError)
      expect((err as InstanceType<typeof NativeAppRequiredError>).command).toBe(command)
      expect(String((err as Error).message)).toMatch(/requires the native app/)
    }
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('fetchExternal has no proxy behind a static host either', async () => {
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    const fetchSpy = vi.fn()
    vi.stubGlobal('fetch', fetchSpy)
    const { fetchExternal, isNativeAppRequired } = await backend()
    const err = await fetchExternal('https://civitai.com/api/v1/models').then(() => null, (e: unknown) => e)
    expect(isNativeAppRequired(err)).toBe(true)
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('COUNTER-CHECK: browser dev mode still maps to the /local-api route', async () => {
    standOn('windows')
    const fetchSpy = vi.fn(async (_url: string, _init?: RequestInit) => new Response(JSON.stringify({ ok: true }), { status: 200 }))
    vi.stubGlobal('fetch', fetchSpy)
    const { backendCall } = await backend()
    await backendCall('shell_execute', { command: 'echo' })
    expect(fetchSpy).toHaveBeenCalledTimes(1)
    expect(fetchSpy.mock.calls[0][0]).toBe('/local-api/shell-execute')
  })
})

describe('local model servers from the browser', () => {
  it('Ollama: an absolute address, 127.0.0.1 by default, the stored one after setOllamaBase', async () => {
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    const b = await backend()
    expect(b.ollamaUrl('/tags')).toBe('http://127.0.0.1:11434/api/tags')
    b.setOllamaBase('192.168.1.20:11434')
    expect(b.ollamaUrl('/chat')).toBe('http://192.168.1.20:11434/api/chat')
    b.setOllamaBase('')
    expect(b.ollamaUrl('/tags')).toBe('http://127.0.0.1:11434/api/tags')
  })

  it('the providers store starts the web build on the same Ollama address', async () => {
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    const { useProviderStore } = await fresh(() => import('../../stores/providerStore'))
    expect(useProviderStore.getState().providers.ollama.baseUrl).toBe('http://127.0.0.1:11434')
  })

  it('ComfyUI: an absolute address, and the address set in Settings is kept in the page', async () => {
    vi.stubEnv('VITE_LU_BUILD_TARGET', 'web')
    const fetchSpy = vi.fn(async (_url: string, _init?: RequestInit) => new Response('{}', { status: 200 }))
    vi.stubGlobal('fetch', fetchSpy)
    const b = await backend()
    expect(b.comfyuiUrl('/prompt')).toBe('http://127.0.0.1:8188/prompt')

    await b.backendCall('set_comfyui_host', { host: '10.0.0.7' })
    await b.backendCall('set_comfyui_port', { port: 8190 })
    expect(b.comfyuiUrl('/prompt')).toBe('http://10.0.0.7:8190/prompt')
    await expect(b.backendCall('set_comfyui_host', { host: 'not a host/' })).rejects.toThrow(/Invalid host/)

    // A reload: the module starts on its defaults, comfyui_status brings the
    // stored address back (AppShell asks it once at boot).
    const again = await backend()
    expect(again.comfyuiUrl('/prompt')).toBe('http://127.0.0.1:8188/prompt')
    const status = await again.backendCall<{ running: boolean; host: string; port: number; isLocal: boolean; ownedByApp: boolean }>('comfyui_status')
    expect(status).toMatchObject({ running: true, host: '10.0.0.7', port: 8190, isLocal: false, ownedByApp: false })
    expect(again.comfyuiUrl('/prompt')).toBe('http://10.0.0.7:8190/prompt')
    expect(fetchSpy).toHaveBeenCalledWith('http://10.0.0.7:8190/system_stats', expect.anything())
  })

  it('COUNTER-CHECK: browser dev mode keeps the Vite proxy paths', async () => {
    const b = await backend()
    expect(b.ollamaUrl('/tags')).toBe('/api/tags')
    expect(b.comfyuiUrl('/prompt')).toBe('/comfyui/prompt')
  })
})

describe('the AGPL source offer', () => {
  it('points at the Yolo fork by default, at the build constants when set', async () => {
    const t = await import('../build-target')
    expect(t.sourceLink()).toBe('https://github.com/yolo-rs/locally-uncensored')
    vi.stubEnv('VITE_LU_SOURCE_URL', 'https://git.example.org/me/lu.git')
    vi.stubEnv('VITE_LU_SOURCE_COMMIT', 'a6c283b3a0')
    expect(t.sourceUrl()).toBe('https://git.example.org/me/lu.git')
    expect(t.sourceCommit()).toBe('a6c283b3a0')
    expect(t.sourceLink()).toBe('https://git.example.org/me/lu/tree/a6c283b3a0')
    // Not an address, not a commit: the defaults stand.
    vi.stubEnv('VITE_LU_SOURCE_URL', 'javascript:alert(1)')
    vi.stubEnv('VITE_LU_SOURCE_COMMIT', 'main; rm -rf')
    expect(t.sourceLink()).toBe('https://github.com/yolo-rs/locally-uncensored')
  })
})
