import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string, payload?: unknown) => {
    if (command !== 'ping') throw new Error(`unexpected command ${command}`)
    return { message: `pong: ${(payload as { payload: string }).payload}`, job_id: 'job-test' }
  }),
}))

import { ping } from '../api/ping'

describe('ping api client', () => {
  it('invokes the Tauri ping command', async () => {
    await expect(ping('phase0')).resolves.toEqual({ message: 'pong: phase0', job_id: 'job-test' })
  })
})
