import { describe, it, expect } from 'vitest'
import { parseYoloLocalEndpoints } from '../yolo-local-endpoints'

describe('parseYoloLocalEndpoints', () => {
  it('parses name=url pairs and drops non-loopback or malformed items', () => {
    expect(parseYoloLocalEndpoints('a=http://127.0.0.1:8083/v1/, b=http://localhost:8082/v1,c=https://evil.example/v1,junk,=http://127.0.0.1:1/v1')).toEqual([
      { name: 'a', baseUrl: 'http://127.0.0.1:8083/v1' },
      { name: 'b', baseUrl: 'http://localhost:8082/v1' },
    ])
  })
  it('is empty for unset values and de-duplicates by URL', () => {
    expect(parseYoloLocalEndpoints(undefined)).toEqual([])
    expect(parseYoloLocalEndpoints('a=http://127.0.0.1:1/v1,b=http://127.0.0.1:1/v1')).toHaveLength(1)
  })
})
