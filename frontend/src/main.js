import { createApp } from 'vue'
import App from './App.vue'
import router from './router'

const app = createApp(App)

// Disable Vue Devtools in production
if (import.meta.env.PROD) {
  app.config.devtools = false;
}

app.use(router)

app.mount('#app')
