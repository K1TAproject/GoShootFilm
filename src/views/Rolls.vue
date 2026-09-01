<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'

interface Photo {
  id: number
  frame_number?: number
  lab_scan_path?: string
  edit_scan_path?: string
  is_favorite?: number | boolean
}

interface Camera {
  id: number
  brand: string
  model: string
  status: string
}

interface FilmStock {
  id: number
  brand: string
  name: string
  iso: number
  type: string
}

interface RollItem {
  id: number
  cameraId: number
  filmId: number
  index: number
  shot_month?: string
  city?: string
  note?: string
  camera_brand?: string
  camera_model?: string
  film_brand?: string
  film_name?: string
  film_iso?: number
  camera_info: string
  film_info: string
  photos: Photo[]
}

const props = defineProps<{
  initialRollId?: number | null
}>()

const rolls = ref<RollItem[]>([])
const cameras = ref<Camera[]>([])
const films = ref<FilmStock[]>([])
const currentView = ref<'grid' | 'add' | 'detail'>('grid')
const selectedRoll = ref<RollItem | null>(null)
const isEditing = ref(false)
const isDraggingFiles = ref(false)
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

function getImageUrl(rawPath?: string) {
  if (!rawPath) return ''
  return convertFileSrc(rawPath)
}

function handleRollImageError(event: Event) {
  const image = event.target as HTMLImageElement
  image.src = '/film-stocks/placeholder.svg'
}

function handlePhotoImageError(event: Event) {
  const image = event.target as HTMLImageElement
  image.src = '/film-stocks/placeholder.svg'
}

const imageExtensions = new Set(['png', 'jpg', 'jpeg', 'tif', 'tiff', 'webp'])

function isImagePath(path: string) {
  const ext = path.split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase()
  return ext ? imageExtensions.has(ext) : false
}

async function fetchRolls() {
  try {
    const data: any = await invoke('get_all_data')
    rolls.value = data.rolls || []
    cameras.value = data.cameras || []
    films.value = data.films || []

    if (!formCameraId.value && cameras.value.length > 0) {
      formCameraId.value = cameras.value[0].id
    }
    if (!formFilmId.value && films.value.length > 0) {
      formFilmId.value = films.value[0].id
    }

    if (selectedRoll.value) {
      const updated = rolls.value.find(roll => roll.id === selectedRoll.value?.id)
      if (updated) {
        selectedRoll.value = { ...updated }
      }
    }

    if (props.initialRollId) {
      const target = rolls.value.find(roll => roll.id === props.initialRollId)
      if (target) {
        viewDetail(target)
      }
    }
  } catch (err) {
    console.error('Failed to fetch rolls:', err)
  }
}

const availableFilmBrands = computed(() => {
  return Array.from(new Set(films.value.map(film => film.brand))).sort()
})

const availableFilmNames = computed(() => {
  return Array.from(new Set(films.value.map(film => film.name))).sort()
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
      const matchFilmBrand = activeFilmBrand.value ? roll.film_brand === activeFilmBrand.value : true
      const matchFilmName = activeFilmName.value ? roll.film_name === activeFilmName.value : true
      const matchCameraBrand = activeCameraBrand.value ? roll.camera_brand === activeCameraBrand.value : true
      const matchCameraModel = activeCameraModel.value ? roll.camera_model === activeCameraModel.value : true
      const matchShotMonth = activeShotMonth.value ? roll.shot_month === activeShotMonth.value : true
      return matchFilmBrand && matchFilmName && matchCameraBrand && matchCameraModel && matchShotMonth
    })
    .sort((a, b) => {
      const aTime = a.shot_month ? new Date(`${a.shot_month}-01`).getTime() : 0
      const bTime = b.shot_month ? new Date(`${b.shot_month}-01`).getTime() : 0
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

function resetEditForm(roll: RollItem) {
  editFilmId.value = roll.filmId
  editCameraId.value = roll.cameraId
  editShotMonth.value = roll.shot_month || ''
  editCity.value = roll.city || ''
  editNote.value = roll.note || ''
}

function openAddForm() {
  resetAddForm()
  currentView.value = 'add'
}

function viewDetail(roll: RollItem) {
  selectedRoll.value = { ...roll }
  resetEditForm(roll)
  isEditing.value = false
  currentView.value = 'detail'
}

function backToGrid() {
  currentView.value = 'grid'
  selectedRoll.value = null
  isEditing.value = false
  isDraggingFiles.value = false
}

async function handleAddRoll() {
  if (!formCameraId.value || !formFilmId.value) return

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
      viewDetail(created)
    } else {
      backToGrid()
    }
  } catch (err) {
    console.error('Failed to add roll:', err)
  }
}

