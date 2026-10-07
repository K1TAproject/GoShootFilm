<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRoute, useRouter } from 'vue-router'
import PageHeader from '../components/PageHeader.vue'
import EquipmentItems from '../components/EquipmentItems.vue'
import StatCard from '../components/StatCard.vue'
import type { Camera, CameraDetail, CameraRoll, DigitalAlbum, EquipmentItem, RollSummary } from '../types'
import { errorMessage as formatError } from '../utils/errors'
import { filmDisplayName } from '../utils/films'

const cameras = ref<Camera[]>([])
const route = useRoute()
const router = useRouter()
const pageRoot = ref<HTMLElement>()
const gridScroll = ref(0)
const allRolls = ref<RollSummary[]>([])
const equipmentItems = ref<EquipmentItem[]>([])
const digitalAlbums = ref<DigitalAlbum[]>([])
const activeCategory = ref<'all' | 'camera' | 'lens' | 'other'>('all')
const categoryOptions = [
  { value: 'all', label: '全部' },
  { value: 'camera', label: '机身' },
  { value: 'lens', label: '镜头' },
  { value: 'other', label: '其他' },
] as const
const currentView = ref<'grid' | 'add' | 'detail'>('grid')

const formBrand = ref('')
const formModel = ref('')
const formFormat = ref('135')
const formPurchaseDate = ref('')
const formNote = ref('')

const selectedCamera = ref<Camera | null>(null)
const editSnapshot = ref<Camera | null>(null)
const relatedRolls = ref<CameraRoll[]>([])
const isEditing = ref(false)
const isLoading = ref(false)
const isBusy = ref(false)
const visibleError = ref('')
const today = (() => {
  const now = new Date()
  const year = now.getFullYear()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
})()

const emit = defineEmits<{
  (e: 'jump-to-roll', rollId: number): void
  (e: 'jump-to-album', albumId: number): void
}>()

async function fetchCameras() {
  isLoading.value = true
  visibleError.value = ''
  try {
    const [cameraData, rollData, equipmentData, albumData] = await Promise.all([
      invoke<Camera[]>('get_cameras'),
      invoke<RollSummary[]>('get_rolls'),
      invoke<EquipmentItem[]>('get_equipment_items'),
      invoke<DigitalAlbum[]>('get_digital_albums')
    ])
    cameras.value = cameraData
    allRolls.value = rollData
    equipmentItems.value = equipmentData
    digitalAlbums.value = albumData

    if (selectedCamera.value) {
      const updated = cameras.value.find(camera => camera.id === selectedCamera.value?.id)
      if (updated) {
        selectedCamera.value = { ...updated }
      }
    }
  } catch (err) {
    console.error('Failed to fetch cameras:', err)
    visibleError.value = formatError(err, '无法读取相机数据')
  } finally {
    isLoading.value = false
  }
}

const rollCounts = computed(() => {
  const counts = new Map<number, number>()
  for (const roll of allRolls.value) counts.set(roll.cameraId, (counts.get(roll.cameraId) ?? 0) + 1)
  return counts
})

function rollCount(cameraId: number) {
  return rollCounts.value.get(cameraId) ?? 0
}

const sortedCameras = computed(() => {
  return [...cameras.value].sort((a, b) =>
    rollCount(b.id) - rollCount(a.id)
    || a.brand.localeCompare(b.brand)
    || a.model.localeCompare(b.model)
    || a.id - b.id
  )
})

const lensCount = computed(() => equipmentItems.value.filter(item => item.category === 'lens').length)
const otherCount = computed(() => equipmentItems.value.filter(item => item.category === 'other').length)
const equipmentCount = computed(() => cameras.value.length + equipmentItems.value.length)
const cameraAlbums = computed(() => selectedCamera.value
  ? digitalAlbums.value.filter(album => album.cameraId === selectedCamera.value?.id)
  : [])

function resetCameraForm() {
  formBrand.value = ''
  formModel.value = ''
  formFormat.value = '135'
  formPurchaseDate.value = ''
  formNote.value = ''
}

function openAddForm() {
  resetCameraForm()
  currentView.value = 'add'
}

