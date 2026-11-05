/**
 * yolo-ai's yolo-app-server (crates/yolo-app-server) is a first-class local
 * backend: it has a preset, the startup scan probes it, and, when a server is
 * running (YOLO_APP_SERVER_URL=http://127.0.0.1:1338), the scan finds it over
 * real HTTP and its OpenAI-compatible /v1/models and /v1/chat/completions
 * answer the way this app's OpenAI provider expects.
 *
 * Run: bunx vitest run src/lib/__tests__/yolo-app-server-backend.test.ts
 */
import { describe, expect, it, vi } from 'vitest'
import { standOn } from '../../test-support/devices'

vi.mock('../../api/backend', () => ({
  isTauri: () => false,
  localFetch: vi.fn(),
  backendCall: vi.fn(async () => undefined),
  getOllamaBase: () => 'http://localhost:11434',
  isOllamaLocal: () => true,
}))

import { PROVIDER_PRESETS } from '../../api/providers/types'
import { detectLocalBackends, PROBE_TARGETS } from '../backend-detector'

const LIVE = process.env.YOLO_APP_SERVER_URL
const live = LIVE ? it : it.skip

describe('YOLO App Server preset', () => {
  it('is a local, keyless OpenAI-compatible preset on 127.0.0.1:1338', () => {
    const p = PROVIDER_PRESETS.find((x) => x.id === 'yolo-app-server')
    expect(p).toMatchObject({ providerId: 'openai', baseUrl: 'http://127.0.0.1:1338/v1', isLocal: true })
    expect(p?.placeholder).toBeUndefined()
  })

  it('is one of the scan targets', () => {
    expect(PROBE_TARGETS.some((p) => p.id === 'yolo-app-server')).toBe(true)
  })

  live('the scan finds a running server over real HTTP', async () => {
    standOn('windows')
    const found = await detectLocalBackends()
    expect(found.find((b) => b.id === 'yolo-app-server')).toMatchObject({ port: 1338 })
  })

  live('models and a streamed chat answer in OpenAI shape', async () => {
    const models = await (await fetch(`${LIVE}/v1/models`)).json()
    expect(models.data.map((m: { id: string }) => m.id)).toContain('yolo-echo')
    const res = await fetch(`${LIVE}/v1/chat/completions`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ model: 'yolo-echo', stream: true, messages: [{ role: 'user', content: 'hi lu' }] }),
    })
    const frames = (await res.text()).split('\n').filter((l) => l.startsWith('data: ')).map((l) => l.slice(6))
    expect(frames.at(-1)).toBe('[DONE]')
    const text = frames.slice(0, -1).map((f) => JSON.parse(f).choices[0].delta.content ?? '').join('')
    expect(text).toBe('echo: hi lu')
  })
})
