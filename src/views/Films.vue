<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface FilmStock {
  id: number
  brand: string
  name: string
  iso: number
  type: string
  target_status?: string
  note?: string
}

interface RollItem {
  id: number
  cameraId: number
  filmId: number
  index: number
  shot_month?: string
  city?: string
  camera_info?: string
  film_info?: string
}

const filmTypes = ['Color Negative', 'B&W', 'Slide']

const films = ref<FilmStock[]>([])
const rolls = ref<RollItem[]>([])
const currentView = ref<'grid' | 'add' | 'detail'>('grid')

const draftBrand = ref('')
const draftType = ref('')
const activeBrand = ref('')
const activeType = ref('')

const formBrand = ref('')
const formName = ref('')
const formIso = ref<number | null>(null)
const formType = ref('Color Negative')
const formTargetStatus = ref('untested')
const formNote = ref('')

const selectedFilm = ref<FilmStock | null>(null)
const relatedRolls = ref<RollItem[]>([])
const isEditing = ref(false)

const emit = defineEmits<{
  (e: 'jump-to-roll', rollId: number): void
}>()

async function fetchData() {
  try {
    const data: any = await invoke('get_all_data')
    films.value = data.films || []
    rolls.value = data.rolls || []
    if (selectedFilm.value) {
      const updated = films.value.find(film => film.id === selectedFilm.value?.id)
      if (updated) {
        selectedFilm.value = { ...updated }
        relatedRolls.value = rolls.value.filter(roll => roll.filmId === updated.id)
      }
    }
  } catch (err) {
    console.error('Failed to fetch films:', err)
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
    return matchBrand && matchType
  })
})

function applyFilters() {
  activeBrand.value = draftBrand.value
  activeType.value = draftType.value
}

function resetFilters() {
  draftBrand.value = ''
  draftType.value = ''
  activeBrand.value = ''
  activeType.value = ''
}

function isFilmShot(filmId: number) {
  return rolls.value.some(roll => roll.filmId === filmId)
}

function filmImageSrc(film: FilmStock) {
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
  if (!formBrand.value || !formName.value || !formIso.value || !formType.value) return

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
  }
}

function viewFilmDetail(film: FilmStock) {
  selectedFilm.value = { ...film }
  relatedRolls.value = rolls.value.filter(roll => roll.filmId === film.id)
  isEditing.value = false
  currentView.value = 'detail'
}

async function handleUpdateFilm() {
  if (!selectedFilm.value) return

  try {
    await invoke('update_film_stock', {
      id: selectedFilm.value.id,
      brand: selectedFilm.value.brand,
      name: selectedFilm.value.name,
      iso: Number(selectedFilm.value.iso),
      filmType: selectedFilm.value.type,
      targetStatus: selectedFilm.value.target_status || 'untested',
      note: selectedFilm.value.note || null
    })
    isEditing.value = false
    await fetchData()
  } catch (err) {
    console.error('Failed to update film:', err)
  }
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
    <div v-if="currentView === 'grid'" class="stack">
      <div class="page-header">
        <h1>Films</h1>
      </div>

      <div class="filter-panel">
        <select v-model="draftBrand">
          <option value="">全部品牌</option>
          <option v-for="brand in availableBrands" :key="brand" :value="brand">{{ brand }}</option>
        </select>
        <select v-model="draftType">
          <option value="">全部类型</option>
          <option v-for="type in availableTypes" :key="type" :value="type">{{ type }}</option>
        </select>
        <button class="primary-btn" @click="applyFilters">确定</button>
        <button class="secondary-btn fixed-action" @click="resetFilters">重置筛选</button>
      </div>

      <div class="cards-grid">
        <article
          v-for="film in filteredFilms"
          :key="film.id"
          class="film-card"
          @click="viewFilmDetail(film)"
        >
          <div class="film-image">
            <img :src="filmImageSrc(film)" :alt="film.name" @error="handleFilmImageError" />
          </div>
          <div class="card-body">
            <div class="card-kicker">{{ film.brand }}</div>
            <h2>{{ film.name }}</h2>
            <div class="meta-row">
              <span>ISO {{ film.iso }}</span>
              <span>{{ film.type }}</span>
              <span>{{ isFilmShot(film.id) ? '已拍摄' : '未拍摄' }}</span>
            </div>
          </div>
        </article>

        <button class="add-card" @click="openAddForm">
          <span class="plus-mark">+</span>
          <span>添加新卷</span>
        </button>
      </div>

      <div v-if="filteredFilms.length === 0" class="empty-state">没有符合条件的胶卷。</div>
    </div>

    <div v-else-if="currentView === 'add'" class="stack">
      <div class="page-header">
        <h1>新增胶卷</h1>
        <button class="secondary-btn" @click="backToGrid">返回</button>
      </div>

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
            <option v-for="type in filmTypes" :key="type" :value="type">{{ type }}</option>
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
          <button class="primary-btn" @click="handleAddFilm">保存</button>
          <button class="secondary-btn" @click="backToGrid">取消</button>
        </div>
      </div>
    </div>

    <div v-else-if="currentView === 'detail' && selectedFilm" class="stack">
      <div class="page-header">
        <h1>胶卷详情</h1>
        <div class="actions">
          <button v-if="!isEditing" class="secondary-btn" @click="isEditing = true">编辑</button>
          <button class="secondary-btn" @click="backToGrid">返回</button>
        </div>
      </div>

      <div class="detail-layout">
        <div class="detail-image">
          <img :src="filmImageSrc(selectedFilm)" :alt="selectedFilm.name" @error="handleFilmImageError" />
        </div>

        <div v-if="!isEditing" class="detail-panel">
          <div class="card-kicker">{{ selectedFilm.brand }}</div>
          <h2>{{ selectedFilm.name }}</h2>
          <div class="detail-grid">
            <span>ISO</span><strong>{{ selectedFilm.iso }}</strong>
            <span>类型</span><strong>{{ selectedFilm.type }}</strong>
            <span>状态</span><strong>{{ selectedFilm.target_status || 'untested' }}</strong>
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
              <option v-for="type in filmTypes" :key="type" :value="type">{{ type }}</option>
            </select>
          </label>
          <label>
            <span>状态</span>
            <select v-model="selectedFilm.target_status">
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
            <button class="primary-btn" @click="handleUpdateFilm">保存</button>
            <button class="secondary-btn" @click="isEditing = false">取消</button>
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
          <span class="related-title">{{ roll.film_info || selectedFilm.name }}</span>
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
  gap: 10px;
  flex-wrap: wrap;
}

.filter-panel select {
  min-width: 180px;
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

.fixed-action {
  min-width: 96px;
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
  min-height: 230px;
  border: 1px solid #262c38;
  border-radius: 8px;
  background: #151922;
  color: inherit;
  cursor: pointer;
  overflow: hidden;
  transition: border-color 0.16s ease, background 0.16s ease, transform 0.16s ease;
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
}
</style>
