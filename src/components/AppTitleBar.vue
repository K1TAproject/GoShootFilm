<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import appIcon from '../assets/app-icon.png'

const appWindow = getCurrentWindow()
const maximized = ref(false)
let stopResize: (() => void) | undefined

async function syncMaximized() {
  try {
    maximized.value = await appWindow.isMaximized()
  } catch {
    // Window actions are optional UI affordances; failure must not break the application.
  }
}

async function runWindowAction(action: () => Promise<void>) {
  try {
    await action()
  } catch {
    // Keep the page usable if the host rejects a window operation.
  }
}

async function toggleMaximize() {
  await runWindowAction(() => appWindow.toggleMaximize())
  await syncMaximized()
}

function handleTitlebarMouseDown(event: MouseEvent) {
  if (event.button !== 0) return
  if (event.detail === 2) {
    void toggleMaximize()
  } else {
    void runWindowAction(() => appWindow.startDragging())
  }
}

onMounted(async () => {
  await syncMaximized()
  try {
    stopResize = await appWindow.onResized(() => void syncMaximized())
  } catch {
    // The controls still work even if state-change observation is unavailable.
  }
})

onBeforeUnmount(() => stopResize?.())
</script>

<template>
  <header class="titlebar">
    <div class="titlebar-drag" @mousedown="handleTitlebarMouseDown">
      <img :src="appIcon" alt="">
      <span>GoShootFilm</span>
    </div>
    <div class="window-controls">
      <button type="button" aria-label="最小化" title="最小化" @click="runWindowAction(() => appWindow.minimize())">
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 8.5h8" /></svg>
      </button>
      <button type="button" :aria-label="maximized ? '还原' : '最大化'" :title="maximized ? '还原' : '最大化'" @click="toggleMaximize">
        <svg v-if="maximized" viewBox="0 0 12 12" aria-hidden="true">
          <path d="M4 3.5V2h6v6H8.5M2 4h6v6H2z" />
        </svg>
        <svg v-else viewBox="0 0 12 12" aria-hidden="true"><rect x="2" y="2" width="8" height="8" /></svg>
      </button>
      <button class="close-button" type="button" aria-label="关闭" title="关闭" @click="runWindowAction(() => appWindow.close())">
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="m2.5 2.5 7 7m0-7-7 7" /></svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  z-index: 900;
  height: 40px;
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--app-border);
  background: rgba(8, 11, 16, 0.98);
  color: var(--app-text);
  user-select: none;
}

.titlebar-drag {
  min-width: 0;
  flex: 1;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  font-size: 12px;
  font-weight: 650;
  letter-spacing: 0.01em;
}

.titlebar-drag img {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  pointer-events: none;
}

.titlebar-drag span { pointer-events: none; }

.window-controls { display: flex; }

.window-controls button {
  width: 46px;
  height: 39px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: #aeb7c6;
  cursor: pointer;
}

.window-controls button:hover {
  background: #1a2130;
  color: #ffffff;
}

.window-controls .close-button:hover {
  background: #7f2934;
}

.window-controls button:focus-visible { outline-offset: -3px; }

.window-controls svg {
  width: 12px;
  height: 12px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1;
  shape-rendering: crispEdges;
}
</style>
