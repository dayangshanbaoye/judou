<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import {
  getImportJob,
  importEpub,
  onImportDone,
  onImportError,
  onImportProgress,
  type ImportReport,
} from '../api/importEpub'

const referenceEpubPath =
  'C:\\Users\\zheng\\Documents\\agentic-engineering\\judou\\fixtures\\epub\\Inside the Box - David Epstein.epub'

const epubPath = ref(referenceEpubPath)
const activeJobId = ref('')
const statusMessage = ref('等待选择 EPUB 文件路径')
const percent = ref(0)
const report = ref<ImportReport | null>(null)
const errorMessage = ref('')
let pollTimer: number | undefined
const unlisteners: Array<() => void> = []

async function startImport() {
  errorMessage.value = ''
  report.value = null
  const normalizedPath = normalizeWindowsPath(epubPath.value)

  if (!normalizedPath) {
    statusMessage.value = '请先填写 EPUB 文件路径'
    return
  }

  epubPath.value = normalizedPath
  statusMessage.value = '正在请求后端导入…'
  percent.value = 0

  try {
    const response = await importEpub(normalizedPath)
    activeJobId.value = response.job_id
    statusMessage.value = '导入任务已启动'
    startPolling(response.job_id)
  } catch (error) {
    statusMessage.value = '导入启动失败'
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

function startPolling(jobId: string) {
  stopPolling()
  pollTimer = window.setInterval(async () => {
    try {
      const status = await getImportJob(jobId)
      applyJobStatus(status)
      if (status.state !== 'running') stopPolling()
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error)
      statusMessage.value = '读取导入进度失败'
      stopPolling()
    }
  }, 250)
}

function stopPolling() {
  if (pollTimer !== undefined) {
    window.clearInterval(pollTimer)
    pollTimer = undefined
  }
}

function applyJobStatus(status: {
  job_id: string
  state: string
  percent: number
  message: string
  report?: ImportReport | null
  error?: { message: string } | null
}) {
  if (activeJobId.value && status.job_id !== activeJobId.value) return
  activeJobId.value = status.job_id
  percent.value = status.percent
  statusMessage.value = status.message
  if (status.report) report.value = status.report
  if (status.error) errorMessage.value = status.error.message
}

function useReferenceBook() {
  epubPath.value = referenceEpubPath
  errorMessage.value = ''
  statusMessage.value = '已填入参考书路径'
}

function normalizeWindowsPath(path: string): string {
  return path.trim().replaceAll('\\\\', '\\')
}

onMounted(async () => {
  unlisteners.push(
    await onImportProgress((event) => {
      if (activeJobId.value && event.job_id !== activeJobId.value) return
      applyJobStatus({
        job_id: event.job_id,
        state: 'running',
        percent: event.percent,
        message: event.message,
      })
    }),
  )
  unlisteners.push(
    await onImportDone((event) => {
      if (activeJobId.value && event.job_id !== activeJobId.value) return
      applyJobStatus({
        job_id: event.job_id,
        state: 'done',
        percent: 100,
        message: '导入完成',
        report: event.report,
      })
      stopPolling()
    }),
  )
  unlisteners.push(
    await onImportError((event) => {
      if (activeJobId.value && event.job_id !== activeJobId.value) return
      applyJobStatus({
        job_id: event.job_id,
        state: 'error',
        percent: percent.value,
        message: '导入失败',
        error: event.error,
      })
      stopPolling()
    }),
  )
})

onUnmounted(() => {
  stopPolling()
  for (const unlisten of unlisteners) unlisten()
})
</script>

<template>
  <section class="import-panel">
    <div>
      <p class="panel-label">Phase 1 · EPUB 导入</p>
      <h2>导入一本 EPUB，查看解析报告。</h2>
    </div>

    <label class="path-field">
      <span>EPUB 路径</span>
      <input
        v-model="epubPath"
        data-test="epub-path"
        placeholder="C:\\Users\\zheng\\Documents\\agentic-engineering\\judou\\fixtures\\epub\\Inside the Box - David Epstein.epub"
      />
    </label>

    <div class="import-actions">
      <button data-test="import-button" type="button" @click="startImport">开始导入</button>
      <button class="secondary-button" type="button" @click="useReferenceBook">使用参考书路径</button>
    </div>

    <div class="import-status">
      <span>{{ statusMessage }}</span>
      <span v-if="activeJobId">Job: {{ activeJobId }}</span>
      <span>{{ percent }}%</span>
    </div>
    <progress data-test="import-progress" max="100" :value="percent">{{ percent }}%</progress>

    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>

    <dl v-if="report" class="report-grid">
      <div>
        <dt>书名</dt>
        <dd>{{ report.title }}</dd>
      </div>
      <div>
        <dt>目录节点</dt>
        <dd>{{ report.toc_nodes_total }}</dd>
      </div>
      <div>
        <dt>入库节点</dt>
        <dd>{{ report.included_toc_nodes }}</dd>
      </div>
      <div>
        <dt>已导章节</dt>
        <dd>{{ report.chapters_imported }}</dd>
      </div>
      <div>
        <dt>段落</dt>
        <dd>{{ report.paragraphs_imported }}</dd>
      </div>
    </dl>
  </section>
</template>
