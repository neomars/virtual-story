import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vueDevTools(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  server: {
    proxy: {
      // Single, simple rule for all API calls.
      // The backend now expects the /api prefix, so no rewrite is needed.
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },

      // Asset proxy rules remain the same
      '/videos': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
      '/thumbnails': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
      '/parts': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
      '/backgrounds': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      }
    }
  }
})
