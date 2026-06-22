import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type ImportJobResponse = {
  job_id: string
}

export type ImportReport = {
  book_id: number
  title: string
  root_toc_nodes: number
  toc_nodes_total: number
  included_toc_nodes: number
  title_only_toc_nodes: number
  excluded_toc_nodes: number
  chapters_imported: number
  paragraphs_imported: number
  sentences_imported: number
}

export type ImportProgressEvent = {
  job_id: string
  stage: string
  percent: number
  message: string
}

export type ImportDoneEvent = {
  job_id: string
  book_id: number
  report: ImportReport
}

export type ImportErrorEvent = {
  job_id: string
  error: {
    code: string
    message: string
  }
}

export type ImportJobStatus = {
  job_id: string
  state: 'running' | 'done' | 'error'
  stage: string
  percent: number
  message: string
  report?: ImportReport | null
  error?: {
    code: string
    message: string
  } | null
}

export async function importEpub(path: string): Promise<ImportJobResponse> {
  return invoke<ImportJobResponse>('import_epub', { path })
}

export async function getImportJob(jobId: string): Promise<ImportJobStatus> {
  return invoke<ImportJobStatus>('get_import_job', { jobId })
}

export async function getImportReport(bookId: number): Promise<ImportReport> {
  return invoke<ImportReport>('get_import_report', { bookId })
}

export async function onImportProgress(
  handler: (event: ImportProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ImportProgressEvent>('import://progress', (event) => handler(event.payload))
}

export async function onImportDone(handler: (event: ImportDoneEvent) => void): Promise<UnlistenFn> {
  return listen<ImportDoneEvent>('import://done', (event) => handler(event.payload))
}

export async function onImportError(
  handler: (event: ImportErrorEvent) => void,
): Promise<UnlistenFn> {
  return listen<ImportErrorEvent>('import://error', (event) => handler(event.payload))
}
