import { invoke } from '@tauri-apps/api/core'

export type ReaderView = {
  book_id: number
  book_title: string
  active_toc_node_id: number
  breadcrumb: ReaderBreadcrumb[]
  toc_nodes: ReaderTocNode[]
  paragraphs: ReaderParagraph[]
}

export type ReaderBreadcrumb = {
  id: number
  title: string
}

export type ReaderTocNode = {
  id: number
  parent_id: number | null
  title: string
  level: number
  order_index: number
  content_type: string
  included: boolean
}

export type ReaderParagraph = {
  id: number
  toc_node_id: number
  order_index: number
  source_href: string
  sentences: ReaderSentence[]
}

export type ReaderSentence = {
  id: number
  paragraph_id: number
  order_index: number
  text: string
  status: 'unread' | 'read' | 'understood' | 'flagged'
}

export async function getReaderView(
  bookId: number,
  tocNodeId?: number,
): Promise<ReaderView> {
  return invoke<ReaderView>('get_reader_view', {
    bookId,
    tocNodeId: tocNodeId ?? null,
  })
}

export async function updateSentenceStatus(
  sentenceId: number,
  status: ReaderSentence['status'],
): Promise<ReaderSentence> {
  return invoke<ReaderSentence>('update_sentence_status', { sentenceId, status })
}
