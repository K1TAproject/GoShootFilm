<script setup lang="ts">
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { EquipmentItem } from '../types'
import { errorMessage } from '../utils/errors'

const props = defineProps<{
  items: EquipmentItem[]
  category: 'all' | 'lens' | 'other'
}>()

const emit = defineEmits<{
  (event: 'refresh'): void
  (event: 'error', message: string): void
  (event: 'add-camera'): void
}>()

const editing = ref<EquipmentItem | null>(null)
const adding = ref<'lens' | 'other' | null>(null)
const busy = ref(false)
const choosing = ref(false)
const brand = ref('')
const model = ref('')
const subtype = ref('')
const mount = ref('')
const status = ref<'active' | 'inactive'>('active')
const purchaseDate = ref('')
const note = ref('')

const visibleItems = computed(() => props.category === 'all'
  ? props.items
  : props.items.filter(item => item.category === props.category))

function resetForm() {
  brand.value = ''
  model.value = ''
  subtype.value = ''
  mount.value = ''
  status.value = 'active'
  purchaseDate.value = ''
  note.value = ''
}

function startAdd(category: 'lens' | 'other') {
  editing.value = null
  resetForm()
  adding.value = category
}

function startEdit(item: EquipmentItem) {
  adding.value = null
  editing.value = item
  brand.value = item.brand
  model.value = item.model
  subtype.value = item.subtype || ''
  mount.value = item.mount || ''
  status.value = item.status
  purchaseDate.value = item.purchaseDate || ''
  note.value = item.note || ''
}

function closeForm() {
  adding.value = null
  editing.value = null
  resetForm()
}

async function save() {
  const category = editing.value?.category || adding.value
  if (!category || !brand.value.trim() || !model.value.trim()) {
    emit('error', '请填写器材品牌和型号')
    return
  }
  if (category === 'lens' && !mount.value.trim()) {
    emit('error', '请填写镜头卡口')
    return
  }
  if (category === 'other' && !subtype.value.trim()) {
    emit('error', '请填写其他器材的具体类型')
    return
  }
  busy.value = true
  try {
    const payload = {
      category,
      subtype: category === 'other' ? subtype.value : null,
      brand: brand.value,
      model: model.value,
      mount: category === 'lens' ? mount.value : null,
      status: status.value,
      purchaseDate: purchaseDate.value || null,
      note: note.value || null,
    }
    if (editing.value) {
      await invoke('update_equipment_item', { id: editing.value.id, ...payload })
    } else {
      await invoke('add_equipment_item', payload)
    }
    closeForm()
    emit('refresh')
  } catch (error) {
    emit('error', errorMessage(error, editing.value ? '更新器材失败' : '新增器材失败'))
  } finally {
    busy.value = false
  }
}

