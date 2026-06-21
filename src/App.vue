<script setup lang="ts">
import { ref } from 'vue'
import { ping } from './api/ping'

const message = ref('未连接')

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
  </main>
</template>
