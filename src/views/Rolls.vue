<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { useRoute, useRouter } from 'vue-router'
import PageHeader from '../components/PageHeader.vue'
import type { Camera, Film, Photo, RollDetail, RollSummary } from '../types'
import { errorMessage as formatError } from '../utils/errors'

const route = useRoute()
const router = useRouter()

const rolls = ref<RollSummary[]>([])
const cameras = ref<Camera[]>([])
const films = ref<Film[]>([])
const currentView = ref<'grid' | 'add' | 'detail'>('grid')
const selectedRoll = ref<RollDetail | null>(null)
const isEditing = ref(false)
const isDraggingFiles = ref(false)
const isLoading = ref(false)
const isBusy = ref(false)
const visibleError = ref('')
const visibleInfo = ref('')
let unlistenDragDrop: (() => void) | null = null

const draftFilmBrand = ref('')
const draftFilmName = ref('')
const draftCameraBrand = ref('')
const draftCameraModel = ref('')
const draftShotMonth = ref('')

const activeFilmBrand = ref('')
const activeFilmName = ref('')
const activeCameraBrand = ref('')
const activeCameraModel = ref('')
const activeShotMonth = ref('')

const formFilmId = ref<number | null>(null)
const formCameraId = ref<number | null>(null)
const formShotMonth = ref('')
const formCity = ref('')
const formNote = ref('')

const editFilmId = ref<number | null>(null)
const editCameraId = ref<number | null>(null)
const editShotMonth = ref('')
const editCity = ref('')
const editNote = ref('')

const lightboxImageSrc = ref<string | null>(null)
const isLightboxOpen = ref(false)

function openLightbox(src: string) {
  lightboxImageSrc.value = src
  isLightboxOpen.value = true
}

function closeLightbox() {
  isLightboxOpen.value = false
  lightboxImageSrc.value = null
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && isLightboxOpen.value) closeLightbox()
}

function getImageUrl(rawPath?: string) {
  if (!rawPath) return ''
  if (rawPath.startsWith('/')) return rawPath
  return convertFileSrc(rawPath)
}

function handleImageError(event: Event) {
  const image = event.target as HTMLImageElement
  image.src = '/film-stocks/placeholder.svg'
}

const imageExtensions = new Set(['png', 'jpg', 'jpeg', 'tif', 'tiff', 'webp'])

function isImagePath(path: string) {
  const ext = path.split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase()
  return ext ? imageExtensions.has(ext) : false
}

async function fetchRolls() {
  isLoading.value = true
  visibleError.value = ''
  try {
    const [rollData, cameraData, filmData] = await Promise.all([
      invoke<RollSummary[]>('get_rolls'),
      invoke<Camera[]>('get_cameras'),
      invoke<Film[]>('get_films')
    ])
    rolls.value = rollData
    cameras.value = cameraData
    films.value = filmData

    if (!formCameraId.value && cameras.value.length > 0) {
      formCameraId.value = cameras.value[0].id
    }
    if (!formFilmId.value && films.value.length > 0) {
      formFilmId.value = films.value[0].id
    }

    if (selectedRoll.value) {
      const updated = rolls.value.find(roll => roll.id === selectedRoll.value?.id)
      if (updated) {
        selectedRoll.value = { ...selectedRoll.value, ...updated }
      }
    }

    const routeRollId = Number(route.query.roll)
    if (Number.isInteger(routeRollId) && routeRollId > 0) {
      await openRollDetail(routeRollId, false)
    }
  } catch (err) {
    console.error('Failed to fetch rolls:', err)
    visibleError.value = formatError(err, '无法读取拍摄卷数据')
  } finally {
    isLoading.value = false
  }
}

const availableFilmBrands = computed(() => {
  return Array.from(new Set(films.value.map(film => film.brand))).sort()
})

const availableFilmNames = computed(() => {
  return Array.from(new Set(
    films.value
      .filter(film => !draftFilmBrand.value || film.brand === draftFilmBrand.value)
      .map(film => film.name)
  )).sort()
})

