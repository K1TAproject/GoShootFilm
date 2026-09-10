<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { useRouter } from 'vue-router'
import UpdateDialog from './components/UpdateDialog.vue'
import type { LibraryMigrationResult, LibraryStatus } from './types'
import { errorMessage } from './utils/errors'

const router = useRouter()
const libraryStatus = ref<LibraryStatus>()
const libraryDialogOpen = ref(false)
const libraryBusy = ref(false)
const libraryError = ref('')
const libraryInfo = ref('')
const updateHandle = ref<Update>()
const updateDialogOpen = ref(false)
const updateBusy = ref(false)
const updateError = ref('')
const updateStatus = ref('')
const updateProgress = ref<number>()
const updateNotice = ref('')

function openRollFromRelation(rollId: number) {
  void router.push({ name: 'rolls', query: { roll: String(rollId) } })
}

async function refreshLibraryStatus() {
  try {
    libraryStatus.value = await invoke<LibraryStatus>('get_library_status')
  } catch (error) {
    libraryError.value = errorMessage(error, '无法读取图库设置')
  }
}

async function chooseLibraryPath() {
  const selected = await open({ directory: true, multiple: false, title: '选择 GoShootFilm 图库目录' })
  if (!selected || Array.isArray(selected)) return
  if (libraryStatus.value?.needsMigration
    && !confirm('将复制并校验现有图库后再切换位置。旧 AppData 图库会保留，是否继续？')) return
  libraryBusy.value = true
  libraryError.value = ''
  libraryInfo.value = ''
  try {
    const result = await invoke<LibraryMigrationResult>('set_library_path', { libraryPath: selected })
    await refreshLibraryStatus()
    window.dispatchEvent(new Event('goshootfilm:library-changed'))
    libraryInfo.value = result.migrated
      ? `迁移完成：已校验 ${result.fileCount} 个文件，共 ${result.totalBytes} 字节。旧图库仍保留。`
      : '图库位置已设置。'
  } catch (error) {
    libraryError.value = errorMessage(error, '设置图库位置失败')
  } finally {
    libraryBusy.value = false
  }
}

async function checkForUpdates(manual = false) {
  if (updateBusy.value) return
  updateBusy.value = true
  updateError.value = ''
  updateNotice.value = ''
  try {
    const update = await check({ timeout: 10_000 })
    if (!update) {
      if (manual) updateNotice.value = '当前已是最新版本。'
      return
    }
    updateHandle.value = update
    updateDialogOpen.value = true
  } catch (error) {
    const detail = errorMessage(error, '请检查网络连接后重试')
    const message = `检查更新失败：${detail}`
    if (manual) {
      updateError.value = message
      updateDialogOpen.value = true
    } else {
      updateNotice.value = message
    }
  } finally {
    updateBusy.value = false
  }
}

async function installUpdate() {
  const update = updateHandle.value
  if (!update) {
    await checkForUpdates(true)
    return
  }
  updateBusy.value = true
  updateError.value = ''
  updateProgress.value = 0
  updateStatus.value = '正在下载更新…'
  let downloaded = 0
  let total = 0
  try {
    await update.download(event => {
      if (event.event === 'Started') total = event.data.contentLength ?? 0
      if (event.event === 'Progress') downloaded += event.data.chunkLength
      if (total > 0) updateProgress.value = Math.min(100, Math.round(downloaded / total * 100))
      if (event.event === 'Finished') updateProgress.value = 100
    })
    updateStatus.value = '下载和验签完成。应用即将关闭并安装更新…'
    await update.install({ restartAfterInstall: true })
    await relaunch()
  } catch (error) {
    updateError.value = errorMessage(error, '更新下载、验签或安装失败')
    updateStatus.value = ''
  } finally {
    updateBusy.value = false
  }
}

async function deferUpdate() {
  if (updateBusy.value) return
  await updateHandle.value?.close()
  updateHandle.value = undefined
  updateDialogOpen.value = false
  updateError.value = ''
  updateStatus.value = ''
  updateProgress.value = undefined
}

