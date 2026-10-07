<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useRoute, useRouter } from 'vue-router'
import DigitalAlbumCover from '../components/DigitalAlbumCover.vue'
import PageHeader from '../components/PageHeader.vue'
import PhotoGallery from '../components/PhotoGallery.vue'
import PhotoLightbox from '../components/PhotoLightbox.vue'
import PhotoVersionTabs from '../components/PhotoVersionTabs.vue'
import type {
  Camera,
  DigitalAlbum,
  DigitalAlbumDetail,
  DigitalImportAnalysisItem,
  ImportResult,
  LabPreview,
  LabPreviewState,
  LibraryStatus,
  Photo,
  PhotoVersion,
} from '../types'
import { errorMessage } from '../utils/errors'
import { photoImageUrl } from '../utils/photos'

const route = useRoute()
const router = useRouter()
const pageRoot = ref<HTMLElement>()
const albums = ref<DigitalAlbum[]>([])
const cameras = ref<Camera[]>([])
const selected = ref<DigitalAlbumDetail | null>(null)
const view = ref<'grid' | 'form' | 'detail'>('grid')
const editing = ref(false)
const loading = ref(false)
const busy = ref(false)
const error = ref('')
const info = ref('')
const libraryAvailable = ref(false)
const shotDate = ref('')
const cameraId = ref<number | null>(null)
const city = ref('')
const note = ref('')
const activeVersion = ref<PhotoVersion>('edit')
const previews = ref<Record<string, LabPreviewState>>({})
const imageErrors = ref<Record<string, string>>({})
const lightboxPhoto = ref<Photo>()
const lightboxSource = ref('')
const lightboxOpen = ref(false)
const importOpen = ref(false)
const importVersion = ref<'raw' | 'edit'>('edit')
const importItems = ref<DigitalImportAnalysisItem[]>([])
const importError = ref('')
const listScroll = ref(0)

const photoAdapters = computed<Photo[]>(() => (selected.value?.photos || []).map(photo => ({
  id: photo.id,
  displayName: photo.pairingKey,
  labScanPath: photo.rawPath,
  editScanPath: photo.editPath,
  isFavorite: photo.isFavorite,
})))
const rawCount = computed(() => selected.value?.rawCount || 0)
const editCount = computed(() => selected.value?.editCount || 0)
const duplicatePairingKeys = computed(() => {
  const counts = new Map<string, number>()
  for (const item of importItems.value) {
    const key = item.pairingKey.trim().toLocaleLowerCase()
    if (key) counts.set(key, (counts.get(key) || 0) + 1)
  }
  return new Set([...counts].filter(([, count]) => count > 1).map(([key]) => key))
})

function resetForm() {
  shotDate.value = ''
  cameraId.value = null
  city.value = ''
  note.value = ''
}

async function fetchAlbums() {
  loading.value = true
  error.value = ''
  try {
    const [albumData, cameraData, library] = await Promise.all([
      invoke<DigitalAlbum[]>('get_digital_albums'),
      invoke<Camera[]>('get_cameras'),
      invoke<LibraryStatus>('get_library_status'),
    ])
    albums.value = albumData
    cameras.value = cameraData
    libraryAvailable.value = library.available
  } catch (cause) {
    error.value = errorMessage(cause, '无法读取数码相册')
  } finally {
    loading.value = false
  }
}

function openCreate() {
  resetForm()
  editing.value = false
  view.value = 'form'
}

async function saveAlbum() {
  busy.value = true
  error.value = ''
  try {
    const payload = {
      cameraId: cameraId.value,
      shotDate: shotDate.value || null,
      city: city.value || null,
      note: note.value || null,
    }
    const id = editing.value && selected.value
      ? (await invoke('update_digital_album', { id: selected.value.id, ...payload }), selected.value.id)
      : await invoke<number>('add_digital_album', payload)
    await fetchAlbums()
    await openDetail(id, true)
  } catch (cause) {
    error.value = errorMessage(cause, editing.value ? '更新相册失败' : '新建相册失败')
  } finally {
    busy.value = false
  }
}

