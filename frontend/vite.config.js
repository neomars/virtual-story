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
    },
  },
  server: {
    proxy: {
      // Redirige les appels API (ex: /api/scenes) vers le backend
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
      // Redirige les requêtes de fichiers vidéo (ex: /videos/monfichier.mp4)
      '/videos': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
      // Redirige les requêtes de miniatures (ex: /thumbnails/monimage.png)
      '/thumbnails': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      }
    }
  }
})
