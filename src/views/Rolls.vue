<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { useRoute, useRouter } from 'vue-router'
import PageHeader from '../components/PageHeader.vue'
import PhotoGallery from '../components/PhotoGallery.vue'
import PhotoImportDialog from '../components/PhotoImportDialog.vue'
import PhotoLightbox from '../components/PhotoLightbox.vue'
import PhotoVersionTabs from '../components/PhotoVersionTabs.vue'
import type {
  Camera,
  ExportOriginalResult,
  Film,
  ImportAnalysisItem,
  ImportConflictAction,
  ImportDraft,
  ImportResult,
  LabOriginal,
  LabPreview,
  LabPreviewState,
  Photo,
  PhotoImportEntry,
  PhotoVersion,
  RollDetail,
  RollSummary,
} from '../types'
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
const activePhotoVersion = ref<PhotoVersion>('edit')
const labPreviews = ref<Record<number, LabPreviewState>>({})
const imageErrors = ref<Record<string, string>>({})
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

const lightboxImageSrc = ref<string>()
const lightboxPhoto = ref<Photo>()
const lightboxVersion = ref<PhotoVersion>('edit')
const isLightboxOpen = ref(false)
const importDialogOpen = ref(false)
const importVersion = ref<PhotoVersion>('edit')
const importItems = ref<ImportAnalysisItem[]>([])
const importError = ref('')
const importBusy = ref(false)

const editCount = computed(() => selectedRoll.value?.photos.filter(photo => photo.editScanPath).length ?? 0)
const labCount = computed(() => selectedRoll.value?.photos.filter(photo => photo.labScanPath).length ?? 0)

function openLightbox(photo: Photo, src: string) {
  lightboxPhoto.value = photo
  lightboxVersion.value = activePhotoVersion.value
  lightboxImageSrc.value = getImageUrl(src)
  isLightboxOpen.value = true
}

function closeLightbox() {
  isLightboxOpen.value = false
  lightboxImageSrc.value = undefined
  lightboxPhoto.value = undefined
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && isLightboxOpen.value) closeLightbox()
}

function getImageUrl(rawPath?: string) {
  if (!rawPath) return ''
  if (rawPath.startsWith('/')) return rawPath
  return convertFileSrc(rawPath)
}

function handleRollImageError(event: Event) {
  const image = event.target as HTMLImageElement
  image.src = '/film-stocks/placeholder.svg'
}

const imageExtensions = new Set(['png', 'jpg', 'jpeg', 'tif', 'tiff', 'webp', 'bmp', 'gif'])

function isImagePath(path: string) {
  const ext = path.split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase()
  return ext ? imageExtensions.has(ext) : false
}

function chooseDefaultPhotoVersion(photos: Photo[]) {
  activePhotoVersion.value = photos.some(photo => photo.editScanPath)
    ? 'edit'
    : photos.some(photo => photo.labScanPath)
      ? 'lab'
      : 'edit'
}

async function loadLabPreview(photo: Photo) {
  if (!photo.labScanPath || labPreviews.value[photo.id]?.loading || labPreviews.value[photo.id]?.previewPath) return
  labPreviews.value = {
    ...labPreviews.value,
    [photo.id]: { loading: true },
  }
  try {
    const preview = await invoke<LabPreview>('get_lab_preview', { photoId: photo.id })
    labPreviews.value = {
      ...labPreviews.value,
      [photo.id]: { loading: false, previewPath: preview.previewPath },
    }
  } catch (error) {
    labPreviews.value = {
      ...labPreviews.value,
      [photo.id]: { loading: false, error: formatError(error, '原始扫描图片无法读取') },
    }
  }
}

function loadCurrentLabPreviews() {
  if (activePhotoVersion.value !== 'lab' || !selectedRoll.value) return
  for (const photo of selectedRoll.value.photos) {
    if (photo.labScanPath) void loadLabPreview(photo)
  }
}

