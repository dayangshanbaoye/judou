import { describe, expect, it, vi } from 'vitest'

const { listenMock } = vi.hoisted(() => ({
  listenMock: vi.fn(async () => vi.fn()),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string, payload?: unknown) => {
    if (command === 'import_epub') {
      return { job_id: `job-for-${(payload as { path: string }).path}` }
    }
    if (command === 'get_import_report') {
      return {
        book_id: (payload as { bookId: number }).bookId,
        title: 'Inside the Box',
        sentences_imported: 233,
      }
    }
    throw new Error(`unexpected command ${command}`)
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}))

import { getImportReport, importEpub, onImportDone, onImportProgress } from '../api/importEpub'

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

  it('loads import report by book id', async () => {
    await expect(getImportReport(42)).resolves.toMatchObject({
      book_id: 42,
      title: 'Inside the Box',
    })
  })
})