const availableCameraBrands = computed(() => {
  return Array.from(new Set(cameras.value.map(camera => camera.brand))).sort()
})

const availableCameraModels = computed(() => {
  return Array.from(new Set(cameras.value.map(camera => camera.model))).sort()
})

const filteredRolls = computed(() => {
  return [...rolls.value]
    .filter(roll => {
      const matchFilmBrand = activeFilmBrand.value ? roll.filmBrand === activeFilmBrand.value : true
      const matchFilmName = activeFilmName.value ? roll.filmName === activeFilmName.value : true
      const matchCameraBrand = activeCameraBrand.value ? roll.cameraBrand === activeCameraBrand.value : true
      const matchCameraModel = activeCameraModel.value ? roll.cameraModel === activeCameraModel.value : true
      const matchShotMonth = activeShotMonth.value ? roll.shotMonth === activeShotMonth.value : true
      return matchFilmBrand && matchFilmName && matchCameraBrand && matchCameraModel && matchShotMonth
    })
    .sort((a, b) => {
      const aTime = a.shotMonth ? new Date(`${a.shotMonth}-01`).getTime() : 0
      const bTime = b.shotMonth ? new Date(`${b.shotMonth}-01`).getTime() : 0
      if (aTime !== bTime) return bTime - aTime
      return b.id - a.id
    })
})

function applyFilters() {
  activeFilmBrand.value = draftFilmBrand.value
  activeFilmName.value = draftFilmName.value
  activeCameraBrand.value = draftCameraBrand.value
  activeCameraModel.value = draftCameraModel.value
  activeShotMonth.value = draftShotMonth.value
}

function resetFilters() {
  draftFilmBrand.value = ''
  draftFilmName.value = ''
  draftCameraBrand.value = ''
  draftCameraModel.value = ''
  draftShotMonth.value = ''
  activeFilmBrand.value = ''
  activeFilmName.value = ''
  activeCameraBrand.value = ''
  activeCameraModel.value = ''
  activeShotMonth.value = ''
}

function resetAddForm() {
  formFilmId.value = films.value[0]?.id ?? null
  formCameraId.value = cameras.value[0]?.id ?? null
  formShotMonth.value = ''
  formCity.value = ''
  formNote.value = ''
}

function resetEditForm(roll: RollDetail) {
  editFilmId.value = roll.filmId
  editCameraId.value = roll.cameraId
  editShotMonth.value = roll.shotMonth || ''
  editCity.value = roll.city || ''
  editNote.value = roll.note || ''
}

function openAddForm() {
  if (cameras.value.length === 0) {
    visibleError.value = '请先在 Cameras 页面添加至少一台相机'
    return
  }
  resetAddForm()
  currentView.value = 'add'
}

async function openRollDetail(rollId: number, syncRoute = true) {
  isLoading.value = true
  visibleError.value = ''
  try {
    const roll = await invoke<RollDetail>('get_roll_detail', { id: rollId })
    selectedRoll.value = { ...roll, photos: roll.photos || [] }
    resetEditForm(selectedRoll.value)
    isEditing.value = false
    currentView.value = 'detail'
    if (syncRoute && String(route.query.roll || '') !== String(rollId)) {
      await router.replace({ name: 'rolls', query: { roll: String(rollId) } })
    }
  } catch (err) {
    console.error('Failed to fetch roll detail:', err)
    visibleError.value = formatError(err, '读取拍摄卷详情失败')
  } finally {
    isLoading.value = false
  }
}

function viewDetail(roll: RollSummary) {
  void openRollDetail(roll.id)
}

function backToGrid(syncRoute = true) {
  currentView.value = 'grid'
  selectedRoll.value = null
  isEditing.value = false
  isDraggingFiles.value = false
  if (syncRoute && route.query.roll) {
    void router.replace({ name: 'rolls' })
  }
}

