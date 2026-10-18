import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  root: 'src-mobile',
  base: './',
  plugins: [vue()],
  server: {
    port: 1422,
    host: true,
    fs: {
      allow: ['..'],
    },
  },
  build: {
    outDir: '../dist-mobile',
    emptyOutDir: true,
    target: 'es2020',
  },
})
