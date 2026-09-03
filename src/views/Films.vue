<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from '../components/PageHeader.vue'
import type { Film, RollSummary } from '../types'
import { errorMessage as formatError } from '../utils/errors'

const filmTypes = ['Color Negative', 'B&W', 'Slide']

const films = ref<Film[]>([])
const rolls = ref<RollSummary[]>([])
const currentView = ref<'grid' | 'add' | 'detail'>('grid')

const draftBrand = ref('')
const draftType = ref('')
const draftShotStatus = ref('')
const activeBrand = ref('')
const activeType = ref('')
const activeShotStatus = ref('')

const formBrand = ref('')
const formName = ref('')
const formIso = ref<number | null>(null)
const formType = ref('Color Negative')
const formTargetStatus = ref('untested')
const formNote = ref('')

const selectedFilm = ref<Film | null>(null)
const editSnapshot = ref<Film | null>(null)
const relatedRolls = ref<RollSummary[]>([])
const isEditing = ref(false)
const isLoading = ref(false)
const isBusy = ref(false)
const visibleError = ref('')

const emit = defineEmits<{
  (e: 'jump-to-roll', rollId: number): void
}>()

async function fetchData() {
  isLoading.value = true
  visibleError.value = ''
  try {
    const [filmData, rollData] = await Promise.all([
      invoke<Film[]>('get_films'),
      invoke<RollSummary[]>('get_rolls')
    ])
    films.value = filmData
    rolls.value = rollData
    if (selectedFilm.value) {
      const updated = films.value.find(film => film.id === selectedFilm.value?.id)
      if (updated) {
        selectedFilm.value = { ...updated }
        relatedRolls.value = rolls.value.filter(roll => roll.filmId === updated.id)
      }
    }
  } catch (err) {
    console.error('Failed to fetch films:', err)
    visibleError.value = formatError(err, '无法读取胶片数据')
  } finally {
    isLoading.value = false
  }
}

const availableBrands = computed(() => {
  return Array.from(new Set(films.value.map(film => film.brand))).sort()
})

const availableTypes = computed(() => {
  return Array.from(new Set([...filmTypes, ...films.value.map(film => film.type)]))
})

const filteredFilms = computed(() => {
  return films.value.filter(film => {
    const matchBrand = activeBrand.value ? film.brand === activeBrand.value : true
    const matchType = activeType.value ? film.type === activeType.value : true
    const shot = isFilmShot(film.id)
    const matchShotStatus = activeShotStatus.value === 'shot'
      ? shot
      : activeShotStatus.value === 'unshot'
        ? !shot
        : true
    return matchBrand && matchType && matchShotStatus
  })
})

function applyFilters() {
  activeBrand.value = draftBrand.value
  activeType.value = draftType.value
  activeShotStatus.value = draftShotStatus.value
}

function resetFilters() {
  draftBrand.value = ''
  draftType.value = ''
  draftShotStatus.value = ''
  activeBrand.value = ''
  activeType.value = ''
  activeShotStatus.value = ''
}

function isFilmShot(filmId: number) {
  return rolls.value.some(roll => roll.filmId === filmId)
}

function filmTypeLabel(type: string) {
  return type === 'Color Negative' ? 'Color' : type
}

function filmImageSrc(film: Film) {
  return `/film-stocks/${film.id}.jpg`
}

function handleFilmImageError(event: Event) {
  const image = event.target as HTMLImageElement
  image.src = '/film-stocks/placeholder.svg'
}

function openAddForm() {
  resetFilmForm()
  currentView.value = 'add'
}

function resetFilmForm() {
  formBrand.value = ''
  formName.value = ''
  formIso.value = null
  formType.value = 'Color Negative'
  formTargetStatus.value = 'untested'
  formNote.value = ''
}

async function handleAddFilm() {
  if (!formBrand.value.trim() || !formName.value.trim() || !formIso.value || !formType.value) {
    visibleError.value = '请填写品牌、名称、ISO 和类型'
    return
  }

  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('add_film_stock', {
      brand: formBrand.value,
      name: formName.value,
      iso: Number(formIso.value),
      filmType: formType.value,
      targetStatus: formTargetStatus.value || 'untested',
      note: formNote.value || null
    })
    resetFilmForm()
    currentView.value = 'grid'
    await fetchData()
  } catch (err) {
    console.error('Failed to add film:', err)
    visibleError.value = formatError(err, '新增胶片型号失败')
  } finally {
    isBusy.value = false
  }
}

function viewFilmDetail(film: Film) {
  selectedFilm.value = { ...film }
  relatedRolls.value = rolls.value.filter(roll => roll.filmId === film.id)
  isEditing.value = false
  currentView.value = 'detail'
}

async function handleUpdateFilm() {
  if (!selectedFilm.value) return

  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('update_film_stock', {
      id: selectedFilm.value.id,
      brand: selectedFilm.value.brand,
      name: selectedFilm.value.name,
      iso: Number(selectedFilm.value.iso),
      filmType: selectedFilm.value.type,
      targetStatus: selectedFilm.value.targetStatus || 'untested',
      note: selectedFilm.value.note || null
    })
    isEditing.value = false
    editSnapshot.value = null
    await fetchData()
  } catch (err) {
    console.error('Failed to update film:', err)
    visibleError.value = formatError(err, '更新胶片型号失败')
  } finally {
    isBusy.value = false
  }
}