async function handleAddRoll() {
  if (!formCameraId.value || !formFilmId.value) {
    visibleError.value = '请先添加相机并选择胶片型号'
    return
  }

  isBusy.value = true
  visibleError.value = ''
  try {
    const newRollId = await invoke<number>('add_roll', {
      cameraId: formCameraId.value,
      filmStockId: formFilmId.value,
      shotMonth: formShotMonth.value || null,
      city: formCity.value || null,
      note: formNote.value || null
    })
    resetAddForm()
    await fetchRolls()
    const created = rolls.value.find(roll => roll.id === newRollId)
    if (created) {
      await openRollDetail(created.id)
    } else {
      backToGrid()
    }
  } catch (err) {
    console.error('Failed to add roll:', err)
    visibleError.value = formatError(err, '新增拍摄卷失败')
  } finally {
    isBusy.value = false
  }
}

async function handleUpdateRoll() {
  if (!selectedRoll.value || !editFilmId.value || !editCameraId.value) {
    visibleError.value = '请选择有效的相机和胶片型号'
    return
  }

  isBusy.value = true
  visibleError.value = ''
  try {
    const selectedId = selectedRoll.value.id
    await invoke('update_roll', {
      id: selectedRoll.value.id,
      cameraId: editCameraId.value,
      filmStockId: editFilmId.value,
      shotMonth: editShotMonth.value || null,
      city: editCity.value || null,
      note: editNote.value || null
    })
    isEditing.value = false
    await fetchRolls()
    await openRollDetail(selectedId)
  } catch (err) {
    console.error('Failed to update roll:', err)
    visibleError.value = formatError(err, '更新拍摄卷失败')
  } finally {
    isBusy.value = false
  }
}

function getFilmOptionLabel(film: Film) {
  return `${film.brand} ${film.name} (ISO ${film.iso})`
}

function getCameraOptionLabel(camera: Camera) {
  return `${camera.brand} ${camera.model}`
}

async function importPhotoPaths(filePaths: string[]) {
  if (!selectedRoll.value) return

  const imagePaths = filePaths.filter(isImagePath)
  if (imagePaths.length === 0) return

  isBusy.value = true
  visibleError.value = ''
  visibleInfo.value = ''
  const rollId = selectedRoll.value.id
  try {
    const importedCount = await invoke<number>('import_photos', {
      rollId,
      filePaths: imagePaths
    })
    await fetchRolls()
    await openRollDetail(rollId)
    visibleInfo.value = importedCount > 0
      ? `已复制 ${importedCount} 张照片到应用图库。`
      : '没有可导入的照片。'
  } finally {
    isBusy.value = false
  }
}

async function selectAndImportPhotos() {
  if (!selectedRoll.value) return

  try {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'tif', 'tiff', 'webp']
        }
      ]
    })

    if (!selected) return
    const filePaths = Array.isArray(selected) ? selected : [selected]
    await importPhotoPaths(filePaths)
  } catch (err) {
    console.error('Failed to import photos:', err)
    visibleError.value = formatError(err, '导入照片失败，本批次未写入')
    isBusy.value = false
  }
}

async function setupNativeDragDrop() {
  try {
    unlistenDragDrop = await getCurrentWebview().onDragDropEvent(event => {
      if (currentView.value !== 'detail' || !selectedRoll.value) return

      if (event.payload.type === 'enter') {
        isDraggingFiles.value = true
        return
      }

      if (event.payload.type === 'leave') {
        isDraggingFiles.value = false
        return
      }

      if (event.payload.type === 'drop') {
        isDraggingFiles.value = false
        void importPhotoPaths(event.payload.paths).catch(err => {
          console.error('Failed to import dropped photos:', err)
          visibleError.value = formatError(err, '拖入照片失败，本批次未写入')
          isBusy.value = false
        })
      }
    })
  } catch (err) {
    console.warn('Native drag-and-drop listener is unavailable:', err)
  }
}

async function toggleFavorite(photo: Photo) {
  const next = !photo.isFavorite
  try {
    await invoke('toggle_photo_favorite', {
      photoId: photo.id,
      isFavorite: next
    })
    photo.isFavorite = next
  } catch (err) {
    console.error('Failed to update favorite status:', err)
    visibleError.value = formatError(err, '更新收藏状态失败')
  }
}

