import { invoke } from '@tauri-apps/api/core'

export type ProcessingLogEntry = {
  id: number
  book_id: number | null
  stage: string
  severity: string
  location_ref: string | null
  raw_snippet: string | null
  action_taken: string
  source: string
  rule_id: number | null
  resolved: boolean
  created_at: string
}

export type ProcessingRule = {
  id: number
  name: string
  stage: string
  pattern: string | null
  action: string
  enabled: boolean
  version: number
  notes: string | null
  created_at: string
}

export type PromoteRuleInput = {
  name: string
  pattern?: string | null
  action: string
  notes?: string | null
}

export async function listProcessingLog(
  bookId?: number | null,
  resolved = false,
): Promise<ProcessingLogEntry[]> {
  return invoke<ProcessingLogEntry[]>('list_processing_log', {
    bookId: bookId ?? null,
    resolved,
  })
}

export async function promoteLogToRule(
  logId: number,
  input: PromoteRuleInput,
): Promise<ProcessingRule> {
  return invoke<ProcessingRule>('promote_log_to_rule', { logId, input })
}
