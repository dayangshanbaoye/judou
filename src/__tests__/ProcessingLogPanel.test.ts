import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const listProcessingLog = vi.fn()
const promoteLogToRule = vi.fn()

vi.mock('../api/processingLog', () => ({
  listProcessingLog: (...args: unknown[]) => listProcessingLog(...args),
  promoteLogToRule: (...args: unknown[]) => promoteLogToRule(...args),
}))

import ProcessingLogPanel from '../components/ProcessingLogPanel.vue'

describe('ProcessingLogPanel', () => {
  beforeEach(() => {
    listProcessingLog.mockReset()
    promoteLogToRule.mockReset()
    listProcessingLog.mockResolvedValue([
      {
        id: 77,
        book_id: 7,
        stage: 'segment',
        severity: 'warn',
        location_ref: 'paragraph:101@0',
        raw_snippet: 'There is, perhaps',
        action_taken: 'manual_merge: merged sentence ids 1001,1002',
        source: 'manual',
        rule_id: null,
        resolved: false,
        created_at: '2026-06-22 12:00:00',
      },
    ])
    promoteLogToRule.mockResolvedValue({
      id: 9,
      name: 'manual_merge',
      stage: 'segment',
      pattern: 'manual_merge',
      action: 'review_sentence_boundary',
      enabled: true,
      version: 1,
      notes: 'Created from processing log',
      created_at: '2026-06-22 12:01:00',
    })
  })

  it('lists processing logs for the active book', async () => {
    const wrapper = mount(ProcessingLogPanel, { props: { bookId: 7 } })
    await flushPromises()

    expect(listProcessingLog).toHaveBeenCalledWith(7, false)
    expect(wrapper.text()).toContain('处理台账')
    expect(wrapper.text()).toContain('segment')
    expect(wrapper.text()).toContain('manual_merge')
    expect(wrapper.text()).toContain('There is, perhaps')
  })

  it('promotes a log entry to a rule and reloads the list', async () => {
    const wrapper = mount(ProcessingLogPanel, { props: { bookId: 7 } })
    await flushPromises()

    await wrapper.get('[data-test="promote-log-77"]').trigger('click')
    await flushPromises()

    expect(promoteLogToRule).toHaveBeenCalledWith(77, {
      name: 'manual_merge',
      pattern: 'manual_merge',
      action: 'review_sentence_boundary',
      notes: 'Created from processing log',
    })
    expect(wrapper.text()).toContain('已固化为规则：manual_merge')
    expect(listProcessingLog).toHaveBeenCalledTimes(2)
  })
})

async function flushPromises() {
  await Promise.resolve()
  await Promise.resolve()
}