async function handleAddCamera() {
  if (!formBrand.value.trim() || !formModel.value.trim()) {
    visibleError.value = '请填写相机品牌和型号'
    return
  }

  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('add_camera', {
      brand: formBrand.value,
      model: formModel.value,
      format: formFormat.value || '135',
      purchaseDate: formPurchaseDate.value || null,
      note: formNote.value || null
    })
    resetCameraForm()
    currentView.value = 'grid'
    await fetchCameras()
  } catch (err) {
    console.error('Failed to add camera:', err)
    visibleError.value = formatError(err, '新增相机失败')
  } finally {
    isBusy.value = false
  }
}

function scrollContainer(): Window | HTMLElement {
  return pageRoot.value?.closest('.main-content') as HTMLElement || window
}

function scrollPosition() {
  const container = scrollContainer()
  return container === window ? window.scrollY : (container as HTMLElement).scrollTop
}

async function viewDetail(camera: Camera, updateRoute = true) {
  if (currentView.value === 'grid') gridScroll.value = scrollPosition()
  isLoading.value = true
  visibleError.value = ''
  try {
    const res = await invoke<CameraDetail>('get_camera_detail', { id: camera.id })
    selectedCamera.value = res.camera
    relatedRolls.value = res.rolls || []
    isEditing.value = false
    currentView.value = 'detail'
    if (updateRoute) await router.push({ name: 'cameras', query: { camera: String(camera.id) } })
  } catch (err) {
    console.error('Failed to fetch camera detail:', err)
    visibleError.value = formatError(err, '读取相机详情失败')
  } finally {
    isLoading.value = false
  }
}

async function handleUpdateCamera() {
  if (!selectedCamera.value) return

  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('update_camera', {
      id: selectedCamera.value.id,
      brand: selectedCamera.value.brand,
      model: selectedCamera.value.model,
      status: selectedCamera.value.status,
      format: selectedCamera.value.format || '135',
      purchaseDate: selectedCamera.value.purchaseDate || null,
      note: selectedCamera.value.note || null
    })
    isEditing.value = false
    editSnapshot.value = null
    await fetchCameras()
  } catch (err) {
    console.error('Failed to update camera:', err)
    visibleError.value = formatError(err, '更新相机失败')
  } finally {
    isBusy.value = false
  }
}

function startEditing() {
  if (!selectedCamera.value) return
  editSnapshot.value = { ...selectedCamera.value }
  isEditing.value = true
}

function cancelEditing() {
  if (editSnapshot.value) selectedCamera.value = { ...editSnapshot.value }
  editSnapshot.value = null
  isEditing.value = false
}

async function handleDeleteCamera(id: number) {
  if (!confirm('确定要删除这台相机吗？相关拍摄卷和照片记录也会被删除，但正式图库文件会保留。')) return

  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('delete_camera', { id })
    await backToGrid()
    await fetchCameras()
  } catch (err) {
    console.error('Failed to delete camera:', err)
    visibleError.value = formatError(err, '删除相机失败')
  } finally {
    isBusy.value = false
  }
}

async function backToGrid(updateRoute = true) {
  currentView.value = 'grid'
  selectedCamera.value = null
  relatedRolls.value = []
  isEditing.value = false
  if (updateRoute) await router.push({ name: 'cameras' })
  requestAnimationFrame(() => scrollContainer().scrollTo({ top: gridScroll.value, behavior: 'auto' }))
}

watch(() => route.query.camera, value => {
  const id = Number(value)
  if (Number.isInteger(id) && id > 0 && selectedCamera.value?.id !== id) {
    const camera = cameras.value.find(item => item.id === id)
    if (camera) void viewDetail(camera, false)
  } else if (!value && currentView.value === 'detail') {
    void backToGrid(false)
  }
})

onMounted(async () => {
  await fetchCameras()
  const id = Number(route.query.camera)
  const camera = cameras.value.find(item => item.id === id)
  if (camera) await viewDetail(camera, false)
})
</script>

