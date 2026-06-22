<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import {
  getReaderView,
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
            <span>{{ sentence.text }}</span>
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
