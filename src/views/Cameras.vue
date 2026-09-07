<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from '../components/PageHeader.vue'
import type { Camera, CameraDetail, CameraRoll, RollSummary } from '../types'
import { errorMessage as formatError } from '../utils/errors'
import { filmDisplayName } from '../utils/films'

const cameras = ref<Camera[]>([])
const allRolls = ref<RollSummary[]>([])
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

const emit = defineEmits<{
  (e: 'jump-to-roll', rollId: number): void
}>()

async function fetchCameras() {
  isLoading.value = true
  visibleError.value = ''
  try {
    const [cameraData, rollData] = await Promise.all([
      invoke<Camera[]>('get_cameras'),
      invoke<RollSummary[]>('get_rolls')
    ])
    cameras.value = cameraData
    allRolls.value = rollData

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

async function viewDetail(camera: Camera) {
  isLoading.value = true
  visibleError.value = ''
  try {
    const res = await invoke<CameraDetail>('get_camera_detail', { id: camera.id })
    selectedCamera.value = res.camera
    relatedRolls.value = res.rolls || []
    isEditing.value = false
    currentView.value = 'detail'
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
    backToGrid()
    await fetchCameras()
  } catch (err) {
    console.error('Failed to delete camera:', err)
    visibleError.value = formatError(err, '删除相机失败')
  } finally {
    isBusy.value = false
  }
}

function backToGrid() {
  currentView.value = 'grid'
  selectedCamera.value = null
  relatedRolls.value = []
  isEditing.value = false
}

onMounted(() => {
  fetchCameras()
})
</script>

<template>
  <section class="page">
    <div v-if="visibleError" class="feedback-error" role="alert">{{ visibleError }}</div>
    <div v-else-if="isLoading" class="feedback-info">正在读取相机数据…</div>
    <div v-if="currentView === 'grid'" class="stack">
      <PageHeader title="Cameras" subtitle="管理相机设备与每台相机的拍摄记录。" />

      <div class="cards-grid">
        <button
          type="button"
          v-for="camera in sortedCameras"
          :key="camera.id"
          class="camera-card"
          @click="viewDetail(camera)"
        >
          <div class="camera-main">
            <h2>{{ camera.brand }} {{ camera.model }}</h2>
          </div>
          <div class="camera-footer">
            <span>{{ camera.format || '135' }}</span>
            <span>已拍摄{{ rollCount(camera.id) }}卷</span>
            <span>{{ camera.status === 'active' ? '在用' : '闲置' }}</span>
          </div>
        </button>

        <button class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加新设备</span>
        </button>
      </div>
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <PageHeader title="新增相机">
        <button class="secondary-btn" @click="backToGrid">返回</button>
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
          <input v-model="formPurchaseDate" type="date" />
        </label>
        <label class="full-width">
          <span>备注</span>
          <textarea v-model="formNote"></textarea>
        </label>
        <div class="form-actions full-width">
          <button class="primary-btn" :disabled="isBusy" @click="handleAddCamera">保存</button>
          <button class="secondary-btn" @click="backToGrid">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedCamera" class="stack">
      <PageHeader title="相机详情">
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="startEditing">编辑</button>
          <button v-if="!isEditing" class="danger-btn" :disabled="isBusy" @click="handleDeleteCamera(selectedCamera.id)">删除</button>
          <button class="secondary-btn" @click="backToGrid">返回</button>
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
            <option value="disable">闲置</option>
          </select>
        </label>
        <label>
          <span>画幅</span>
          <input v-model="selectedCamera.format" />
        </label>
        <label>
          <span>购入日期</span>
          <input v-model="selectedCamera.purchaseDate" type="date" />
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
        <div class="section-title">关联 Rolls</div>
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
  font-size: 22px;
  line-height: 1.2;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 16px;
  min-width: 0;
}

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

.form-panel,
.detail-panel,
.related-section {
  padding: 16px;
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

.full-width {
  grid-column: 1 / -1;
}

.form-actions,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.primary-btn,
.secondary-btn,
.danger-btn {
  border: 1px solid #384152;
  border-radius: 6px;
  padding: 9px 14px;
  cursor: pointer;
  transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease;
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

.camera-title span {
  display: block;
  margin-bottom: 6px;
  color: #9ca3af;
}

.detail-grid {
  display: grid;
  grid-template-columns: 110px minmax(0, 1fr);
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

@media (max-width: 760px) {
  .form-panel {
    grid-template-columns: 1fr;
  }
}
</style>
