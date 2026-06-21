import { describe, expect, it, vi } from 'vitest'

const { listenMock } = vi.hoisted(() => ({
  listenMock: vi.fn(async () => vi.fn()),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string, payload?: unknown) => {
    if (command !== 'import_epub') throw new Error(`unexpected command ${command}`)
    return { job_id: `job-for-${(payload as { path: string }).path}` }
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}))

import { importEpub, onImportDone, onImportProgress } from '../api/importEpub'

describe('import epub api client', () => {
  it('starts import job through Tauri command', async () => {
    await expect(importEpub('book.epub')).resolves.toEqual({ job_id: 'job-for-book.epub' })
  })

  it('subscribes to import progress and done events', async () => {
    await onImportProgress(() => undefined)
    await onImportDone(() => undefined)

    expect(listenMock).toHaveBeenCalledWith('import://progress', expect.any(Function))
    expect(listenMock).toHaveBeenCalledWith('import://done', expect.any(Function))
  })
})
