import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

let doneHandler: ((event: unknown) => void) | undefined

vi.mock('../api/importEpub', () => ({
  importEpub: vi.fn(async () => ({ job_id: 'job-1' })),
  onImportProgress: vi.fn(async () => vi.fn()),
  onImportError: vi.fn(async () => vi.fn()),
  onImportDone: vi.fn(async (handler: (event: unknown) => void) => {
    doneHandler = handler
    return vi.fn()
  }),
}))

import ImportPanel from '../components/ImportPanel.vue'

describe('ImportPanel', () => {
  it('starts import and renders the returned report', async () => {
    const wrapper = mount(ImportPanel)

    await wrapper.get('[data-test="epub-path"]').setValue('book.epub')
    await wrapper.get('[data-test="import-button"]').trigger('click')

    expect(wrapper.text()).toContain('job-1')

    doneHandler?.({
      job_id: 'job-1',
      book_id: 7,
      report: {
        book_id: 7,
        title: 'Inside the Box',
        root_toc_nodes: 16,
        toc_nodes_total: 31,
        included_toc_nodes: 16,
        title_only_toc_nodes: 4,
        excluded_toc_nodes: 11,
        chapters_imported: 2,
        paragraphs_imported: 101,
      },
    })
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Inside the Box')
    expect(wrapper.text()).toContain('31')
    expect(wrapper.text()).toContain('101')
  })
})
