<script setup lang="ts">
import { computed, ref } from 'vue'
import MobileHeader from '../components/MobileHeader.vue'
import SyncNotice from '../components/SyncNotice.vue'
import EmptyState from '../components/EmptyState.vue'
import type { CameraEntity, EquipmentEntity } from '../../../src/shared/entities'
import {
  albumsForCamera,
  cameras,
  equipment,
  rollsForCamera,
  store,
  syncNow,
  type TypedEntity,
} from '../lib/store'

type Tab = 'all' | 'camera' | 'lens' | 'other'

const activeTab = ref<Tab>('all')
const selectedCamera = ref<TypedEntity<CameraEntity>>()
const selectedEquipment = ref<TypedEntity<EquipmentEntity>>()

const cameraList = computed(() => cameras())
const equipmentList = computed(() => equipment())
const lensList = computed(() => equipmentList.value.filter((item) => item.data.category === 'lens'))
const otherList = computed(() => equipmentList.value.filter((item) => item.data.category === 'other'))
const relatedRolls = computed(() =>
  selectedCamera.value ? rollsForCamera(selectedCamera.value.syncId) : [],
)
const relatedAlbums = computed(() =>
  selectedCamera.value ? albumsForCamera(selectedCamera.value.syncId) : [],
)

const tabs = computed<{ value: Tab; label: string; count: number }[]>(() => [
  { value: 'all', label: '全部', count: cameraList.value.length + equipmentList.value.length },
  { value: 'camera', label: '机身', count: cameraList.value.length },
  { value: 'lens', label: '镜头', count: lensList.value.length },
  { value: 'other', label: '其他', count: otherList.value.length },
])

function close() {
  selectedCamera.value = undefined
  selectedEquipment.value = undefined
}

function cameraStatus(status: string) {
  return status === 'active' ? '在用' : '闲置'
}

function equipmentStatus(status: string) {
  return status === 'active' ? '在用' : '闲置'
}
</script>