async function handleDeleteFilm() {
  if (!selectedFilm.value) return
  const relatedCount = relatedRolls.value.length
  const message = relatedCount > 0
    ? `确定删除 ${selectedFilm.value.brand} ${selectedFilm.value.name}，并级联删除关联的 ${relatedCount} 个拍摄卷及照片记录吗？正式图库文件仍会保留。`
    : `确定删除 ${selectedFilm.value.brand} ${selectedFilm.value.name} 吗？`
  if (!confirm(message)) return

  isBusy.value = true
  visibleError.value = ''
  try {
    await invoke('delete_film_stock', { id: selectedFilm.value.id })
    backToGrid()
    await fetchData()
  } catch (err) {
    console.error('Failed to delete film stock:', err)
    visibleError.value = formatError(err, '删除胶片型号失败')
  } finally {
    isBusy.value = false
  }
}

function startEditing() {
  if (!selectedFilm.value) return
  editSnapshot.value = { ...selectedFilm.value }
  isEditing.value = true
}

function cancelEditing() {
  if (editSnapshot.value) selectedFilm.value = { ...editSnapshot.value }
  editSnapshot.value = null
  isEditing.value = false
}

function backToGrid() {
  currentView.value = 'grid'
  selectedFilm.value = null
  relatedRolls.value = []
  isEditing.value = false
}

onMounted(() => {
  fetchData()
})
</script>

<template>
  <section class="page">
    <div v-if="visibleError" class="feedback-error" role="alert">{{ visibleError }}</div>
    <div v-else-if="isLoading" class="feedback-info">正在读取胶片数据…</div>
    <div v-if="currentView === 'grid'" class="stack">
      <PageHeader title="Films" subtitle="整理胶卷资料，并按拍摄状态快速筛选。" />

      <div class="filter-panel film-filter-panel">
        <div class="filter-fields">
          <select v-model="draftBrand">
            <option value="">全部品牌</option>
            <option v-for="brand in availableBrands" :key="brand" :value="brand">{{ brand }}</option>
          </select>
          <select v-model="draftType">
            <option value="">全部类型</option>
            <option v-for="type in availableTypes" :key="type" :value="type">{{ filmTypeLabel(type) }}</option>
          </select>
          <select v-model="draftShotStatus">
            <option value="">全部拍摄状态</option>
            <option value="shot">已拍摄</option>
            <option value="unshot">未拍摄</option>
          </select>
        </div>
        <div class="filter-actions">
          <button class="primary-btn" @click="applyFilters">确定</button>
          <button class="secondary-btn" @click="resetFilters">重置</button>
        </div>
      </div>

      <div class="cards-grid">
        <button
          type="button"
          v-for="film in filteredFilms"
          :key="film.id"
          :class="['film-card', isFilmShot(film.id) ? 'is-shot' : 'is-unshot']"
          @click="viewFilmDetail(film)"
        >
          <div class="film-image">
            <img :src="filmImageSrc(film)" :alt="film.name" @error="handleFilmImageError" />
          </div>
          <div class="card-body">
            <div class="card-kicker">{{ film.brand }}</div>
            <h2 class="film-name" :title="film.name">{{ film.name }}</h2>
            <div class="meta-row">
              <span>ISO {{ film.iso }}</span>
              <span>{{ isFilmShot(film.id) ? '已拍摄' : '未拍摄' }}</span>
              <span>{{ filmTypeLabel(film.type) }}</span>
            </div>
          </div>
        </button>

        <button class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加新卷</span>
        </button>
      </div>

      <div v-if="filteredFilms.length === 0" class="empty-state">没有符合条件的胶卷。</div>
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <PageHeader title="新增胶卷">
        <button class="secondary-btn" @click="backToGrid">返回</button>
      </PageHeader>

      <div class="form-panel">
        <label>
          <span>品牌 *</span>
          <input v-model="formBrand" placeholder="Kodak" />
        </label>
        <label>
          <span>名称 *</span>
          <input v-model="formName" placeholder="Gold 200" />
        </label>
        <label>
          <span>ISO *</span>
          <input v-model.number="formIso" type="number" min="1" step="1" placeholder="200" />
        </label>
        <label>
          <span>类型 *</span>
          <select v-model="formType">
            <option v-for="type in filmTypes" :key="type" :value="type">{{ filmTypeLabel(type) }}</option>
          </select>
        </label>
        <label>
          <span>状态</span>
          <select v-model="formTargetStatus">
            <option value="untested">未测试</option>
            <option value="unshot">未拍摄</option>
            <option value="shot">已拍摄</option>
          </select>
        </label>
        <label class="full-width">
          <span>备注</span>
          <textarea v-model="formNote"></textarea>
        </label>
        <div class="form-actions full-width">
          <button class="primary-btn" :disabled="isBusy" @click="handleAddFilm">保存</button>
          <button class="secondary-btn" @click="backToGrid">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedFilm" class="stack">
      <PageHeader title="胶卷详情">
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="startEditing">编辑</button>
          <button v-if="!isEditing" class="danger-btn" :disabled="isBusy" @click="handleDeleteFilm">删除</button>
          <button class="secondary-btn" @click="backToGrid">返回</button>
        </div>
      </PageHeader>

      <div class="detail-layout">
        <div class="detail-image">
          <img :src="filmImageSrc(selectedFilm)" :alt="selectedFilm.name" @error="handleFilmImageError" />
        </div>

        <div v-if="!isEditing" class="detail-panel">
          <div class="card-kicker">{{ selectedFilm.brand }}</div>
          <h2>{{ selectedFilm.name }}</h2>
          <div class="detail-grid">
            <span>ISO</span><strong>{{ selectedFilm.iso }}</strong>
            <span>类型</span><strong>{{ filmTypeLabel(selectedFilm.type) }}</strong>
            <span>状态</span><strong>{{ selectedFilm.targetStatus || 'untested' }}</strong>
            <span>备注</span><strong>{{ selectedFilm.note || '暂无备注' }}</strong>
          </div>
        </div>

        <div v-else class="form-panel detail-panel">
          <label>
            <span>品牌</span>
            <input v-model="selectedFilm.brand" />
          </label>
          <label>
            <span>名称</span>
            <input v-model="selectedFilm.name" />
          </label>
          <label>
            <span>ISO</span>
            <input v-model.number="selectedFilm.iso" type="number" min="1" step="1" />
          </label>
          <label>
            <span>类型</span>
            <select v-model="selectedFilm.type">
              <option v-for="type in filmTypes" :key="type" :value="type">{{ filmTypeLabel(type) }}</option>
            </select>
          </label>
          <label>
            <span>状态</span>
            <select v-model="selectedFilm.targetStatus">
              <option value="untested">未测试</option>
              <option value="unshot">未拍摄</option>
              <option value="shot">已拍摄</option>
            </select>
          </label>
          <label class="full-width">
            <span>备注</span>
            <textarea v-model="selectedFilm.note"></textarea>
          </label>
          <div class="form-actions full-width">
            <button class="primary-btn" :disabled="isBusy" @click="handleUpdateFilm">保存</button>
            <button class="secondary-btn" @click="cancelEditing">取消</button>
          </div>
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
          <span class="related-title">第 {{ roll.index }} 卷 · {{ roll.filmInfo || selectedFilm.name }}</span>
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 12px;
}

