import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  test: {
    environment: 'jsdom',
  },
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
  },
})