<template>
  <main class="page">
    <MobileHeader
      title="器材"
      subtitle="机身、镜头与其他器材"
      settings
      @sync="syncNow"
    />
    <SyncNotice />

    <div class="chips" style="margin-bottom: 12px">
      <button
        v-for="tab in tabs"
        :key="tab.value"
        class="chip"
        :class="{ active: activeTab === tab.value }"
        type="button"
        @click="activeTab = tab.value"
      >
        {{ tab.label }} {{ tab.count }}
      </button>
    </div>

    <div class="grid-3" style="margin-bottom: 14px">
      <div class="stat-card">
        <span>器材总数</span>
        <strong>{{ store.stats.equipmentCount }}</strong>
      </div>
      <div class="stat-card">
        <span>机身</span>
        <strong>{{ cameraList.length }}</strong>
      </div>
      <div class="stat-card">
        <span>镜头</span>
        <strong>{{ lensList.length }}</strong>
      </div>
    </div>

    <template v-if="activeTab === 'all' || activeTab === 'camera'">
      <section class="section-title"><h2>机身</h2></section>
      <div v-if="cameraList.length" class="stack" style="margin-top: 8px">
        <button
          v-for="camera in cameraList"
          :key="camera.syncId"
          class="list-card"
          type="button"
          @click="selectedCamera = camera"
        >
          <div class="cover-thumb" style="display: grid; place-items: center; color: var(--accent)">◉</div>
          <div class="list-main">
            <strong>{{ camera.data.brand }} {{ camera.data.model }}</strong>
            <p>
              {{ camera.data.cameraType === 'digital' ? (camera.data.sensorFormat || '数码机身') : `${camera.data.format || '135'} 画幅` }}
              · {{ cameraStatus(camera.data.status) }}
            </p>
            <p v-if="camera.data.note">{{ camera.data.note }}</p>
          </div>
          <span class="pill">{{ camera.data.cameraType === 'digital' ? '数码' : '胶片' }}</span>
        </button>
      </div>
      <EmptyState v-else text="还没有机身" hint="在电脑端 Equipment 中添加机身后同步" />
    </template>

    <template v-if="activeTab === 'all' || activeTab === 'lens' || activeTab === 'other'">
      <section class="section-title" style="margin-top: 18px"><h2>镜头与其他器材</h2></section>
      <div v-if="equipmentList.length" class="stack" style="margin-top: 8px">
        <button
          v-for="item in equipmentList"
          :key="item.syncId"
          class="list-card"
          type="button"
          @click="selectedEquipment = item"
        >
          <div class="cover-thumb" style="display: grid; place-items: center; color: var(--muted)">▣</div>
          <div class="list-main">
            <strong>{{ item.data.brand }} {{ item.data.model }}</strong>
            <p>
              {{ item.data.category === 'lens' ? `镜头 · ${item.data.mount || '未知卡口'}` : `其他 · ${item.data.subtype || '未分类'}` }}
              · {{ equipmentStatus(item.data.status) }}
            </p>
            <p v-if="item.data.note">{{ item.data.note }}</p>
          </div>
          <span class="pill">{{ item.data.category === 'lens' ? '镜头' : '其他' }}</span>
        </button>
      </div>
      <EmptyState v-else text="还没有镜头或其他器材" />
    </template>

    <div v-if="selectedCamera" class="modal-overlay" @click.self="close">
      <section class="modal-sheet">
        <div class="spread">
          <div>
            <span class="eyebrow">{{ selectedCamera.data.cameraType === 'digital' ? 'Digital body' : 'Film body' }}</span>
            <h2>{{ selectedCamera.data.brand }} {{ selectedCamera.data.model }}</h2>
          </div>
          <button class="header-button" type="button" aria-label="关闭" @click="close">×</button>
        </div>
        <dl class="detail-list">
          <div><dt>类型</dt><dd>{{ selectedCamera.data.cameraType === 'digital' ? '数码相机' : '胶片相机' }}</dd></div>
          <div v-if="selectedCamera.data.cameraType === 'film'"><dt>画幅</dt><dd>{{ selectedCamera.data.format || '135' }}</dd></div>
          <div v-else><dt>CMOS 规格</dt><dd>{{ selectedCamera.data.sensorFormat || '未记录' }}</dd></div>
          <div><dt>状态</dt><dd>{{ cameraStatus(selectedCamera.data.status) }}</dd></div>
          <div><dt>购入日期</dt><dd>{{ selectedCamera.data.purchaseDate || '未记录' }}</dd></div>
          <div v-if="selectedCamera.data.note"><dt>备注</dt><dd>{{ selectedCamera.data.note }}</dd></div>
        </dl>
        <template v-if="relatedRolls.length">
          <h2 class="section-title" style="margin-top: 18px; font-size: 15px">关联拍摄卷</h2>
          <div class="stack" style="margin-top: 8px">
            <RouterLink
              v-for="roll in relatedRolls"
              :key="roll.syncId"
              class="list-card"
              :to="`/rolls/${roll.syncId}`"
              @click="close"
            >
              <div class="list-main">
                <strong>Roll #{{ String(roll.data.index).padStart(3, '0') }}</strong>
                <p>{{ roll.data.filmInfo }}</p>
              </div>
              <span class="pill">{{ roll.data.photoCount }} 张</span>
            </RouterLink>
          </div>
        </template>
        <template v-if="relatedAlbums.length">
          <h2 class="section-title" style="margin-top: 18px; font-size: 15px">关联数码相册</h2>
          <div class="stack" style="margin-top: 8px">
            <RouterLink
              v-for="album in relatedAlbums"
              :key="album.syncId"
              class="list-card"
              :to="`/albums/${album.syncId}`"
              @click="close"
            >
              <div class="list-main">
                <strong>{{ album.data.title }}</strong>
                <p>{{ album.data.shotDate || '未记录日期' }}</p>
              </div>
              <span class="pill">{{ album.data.photoCount }} 张</span>
            </RouterLink>
          </div>
        </template>
      </section>
    </div>

    <div v-if="selectedEquipment" class="modal-overlay" @click.self="close">
      <section class="modal-sheet">
        <div class="spread">
          <div>
            <span class="eyebrow">{{ selectedEquipment.data.category === 'lens' ? 'Lens' : 'Equipment' }}</span>
            <h2>{{ selectedEquipment.data.brand }} {{ selectedEquipment.data.model }}</h2>
          </div>
          <button class="header-button" type="button" aria-label="关闭" @click="close">×</button>
        </div>
        <dl class="detail-list">
          <div><dt>分类</dt><dd>{{ selectedEquipment.data.category === 'lens' ? '镜头' : '其他器材' }}</dd></div>
          <div v-if="selectedEquipment.data.category === 'lens'"><dt>卡口</dt><dd>{{ selectedEquipment.data.mount || '未记录' }}</dd></div>
          <div v-else><dt>具体类型</dt><dd>{{ selectedEquipment.data.subtype || '未记录' }}</dd></div>
          <div><dt>状态</dt><dd>{{ equipmentStatus(selectedEquipment.data.status) }}</dd></div>
          <div><dt>购入日期</dt><dd>{{ selectedEquipment.data.purchaseDate || '未记录' }}</dd></div>
          <div v-if="selectedEquipment.data.note"><dt>备注</dt><dd>{{ selectedEquipment.data.note }}</dd></div>
        </dl>
      </section>
    </div>
  </main>
</template>
