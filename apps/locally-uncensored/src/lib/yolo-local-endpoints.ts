/**
 * Yolo local inference endpoints (llama.cpp servers started by yolo-ai: `bun scripts/yolo-sync.ts models serve`).
 *
 * `VITE_YOLO_LOCAL_LLM_ENDPOINTS` is a comma list of `name=baseUrl` pairs, for example
 * `local-qwen-small=http://127.0.0.1:8083/v1,local-qwen-coder=http://127.0.0.1:8082/v1`.
 * Unset (the default, and in every unit test) means no entries are seeded.
 */
export interface YoloLocalEndpoint {
  name: string
  baseUrl: string
}

export function parseYoloLocalEndpoints(raw: unknown): YoloLocalEndpoint[] {
  if (typeof raw !== 'string') return []
  const out: YoloLocalEndpoint[] = []
  for (const part of raw.split(',')) {
    const i = part.indexOf('=')
    if (i < 1) continue
    const name = part.slice(0, i).trim()
    const baseUrl = part.slice(i + 1).trim().replace(/\/+$/, '')
    if (!name || !/^https?:\/\/(127\.0\.0\.1|localhost|\[::1\])(:\d+)?(\/|$)/i.test(baseUrl)) continue
    if (!out.some((e) => e.baseUrl === baseUrl)) out.push({ name, baseUrl })
  }
  return out
}

export function yoloLocalEndpointsFromEnv(): YoloLocalEndpoint[] {
  return parseYoloLocalEndpoints((import.meta as { env?: Record<string, unknown> }).env?.VITE_YOLO_LOCAL_LLM_ENDPOINTS)
}