async function deletePhoto(photoId: number) {
  if (!confirm('确定删除应用图库中的这张照片吗？外部原始文件不会被删除。')) return
  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('delete_photo', { photoId })
    if (selectedRoll.value) {
      selectedRoll.value.photos = selectedRoll.value.photos.filter(photo => photo.id !== photoId)
    }
  } catch (err) {
    console.error('Failed to delete photo:', err)
    visibleError.value = formatError(err, '移除照片记录失败')
  } finally {
    isBusy.value = false
  }
}

function rollCover(roll: RollSummary) {
  return roll.coverPath || '/film-stocks/placeholder.svg'
}

watch(draftFilmBrand, () => {
  if (!availableFilmNames.value.includes(draftFilmName.value)) {
    draftFilmName.value = ''
  }
})

watch(
  () => route.query.roll,
  newValue => {
    const newId = Number(newValue)
    if (!Number.isInteger(newId) || newId <= 0) {
      if (currentView.value !== 'grid') {
        backToGrid(false)
      }
      return
    }
    if (selectedRoll.value?.id === newId && currentView.value === 'detail') return
    void openRollDetail(newId, false)
  },
)

onMounted(() => {
  fetchRolls()
  setupNativeDragDrop()
  window.addEventListener('keydown', handleWindowKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleWindowKeydown)
  if (unlistenDragDrop) {
    unlistenDragDrop()
    unlistenDragDrop = null
  }
})
</script>