async function openDetail(id: number, updateRoute = true) {
  if (view.value === 'grid') listScroll.value = scrollPosition()
  loading.value = true
  error.value = ''
  try {
    selected.value = await invoke<DigitalAlbumDetail>('get_digital_album_detail', { id })
    activeVersion.value = selected.value.editCount > 0 ? 'edit' : selected.value.rawCount > 0 ? 'lab' : 'edit'
    previews.value = {}
    imageErrors.value = {}
    view.value = 'detail'
    if (updateRoute) await router.push({ name: 'albums', query: { album: String(id) } })
  } catch (cause) {
    error.value = errorMessage(cause, '无法读取相册详情')
  } finally {
    loading.value = false
  }
}

async function backToGrid(updateRoute = true) {
  selected.value = null
  view.value = 'grid'
  if (updateRoute) await router.push({ name: 'albums' })
  requestAnimationFrame(() => scrollContainer().scrollTo({ top: listScroll.value, behavior: 'auto' }))
}

function scrollContainer(): Window | HTMLElement {
  return pageRoot.value?.closest('.main-content') as HTMLElement || window
}

function scrollPosition() {
  const container = scrollContainer()
  return container === window ? window.scrollY : (container as HTMLElement).scrollTop
}

function startEdit() {
  if (!selected.value) return
  shotDate.value = selected.value.shotDate?.slice(0, 7) || ''
  cameraId.value = selected.value.cameraId || null
  city.value = selected.value.city || ''
  note.value = selected.value.note || ''
  editing.value = true
  view.value = 'form'
}

async function removeAlbum() {
  if (!selected.value || !confirm('确定删除这个相册及照片记录吗？可再生预览会清理，但正式 CR2 和 PNG 文件会保留。')) return
  busy.value = true
  try {
    await invoke('delete_digital_album', { id: selected.value.id })
    await fetchAlbums()
    await backToGrid()
    info.value = '相册记录已删除；正式图库文件已保留。'
  } catch (cause) {
    error.value = errorMessage(cause, '删除相册失败')
  } finally {
    busy.value = false
  }
}

function key(photo: Photo, version: PhotoVersion) { return `${version}:${photo.id}` }

async function loadPreview(photo: Photo, version: PhotoVersion, thumbnail = true) {
  const previewKey = key(photo, version)
  if (thumbnail && (previews.value[previewKey]?.loading || previews.value[previewKey]?.previewPath)) return
  if (thumbnail) previews.value[previewKey] = { loading: true }
  try {
    const result = await invoke<LabPreview>('get_digital_photo_preview', {
      photoId: photo.id,
      version: version === 'lab' ? 'raw' : 'edit',
      thumbnail,
    })
    if (thumbnail) previews.value[previewKey] = { loading: false, previewPath: result.previewPath }
    return result.previewPath
  } catch (cause) {
    const message = errorMessage(cause, version === 'lab' ? 'CR2 内嵌预览无法生成' : '调色图无法读取')
    if (thumbnail) previews.value[previewKey] = { loading: false, error: message }
    else error.value = message
  }
}

function retryPreview(photo: Photo, version: PhotoVersion) {
  delete previews.value[key(photo, version)]
  delete imageErrors.value[key(photo, version)]
  void loadPreview(photo, version)
}

async function openLightbox(photo: Photo) {
  lightboxPhoto.value = photo
  const path = await loadPreview(photo, activeVersion.value, false)
  if (!path) return
  lightboxSource.value = photoImageUrl(path)
  lightboxOpen.value = true
}

async function toggleFavorite(photo: Photo) {
  try {
    const next = await invoke<boolean>('toggle_digital_photo_favorite', { photoId: photo.id })
    const target = selected.value?.photos.find(item => item.id === photo.id)
    if (target) target.isFavorite = next
  } catch (cause) {
    error.value = errorMessage(cause, '更新收藏状态失败')
  }
}

async function deletePhoto(photoId: number) {
  if (!confirm('确定删除这条照片记录吗？只清理预览，正式 CR2 和 PNG 文件会保留。')) return
  busy.value = true
  try {
    await invoke('delete_digital_photo', { photoId })
    if (selected.value) await openDetail(selected.value.id, false)
    info.value = '照片记录已删除；正式文件已保留。'
  } catch (cause) {
    error.value = errorMessage(cause, '删除照片记录失败')
  } finally {
    busy.value = false
  }
}

