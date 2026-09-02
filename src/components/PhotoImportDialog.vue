<script setup lang="ts">
import type { ImportAnalysisItem, ImportConflictAction, PhotoVersion } from '../types'

defineProps<{
  open: boolean
  version: PhotoVersion
  items: ImportAnalysisItem[]
  busy: boolean
  error?: string
}>()

const emit = defineEmits<{
  (event: 'close'): void
  (event: 'update-frame', index: number, value?: number): void
  (event: 'update-action', index: number, value: ImportConflictAction): void
  (event: 'confirm'): void
}>()

function frameValue(event: Event) {
  const value = Number((event.target as HTMLInputElement).value)
  return Number.isInteger(value) ? value : undefined
}
</script>

<template>
  <div v-if="open" class="dialog-backdrop" role="presentation" @click.self="emit('close')">
    <section class="import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-title">
      <header>
        <div>
          <span>{{ version === 'lab' ? '原始扫描' : '调色图' }}</span>
          <h2 id="import-title">确认 Frame 与配对</h2>
        </div>
        <button class="icon-button" type="button" aria-label="关闭导入窗口" @click="emit('close')">×</button>
      </header>

      <p class="dialog-tip">Frame 优先取文件名末尾两位数字。请先处理所有红色问题和同版本冲突，整批确认后才会写入。</p>
      <div v-if="error" class="feedback-error" role="alert">{{ error }}</div>

      <div class="import-list">
        <article v-for="(item, index) in items" :key="item.sourcePath" :class="['import-row', { invalid: item.issue }]">
          <div class="file-info">
            <strong>{{ item.fileName }}</strong>
            <small v-if="item.pairedVersion">将与已有{{ version === 'lab' ? '调色图' : '原始扫描' }}配对</small>
            <small v-if="item.issue" class="issue">{{ item.issue }}</small>
          </div>
          <label>
            <span>Frame</span>
            <input
              type="number"
              min="1"
              max="99"
              :value="item.frameNumber"
              placeholder="01–99"
              @input="emit('update-frame', index, frameValue($event))"
            />
          </label>
          <label v-if="item.existingVersion">
            <span>已有同版本</span>
            <select :value="item.conflictAction" @change="emit('update-action', index, ($event.target as HTMLSelectElement).value as ImportConflictAction)">
              <option value="ask">请选择</option>
              <option value="skip">跳过</option>
              <option value="replace">替换引用</option>
              <option value="cancel">取消整批</option>
            </select>
          </label>
          <div v-else class="status-chip">{{ item.pairedVersion ? '补充版本' : '新建 Frame' }}</div>
        </article>
      </div>

      <footer>
        <button type="button" class="secondary-btn" :disabled="busy" @click="emit('close')">取消</button>
        <button type="button" class="primary-btn" :disabled="busy" @click="emit('confirm')">
          {{ busy ? '正在检查并导入…' : '检查并导入整批' }}
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.dialog-backdrop {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  z-index: 10020;
  background: rgba(4, 7, 11, 0.78);
  padding: 24px;
}

.import-dialog {
  width: min(820px, 96vw);
  max-height: 88vh;
  display: flex;
  flex-direction: column;
  border: 1px solid #364050;
  border-radius: 12px;
  background: #151a22;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
}

header,
footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px;
}

header { border-bottom: 1px solid #29313d; }
footer { justify-content: flex-end; border-top: 1px solid #29313d; }
header span { color: #8f9bad; font-size: 12px; }
h2 { margin: 4px 0 0; color: #f8fafc; font-size: 19px; }

.dialog-tip {
  margin: 0;
  padding: 12px 18px 0;
  color: #9ca7b7;
  font-size: 13px;
}

.feedback-error { margin: 12px 18px 0; width: auto; }
.import-list { overflow: auto; padding: 14px 18px 18px; }

.import-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 100px 150px;
  align-items: center;
  gap: 12px;
  border: 1px solid #29313d;
  border-radius: 8px;
  background: #10151c;
  padding: 12px;
}

.import-row + .import-row { margin-top: 8px; }
.import-row.invalid { border-color: #7f343b; }
.file-info { min-width: 0; }
.file-info strong { display: block; overflow: hidden; color: #e5e7eb; font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
.file-info small { display: block; margin-top: 4px; color: #8490a0; }
.file-info .issue { color: #fca5a5; }

label { display: flex; flex-direction: column; gap: 5px; color: #8f9bad; font-size: 11px; }
input,
select { width: 100%; border: 1px solid #354052; border-radius: 6px; background: #0d1118; color: #e5e7eb; padding: 8px; }
.status-chip { justify-self: end; border-radius: 999px; background: #222b37; color: #aeb9c7; padding: 6px 9px; font-size: 11px; }

.icon-button,
.primary-btn,
.secondary-btn { border: 1px solid #384152; border-radius: 6px; padding: 8px 12px; cursor: pointer; }
.icon-button { border: 0; background: transparent; color: #aeb8c5; font-size: 22px; }
.primary-btn { background: #e5e7eb; color: #111827; }
.secondary-btn { background: #1d2430; color: #d1d5db; }

@media (max-width: 680px) {
  .import-row { grid-template-columns: 1fr; }
  .status-chip { justify-self: start; }
}
</style>