function switchPhotoVersion(version: PhotoVersion) {
  activePhotoVersion.value = version
  if (version === 'lab') loadCurrentLabPreviews()
}

function handleVersionImageError(photo: Photo, version: PhotoVersion) {
  const reason = version === 'lab' ? '原始扫描预览无法显示' : '调色图不存在或图片无法读取'
  imageErrors.value = { ...imageErrors.value, [`${version}:${photo.id}`]: reason }
}

function handleLightboxImageError() {
  if (lightboxPhoto.value) {
    handleVersionImageError(lightboxPhoto.value, lightboxVersion.value)
  }
  visibleError.value = lightboxVersion.value === 'lab'
    ? '原始扫描预览无法显示；原始文件未被修改。'
    : '调色图不存在或图片无法读取。'
  closeLightbox()
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
    labPreviews.value = {}
    imageErrors.value = {}
    chooseDefaultPhotoVersion(selectedRoll.value.photos)
    loadCurrentLabPreviews()
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

async function analyzeImport(version: PhotoVersion, drafts: ImportDraft[]) {
  if (!selectedRoll.value) return []
  return invoke<Omit<ImportAnalysisItem, 'conflictAction'>[]>('analyze_photo_import', {
    rollId: selectedRoll.value.id,
    version,
    drafts,
  })
}

function mergeImportAnalysis(
  analyzed: Omit<ImportAnalysisItem, 'conflictAction'>[],
  previous: ImportAnalysisItem[] = [],
) {
  return analyzed.map(item => {
    const oldAction = previous.find(previousItem => previousItem.sourcePath === item.sourcePath)?.conflictAction
    return {
      ...item,
      conflictAction: item.existingVersion
        ? oldAction && oldAction !== 'add' ? oldAction : 'ask'
        : 'add',
    } satisfies ImportAnalysisItem
  })
}

async function openImportReview(version: PhotoVersion, filePaths: string[]) {
  const imagePaths = filePaths.filter(isImagePath)
  if (imagePaths.length === 0) {
    visibleError.value = '所选内容中没有支持的图片文件'
    return
  }
  visibleError.value = ''
  visibleInfo.value = ''
  importError.value = ''
  importItems.value = []
  importVersion.value = version
  importBusy.value = true
  importDialogOpen.value = true
  try {
    const analyzed = await analyzeImport(version, imagePaths.map(sourcePath => ({ sourcePath })))
    importItems.value = mergeImportAnalysis(analyzed)
  } catch (error) {
    importError.value = formatError(error, '无法检查待导入图片')
  } finally {
    importBusy.value = false
  }
}

async function selectAndImportPhotos(version: PhotoVersion) {
  if (!selectedRoll.value) return

  try {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: version === 'lab' ? '原始扫描图片' : '调色图片',
          extensions: version === 'lab'
            ? ['png', 'jpg', 'jpeg', 'tif', 'tiff', 'webp', 'bmp', 'gif']
            : ['png', 'jpg', 'jpeg', 'webp']
        }
      ]
    })

    if (!selected) return
    const filePaths = Array.isArray(selected) ? selected : [selected]
    await openImportReview(version, filePaths)
  } catch (err) {
    console.error('Failed to import photos:', err)
    visibleError.value = formatError(err, '选择照片失败')
    isBusy.value = false
  }
}

function updateImportFrame(index: number, value?: number) {
  const item = importItems.value[index]
  if (!item) return
  importItems.value[index] = { ...item, frameNumber: value, issue: undefined }
}

function updateImportAction(index: number, value: ImportConflictAction) {
  const item = importItems.value[index]
  if (!item) return
  importItems.value[index] = { ...item, conflictAction: value }
}

function closeImportDialog() {
  if (importBusy.value) return
  importDialogOpen.value = false
  importItems.value = []
  importError.value = ''
}