async function chooseImport(version: 'raw' | 'edit') {
  if (!selected.value || !libraryAvailable.value) {
    error.value = '请先在设置中选择可用的图库目录。'
    return
  }
  const result = await open({
    multiple: true,
    title: version === 'raw' ? '选择 CR2 原件' : '选择 PNG 调色图',
    filters: [{ name: version === 'raw' ? 'Canon CR2' : 'PNG', extensions: [version === 'raw' ? 'cr2' : 'png'] }],
  })
  const paths = !result ? [] : Array.isArray(result) ? result : [result]
  if (!paths.length) return
  try {
    const analysis = await invoke<Omit<DigitalImportAnalysisItem, 'conflictAction'>[]>('analyze_digital_import', {
      albumId: selected.value.id,
      version,
      sourcePaths: paths,
    })
    importVersion.value = version
    importItems.value = analysis.map(item => ({ ...item, conflictAction: item.existingVersion ? 'ask' : 'add' }))
    importError.value = ''
    importOpen.value = true
  } catch (cause) {
    error.value = errorMessage(cause, '检查导入文件失败')
  }
}

async function confirmImport() {
  if (!selected.value) return
  const empty = importItems.value.find(item => !item.pairingKey.trim())
  if (empty) {
    importError.value = `${empty.fileName} 的配对名称不能为空。`
    return
  }
  if (duplicatePairingKeys.value.size) {
    importError.value = '本批次仍有重复配对名称，请修改后重试。'
    return
  }
  const unresolved = importItems.value.find(item => item.existingVersion && !['skip', 'replace', 'cancel'].includes(item.conflictAction))
  if (unresolved) {
    importError.value = `${unresolved.pairingKey} 已存在当前版本，请选择跳过、替换或取消。`
    return
  }
  busy.value = true
  try {
    const result = await invoke<ImportResult>('import_digital_photos', {
      albumId: selected.value.id,
      version: importVersion.value,
      entries: importItems.value.map(item => ({
        sourcePath: item.sourcePath,
        pairingKey: item.pairingKey,
        conflictAction: item.conflictAction,
      })),
    })
    importOpen.value = false
    await openDetail(selected.value.id, false)
    info.value = `整批导入完成：新增 ${result.importedCount}，配对或替换 ${result.updatedCount}，跳过 ${result.skippedCount}。`
  } catch (cause) {
    importError.value = errorMessage(cause, '导入失败，整批未写入')
  } finally {
    busy.value = false
  }
}

async function openDirectory() {
  if (!selected.value) return
  try {
    await invoke('get_digital_media_directory', {
      albumId: selected.value.id,
      version: activeVersion.value === 'lab' ? 'raw' : 'edit',
    })
  } catch (cause) {
    error.value = errorMessage(cause, '无法打开图片原始位置')
  }
}

function handleKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && lightboxOpen.value) lightboxOpen.value = false
}

watch(() => route.query.album, value => {
  const id = Number(value)
  if (Number.isInteger(id) && id > 0 && selected.value?.id !== id) void openDetail(id, false)
  else if (!value && view.value === 'detail') void backToGrid(false)
})

onMounted(async () => {
  await fetchAlbums()
  const id = Number(route.query.album)
  if (Number.isInteger(id) && id > 0) await openDetail(id, false)
  window.addEventListener('keydown', handleKey)
})
onUnmounted(() => window.removeEventListener('keydown', handleKey))
</script>

