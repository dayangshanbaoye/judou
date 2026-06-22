<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import {
  listProcessingLog,
  promoteLogToRule,
  type ProcessingLogEntry,
} from '../api/processingLog'

const props = defineProps<{
  bookId: number | null
}>()

const logs = ref<ProcessingLogEntry[]>([])
const errorMessage = ref('')
const successMessage = ref('')

async function loadLogs() {
  if (!props.bookId) {
    logs.value = []
    return
  }
  errorMessage.value = ''
  try {
    logs.value = await listProcessingLog(props.bookId, false)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

async function promote(log: ProcessingLogEntry) {
  errorMessage.value = ''
  successMessage.value = ''
  const ruleName = ruleNameFromLog(log)
  try {
    await promoteLogToRule(log.id, {
      name: ruleName,
      pattern: ruleName,
      action: 'review_sentence_boundary',
      notes: 'Created from processing log',
    })
    successMessage.value = `已固化为规则：${ruleName}`
    await loadLogs()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

function ruleNameFromLog(log: ProcessingLogEntry): string {
  const action = log.action_taken.split(':')[0]?.trim()
  return action || `${log.stage}_${log.source}`
}

onMounted(loadLogs)

watch(
  () => props.bookId,
  () => loadLogs(),
)
</script>

<template>
  <section class="processing-log-panel">
    <div>
      <p class="panel-label">Phase 3 · 处理台账</p>
      <h2>查看异常处理，并固化为规则。</h2>
    </div>

    <p v-if="!bookId" class="reader-empty">导入一本书后显示处理台账。</p>
    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
    <p v-if="successMessage" class="success">{{ successMessage }}</p>

    <div v-if="logs.length" class="log-list">
      <article v-for="log in logs" :key="log.id" class="log-entry">
        <header>
          <strong>{{ log.stage }}</strong>
          <span>{{ log.severity }} · {{ log.source }}</span>
        </header>
        <p>{{ log.action_taken }}</p>
        <p v-if="log.raw_snippet" class="log-snippet">{{ log.raw_snippet }}</p>
        <small>{{ log.location_ref }} · {{ log.created_at }}</small>
        <button
          class="secondary-button"
          :data-test="`promote-log-${log.id}`"
          type="button"
          @click="promote(log)"
        >
          固化为规则
        </button>
      </article>
    </div>
    <p v-else-if="bookId" class="reader-empty">暂无未解决台账。</p>
  </section>
</template>
