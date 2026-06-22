<script setup lang="ts">
import { ref } from 'vue'
import { ping } from './api/ping'
import ImportPanel from './components/ImportPanel.vue'
import ProcessingLogPanel from './components/ProcessingLogPanel.vue'
import ReaderPanel from './components/ReaderPanel.vue'

const message = ref('未连接')
const selectedBookId = ref<number | null>(null)

async function checkBackend() {
  const response = await ping('judou')
  message.value = response.message
}
</script>

<template>
  <main class="app-shell">
    <section class="hero">
      <p class="eyebrow">Judou · 句读</p>
      <h1>逐句精读，结构化内化。</h1>
      <button type="button" @click="checkBackend">Ping 后端</button>
      <p class="status">{{ message }}</p>
    </section>
    <ImportPanel @imported="selectedBookId = $event" />
    <ReaderPanel :book-id="selectedBookId" />
    <ProcessingLogPanel :book-id="selectedBookId" />
  </main>
</template>