<template>
  <section ref="pageRoot" class="page">
    <div v-if="error" class="feedback-error" role="alert">{{ error }}</div>
    <div v-else-if="info" class="feedback-info" role="status">{{ info }}</div>
    <div v-else-if="loading" class="feedback-info">正在读取数码相册…</div>

    <div v-if="view === 'grid'" class="stack">
      <PageHeader title="Albums" subtitle="整理数码 RAW 原件与调色照片">
        <button class="primary-btn" type="button" @click="openCreate">新建相册</button>
      </PageHeader>
      <div v-if="albums.length" class="album-grid">
        <button v-for="album in albums" :key="album.id" type="button" class="album-card" @click="openDetail(album.id)">
          <DigitalAlbumCover :photo-id="album.coverPhotoId" :version="album.coverVersion" />
          <div class="album-body">
            <h2 :title="album.title">{{ album.title }}</h2>
            <p>{{ album.cameraBrand && album.cameraModel ? `${album.cameraBrand} ${album.cameraModel}` : '未指定相机' }}</p>
            <footer><span>{{ album.photoCount }} 条记录</span><span>RAW {{ album.rawCount }}</span><span>调色 {{ album.editCount }}</span></footer>
          </div>
        </button>
      </div>
      <div v-else class="empty-state">还没有数码相册。</div>
    </div>

    <div v-else-if="view === 'form'" class="stack">
      <PageHeader :title="editing ? '编辑相册' : '新建相册'">
        <button class="secondary-btn" type="button" @click="editing && selected ? openDetail(selected.id, false) : backToGrid()">取消</button>
      </PageHeader>
      <div class="form-panel">
        <label><span>拍摄月份</span><input v-model="shotDate" type="month" /></label>
        <label><span>相机机身</span><select v-model="cameraId"><option :value="null">未指定</option><option v-for="camera in cameras" :key="camera.id" :value="camera.id">{{ camera.brand }} {{ camera.model }}</option></select></label>
        <label><span>地点</span><input v-model="city" /></label>
        <label class="wide"><span>备注</span><textarea v-model="note"></textarea></label>
        <div class="form-actions wide"><button class="primary-btn" type="button" :disabled="busy" @click="saveAlbum">{{ busy ? '保存中…' : '保存' }}</button></div>
      </div>
    </div>

    <div v-else-if="view === 'detail' && selected" class="stack">
      <PageHeader :title="selected.title" :subtitle="selected.cameraBrand && selected.cameraModel ? `${selected.cameraBrand} ${selected.cameraModel}` : '未指定相机'">
        <div class="actions"><button class="secondary-btn" @click="startEdit">编辑</button><button class="danger-btn" :disabled="busy" @click="removeAlbum">删除</button><button class="secondary-btn" @click="backToGrid()">返回相册</button></div>
      </PageHeader>
      <p v-if="selected.note" class="album-note">{{ selected.note }}</p>
      <section class="gallery-panel">
        <div class="gallery-toolbar">
          <div class="section-title">数码影像</div>
          <PhotoVersionTabs v-model="activeVersion" :edit-count="editCount" :lab-count="rawCount" lab-label="RAW" />
          <div class="actions">
            <button v-if="activeVersion === 'edit' ? editCount : rawCount" class="secondary-btn" @click="openDirectory">打开图片原始位置</button>
            <button class="secondary-btn" :disabled="busy || !libraryAvailable" @click="chooseImport(activeVersion === 'lab' ? 'raw' : 'edit')">{{ activeVersion === 'lab' ? '导入 CR2' : '导入 PNG' }}</button>
          </div>
        </div>
        <PhotoGallery
          :photos="photoAdapters"
          :version="activeVersion"
          :previews="previews"
          :image-errors="imageErrors"
          :busy="busy"
          lab-label="CR2 内嵌预览"
          @view="openLightbox"
          @favorite="toggleFavorite"
          @delete="deletePhoto"
          @request-preview="loadPreview"
          @retry-preview="retryPreview"
          @image-error="(photo, version) => imageErrors[key(photo, version)] = '图片无法显示'"
        />
      </section>
    </div>

    <PhotoLightbox :open="lightboxOpen" :source="lightboxSource" :display-name="lightboxPhoto?.displayName" :version="activeVersion" lab-label="CR2 内嵌预览" @close="lightboxOpen = false" @image-error="error = '图片无法显示'; lightboxOpen = false" />

    <div v-if="importOpen" class="modal" @click.self="!busy && (importOpen = false)">
      <section class="import-dialog" role="dialog" aria-modal="true">
        <header><div><h2>导入{{ importVersion === 'raw' ? ' CR2 原件' : ' PNG 调色图' }}</h2><p>按完整文件名配对；可以在导入前修改配对名称。</p></div><button class="close" @click="importOpen = false">×</button></header>
        <div v-if="importError" class="feedback-error">{{ importError }}</div>
        <div class="import-list">
          <article v-for="item in importItems" :key="item.sourcePath">
            <strong>{{ item.fileName }}</strong>
            <label><span>配对名称</span><input v-model="item.pairingKey" /></label>
            <span v-if="duplicatePairingKeys.has(item.pairingKey.trim().toLocaleLowerCase())" class="issue">本批次存在重复配对名称</span>
            <select v-if="item.existingVersion" v-model="item.conflictAction">
              <option value="ask" disabled>请选择冲突处理</option><option value="skip">跳过</option><option value="replace">替换记录版本</option><option value="cancel">取消整批</option>
            </select>
            <small v-else>{{ item.pairedVersion ? '将补充到已有配对记录' : '将新增照片记录' }}</small>
          </article>
        </div>
        <div class="form-actions"><button class="primary-btn" :disabled="busy" @click="confirmImport">{{ busy ? '导入中…' : '确认整批导入' }}</button><button class="secondary-btn" :disabled="busy" @click="importOpen = false">取消</button></div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.stack { gap: 18px; }