.filter-fields,
.filter-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.filter-fields {
  min-width: 0;
  flex: 1;
}

.filter-fields select {
  min-width: 0;
  max-width: 190px;
}

.filter-actions {
  flex: none;
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
  border-color: #56363d;
  background: #261a1e;
  color: #e7a6ae;
}

.danger-btn:hover {
  border-color: #76444d;
  background: #352127;
  color: #fecdd3;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 16px;
  min-width: 0;
}

.film-card,
.add-card {
  min-width: 0;
  height: 268px;
  min-height: 0;
  border: 1px solid #262c38;
  border-radius: 8px;
  background: #151922;
  color: inherit;
  cursor: pointer;
  overflow: hidden;
  transition: border-color 0.16s ease, background 0.16s ease, transform 0.16s ease;
}

.film-card {
  display: grid;
  grid-template-rows: 118px minmax(0, 1fr);
  padding: 0;
  text-align: left;
  white-space: normal;
}

.film-card.is-shot {
  border-color: #303947;
  background: #171c25;
}

.film-card.is-unshot {
  border-color: #222a35;
  background: #12161d;
}

.film-card.is-unshot .film-image img {
  filter: brightness(0.88) saturate(0.9);
}

.film-card:hover,
.add-card:hover {
  border-color: #4b5563;
  background: #1a202b;
  transform: translateY(-2px);
}

.film-image,
.detail-image {
  background: #0f131b;
  border-bottom: 1px solid #262c38;
}

.film-image {
  height: 118px;
}

.film-image img,
.detail-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.card-body {
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding: 15px;
}

.card-kicker {
  margin-bottom: 8px;
  color: #9ca3af;
  font-size: 12px;
}

.film-name {
  min-height: calc(2 * 1.35em);
  max-height: calc(2 * 1.35em);
  display: -webkit-box;
  overflow: hidden;
  overflow-wrap: anywhere;
  white-space: normal;
  word-break: break-word;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
}

.meta-row {
  display: flex;
  flex-wrap: nowrap;
  gap: 8px;
  margin-top: auto;
  padding-top: 12px;
}

.meta-row span {
  border-radius: 999px;
  background: #202737;
  color: #cbd5e1;
  padding: 4px 8px;
  font-size: 11px;
  white-space: nowrap;
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
  overflow: hidden;
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
  .detail-layout,
  .form-panel {
    grid-template-columns: 1fr;
  }

  .filter-panel,
  .filter-fields {
    align-items: stretch;
    flex-direction: column;
  }

  .filter-fields select {
    max-width: none;
  }

  .filter-actions {
    justify-content: flex-end;
  }
}
</style>