async function confirmPhotoImport() {
  if (!selectedRoll.value) return
  if (importItems.value.some(item => item.conflictAction === 'cancel')) {
    closeImportDialog()
    visibleInfo.value = '已取消整批导入，未写入任何数据。'
    return
  }
  importBusy.value = true
  importError.value = ''
  try {
    const analyzed = await analyzeImport(importVersion.value, importItems.value.map(item => ({
      sourcePath: item.sourcePath,
      frameNumber: item.frameNumber,
    })))
    importItems.value = mergeImportAnalysis(analyzed, importItems.value)
    const issue = importItems.value.find(item => item.issue)?.issue
    if (issue) {
      importError.value = `请修正后重试：${issue}`
      return
    }
    const unresolved = importItems.value.find(item => item.existingVersion && !['skip', 'replace'].includes(item.conflictAction))
    if (unresolved) {
      importError.value = `Frame ${String(unresolved.frameNumber).padStart(2, '0')} 已有同版本图片，请选择跳过、替换或取消。`
      return
    }
    const entries: PhotoImportEntry[] = importItems.value.map(item => ({
      sourcePath: item.sourcePath,
      frameNumber: item.frameNumber!,
      conflictAction: item.conflictAction,
    }))
    const result = await invoke<ImportResult>('import_photo_versions', {
      rollId: selectedRoll.value.id,
      version: importVersion.value,
      entries,
    })
    const rollId = selectedRoll.value.id
    const importedVersion = importVersion.value
    importDialogOpen.value = false
    importItems.value = []
    labPreviews.value = {}
    imageErrors.value = {}
    await fetchRolls()
    await openRollDetail(rollId)
    switchPhotoVersion(importedVersion)
    visibleInfo.value = `整批导入完成：新增 ${result.importedCount}，配对或替换 ${result.updatedCount}，跳过 ${result.skippedCount}。`
  } catch (error) {
    importError.value = formatError(error, '导入失败，整批未写入')
  } finally {
    importBusy.value = false
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
        const paths = event.payload.paths.filter(isImagePath)
        const inferredVersion: PhotoVersion = paths.some(path => /\.tiff?$/i.test(path))
          ? 'lab'
          : activePhotoVersion.value
        void openImportReview(inferredVersion, paths).catch(err => {
          console.error('Failed to import dropped photos:', err)
          visibleError.value = formatError(err, '拖入照片失败，本批次未写入')
          isBusy.value = false
        })
      }
    })
  } catch (err) {
    console.warn('Native drag-and-drop listener is unavailable:', err)
    visibleError.value = formatError(err, '系统拖拽导入不可用，请使用导入按钮选择文件')
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

async function resolveLabOriginal(photo: Photo) {
  return invoke<LabOriginal>('get_lab_original', { photoId: photo.id })
}

async function openLabOriginal(photo: Photo) {
  visibleError.value = ''
  try {
    const original = await resolveLabOriginal(photo)
    await openPath(original.path)
  } catch (error) {
    visibleError.value = formatError(error, '打开原件失败')
  }
}

async function revealLabOriginal(photo: Photo) {
  visibleError.value = ''
  try {
    const original = await resolveLabOriginal(photo)
    await revealItemInDir(original.path)
  } catch (error) {
    visibleError.value = formatError(error, '无法在文件夹中定位原件')
  }
}

async function saveLabOriginal(photo: Photo) {
  visibleError.value = ''
  try {
    const original = await resolveLabOriginal(photo)
    const extension = original.fileName.split('.').pop()?.toLowerCase() || 'tif'
    const destination = await save({
      defaultPath: original.fileName,
      filters: [{ name: '原始扫描文件', extensions: [extension] }],
    })
    if (!destination) return
    let result = await invoke<ExportOriginalResult>('export_lab_original', {
      photoId: photo.id,
      destinationPath: destination,
      overwrite: false,
    })
    if (result.status === 'exists') {
      if (!confirm(`目标位置已存在同名文件：\n${result.path}\n\n是否覆盖？`)) return
      result = await invoke<ExportOriginalResult>('export_lab_original', {
        photoId: photo.id,
        destinationPath: destination,
        overwrite: true,
      })
    }
    visibleInfo.value = `原始扫描图已另存至：${result.path}`
  } catch (error) {
    visibleError.value = formatError(error, '导出原件失败')
  }
}

async function deletePhoto(photoId: number) {
  if (!confirm('确定删除这条照片记录吗？只会清理可再生预览，原始扫描图和调色图文件都会保留。')) return
  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('delete_photo', { photoId })
    if (selectedRoll.value) {
      selectedRoll.value.photos = selectedRoll.value.photos.filter(photo => photo.id !== photoId)
    }
    const remainingPreviews = { ...labPreviews.value }
    delete remainingPreviews[photoId]
    labPreviews.value = remainingPreviews
    visibleInfo.value = '照片记录已删除；正式图库文件已保留。'
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
              @error="handleRollImageError"
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
        <div class="gallery-toolbar">
          <div>
            <div class="section-title">照片</div>
            <small>同一 Frame 的两个版本共用一条照片记录。</small>
          </div>
          <PhotoVersionTabs
            :model-value="activePhotoVersion"
            :edit-count="editCount"
            :lab-count="labCount"
            @update:model-value="switchPhotoVersion"
          />
          <div class="gallery-actions">
            <button class="secondary-btn" :disabled="isBusy" @click="selectAndImportPhotos('edit')">导入调色图</button>
            <button class="secondary-btn" :disabled="isBusy" @click="selectAndImportPhotos('lab')">导入原始扫描</button>
          </div>
        </div>
        <div class="feedback-info path-notice">
          原始扫描与调色图存放在正式图库；原始扫描预览存放在独立、可再生的预览图库。界面中的 TIFF 预览不是原件。
        </div>
        <div
          :class="['drop-zone', isDraggingFiles ? 'drag-active' : '']"
        >
          <PhotoGallery
            :photos="selectedRoll.photos"
            :version="activePhotoVersion"
            :previews="labPreviews"
            :image-errors="imageErrors"
            :busy="isBusy"
            @view="openLightbox"
            @switch-version="switchPhotoVersion"
            @favorite="toggleFavorite"
            @delete="deletePhoto"
            @image-error="handleVersionImageError"
            @open-original="openLabOriginal"
            @reveal-original="revealLabOriginal"
            @save-original="saveLabOriginal"
          />
        </div>
      </section>
    </div>

    <PhotoLightbox
      :open="isLightboxOpen"
      :source="lightboxImageSrc"
      :frame-number="lightboxPhoto?.frameNumber"
      :version="lightboxVersion"
      @close="closeLightbox"
      @image-error="handleLightboxImageError"
      @open-original="lightboxPhoto && openLabOriginal(lightboxPhoto)"
      @reveal-original="lightboxPhoto && revealLabOriginal(lightboxPhoto)"
      @save-original="lightboxPhoto && saveLabOriginal(lightboxPhoto)"
    />

    <PhotoImportDialog
      :open="importDialogOpen"
      :version="importVersion"
      :items="importItems"
      :busy="importBusy"
      :error="importError"
      @close="closeImportDialog"
      @update-frame="updateImportFrame"
      @update-action="updateImportAction"
      @confirm="confirmPhotoImport"
    />
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
  color: #f9fafb;
  font-weight: 600;
}

.gallery-toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 14px;
  margin-bottom: 14px;
}

.gallery-toolbar small {
  display: block;
  margin-top: 5px;
  color: #7f8a99;
  font-size: 11px;
}

.gallery-actions {
  display: flex;
  gap: 8px;
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

.empty-state {
  color: #9ca3af;
  font-size: 13px;
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

  .gallery-toolbar {
    grid-template-columns: 1fr;
    align-items: start;
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