onMounted(() => {
  void refreshLibraryStatus()
  window.setTimeout(() => void checkForUpdates(false), 1200)
})
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <RouterLink class="brand-block" to="/">
        <div class="app-mark">G</div>
        <div>
          <div class="app-title">GoShootFilm</div>
          <div class="app-caption">Film archive</div>
        </div>
      </RouterLink>
      <nav class="nav-tabs">
        <RouterLink to="/" active-class="" exact-active-class="router-link-exact-active">Home</RouterLink>
        <RouterLink to="/cameras">Cameras</RouterLink>
        <RouterLink to="/films">Films</RouterLink>
        <RouterLink to="/rolls">Rolls</RouterLink>
      </nav>
      <button class="library-settings-link" type="button" @click="libraryDialogOpen = true">图库设置</button>
      <button class="update-check-link" type="button" :disabled="updateBusy" @click="checkForUpdates(true)">检查更新</button>
      <span v-if="updateNotice" class="update-notice" role="status">{{ updateNotice }}</span>
    </aside>

    <main class="main-content">
      <div v-if="libraryStatus && (!libraryStatus.available || libraryStatus.needsMigration)" class="library-banner" role="alert">
        <span>{{ libraryStatus.error || (libraryStatus.needsMigration ? '检测到 AppData 中的旧图库，请选择新图库位置完成安全迁移。' : '尚未设置图库位置；照片导入暂不可用。') }}</span>
        <button type="button" @click="libraryDialogOpen = true">处理</button>
      </div>
      <RouterView v-slot="{ Component }">
        <component :is="Component" @jump-to-roll="openRollFromRelation" />
      </RouterView>
    </main>

    <div v-if="libraryDialogOpen" class="settings-overlay" @click.self="libraryDialogOpen = false">
      <section class="settings-dialog" role="dialog" aria-modal="true" aria-label="图库设置">
        <div class="settings-heading">
          <h2>图库设置</h2>
          <button type="button" aria-label="关闭" @click="libraryDialogOpen = false">×</button>
        </div>
        <p>数据库继续保存在 AppData；正式图库与可再生预览存放在这里。</p>
        <div class="library-path">{{ libraryStatus?.libraryPath || '尚未选择' }}</div>
        <div v-if="libraryError" class="feedback-error" role="alert">{{ libraryError }}</div>
        <div v-if="libraryInfo" class="feedback-info" role="status">{{ libraryInfo }}</div>
        <div class="settings-actions">
          <button type="button" :disabled="libraryBusy" @click="chooseLibraryPath">
            {{ libraryBusy ? '正在复制并校验…' : '选择图库目录' }}
          </button>
          <button type="button" :disabled="libraryBusy" @click="libraryDialogOpen = false">关闭</button>
        </div>
      </section>
    </div>
    <UpdateDialog
      :open="updateDialogOpen"
      :current-version="updateHandle?.currentVersion"
      :next-version="updateHandle?.version"
      :notes="updateHandle?.body"
      :progress="updateProgress"
      :status="updateStatus"
      :error="updateError"
      :busy="updateBusy"
      @install="installUpdate"
      @retry="installUpdate"
      @later="deferUpdate"
    />
  </div>
</template>

<style scoped>
.app-layout {
  width: 100%;
  height: 100vh;
  min-width: 0;
  min-height: 100vh;
  display: grid;
  grid-template-columns: 180px minmax(0, 1fr);
  background: #0f1115;
  color: #e5e7eb;
  overflow-x: hidden;
}

.sidebar {
  min-width: 0;
  height: 100vh;
  border-right: 1px solid #242833;
  background: #10141a;
  padding: 22px 16px;
  display: flex;
  flex-direction: column;
}

.library-settings-link,
.update-check-link {
  width: 100%;
  margin-top: auto;
  color: #9ca3af;
  background: transparent;
}

.update-check-link { margin-top: 8px; }
.update-notice { margin-top: 8px; color: #7f8b9b; font-size: 11px; line-height: 1.4; }

.library-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
  padding: 10px 12px;
  border: 1px solid #67552c;
  border-radius: 7px;
  background: #211d14;
  color: #e4cf94;
  font-size: 13px;
}

.settings-overlay {
  position: fixed;
  z-index: 1000;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(4, 7, 11, 0.76);
}

.settings-dialog {
  width: min(560px, 100%);
  padding: 20px;
  border: 1px solid #303846;
  border-radius: 10px;
  background: #151922;
}

.settings-heading,
.settings-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.settings-heading h2 { margin: 0; }
.settings-dialog p { color: #9ca3af; font-size: 13px; }
.library-path { margin: 14px 0; padding: 10px; border-radius: 6px; background: #0f131a; overflow-wrap: anywhere; }
.settings-actions { justify-content: flex-end; margin-top: 16px; }

.brand-block {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 4px 20px;
  border-bottom: 1px solid #242833;
  text-decoration: none;
}

.app-mark {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  background: #e5e7eb;
  color: #111827;
  font-weight: 800;
}

.app-title {
  color: #f9fafb;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0;
}

.app-caption {
  margin-top: 2px;
  color: #6f7b8c;
  font-size: 11px;
}

.nav-tabs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 18px;
}

.nav-tabs a {
  width: 100%;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  padding: 10px 12px;
  text-align: left;
  text-decoration: none;
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
}

.nav-tabs a:hover,
.nav-tabs a.router-link-exact-active {
  background: #1a1f2a;
  border-color: #2f3746;
  color: #f9fafb;
}

.main-content {
  width: 100%;
  height: 100vh;
  min-width: 0;
  padding: clamp(20px, 4vw, 44px);
  overflow-x: hidden;
  overflow-y: auto;
}

@media (max-width: 720px) {
  .app-layout {
    height: auto;
    grid-template-columns: 1fr;
  }

  .sidebar {
    height: auto;
    min-height: auto;
    border-right: 0;
    border-bottom: 1px solid #242833;
  }

  .nav-tabs {
    flex-direction: row;
  }

  .main-content {
    height: auto;
    overflow-y: visible;
    padding: 18px;
  }
}
</style>