async function remove(item: EquipmentItem) {
  if (!confirm(`确定删除 ${item.brand} ${item.model} 的器材档案吗？不会影响拍摄卷、相册或图片文件。`)) return
  busy.value = true
  try {
    await invoke('delete_equipment_item', { id: item.id })
    closeForm()
    emit('refresh')
  } catch (error) {
    emit('error', errorMessage(error, '删除器材失败'))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="equipment-items">
    <div class="equipment-actions">
      <button class="secondary-btn" type="button" @click="choosing = true">添加器材</button>
    </div>

    <div v-if="visibleItems.length" class="equipment-grid">
      <button v-for="item in visibleItems" :key="item.id" type="button" class="equipment-card" @click="startEdit(item)">
        <span class="kind">{{ item.category === 'lens' ? '镜头' : (item.subtype || '其他器材') }}</span>
        <div class="name" :title="`${item.brand} ${item.model}`">
          <small>{{ item.brand }}</small>
          <strong>{{ item.model }}</strong>
        </div>
        <footer>
          <span v-if="item.category === 'lens'">{{ item.mount }}</span>
          <span>{{ item.status === 'active' ? '在用' : '闲置' }}</span>
        </footer>
      </button>
    </div>
    <div v-else class="empty-state">当前分类还没有器材。</div>

    <div v-if="adding || editing" class="modal" @click.self="closeForm">
      <section class="form-panel" role="dialog" aria-modal="true">
        <header>
          <h2>{{ editing ? '编辑器材' : `新增${adding === 'lens' ? '镜头' : '其他器材'}` }}</h2>
          <button type="button" class="close" aria-label="关闭" @click="closeForm">×</button>
        </header>
        <label><span>品牌 *</span><input v-model="brand" /></label>
        <label><span>型号 *</span><input v-model="model" /></label>
        <label v-if="(editing?.category || adding) === 'lens'"><span>卡口 *</span><input v-model="mount" /></label>
        <label v-else><span>具体类型 *</span><input v-model="subtype" placeholder="闪光灯、扫描仪、三脚架" /></label>
        <label><span>状态</span><select v-model="status"><option value="active">在用</option><option value="inactive">闲置</option></select></label>
        <label><span>购入日期</span><input v-model="purchaseDate" type="date" /></label>
        <label class="wide"><span>备注</span><textarea v-model="note"></textarea></label>
        <div class="form-actions wide">
          <button class="primary-btn" type="button" :disabled="busy" @click="save">{{ busy ? '保存中…' : '保存' }}</button>
          <button v-if="editing" class="danger-btn" type="button" :disabled="busy" @click="remove(editing)">删除</button>
          <button class="secondary-btn" type="button" :disabled="busy" @click="closeForm">取消</button>
        </div>
      </section>
    </div>
    <div v-if="choosing" class="modal" @click.self="choosing = false">
      <section class="choice-panel" role="dialog" aria-modal="true">
        <header><h2>选择器材类型</h2><button type="button" class="close" @click="choosing = false">×</button></header>
        <button type="button" @click="choosing = false; emit('add-camera')"><strong>相机机身</strong><span>继续使用现有相机档案</span></button>
        <button type="button" @click="choosing = false; startAdd('lens')"><strong>镜头</strong><span>记录卡口与状态</span></button>
        <button type="button" @click="choosing = false; startAdd('other')"><strong>其他器材</strong><span>闪光灯、扫描仪、三脚架等</span></button>
      </section>
    </div>
  </div>
</template>

<style scoped>
.equipment-items { display: grid; gap: 14px; }
.equipment-actions { display: flex; justify-content: flex-end; gap: 8px; }
.equipment-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 16px; }
.equipment-card { min-height: 170px; display: flex; flex-direction: column; justify-content: space-between; border: 1px solid #262c38; border-radius: 8px; background: #151922; padding: 18px; color: inherit; text-align: left; cursor: pointer; }
.equipment-card:hover { border-color: #4b5563; background: #1a202b; transform: translateY(-2px); }
.kind { width: fit-content; border: 1px solid #394353; border-radius: 999px; padding: 4px 8px; color: #9da9b9; font-size: 11px; }
.name { display: grid; gap: 5px; place-items: center; text-align: center; }
.name small { color: #909cac; }
.name strong { display: -webkit-box; overflow: hidden; -webkit-box-orient: vertical; -webkit-line-clamp: 2; font-size: 22px; overflow-wrap: anywhere; }
footer { display: flex; gap: 8px; flex-wrap: wrap; }
footer span { border-radius: 999px; background: #202735; padding: 5px 9px; color: #aeb8c7; font-size: 12px; }
.modal { position: fixed; inset: 0; z-index: 10020; display: grid; place-items: center; background: rgba(2, 5, 9, .78); padding: 20px; }
.form-panel { width: min(680px, 100%); display: grid; grid-template-columns: 1fr 1fr; gap: 14px; border: 1px solid #303948; border-radius: 12px; background: #141920; padding: 20px; }
.choice-panel { width: min(440px, 100%); display: grid; gap: 10px; border: 1px solid #303948; border-radius: 12px; background: #141920; padding: 20px; }
.choice-panel > button:not(.close) { display: grid; gap: 4px; border: 1px solid #303948; border-radius: 8px; background: #11161d; padding: 14px; color: #e5e7eb; text-align: left; cursor: pointer; }
.choice-panel > button:not(.close):hover { border-color: #59677a; }
.choice-panel span { color: #8f9bad; font-size: 12px; }
header { grid-column: 1 / -1; display: flex; align-items: center; justify-content: space-between; }
header h2 { margin: 0; }
.close { border: 0; background: transparent; color: #dfe5ec; font-size: 24px; cursor: pointer; }
label { display: grid; gap: 7px; }
label span { color: #9aa6b5; font-size: 12px; }
.wide { grid-column: 1 / -1; }
.empty-state { border: 1px dashed #303846; border-radius: 8px; padding: 28px; color: #8f9bad; text-align: center; }
@media (max-width: 620px) { .form-panel { grid-template-columns: 1fr; } .wide, header { grid-column: auto; } }
</style>