<template>
  <section class="page">
    <div v-if="visibleError" class="feedback-error" role="alert">{{ visibleError }}</div>
    <div v-else-if="visibleInfo" class="feedback-info" role="status">{{ visibleInfo }}</div>
    <div v-else-if="isLoading" class="feedback-info">正在读取拍摄卷数据…</div>
    <div v-if="currentView === 'grid'" class="stack">
      <PageHeader title="Rolls" subtitle="按胶卷、设备和拍摄时间整理全部拍摄卷。" />

      <div class="filter-panel">
        <select v-model="draftFilmBrand">
          <option value="">全部胶卷品牌</option>
          <option v-for="brand in availableFilmBrands" :key="brand" :value="brand">{{ brand }}</option>
        </select>
        <select v-model="draftFilmName">
          <option value="">全部胶卷型号</option>
          <option v-for="name in availableFilmNames" :key="name" :value="name">{{ name }}</option>
        </select>
        <select v-model="draftCameraBrand">
          <option value="">全部设备品牌</option>
          <option v-for="brand in availableCameraBrands" :key="brand" :value="brand">{{ brand }}</option>
        </select>
        <select v-model="draftCameraModel">
          <option value="">全部设备型号</option>
          <option v-for="model in availableCameraModels" :key="model" :value="model">{{ model }}</option>
        </select>
        <input v-model="draftShotMonth" type="month" />
        <button class="primary-btn" @click="applyFilters">确定</button>
        <button class="secondary-btn fixed-action" @click="resetFilters">重置筛选</button>
      </div>

      <div class="roll-list">
        <button
          v-for="roll in filteredRolls"
          :key="roll.id"
          type="button"
          class="roll-card"
          @click="viewDetail(roll)"
        >
          <div class="roll-cover">
            <img
              :src="getImageUrl(rollCover(roll))"
              :alt="roll.filmInfo"
              @error="handleImageError"
            />
          </div>
          <div class="card-body">
            <div class="card-kicker">第 {{ roll.index }} 卷 · {{ roll.filmBrand }}</div>
            <h2>{{ roll.filmName }}</h2>
            <div class="roll-meta-lines">
              <span><b>设备</b>{{ roll.cameraInfo }}</span>
              <span><b>时间</b>{{ roll.shotMonth || '未记录' }}</span>
              <span><b>地点</b>{{ roll.city || '未记录' }}</span>
            </div>
          </div>
          <span class="roll-photo-count">{{ roll.photoCount }} 张</span>
        </button>

        <button class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加新卷</span>
        </button>
      </div>

      <div v-if="filteredRolls.length === 0" class="empty-state">没有符合条件的拍摄卷。</div>
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <PageHeader title="新增拍摄卷">
        <button class="secondary-btn" @click="backToGrid()">返回</button>
      </PageHeader>

      <div class="form-panel">
        <label>
          <span>胶卷型号 *</span>
          <select v-model.number="formFilmId">
            <option v-for="film in films" :key="film.id" :value="film.id">{{ getFilmOptionLabel(film) }}</option>
          </select>
        </label>
        <label>
          <span>使用设备 *</span>
          <select v-model.number="formCameraId">
            <option v-for="camera in cameras" :key="camera.id" :value="camera.id">{{ getCameraOptionLabel(camera) }}</option>
          </select>
        </label>
        <label>
          <span>拍摄日期</span>
          <input v-model="formShotMonth" type="month" />
        </label>
        <label>
          <span>地点</span>
          <input v-model="formCity" placeholder="杭州 / 北京" />
        </label>
        <label class="full-width">
          <span>备注</span>
          <textarea v-model="formNote"></textarea>
        </label>
        <div class="form-actions full-width">
          <button class="primary-btn" :disabled="isBusy" @click="handleAddRoll">保存</button>
          <button class="secondary-btn" @click="backToGrid()">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedRoll" class="stack">
      <PageHeader title="卷详情">
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="isEditing = true">编辑</button>
          <button v-if="!isEditing" class="secondary-btn" :disabled="isBusy" @click="selectAndImportPhotos">导入照片</button>
          <button class="secondary-btn" @click="backToGrid()">返回</button>
        </div>
      </PageHeader>

      <div class="detail-layout">
        <div v-if="!isEditing" class="detail-panel">
          <div class="card-kicker">第 {{ selectedRoll.index }} 卷</div>
          <h2>{{ selectedRoll.filmInfo }}</h2>
          <div class="detail-grid">
            <span>设备</span><strong>{{ selectedRoll.cameraInfo }}</strong>
            <span>胶卷</span><strong>{{ selectedRoll.filmInfo }}</strong>
            <span>日期</span><strong>{{ selectedRoll.shotMonth || '未记录' }}</strong>
            <span>地点</span><strong>{{ selectedRoll.city || '未记录' }}</strong>
            <span>备注</span><strong>{{ selectedRoll.note || '暂无备注' }}</strong>
          </div>
        </div>

        <div v-else class="form-panel detail-panel">
          <label>
            <span>胶卷型号</span>
            <select v-model.number="editFilmId">
              <option v-for="film in films" :key="film.id" :value="film.id">{{ getFilmOptionLabel(film) }}</option>
            </select>
          </label>
          <label>
            <span>使用设备</span>
            <select v-model.number="editCameraId">
              <option v-for="camera in cameras" :key="camera.id" :value="camera.id">{{ getCameraOptionLabel(camera) }}</option>
            </select>
          </label>
          <label>
            <span>拍摄日期</span>
            <input v-model="editShotMonth" type="month" />
          </label>
          <label>
            <span>地点</span>
            <input v-model="editCity" />
          </label>
          <label class="full-width">
            <span>备注</span>
            <textarea v-model="editNote"></textarea>
          </label>
          <div class="form-actions full-width">
            <button class="primary-btn" :disabled="isBusy" @click="handleUpdateRoll">保存</button>
            <button class="secondary-btn" @click="isEditing = false">取消</button>
          </div>
        </div>
      </div>

      <section class="related-section">
        <div class="section-title">照片</div>
        <div class="feedback-info path-notice">
          导入时会将照片复制到应用图库；外部原始文件移动或删除后，应用内照片仍可正常显示。
        </div>
        <div
          :class="['drop-zone', isDraggingFiles ? 'drag-active' : '']"
        >
          <div v-if="selectedRoll.photos.filter(photo => photo.editScanPath).length === 0" class="empty-state">
            暂无照片，拖入文件或点击导入即可添加。
          </div>

          <div class="photo-grid">
            <div
              v-for="photo in selectedRoll.photos.filter(photo => photo.editScanPath)"
              :key="photo.id"
              class="photo-card"
            >
              <div class="photo-frame">
                <img
                  :src="getImageUrl(photo.editScanPath)"
                  :alt="`Frame ${photo.frameNumber || ''}`"
                  role="button"
                  tabindex="0"
                  :aria-label="`打开 Frame ${photo.frameNumber || ''} 大图`"
                  @click.stop="openLightbox(getImageUrl(photo.editScanPath))"
                  @keydown.enter.stop="openLightbox(getImageUrl(photo.editScanPath))"
                  @keydown.space.prevent.stop="openLightbox(getImageUrl(photo.editScanPath))"
                  @error="handleImageError"
                />
                <button
                  class="photo-action favorite-action"
                  :class="{ active: photo.isFavorite }"
                  :aria-label="photo.isFavorite ? '取消收藏' : '收藏照片'"
                  :title="photo.isFavorite ? '取消收藏' : '收藏照片'"
                  @click.stop="toggleFavorite(photo)"
                >
                  {{ photo.isFavorite ? '★' : '☆' }}
                </button>
                <button class="photo-action delete-action" aria-label="移除照片记录" title="移除照片记录" :disabled="isBusy" @click.stop="deletePhoto(photo.id)">
                  ×
                </button>
              </div>
              <div class="photo-footer">Frame {{ photo.frameNumber || '?' }}</div>
            </div>
          </div>
        </div>
      </section>
    </div>

    <div v-if="isLightboxOpen" class="lightbox-overlay" @click="closeLightbox">
      <div class="lightbox-content">
        <img :src="lightboxImageSrc ?? undefined" alt="Enlarged Photo" class="lightbox-img" />
      </div>
      <button class="close-btn" aria-label="关闭大图" title="关闭大图" @click.stop="closeLightbox">×</button>
    </div>
  </section>