async function handleUpdateRoll() {
  if (!selectedRoll.value || !editFilmId.value || !editCameraId.value) return

  try {
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
    const updated = rolls.value.find(roll => roll.id === selectedRoll.value?.id)
    if (updated) {
      selectedRoll.value = { ...updated }
      resetEditForm(updated)
    }
  } catch (err) {
    console.error('Failed to update roll:', err)
  }
}

function getFilmOptionLabel(film: FilmStock) {
  return `${film.brand} ${film.name} (ISO ${film.iso})`
}

function getCameraOptionLabel(camera: Camera) {
  return `${camera.brand} ${camera.model}`
}

async function importPhotoPaths(filePaths: string[]) {
  if (!selectedRoll.value) return

  const imagePaths = filePaths.filter(isImagePath)
  if (imagePaths.length === 0) return

  await invoke('import_photos', {
    rollId: selectedRoll.value.id,
    filePaths: imagePaths
  })

  await fetchRolls()
  const updated = rolls.value.find(roll => roll.id === selectedRoll.value?.id)
  if (updated) {
    selectedRoll.value = { ...updated }
    resetEditForm(updated)
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
        })
      }
    })
  } catch (err) {
    console.warn('Native drag-and-drop listener is unavailable:', err)
  }
}

async function toggleFavorite(photo: Photo) {
  const next = Boolean(photo.is_favorite) ? 0 : 1
  try {
    await invoke('toggle_photo_favorite', {
      photoId: photo.id,
      isFavorite: next === 1
    })
    photo.is_favorite = next
  } catch (err) {
    console.error('Failed to update favorite status:', err)
  }
}

async function deletePhoto(photoId: number) {
  try {
    await invoke('delete_photo', { photoId })
    if (selectedRoll.value) {
      selectedRoll.value.photos = selectedRoll.value.photos.filter(photo => photo.id !== photoId)
    }
  } catch (err) {
    console.error('Failed to delete photo:', err)
  }
}

function rollCover(roll: RollItem) {
  return roll.photos.find(photo => photo.edit_scan_path)?.edit_scan_path || '/film-stocks/placeholder.svg'
}

watch(
  () => props.initialRollId,
  newId => {
    if (!newId) {
      if (currentView.value !== 'grid') {
        backToGrid()
      }
      return
    }

    const target = rolls.value.find(roll => roll.id === newId)
    if (target) {
      viewDetail(target)
    }
  },
  { immediate: true }
)

onMounted(() => {
  fetchRolls()
  setupNativeDragDrop()
})

onUnmounted(() => {
  if (unlistenDragDrop) {
    unlistenDragDrop()
    unlistenDragDrop = null
  }
})
</script>

