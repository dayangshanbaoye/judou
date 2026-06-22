<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import {
  getReaderView,
  mergeSentences,
  splitSentence,
  updateSentenceStatus,
  type ReaderSentence,
  type ReaderView,
} from '../api/reader'

const props = defineProps<{
  bookId: number | null
}>()

const view = ref<ReaderView | null>(null)
const loading = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const selectedSentenceIds = ref<number[]>([])
const splitTarget = ref<ReaderSentence | null>(null)
const splitOffset = ref('')

async function loadReader(tocNodeId?: number) {
  if (!props.bookId) return
  loading.value = true
  errorMessage.value = ''
  try {
    view.value =
      tocNodeId === undefined
        ? await getReaderView(props.bookId)
        : await getReaderView(props.bookId, tocNodeId)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    loading.value = false
  }
}

async function markUnderstood(sentence: ReaderSentence) {
  const updated = await updateSentenceStatus(sentence.id, 'understood')
  sentence.status = updated.status
}

function toggleSentenceSelection(sentenceId: number, checked: boolean) {
  if (checked) {
    selectedSentenceIds.value = [...new Set([...selectedSentenceIds.value, sentenceId])]
    return
  }
  selectedSentenceIds.value = selectedSentenceIds.value.filter((id) => id !== sentenceId)
}

function toggleSentenceSelectionFromEvent(sentenceId: number, event: Event) {
  const target = event.target
  if (target instanceof HTMLInputElement) {
    toggleSentenceSelection(sentenceId, target.checked)
  }
}

function clearFeedback() {
  errorMessage.value = ''
  successMessage.value = ''
}

async function mergeSelectedSentences() {
  if (selectedSentenceIds.value.length < 2 || !view.value) {
    errorMessage.value = '请至少选择两个连续句子'
    successMessage.value = ''
    return
  }
  clearFeedback()
  await mergeSentences(selectedSentenceIds.value)
  selectedSentenceIds.value = []
  await loadReader(view.value.active_toc_node_id)
  successMessage.value = '已合并句子，并写入处理台账'
}

async function splitActiveSentence() {
  if (!splitTarget.value || !view.value) {
    errorMessage.value = '请先选择要拆分的句子'
    successMessage.value = ''
    return
  }
  const parsedOffset = Number.parseInt(splitOffset.value, 10)
  if (!Number.isFinite(parsedOffset) || parsedOffset <= 0) {
    errorMessage.value = '请输入合法拆分 offset'
    successMessage.value = ''
    return
  }
  clearFeedback()
  await splitSentence(splitTarget.value.id, parsedOffset)
  splitTarget.value = null
  splitOffset.value = ''
  await loadReader(view.value.active_toc_node_id)
  successMessage.value = '已拆分句子，并写入处理台账'
}

function chooseSplitTarget(sentence: ReaderSentence) {
  clearFeedback()
  splitTarget.value = sentence
  splitOffset.value = ''
}

onMounted(() => loadReader())

watch(
  () => props.bookId,
  () => loadReader(),
)
</script>

<template>
  <section class="reader-panel">
    <div>
      <p class="panel-label">Phase 3 · 阅读器</p>
      <h2>目录、句子流与精读状态。</h2>
    </div>

    <p v-if="!bookId" class="reader-empty">导入一本书后显示阅读器。</p>
    <p v-else-if="loading" class="status">正在加载阅读器…</p>
    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
    <p v-if="successMessage" class="success">{{ successMessage }}</p>

    <div v-if="view" class="reader-layout">
      <aside class="reader-toc">
        <h3>{{ view.book_title }}</h3>
        <button
          v-for="node in view.toc_nodes"
          :key="node.id"
          :class="{ active: node.id === view.active_toc_node_id }"
          :data-test="`toc-node-${node.id}`"
          :disabled="!node.included"
          type="button"
          @click="loadReader(node.id)"
        >
          <span :style="{ paddingLeft: `${Math.max(0, node.level - 1) * 12}px` }">
            {{ node.title }}
          </span>
        </button>
      </aside>

      <article class="reader-main">
        <nav class="breadcrumb" aria-label="breadcrumb">
          <span v-for="(crumb, index) in view.breadcrumb" :key="crumb.id">
            <span v-if="index"> / </span>{{ crumb.title }}
          </span>
        </nav>

        <div class="correction-toolbar">
          <button data-test="merge-selected" type="button" @click="mergeSelectedSentences">
            合并选中句子
          </button>
          <span class="toolbar-hint">勾选两个或更多连续句子后合并；系统会写入处理台账。</span>
        </div>

        <div v-if="splitTarget" class="split-panel" data-test="split-panel">
          <div>
            <p class="panel-label">正在拆分</p>
            <p class="split-target-text">{{ splitTarget.text }}</p>
            <p class="context-line">在下方输入拆分位置；当前版本按英文文本 offset 拆分。</p>
          </div>
          <label>
            <span>拆分 offset</span>
            <input v-model="splitOffset" data-test="split-offset" inputmode="numeric" />
          </label>
          <button data-test="split-active" type="button" @click="splitActiveSentence">
            拆分当前句子
          </button>
        </div>

        <section
          v-for="paragraph in view.paragraphs"
          :key="paragraph.id"
          class="reader-paragraph"
        >
          <p class="context-line">{{ paragraph.source_href }} · #{{ paragraph.order_index }}</p>
          <p
            v-for="sentence in paragraph.sentences"
            :key="sentence.id"
            class="reader-sentence"
            :data-status="sentence.status"
          >
            <input
              type="checkbox"
              :checked="selectedSentenceIds.includes(sentence.id)"
              :data-test="`select-sentence-${sentence.id}`"
              @change="toggleSentenceSelectionFromEvent(sentence.id, $event)"
            />
            <span>{{ sentence.text }}</span>
            <button
              class="secondary-button"
              :data-test="`split-target-${sentence.id}`"
              type="button"
              @click="chooseSplitTarget(sentence)"
            >
              拆分这句
            </button>
            <button
              class="secondary-button"
              :data-test="`sentence-status-${sentence.id}`"
              type="button"
              @click="markUnderstood(sentence)"
            >
              {{ sentence.status }}
            </button>
          </p>
        </section>
      </article>
    </div>
  </section>
</template>