</template>

<style scoped>
.page {
  width: 100%;
  min-width: 0;
}

.stack {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

h2 {
  margin: 0;
  color: #f9fafb;
  letter-spacing: 0;
}

h2 {
  font-size: 18px;
  line-height: 1.3;
}

.filter-panel,
.form-panel,
.detail-panel,
.related-section {
  width: 100%;
  min-width: 0;
  background: #151922;
  border: 1px solid #262c38;
  border-radius: 8px;
  padding: 16px;
}

.filter-panel {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr)) auto auto;
  gap: 10px;
  align-items: center;
}

select,
input,
textarea {
  width: 100%;
  min-width: 0;
  border: 1px solid #303846;
  border-radius: 6px;
  background: #0f131b;
  color: #e5e7eb;
  padding: 9px 10px;
  outline: none;
}

select:focus,
input:focus,
textarea:focus {
  border-color: #6b7280;
}

textarea {
  min-height: 90px;
  resize: vertical;
}

.primary-btn,
.secondary-btn {
  border: 1px solid #384152;
  border-radius: 6px;
  padding: 9px 14px;
  cursor: pointer;
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease, opacity 0.16s ease;
}

.primary-btn {
  background: #e5e7eb;
  color: #111827;
}

.primary-btn:hover {
  background: #f9fafb;
}

.secondary-btn {
  background: #1d2430;
  color: #d1d5db;
}

.secondary-btn:hover {
  background: #252d3a;
  color: #f9fafb;
}

.fixed-action {
  min-width: 96px;
}

.roll-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.roll-card,
.add-card {
  min-width: 0;
  border: 1px solid #262c38;
  border-radius: 10px;
  background: #151922;
  color: inherit;
  cursor: pointer;
  overflow: hidden;
  transition: border-color 0.16s ease, background 0.16s ease, transform 0.16s ease;
}

.roll-card {
  position: relative;
  width: 100%;
  min-height: 154px;
  display: grid;
  grid-template-columns: 210px minmax(0, 1fr) auto;
  align-items: stretch;
  padding: 0 20px 0 0;
  text-align: left;
}

.roll-card:hover,
.add-card:hover {
  border-color: #4b5563;
  background: #1a202b;
  transform: translateY(-2px);
}

.roll-cover {
  background: #0f131b;
  border-right: 1px solid #262c38;
  overflow: hidden;
}

