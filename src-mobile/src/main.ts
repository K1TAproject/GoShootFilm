import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import './style.css'

if ('serviceWorker' in navigator && import.meta.env.PROD) {
  window.addEventListener('load', () => {
    void navigator.serviceWorker.register('./sw.js').catch(() => undefined)
  })
}

createApp(App).use(router).mount('#app')
