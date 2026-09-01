<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Camera {
  id: number
  brand: string
  model: string
  status: string
  format?: string
  purchase_date?: string
  note?: string
}

interface RollItem {
  id: number
  cameraId?: number
  roll_index?: number
  index?: number
  shot_month?: string
  city?: string
  film_info: string
}

const cameras = ref<Camera[]>([])
const allRolls = ref<RollItem[]>([])
const currentView = ref<'grid' | 'add' | 'detail'>('grid')

const formBrand = ref('')
const formModel = ref('')
const formFormat = ref('135')
const formPurchaseDate = ref('')
const formNote = ref('')

const selectedCamera = ref<Camera | null>(null)
const relatedRolls = ref<RollItem[]>([])
const isEditing = ref(false)

const emit = defineEmits<{
  (e: 'jump-to-roll', rollId: number): void
}>()

async function fetchCameras() {
  try {
    const data: any = await invoke('get_all_data')
    cameras.value = data.cameras || []
    allRolls.value = data.rolls || []

    if (selectedCamera.value) {
      const updated = cameras.value.find(camera => camera.id === selectedCamera.value?.id)
      if (updated) {
        selectedCamera.value = { ...updated }
      }
    }
  } catch (err) {
    console.error('Failed to fetch cameras:', err)
  }
}

function rollCount(cameraId: number) {
  return allRolls.value.filter(roll => roll.cameraId === cameraId).length
}

const sortedCameras = computed(() => {
  return [...cameras.value].sort((a, b) => a.brand.localeCompare(b.brand) || a.model.localeCompare(b.model))
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
  if (!formBrand.value || !formModel.value) return

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
  }
}

async function viewDetail(camera: Camera) {
  try {
    const res: any = await invoke('get_camera_detail', { id: camera.id })
    selectedCamera.value = res.camera
    relatedRolls.value = res.rolls || []
    isEditing.value = false
    currentView.value = 'detail'
  } catch (err) {
    console.error('Failed to fetch camera detail:', err)
  }
}

async function handleUpdateCamera() {
  if (!selectedCamera.value) return

  try {
    await invoke('update_camera', {
      id: selectedCamera.value.id,
      brand: selectedCamera.value.brand,
      model: selectedCamera.value.model,
      status: selectedCamera.value.status,
      format: selectedCamera.value.format || '135',
      purchaseDate: selectedCamera.value.purchase_date || null,
      note: selectedCamera.value.note || null
    })
    isEditing.value = false
    await fetchCameras()
  } catch (err) {
    console.error('Failed to update camera:', err)
  }
}

async function handleDeleteCamera(id: number) {
  if (!confirm('确定要删除这台相机吗？相关拍摄卷也会被删除。')) return

  try {
    await invoke('delete_camera', { id })
    backToGrid()
    await fetchCameras()
  } catch (err) {
    console.error('Failed to delete camera:', err)
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
    <div v-if="currentView === 'grid'" class="stack">
      <div class="page-header">
        <h1>Cameras</h1>
      </div>

      <div class="cards-grid">
        <article
          v-for="camera in sortedCameras"
          :key="camera.id"
          class="camera-card"
          @click="viewDetail(camera)"
        >
          <div class="camera-main">
            <div class="camera-brand">{{ camera.brand }}</div>
            <h2>{{ camera.model }}</h2>
          </div>
          <div class="camera-footer">
            <span>{{ camera.format || '135' }}</span>
            <span>{{ rollCount(camera.id) }} 卷</span>
            <span>{{ camera.status === 'active' ? '在用' : '闲置' }}</span>
          </div>
        </article>

        <button class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加新设备</span>
        </button>
      </div>
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <div class="page-header">
        <h1>新增相机</h1>
        <button class="secondary-btn" @click="backToGrid">返回</button>
      </div>

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
          <button class="primary-btn" @click="handleAddCamera">保存</button>
          <button class="secondary-btn" @click="backToGrid">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedCamera" class="stack">
      <div class="page-header">
        <h1>相机详情</h1>
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="isEditing = true">编辑</button>
          <button v-if="!isEditing" class="danger-btn" @click="handleDeleteCamera(selectedCamera.id)">删除</button>
          <button class="secondary-btn" @click="backToGrid">返回</button>
        </div>
      </div>

      <div v-if="!isEditing" class="detail-panel">
        <div class="camera-title">
          <span>{{ selectedCamera.brand }}</span>
          <h2>{{ selectedCamera.model }}</h2>
        </div>
        <div class="detail-grid">
          <span>状态</span><strong>{{ selectedCamera.status === 'active' ? '在用' : '闲置' }}</strong>
          <span>画幅</span><strong>{{ selectedCamera.format || '135' }}</strong>
          <span>购入日期</span><strong>{{ selectedCamera.purchase_date || '未记录' }}</strong>
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
          <input v-model="selectedCamera.purchase_date" type="date" />
        </label>
        <label class="full-width">
          <span>备注</span>
          <textarea v-model="selectedCamera.note"></textarea>
        </label>
        <div class="form-actions full-width">
          <button class="primary-btn" @click="handleUpdateCamera">保存</button>
          <button class="secondary-btn" @click="isEditing = false">取消</button>
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
          <span class="related-title">{{ roll.film_info }}</span>
          <span class="related-meta">{{ roll.shot_month || '未记录日期' }} · {{ roll.city || '未记录地点' }}</span>
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
}

.camera-card:hover,
.add-card:hover {
  border-color: #4b5563;
  background: #1a202b;
  transform: translateY(-2px);
}

.camera-brand {
  margin-bottom: 8px;
  color: #d1d5db;
  font-size: 16px;
  font-weight: 600;
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