<template>
  <section ref="pageRoot" class="page">
    <div v-if="visibleError" class="feedback-error" role="alert">{{ visibleError }}</div>
    <div v-else-if="isLoading" class="feedback-info">正在读取相机数据…</div>
    <div v-if="currentView === 'grid'" class="stack">
      <PageHeader title="Equipment" subtitle="管理相机机身、镜头与其他器材" />

      <div class="stats-grid">
        <StatCard label="器材总数" :value="equipmentCount" />
        <StatCard label="机身" :value="cameras.length" />
        <StatCard label="镜头" :value="lensCount" />
        <StatCard label="其他器材" :value="otherCount" />
      </div>

      <div class="category-tabs">
        <button v-for="option in categoryOptions" :key="option.value" type="button" :class="{ active: activeCategory === option.value }" @click="activeCategory = option.value">{{ option.label }}</button>
      </div>

      <div v-if="activeCategory === 'all' || activeCategory === 'camera'" class="cards-grid">
        <button
          type="button"
          v-for="camera in sortedCameras"
          :key="camera.id"
          class="camera-card"
          @click="viewDetail(camera)"
        >
          <span class="camera-kind">机身</span>
          <div class="camera-main">
            <small>{{ camera.brand }}</small>
            <h2 :title="`${camera.brand} ${camera.model}`">{{ camera.model }}</h2>
          </div>
          <div class="camera-footer">
            <span>{{ camera.format || '135' }}</span>
            <span>已拍摄{{ rollCount(camera.id) }}卷</span>
            <span>{{ camera.status === 'active' ? '在用' : '闲置' }}</span>
          </div>
        </button>

        <button v-if="activeCategory === 'camera'" class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加相机机身</span>
        </button>
      </div>
      <EquipmentItems
        v-if="activeCategory !== 'camera'"
        :items="equipmentItems"
        :category="activeCategory === 'all' ? 'all' : activeCategory"
        @refresh="fetchCameras"
        @error="visibleError = $event"
        @add-camera="openAddForm"
      />
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <PageHeader title="新增相机机身">
        <button class="secondary-btn" @click="backToGrid()">返回</button>
      </PageHeader>

      <div class="form-panel">
        <label>
          <span>品牌 *</span>
          <input v-model="formBrand" placeholder="Nikon" />
        </label>
        <label>
          <span>型号 *</span>
          <input v-model="formModel" placeholder="F2" />
        </label>
        <label>
          <span>画幅</span>
          <input v-model="formFormat" placeholder="135" />
        </label>
        <label>
          <span>购入日期</span>
          <input v-model="formPurchaseDate" type="date" :max="today" />
        </label>
        <label class="full-width">
          <span>备注</span>
          <textarea v-model="formNote"></textarea>
        </label>
        <div class="form-actions full-width">
          <button class="primary-btn" :disabled="isBusy" @click="handleAddCamera">保存</button>
          <button class="secondary-btn" @click="backToGrid()">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedCamera" class="stack">
      <PageHeader title="机身详情">
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="startEditing">编辑</button>
          <button v-if="!isEditing" class="danger-btn" :disabled="isBusy" @click="handleDeleteCamera(selectedCamera.id)">删除</button>
          <button class="secondary-btn" @click="backToGrid()">返回</button>
        </div>
      </PageHeader>

      <div v-if="!isEditing" class="detail-panel">
        <div class="camera-title">
          <span>{{ selectedCamera.brand }}</span>
          <h2>{{ selectedCamera.model }}</h2>
        </div>
        <div class="detail-grid">
          <span>状态</span><strong>{{ selectedCamera.status === 'active' ? '在用' : '闲置' }}</strong>
          <span>画幅</span><strong>{{ selectedCamera.format || '135' }}</strong>
          <span>购入日期</span><strong>{{ selectedCamera.purchaseDate || '未记录' }}</strong>
          <span>已拍摄卷数</span><strong>{{ relatedRolls.length }}</strong>
          <span>备注</span><strong>{{ selectedCamera.note || '暂无备注' }}</strong>
        </div>
      </div>

      <div v-else class="form-panel">
        <label>
          <span>品牌</span>
          <input v-model="selectedCamera.brand" />
        </label>
        <label>
          <span>型号</span>
          <input v-model="selectedCamera.model" />
        </label>
        <label>
          <span>状态</span>
          <select v-model="selectedCamera.status">
            <option value="active">在用</option>
            <option value="inactive">闲置</option>
          </select>
        </label>
        <label>
          <span>画幅</span>
          <input v-model="selectedCamera.format" />
        </label>
        <label>
          <span>购入日期</span>
          <input v-model="selectedCamera.purchaseDate" type="date" :max="today" />
        </label>
        <label class="full-width">
          <span>备注</span>
          <textarea v-model="selectedCamera.note"></textarea>
        </label>
        <div class="form-actions full-width">
          <button class="primary-btn" :disabled="isBusy" @click="handleUpdateCamera">保存</button>
          <button class="secondary-btn" @click="cancelEditing">取消</button>
        </div>
      </div>

      <section class="related-section">
        <div class="section-title">胶卷拍摄卷</div>
        <div v-if="relatedRolls.length === 0" class="empty-state">暂无拍摄记录。</div>
        <button
          v-for="roll in relatedRolls"
          v-else
          :key="roll.id"
          class="related-roll"
          @click="emit('jump-to-roll', roll.id)"
        >
          <span class="related-title">第 {{ roll.rollIndex }} 卷 · {{ filmDisplayName(roll.filmBrand, roll.filmName) }}</span>
          <span class="related-meta">{{ roll.shotMonth || '未记录日期' }} · {{ roll.city || '未记录地点' }}</span>
        </button>
      </section>
      <section class="related-section">
        <div class="section-title">数码相册</div>
        <div v-if="cameraAlbums.length === 0" class="empty-state">暂无数码相册。</div>
        <button v-for="album in cameraAlbums" v-else :key="album.id" class="related-roll" @click="emit('jump-to-album', album.id)">
          <span class="related-title">{{ album.title }}</span>
          <span class="related-meta">{{ album.shotDate || '未记录日期' }} · {{ album.photoCount }} 张照片</span>
        </button>
      </section>
    </div>
  </section>