<template>
  <section class="page">
    <div v-if="currentView === 'grid'" class="stack">
      <div class="page-header">
        <h1>Rolls</h1>
      </div>

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

      <div class="cards-grid">
        <article
          v-for="roll in filteredRolls"
          :key="roll.id"
          class="roll-card"
          @click="viewDetail(roll)"
        >
          <div class="roll-cover">
            <img
              :src="getImageUrl(rollCover(roll))"
              :alt="roll.film_info"
              @click.stop="openLightbox(getImageUrl(rollCover(roll)))"
              @error="handleRollImageError"
            />
          </div>
          <div class="card-body">
            <div class="card-kicker">第 {{ roll.index }} 卷</div>
            <h2>{{ roll.film_info }}</h2>
            <div class="meta-row">
              <span>{{ roll.camera_info }}</span>
              <span>{{ roll.shot_month || '未记录日期' }}</span>
              <span>{{ roll.city || '未记录地点' }}</span>
            </div>
          </div>
        </article>

        <button class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加新卷</span>
        </button>
      </div>

      <div v-if="filteredRolls.length === 0" class="empty-state">没有符合条件的拍摄卷。</div>
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <div class="page-header">
        <h1>新增拍摄卷</h1>
        <button class="secondary-btn" @click="backToGrid">返回</button>
      </div>

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
          <button class="primary-btn" @click="handleAddRoll">保存</button>
          <button class="secondary-btn" @click="backToGrid">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedRoll" class="stack">
      <div class="page-header">
        <h1>卷详情</h1>
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="isEditing = true">编辑</button>
          <button v-if="!isEditing" class="secondary-btn" @click="selectAndImportPhotos">导入照片</button>
          <button class="secondary-btn" @click="backToGrid">返回</button>
        </div>
      </div>

      <div class="detail-layout">
        <div class="detail-image">
          <img
            :src="getImageUrl(rollCover(selectedRoll))"
            :alt="selectedRoll.film_info"
            @click.stop="openLightbox(getImageUrl(rollCover(selectedRoll)))"
            @error="handleRollImageError"
          />
        </div>

        <div v-if="!isEditing" class="detail-panel">
          <div class="card-kicker">第 {{ selectedRoll.index }} 卷</div>
          <h2>{{ selectedRoll.film_info }}</h2>
          <div class="detail-grid">
            <span>设备</span><strong>{{ selectedRoll.camera_info }}</strong>
            <span>胶卷</span><strong>{{ selectedRoll.film_info }}</strong>
            <span>日期</span><strong>{{ selectedRoll.shot_month || '未记录' }}</strong>
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
            <button class="primary-btn" @click="handleUpdateRoll">保存</button>
            <button class="secondary-btn" @click="isEditing = false">取消</button>
          </div>
        </div>
      </div>

      <section class="related-section">
        <div class="section-title">照片</div>
        <div
          :class="['drop-zone', isDraggingFiles ? 'drag-active' : '']"
        >
          <div v-if="selectedRoll.photos.filter(photo => photo.edit_scan_path).length === 0" class="empty-state">
            暂无照片，拖入文件或点击导入即可添加。
          </div>

          <div class="photo-grid">
            <div
              v-for="photo in selectedRoll.photos.filter(photo => photo.edit_scan_path)"
              :key="photo.id"
              class="photo-card"
            >
              <div class="photo-frame">
                <img
                  :src="getImageUrl(photo.edit_scan_path)"
                  :alt="`Frame ${photo.frame_number || ''}`"
                  @click.stop="openLightbox(getImageUrl(photo.edit_scan_path))"
                  @error="handlePhotoImageError"
                />
                <button
                  class="photo-action favorite-action"
                  :class="{ active: Boolean(photo.is_favorite) }"
                  @click.stop="toggleFavorite(photo)"
                >
                  {{ Boolean(photo.is_favorite) ? '★' : '☆' }}
                </button>
                <button class="photo-action delete-action" @click.stop="deletePhoto(photo.id)">
                  ×
                </button>
              </div>
              <div class="photo-footer">Frame {{ photo.frame_number || '?' }}</div>
            </div>
          </div>
        </div>
      </section>
    </div>

    <div v-if="isLightboxOpen" class="lightbox-overlay" @click="closeLightbox">
      <div class="lightbox-content">
        <img :src="lightboxImageSrc ?? undefined" alt="Enlarged Photo" class="lightbox-img" />
      </div>
      <button class="close-btn" @click.stop="closeLightbox">×</button>
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

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

h1,
h2 {
  margin: 0;
  color: #f9fafb;
  letter-spacing: 0;
}

h1 {
  font-size: 24px;
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
.secondary-btn,
.danger-btn {
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

.danger-btn {
  background: #271a1d;
  border-color: #5f2a33;
  color: #fca5a5;
}

.danger-btn:hover {
  background: #351f25;
}

.fixed-action {
  min-width: 96px;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
  min-width: 0;
}

.roll-card,
.add-card {
  min-width: 0;
  min-height: 280px;
  border: 1px solid #262c38;
  border-radius: 8px;
  background: #151922;
  color: inherit;
  cursor: pointer;
  overflow: hidden;
  transition: border-color 0.16s ease, background 0.16s ease, transform 0.16s ease;
}

.roll-card:hover,
.add-card:hover {
  border-color: #4b5563;
  background: #1a202b;
  transform: translateY(-2px);
}

.roll-cover,
.detail-image {
  background: #0f131b;
  border-bottom: 1px solid #262c38;
  overflow: hidden;
}

.roll-cover {
  height: 132px;
}

.roll-cover img,
.detail-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.card-body {
  padding: 15px;
}

.card-kicker {
  margin-bottom: 8px;
  color: #9ca3af;
  font-size: 12px;
}

.meta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.meta-row span {
  border-radius: 999px;
  background: #202737;
  color: #cbd5e1;
  padding: 4px 8px;
  font-size: 12px;
}

.add-card {
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
  display: grid;
  grid-template-columns: minmax(220px, 340px) minmax(0, 1fr);
  gap: 16px;
}

.detail-image {
  min-height: 260px;
  border: 1px solid #262c38;
  border-radius: 8px;
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
  min-height: 180px;
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

.related-roll {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border: 1px solid #262c38;
  border-radius: 7px;
  background: #10141c;
  color: #e5e7eb;
  padding: 12px;
  cursor: pointer;
  text-align: left;
}

.related-roll + .related-roll {
  margin-top: 8px;
}

.related-title {
  font-weight: 600;
}

.related-meta,
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

  .detail-layout {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 760px) {
  .form-panel {
    grid-template-columns: 1fr;
  }

  .filter-panel {
    grid-template-columns: 1fr;
  }
}
</style>
