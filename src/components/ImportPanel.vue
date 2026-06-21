<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import {
  importEpub,
  onImportDone,
  onImportError,
  onImportProgress,
  type ImportReport,
} from '../api/importEpub'

const epubPath = ref('')
const activeJobId = ref('')
const statusMessage = ref('等待选择 EPUB 文件路径')
const percent = ref(0)
const report = ref<ImportReport | null>(null)
const errorMessage = ref('')
const unlisteners: Array<() => void> = []

async function startImport() {
  errorMessage.value = ''
  report.value = null
  const response = await importEpub(epubPath.value)
  activeJobId.value = response.job_id
  statusMessage.value = '导入任务已启动'
}

onMounted(async () => {
  unlisteners.push(
    await onImportProgress((event) => {
      if (activeJobId.value && event.job_id !== activeJobId.value) return
      percent.value = event.percent
      statusMessage.value = event.message
    }),
  )
  unlisteners.push(
    await onImportDone((event) => {
      if (activeJobId.value && event.job_id !== activeJobId.value) return
      activeJobId.value = event.job_id
      percent.value = 100
      statusMessage.value = '导入完成'
      report.value = event.report
    }),
  )
  unlisteners.push(
    await onImportError((event) => {
      if (activeJobId.value && event.job_id !== activeJobId.value) return
      errorMessage.value = event.error.message
      statusMessage.value = '导入失败'
    }),
  )
})

onUnmounted(() => {
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

    <button data-test="import-button" type="button" :disabled="!epubPath" @click="startImport">
      开始导入
    </button>

    <div class="import-status">
      <span>{{ statusMessage }}</span>
      <span v-if="activeJobId">Job: {{ activeJobId }}</span>
      <span>{{ percent }}%</span>
    </div>

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