</template>

<style scoped>
.stack {
  gap: 18px;
}

h2 {
  font-size: 22px;
  line-height: 1.2;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 16px;
  min-width: 0;
}

.stats-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; }
.category-tabs { display: flex; gap: 5px; border: 1px solid #2a3340; border-radius: 9px; background: #11161d; padding: 4px; width: fit-content; }
.category-tabs button { border: 0; border-radius: 6px; background: transparent; color: #909cac; padding: 8px 13px; cursor: pointer; }
.category-tabs button.active { background: #283140; color: #f8fafc; }

.camera-card,
.add-card,
.form-panel,
.detail-panel,
.related-section {
  min-width: 0;
  background: #151922;
  border: 1px solid #262c38;
  border-radius: 8px;
}

@media (max-width: 760px) { .stats-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }

.camera-card,
.add-card {
  min-height: 170px;
  padding: 18px;
  color: inherit;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease, transform 0.16s ease;
}

.camera-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  text-align: left;
}

.camera-card:hover,
.add-card:hover {
  border-color: #4b5563;
  background: #1a202b;
  transform: translateY(-2px);
}

.camera-main {
  display: grid;
  flex: 1;
  place-items: center;
  text-align: center;
}

.camera-kind { width: fit-content; border: 1px solid #394353; border-radius: 999px; padding: 4px 8px; color: #9da9b9; font-size: 11px; }
.camera-main small { color: #909cac; }

.camera-main h2 {
  display: -webkit-box;
  max-height: 2.4em;
  overflow: hidden;
  overflow-wrap: anywhere;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
}

.camera-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.camera-footer span {
  border-radius: 999px;
  background: #202737;
  color: #cbd5e1;
  padding: 4px 8px;
  font-size: 12px;
}

.form-panel,
.detail-panel,
.related-section {
  padding: 16px;
}

.primary-btn,
.secondary-btn,
.danger-btn {
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
}

.danger-btn {
  background: #271a1d;
  border-color: #5f2a33;
  color: #fca5a5;
}

.danger-btn:hover {
  background: #351f25;
}

.camera-title span {
  display: block;
  margin-bottom: 6px;
  color: #9ca3af;
}

.detail-grid {
  grid-template-columns: 110px minmax(0, 1fr);
}

.section-title {
  margin-bottom: 12px;
}
</style>
