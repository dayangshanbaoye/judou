import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const getReaderView = vi.fn()
const updateSentenceStatus = vi.fn()
const mergeSentences = vi.fn()
const splitSentence = vi.fn()

vi.mock('../api/reader', () => ({
  getReaderView: (...args: unknown[]) => getReaderView(...args),
  updateSentenceStatus: (...args: unknown[]) => updateSentenceStatus(...args),
  mergeSentences: (...args: unknown[]) => mergeSentences(...args),
  splitSentence: (...args: unknown[]) => splitSentence(...args),
}))

import ReaderPanel from '../components/ReaderPanel.vue'

const introductionView = {
  book_id: 7,
  book_title: 'Inside the Box',
  active_toc_node_id: 10,
  breadcrumb: [{ id: 10, title: 'Introduction: A Textbook Case of Discovery' }],
  toc_nodes: [
    {
      id: 10,
      parent_id: null,
      title: 'Introduction: A Textbook Case of Discovery',
      level: 1,
      order_index: 0,
      content_type: 'body',
      included: true,
    },
    {
      id: 11,
      parent_id: null,
      title: '1. Thinking Inside the Box',
      level: 1,
      order_index: 1,
      content_type: 'body',
      included: true,
    },
  ],
  paragraphs: [
    {
      id: 101,
      toc_node_id: 10,
      order_index: 0,
      source_href: 'OEBPS/c4A.xhtml',
      sentences: [
        {
          id: 1001,
          paragraph_id: 101,
          order_index: 0,
          text: 'There is, perhaps, no more abused phrase.',
          status: 'unread',
        },
        {
          id: 1002,
          paragraph_id: 101,
          order_index: 1,
          text: 'Creativity can be practiced.',
          status: 'unread',
        },
      ],
    },
  ],
}

const chapterView = {
  ...introductionView,
  active_toc_node_id: 11,
  breadcrumb: [{ id: 11, title: '1. Thinking Inside the Box' }],
  paragraphs: [
    {
      id: 201,
      toc_node_id: 11,
      order_index: 0,
      source_href: 'OEBPS/c5.xhtml',
      sentences: [
        {
          id: 2001,
          paragraph_id: 201,
          order_index: 0,
          text: 'Creativity is not magic.',
          status: 'unread',
        },
      ],
    },
  ],
}

describe('ReaderPanel', () => {
  beforeEach(() => {
    getReaderView.mockReset()
    updateSentenceStatus.mockReset()
    mergeSentences.mockReset()
    splitSentence.mockReset()
    getReaderView.mockResolvedValue(introductionView)
    updateSentenceStatus.mockResolvedValue({
      ...introductionView.paragraphs[0].sentences[0],
      status: 'understood',
    })
    mergeSentences.mockResolvedValue({
      id: 1001,
      paragraph_id: 101,
      order_index: 0,
      text: 'There is, perhaps, no more abused phrase.Creativity can be practiced.',
      status: 'flagged',
    })
    splitSentence.mockResolvedValue([
      {
        id: 1001,
        paragraph_id: 101,
        order_index: 0,
        text: 'There is,',
        status: 'flagged',
      },
      {
        id: 1003,
        paragraph_id: 101,
        order_index: 1,
        text: ' perhaps, no more abused phrase.',
        status: 'flagged',
      },
    ])
  })

  it('renders toc, breadcrumb, context, and sentence stream', async () => {
    const wrapper = mount(ReaderPanel, { props: { bookId: 7 } })
    await flushPromises()

    expect(getReaderView).toHaveBeenCalledWith(7)
    expect(wrapper.text()).toContain('Inside the Box')
    expect(wrapper.text()).toContain('Introduction: A Textbook Case of Discovery')
    expect(wrapper.text()).toContain('OEBPS/c4A.xhtml')
    expect(wrapper.text()).toContain('There is, perhaps, no more abused phrase.')
  })

  it('switches sentence stream when a toc node is clicked', async () => {
    getReaderView.mockResolvedValueOnce(introductionView).mockResolvedValueOnce(chapterView)
    const wrapper = mount(ReaderPanel, { props: { bookId: 7 } })
    await flushPromises()

    await wrapper.get('[data-test="toc-node-11"]').trigger('click')
    await flushPromises()

    expect(getReaderView).toHaveBeenLastCalledWith(7, 11)
    expect(wrapper.text()).toContain('1. Thinking Inside the Box')
    expect(wrapper.text()).toContain('Creativity is not magic.')
  })

  it('marks a sentence as understood', async () => {
    const wrapper = mount(ReaderPanel, { props: { bookId: 7 } })
    await flushPromises()

    await wrapper.get('[data-test="sentence-status-1001"]').trigger('click')
    await flushPromises()

    expect(updateSentenceStatus).toHaveBeenCalledWith(1001, 'understood')
    expect(wrapper.text()).toContain('understood')
  })

  it('selects two sentences and merges them', async () => {
    const wrapper = mount(ReaderPanel, { props: { bookId: 7 } })
    await flushPromises()

    await wrapper.get('[data-test="select-sentence-1001"]').setValue(true)
    await wrapper.get('[data-test="select-sentence-1002"]').setValue(true)
    await wrapper.get('[data-test="merge-selected"]').trigger('click')
    await flushPromises()

    expect(mergeSentences).toHaveBeenCalledWith([1001, 1002])
    expect(getReaderView).toHaveBeenLastCalledWith(7, 10)
    expect(wrapper.text()).toContain('已合并句子，并写入处理台账')
  })

  it('opens a clear split panel and splits the active sentence', async () => {
    const wrapper = mount(ReaderPanel, { props: { bookId: 7 } })
    await flushPromises()

    expect(wrapper.text()).toContain('拆分这句')
    expect(wrapper.text()).not.toContain('设为拆分')

    await wrapper.get('[data-test="split-target-1001"]').trigger('click')
    expect(wrapper.text()).toContain('正在拆分')
    expect(wrapper.text()).toContain('There is, perhaps, no more abused phrase.')
    expect(wrapper.text()).toContain('在下方输入拆分位置')

    await wrapper.get('[data-test="split-offset"]').setValue('9')
    await wrapper.get('[data-test="split-active"]').trigger('click')
    await flushPromises()

    expect(splitSentence).toHaveBeenCalledWith(1001, 9)
    expect(getReaderView).toHaveBeenLastCalledWith(7, 10)
    expect(wrapper.text()).toContain('已拆分句子，并写入处理台账')
  })
})

async function flushPromises() {
  await Promise.resolve()
  await Promise.resolve()
}