.album-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
.album-card { overflow: hidden; border: 1px solid #29313d; border-radius: 10px; background: #121820; color: inherit; padding: 0; text-align: left; cursor: pointer; }
.album-card:hover { border-color: #536174; transform: translateY(-2px); }
.album-body { padding: 15px; }
.album-body h2 { min-height: 2.4em; display: -webkit-box; overflow: hidden; -webkit-box-orient: vertical; -webkit-line-clamp: 2; margin: 0; font-size: 20px; }
.album-body p { color: #8f9bad; font-size: 12px; }
footer { display: flex; gap: 7px; flex-wrap: wrap; }
footer span { border-radius: 999px; background: #202735; padding: 5px 8px; color: #aeb8c7; font-size: 11px; }
.form-panel { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; border: 1px solid #29313d; border-radius: 10px; background: #141920; padding: 20px; }
label { display: grid; gap: 7px; }
label span { color: #9aa6b5; font-size: 12px; }
.wide { grid-column: 1 / -1; }
.gallery-panel { display: grid; gap: 16px; border: 1px solid #29313d; border-radius: 10px; background: #141920; padding: 18px; }
.gallery-toolbar { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.gallery-toolbar .section-title { margin-right: auto; font-weight: 700; }
.actions { display: flex; gap: 8px; flex-wrap: wrap; }
.album-note { color: #a4afbd; }
.modal { position: fixed; inset: 0; z-index: 10030; display: grid; place-items: center; background: rgba(2,5,9,.82); padding: 20px; }
.import-dialog { width: min(820px, 100%); max-height: 88vh; display: grid; gap: 14px; overflow: hidden; border: 1px solid #303948; border-radius: 12px; background: #141920; padding: 20px; }
.import-dialog header { display: flex; justify-content: space-between; gap: 12px; }
.import-dialog h2, .import-dialog p { margin: 0; }
.import-dialog p { margin-top: 5px; color: #8f9bad; font-size: 12px; }
.close { border: 0; background: transparent; color: #e5e7eb; font-size: 24px; cursor: pointer; }
.import-list { display: grid; gap: 9px; overflow: auto; }
.import-list article { display: grid; grid-template-columns: minmax(120px, 1fr) minmax(180px, 1fr) auto; align-items: end; gap: 10px; border: 1px solid #29313d; border-radius: 8px; padding: 11px; }
.import-list strong { overflow-wrap: anywhere; }
.import-list small { color: #8f9bad; }
.issue { color: #f1a8ad; font-size: 12px; }
.empty-state { border: 1px dashed #303846; border-radius: 9px; padding: 32px; color: #8f9bad; text-align: center; }
@media (max-width: 720px) { .form-panel { grid-template-columns: 1fr; } .wide { grid-column: auto; } .import-list article { grid-template-columns: 1fr; } }
</style>