.roll-cover {
  min-height: 152px;
}

.roll-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.card-body {
  align-self: center;
  padding: 20px;
}

.card-kicker {
  margin-bottom: 8px;
  color: #9ca3af;
  font-size: 12px;
}

.roll-meta-lines {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 12px;
}

.roll-meta-lines span {
  color: #cbd5e1;
  font-size: 12px;
}

.roll-meta-lines b {
  display: inline-block;
  width: 44px;
  color: #778394;
  font-weight: 500;
}

.roll-photo-count {
  align-self: center;
  color: #8591a1;
  font-size: 12px;
}

.add-card {
  width: 100%;
  min-height: 84px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-style: dashed;
}

.plus-mark {
  font-size: 26px;
  line-height: 1;
}

.form-panel {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  color: #9ca3af;
  font-size: 13px;
}

.full-width {
  grid-column: 1 / -1;
}

.form-actions,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.detail-layout {
  display: block;
}

.detail-grid {
  display: grid;
  grid-template-columns: 90px minmax(0, 1fr);
  gap: 12px;
  margin-top: 18px;
  color: #9ca3af;
}

.detail-grid strong {
  color: #e5e7eb;
  font-weight: 500;
  overflow-wrap: anywhere;
}

.section-title {
  margin-bottom: 12px;
  color: #f9fafb;
  font-weight: 600;
}

.path-notice {
  margin-bottom: 12px;
}

.drop-zone {
  width: 100%;
  min-width: 0;
  border: 1px dashed transparent;
  border-radius: 8px;
  padding: 0;
}

.drop-zone.drag-active {
  border-color: #4b5563;
  background: rgba(75, 85, 99, 0.08);
  padding: 12px;
}

.photo-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 14px;
}

.photo-card {
  min-width: 0;
  border: 1px solid #262c38;
  border-radius: 8px;
  background: #10141c;
  overflow: hidden;
}

.photo-frame {
  position: relative;
  aspect-ratio: 4 / 3;
  background: #0f131b;
  overflow: hidden;
}

.photo-frame img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.photo-action {
  position: absolute;
  width: 30px;
  height: 30px;
  border-radius: 999px;
  border: 1px solid #394152;
  background: rgba(15, 19, 27, 0.85);
  color: #e5e7eb;
  cursor: pointer;
  opacity: 0;
  transform: translateY(-4px);
  transition: opacity 0.16s ease, transform 0.16s ease, background 0.16s ease, border-color 0.16s ease;
}

.photo-frame:hover .photo-action {
  opacity: 1;
  transform: translateY(0);
}

.favorite-action {
  top: 8px;
  right: 8px;
}

.favorite-action.active {
  border-color: #8b5cf6;
  color: #f5d0fe;
}

.delete-action {
  top: 8px;
  left: 8px;
}

.photo-action:hover {
  background: #1d2430;
}

.photo-footer {
  padding: 10px 12px;
  color: #9ca3af;
  font-size: 13px;
}

.empty-state {
  color: #9ca3af;
  font-size: 13px;
}

.lightbox-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.84);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.lightbox-content {
  max-width: 92vw;
  max-height: 92vh;
}

.lightbox-img {
  max-width: 92vw;
  max-height: 92vh;
  object-fit: contain;
}

.close-btn {
  position: absolute;
  top: 18px;
  right: 22px;
  width: 36px;
  height: 36px;
  border-radius: 999px;
  border: 1px solid #394152;
  background: #10141c;
  color: #e5e7eb;
  cursor: pointer;
}

@media (max-width: 900px) {
  .filter-panel {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .roll-card {
    grid-template-columns: 150px minmax(0, 1fr);
    padding-right: 0;
  }

  .roll-photo-count {
    display: none;
  }
}

@media (max-width: 760px) {
  .form-panel {
    grid-template-columns: 1fr;
  }

  .filter-panel {
    grid-template-columns: 1fr;
  }

  .roll-card {
    grid-template-columns: 110px minmax(0, 1fr);
  }

  .roll-cover {
    min-height: 170px;
  }
}
</style>
