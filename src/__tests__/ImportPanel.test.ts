import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

let doneHandler: ((event: unknown) => void) | undefined
const getScopeNodes = vi.fn()
const confirmScope = vi.fn()

vi.mock('../api/importEpub', () => ({
  importEpub: vi.fn(async () => ({ job_id: 'job-1' })),
  getImportJob: vi.fn(async () => ({
    job_id: 'job-1',
    state: 'running',
    stage: 'parse',
    percent: 35,
    message: '解析 EPUB 结构与目录',
  })),
  onImportProgress: vi.fn(async () => vi.fn()),
  onImportError: vi.fn(async () => vi.fn()),
  onImportDone: vi.fn(async (handler: (event: unknown) => void) => {
    doneHandler = handler
    return vi.fn()
  }),
  getScopeNodes: (...args: unknown[]) => getScopeNodes(...args),
  confirmScope: (...args: unknown[]) => confirmScope(...args),
}))

import ImportPanel from '../components/ImportPanel.vue'

describe('ImportPanel', () => {
  beforeEach(() => {
    doneHandler = undefined
    getScopeNodes.mockReset()
    confirmScope.mockReset()
    getScopeNodes.mockResolvedValue([
      {
        id: 10,
        parent_id: null,
        title: 'Introduction: A Textbook Case of Discovery',
        level: 1,
        order_index: 0,
        content_type: 'body',
        included: true,
        sentence_count: 12,
      },
      {
        id: 11,
        parent_id: null,
        title: 'Copyright',
        level: 1,
        order_index: 1,
        content_type: 'excluded',
        included: false,
        sentence_count: 0,
      },
    ])
    confirmScope.mockResolvedValue({
      book_id: 7,
      title: 'Inside the Box',
      root_toc_nodes: 16,
      toc_nodes_total: 31,
      included_toc_nodes: 15,
      title_only_toc_nodes: 4,
      excluded_toc_nodes: 12,
      chapters_imported: 2,
      paragraphs_imported: 101,
      sentences_imported: 221,
    })
  })

  it('shows a clear validation message when path is empty', async () => {
    const wrapper = mount(ImportPanel)

    await wrapper.get('[data-test="epub-path"]').setValue('')
    await wrapper.get('[data-test="import-button"]').trigger('click')

    expect(wrapper.text()).toContain('请先填写 EPUB 文件路径')
  })

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
        sentences_imported: 233,
      },
    })
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Inside the Box')
    expect(wrapper.text()).toContain('31')
    expect(wrapper.text()).toContain('101')
    expect(wrapper.text()).toContain('233')
  })

  it('loads scope nodes and confirms excluded nodes', async () => {
    const wrapper = mount(ImportPanel)

    await wrapper.get('[data-test="epub-path"]').setValue('book.epub')
    await wrapper.get('[data-test="import-button"]').trigger('click')
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
        sentences_imported: 233,
      },
    })
    await flushPromises()

    expect(getScopeNodes).toHaveBeenCalledWith(7)
    expect(wrapper.text()).toContain('范围确认')
    expect(wrapper.text()).toContain('Introduction: A Textbook Case of Discovery')

    await wrapper.get('[data-test="scope-included-10"]').setValue(false)
    await wrapper.get('[data-test="confirm-scope"]').trigger('click')
    await flushPromises()

    expect(confirmScope).toHaveBeenCalledWith(7, [
      { id: 10, content_type: 'excluded', included: false },
      { id: 11, content_type: 'excluded', included: false },
    ])
    expect(wrapper.text()).toContain('221')
  })

  it('renders a progress bar', () => {
    const wrapper = mount(ImportPanel)

    const progress = wrapper.get('[data-test="import-progress"]')
    expect(progress.attributes('max')).toBe('100')
  })
})

async function flushPromises() {
  await Promise.resolve()
  await Promise.resolve()
}
