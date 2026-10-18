import { computed, reactive } from 'vue'
import type {
  MobileAssetDescriptor,
  PairingQrPayload,
  SyncEntityType,
  SyncStats,
} from '../../../src/shared/sync'
import type {
  CameraEntity,
  DigitalAlbumEntity,
  DigitalPhotoEntity,
  EquipmentEntity,
  FilmEntity,
  FilmPhotoEntity,
  RollEntity,
} from '../../../src/shared/entities'
import {
  allAssets,
  applyChanges,
  assetsForEntity,
  cacheUsage,
  clearDisplayCache,
  countEntities,
  enforceCacheLimit,
  entityKey,
  getAsset,
  getAllEntities,
  getMeta,
  putAsset,
  resetSyncData,
  touchAsset,
  type AssetRecord,
  type EntityRecord,
} from './db'
import {
  ApiRequestError,
  fetchChanges,
  fetchResource,
  pairRequest,
  pairStatus,
} from './api'
import {
  clearCredential,
  getCredential,
  setCredential,
} from './secure'
import { sha256Hex } from './hash'

const SERVER_URL_KEY = 'goshootfilm.mobile.serverUrl'
const DEVICE_ID_KEY = 'goshootfilm.mobile.deviceId'
const FINGERPRINT_KEY = 'goshootfilm.mobile.fingerprint'
const SETTINGS_KEY = 'goshootfilm.mobile.settings'
const CACHE_5GB = 5 * 1024 * 1024 * 1024

export interface MobileSettings {
  wifiOnly: boolean
  cacheLimitBytes: number
  deviceName: string
}

export type TypedEntity<T> = Omit<EntityRecord, 'data'> & { data: T }

function emptyStats(): SyncStats {
  return {
    cameraCount: 0,
    filmCount: 0,
    shotFilmCount: 0,
    rollCount: 0,
    photoCount: 0,
    favoritePhotoCount: 0,
    equipmentCount: 0,
    digitalAlbumCount: 0,
    digitalPhotoCount: 0,
    archivedPhotoCount: 0,
  }
}

function defaultDeviceName() {
  if (/Android/i.test(navigator.userAgent)) return '我的 Android 手机'
  if (/iPhone|iPad/i.test(navigator.userAgent)) return '我的 iPhone'
  return '我的手机'
}

function readSettings(): MobileSettings {
  const fallback: MobileSettings = {
    wifiOnly: false,
    cacheLimitBytes: CACHE_5GB,
    deviceName: defaultDeviceName(),
  }
  try {
    const raw = localStorage.getItem(SETTINGS_KEY)
    if (!raw) return fallback
    const parsed = JSON.parse(raw) as Partial<MobileSettings>
    return {
      wifiOnly: parsed.wifiOnly ?? fallback.wifiOnly,
      cacheLimitBytes:
        typeof parsed.cacheLimitBytes === 'number'
          ? parsed.cacheLimitBytes
          : fallback.cacheLimitBytes,
      deviceName:
        typeof parsed.deviceName === 'string' && parsed.deviceName.trim()
          ? parsed.deviceName.trim().slice(0, 80)
          : fallback.deviceName,
    }
  } catch {
    return fallback
  }
}

export const settings = reactive<MobileSettings>(readSettings())

export function updateSettings(patch: Partial<MobileSettings>) {
  Object.assign(settings, patch)
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
}

export const store = reactive({
  ready: false,
  paired: false,
  serverUrl: '',
  deviceId: '',
  fingerprint: '',
  online: navigator.onLine,
  syncing: false,
  lastSyncAt: null as string | null,
  syncError: '',
  syncNotice: '',
  stats: emptyStats(),
  entities: [] as EntityRecord[],
  dataRevision: 0,
  cacheBytes: 0,
  thumbnailBytes: 0,
  displayBytes: 0,
})

export const hasCachedData = computed(() => store.entities.length > 0)
export const activeAssetIds = new Set<string>()

let initialization: Promise<void> | null = null
let syncRunning = false

function sleep(milliseconds: number) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds))
}

function isWifi() {
  const connection = (
    navigator as Navigator & { connection?: { type?: string } }
  ).connection
  if (!connection?.type) return true
  return connection.type === 'wifi' || connection.type === 'ethernet'
}

export function isWifiOnlyBlocked() {
  return settings.wifiOnly && !isWifi()
}

async function refreshCacheUsage() {
  const usage = await cacheUsage()
  store.cacheBytes = usage.totalBytes
  store.thumbnailBytes = usage.thumbnailBytes
  store.displayBytes = usage.displayBytes
}

export async function loadCache() {
  store.entities = await getAllEntities()
  store.stats = await getMeta<SyncStats>('stats', emptyStats())
  store.lastSyncAt = await getMeta<string | null>('lastSyncAt', null)
  store.dataRevision += 1
  await refreshCacheUsage()
}

export function init(): Promise<void> {
  if (!initialization) {
    initialization = (async () => {
      store.serverUrl = localStorage.getItem(SERVER_URL_KEY) ?? ''
      store.deviceId = localStorage.getItem(DEVICE_ID_KEY) ?? ''
      store.fingerprint = localStorage.getItem(FINGERPRINT_KEY) ?? ''
      const credential = await getCredential()
      store.paired = Boolean(credential && store.serverUrl)
      await loadCache()
      store.online = navigator.onLine
      window.addEventListener('online', () => {
        store.online = true
        if (store.paired && !store.syncing) void syncNow()
      })
      window.addEventListener('offline', () => {
        store.online = false
      })
      store.ready = true
      if (store.paired) void syncNow()
    })()
  }
  return initialization
}

function collectThumbnails(changes: { assets?: MobileAssetDescriptor[] }[]) {
  const byEntity = new Map<string, MobileAssetDescriptor[]>()
  for (const change of changes) {
    for (const asset of change.assets ?? []) {
      if (asset.assetType !== 'thumbnail') continue
      const key = `${asset.entityType}:${asset.entitySyncId}`
      const list = byEntity.get(key) ?? []
      list.push(asset)
      byEntity.set(key, list)
    }
  }
  // Download the edit thumbnail when present; the original/lab thumbnail is
  // fetched on demand when the user switches versions.
  return [...byEntity.values()].map(
    (list) =>
      list.find((asset) => asset.version === 'edit') ??
      list[0] as MobileAssetDescriptor,
  )
}

async function downloadThumbnails(
  descriptors: MobileAssetDescriptor[],
  credential: string,
) {
  let index = 0
  let ok = 0
  let failed = 0

  async function worker() {
    while (index < descriptors.length) {
      const descriptor = descriptors[index]
      index += 1
      if (!descriptor) continue
      try {
        const existing = await getAsset(descriptor.id)
        if (
          existing?.blob &&
          existing.sha256 &&
          descriptor.sha256 &&
          existing.sha256 === descriptor.sha256
        ) {
          continue
        }
        const result = await fetchResource(store.serverUrl, credential, descriptor.id)
        const hash = result.sha256 || (await sha256Hex(await result.blob.arrayBuffer()))
        if (descriptor.sha256 && descriptor.sha256 !== hash) {
          throw new Error('thumbnail hash mismatch')
        }
        await putAsset({
          ...descriptor,
          entityKey: entityKey(descriptor.entityType, descriptor.entitySyncId),
          blob: result.blob,
          sha256: hash,
          byteSize: result.blob.size,
          available: true,
          downloadedAt: Date.now(),
          lastAccess: Date.now(),
        })
        ok += 1
      } catch {
        failed += 1
      }
    }
  }

  await Promise.all([worker(), worker(), worker()])
  return { ok, failed }
}

function friendlyError(error: unknown) {
  if (error instanceof ApiRequestError) return error.message
  if (error instanceof Error) return error.message
  return '同步失败，请稍后重试'
}

export async function syncNow(): Promise<void> {
  if (syncRunning || store.syncing) return
  if (!store.paired) {
    store.syncError = '尚未与电脑配对'
    return
  }
  if (isWifiOnlyBlocked()) {
    store.syncError = '已开启仅 Wi-Fi 同步，当前网络不是 Wi-Fi'
    store.syncNotice = ''
    return
  }
  if (!navigator.onLine) {
    store.syncError = '当前离线，已显示本地缓存内容'
    store.syncNotice = ''
    return
  }
  const credential = await getCredential()
  if (!credential) {
    await disconnectLocal()
    store.syncError = '配对凭证已丢失，请重新配对'
    return
  }

  syncRunning = true
  store.syncing = true
  store.syncError = ''
  store.syncNotice = '正在同步…'
  let cursor = Number(await getMeta<number>('cursor', 0)) || 0
  let downloaded = 0
  let failed = 0
  let resetCount = 0

  try {
    for (let batch = 0; batch < 80; batch += 1) {
      const page = await fetchChanges(store.serverUrl, credential, cursor)
      if (page.resetRequired) {
        await resetSyncData()
        cursor = 0
        resetCount += 1
        if (resetCount > 2) throw new Error('电脑端数据已重置，请重新同步')
        continue
      }
      await applyChanges(page.changes, page.cursor, page.stats)
      cursor = page.cursor
      const thumbnails = collectThumbnails(page.changes)
      if (thumbnails.length > 0) {
        const result = await downloadThumbnails(thumbnails, credential)
        downloaded += result.ok
        failed += result.failed
      }
      if (!page.hasMore) break
    }
    store.syncNotice = failed > 0
      ? `同步完成，${downloaded} 张缩略图已更新，${failed} 张稍后重试`
      : '同步完成'
  } catch (error) {
    if (error instanceof ApiRequestError && error.status === 401) {
      await disconnectLocal()
      store.syncError = '设备已被撤销或凭证失效，请重新配对'
    } else {
      store.syncError = friendlyError(error)
    }
    store.syncNotice = ''
  } finally {
    syncRunning = false
    store.syncing = false
    try {
      await loadCache()
    } catch {
      // Keep the last in-memory snapshot if IndexedDB cannot be read.
    }
    await refreshCacheUsage()
  }
}

async function disconnectLocal() {
  await clearCredential()
  localStorage.removeItem(SERVER_URL_KEY)
  localStorage.removeItem(DEVICE_ID_KEY)
  localStorage.removeItem(FINGERPRINT_KEY)
  store.paired = false
  store.serverUrl = ''
  store.deviceId = ''
  store.fingerprint = ''
}

export async function unpair() {
  await disconnectLocal()
  store.syncError = ''
  store.syncNotice = '已解除配对，本地缓存仍可离线浏览'
}

export async function pairWithPayload(payload: PairingQrPayload): Promise<void> {
  if (payload.v !== 1) throw new Error('二维码协议版本不兼容')
  if (!/^https?:\/\//i.test(payload.url)) throw new Error('二维码中的地址无效')
  if (!payload.token || payload.token.length < 32) throw new Error('二维码中的配对令牌无效')
  if (payload.expiresAt * 1000 <= Date.now()) throw new Error('二维码已过期，请在电脑端重新生成')

  const request = await pairRequest(payload.url, payload.token, settings.deviceName)
  const deadline = Math.min(payload.expiresAt * 1000, Date.now() + 150_000)
  let networkErrors = 0
  while (Date.now() < deadline) {
    let status
    try {
      status = await pairStatus(payload.url, request.requestId)
      networkErrors = 0
    } catch (error) {
      networkErrors += 1
      if (networkErrors >= 3) throw error
      await sleep(1500)
      continue
    }
    if (status.status === 'approved' && status.credential) {
      await setCredential(status.credential)
      localStorage.setItem(SERVER_URL_KEY, payload.url)
      localStorage.setItem(DEVICE_ID_KEY, status.deviceId ?? '')
      localStorage.setItem(
        FINGERPRINT_KEY,
        status.serverFingerprint ?? payload.fingerprint,
      )
      store.serverUrl = payload.url
      store.deviceId = status.deviceId ?? ''
      store.fingerprint = status.serverFingerprint ?? payload.fingerprint
      store.paired = true
      store.syncError = ''
      store.syncNotice = '配对成功，正在同步…'
      await syncNow()
      return
    }
    if (status.status === 'denied') throw new Error('电脑端拒绝了这次配对')
    if (status.status === 'expired') throw new Error('配对请求已过期，请重新生成二维码')
    if (status.status === 'delivered') throw new Error('配对凭证已领取，请重新配对')
    if (status.status === 'credentialExpired') throw new Error('配对凭证已过期，请重新生成二维码')
    await sleep(1500)
  }
  throw new Error('等待电脑确认超时，请重试')
}

async function ensureAsset(asset: AssetRecord, credential: string): Promise<AssetRecord> {
  if (asset.blob) {
    void touchAsset(asset.id)
    return asset
  }
  const result = await fetchResource(store.serverUrl, credential, asset.id)
  const hash = result.sha256 || (await sha256Hex(await result.blob.arrayBuffer()))
  if (asset.sha256 && asset.sha256 !== hash) {
    throw new Error('图片校验失败，请稍后重试')
  }
  const record: AssetRecord = {
    ...asset,
    blob: result.blob,
    sha256: hash,
    byteSize: result.blob.size,
    available: true,
    downloadedAt: Date.now(),
    lastAccess: Date.now(),
  }
  await putAsset(record)
  return record
}

export async function getThumbnailBlob(
  entityType: SyncEntityType,
  syncId: string,
  version: string,
): Promise<Blob | null> {
  const assets = await assetsForEntity(entityType, syncId)
  const asset = assets.find(
    (item) => item.assetType === 'thumbnail' && item.version === version,
  )
  if (!asset) return null
  if (asset.blob) {
    void touchAsset(asset.id)
    return asset.blob
  }
  if (!store.paired || !navigator.onLine) return null
  const credential = await getCredential()
  if (!credential) return null
  try {
    const loaded = await ensureAsset(asset, credential)
    return loaded.blob ?? null
  } catch {
    return null
  }
}

export async function getDisplayBlob(
  entityType: SyncEntityType,
  syncId: string,
  version: string,
): Promise<{ blob: Blob; asset: AssetRecord }> {
  const assets = await assetsForEntity(entityType, syncId)
  const displayType = version === 'edit' ? 'edit_display' : 'original_preview'
  const asset =
    assets.find((item) => item.assetType === displayType && item.version === version) ??
    assets.find((item) => item.assetType === 'thumbnail' && item.version === version)
  if (!asset) throw new Error('查看图尚未生成，请先同步')
  if (!store.paired || !navigator.onLine) {
    if (asset.blob) return { blob: asset.blob, asset }
    throw new Error('当前离线，且这张查看图尚未下载')
  }
  const credential = await getCredential()
  if (!credential) throw new Error('配对凭证已失效，请重新配对')
  const loaded = await ensureAsset(asset, credential)
  if (!loaded.blob) throw new Error('图片下载失败')
  activeAssetIds.add(loaded.id)
  await enforceCacheLimit(settings.cacheLimitBytes, activeAssetIds)
  await refreshCacheUsage()
  return { blob: loaded.blob, asset: loaded }
}

export function markAssetActive(id: string, active: boolean) {
  if (active) activeAssetIds.add(id)
  else activeAssetIds.delete(id)
}

export async function clearViewCache() {
  const freed = await clearDisplayCache(activeAssetIds)
  await refreshCacheUsage()
  store.syncNotice = freed > 0 ? `已清理 ${formatBytes(freed)} 查看图缓存` : '没有可清理的查看图缓存'
}

export async function refreshStorageUsage() {
  await refreshCacheUsage()
}

export function entitiesOf<T>(type: SyncEntityType): TypedEntity<T>[] {
  return store.entities
    .filter((entity) => entity.type === type)
    .map((entity) => ({ ...entity, data: entity.data as T }))
}

export function findEntity<T>(type: SyncEntityType, syncId: string): TypedEntity<T> | undefined {
  const entity = store.entities.find((item) => item.type === type && item.syncId === syncId)
  return entity ? { ...entity, data: entity.data as T } : undefined
}

export function cameras() {
  return entitiesOf<CameraEntity>('camera').sort((left, right) =>
    `${left.data.brand} ${left.data.model}`.localeCompare(
      `${right.data.brand} ${right.data.model}`,
      'zh-CN',
    ),
  )
}

export function equipment() {
  return entitiesOf<EquipmentEntity>('equipment').sort((left, right) =>
    `${left.data.brand} ${left.data.model}`.localeCompare(
      `${right.data.brand} ${right.data.model}`,
      'zh-CN',
    ),
  )
}

export function films() {
  return entitiesOf<FilmEntity>('film').sort((left, right) =>
    `${left.data.brand} ${left.data.name}`.localeCompare(
      `${right.data.brand} ${right.data.name}`,
      'zh-CN',
    ),
  )
}

export function rolls() {
  return entitiesOf<RollEntity>('roll').sort((left, right) => {
    const month = (right.data.shotMonth ?? '').localeCompare(left.data.shotMonth ?? '')
    return month !== 0 ? month : right.data.index - left.data.index
  })
}

export function albums() {
  return entitiesOf<DigitalAlbumEntity>('digital_album').sort((left, right) => {
    const date = (right.data.shotDate ?? '').localeCompare(left.data.shotDate ?? '')
    return date !== 0 ? date : right.savedAt - left.savedAt
  })
}

export function photosForRoll(rollSyncId: string) {
  return entitiesOf<FilmPhotoEntity>('film_photo')
    .filter((photo) => photo.data.rollSyncId === rollSyncId)
    .sort((left, right) => {
      const frame = (left.data.frameNumber ?? 999) - (right.data.frameNumber ?? 999)
      return frame !== 0 ? frame : left.syncId.localeCompare(right.syncId)
    })
}

export function photosForAlbum(albumSyncId: string) {
  return entitiesOf<DigitalPhotoEntity>('digital_photo')
    .filter((photo) => photo.data.albumSyncId === albumSyncId)
    .sort((left, right) =>
      left.data.pairingKey.localeCompare(right.data.pairingKey, 'zh-CN', { numeric: true }),
    )
}

export function rollsForCamera(cameraSyncId: string) {
  return rolls().filter((roll) => roll.data.cameraSyncId === cameraSyncId)
}

export function rollsForFilm(filmSyncId: string) {
  return rolls().filter((roll) => roll.data.filmSyncId === filmSyncId)
}

export function albumsForCamera(cameraSyncId: string) {
  return albums().filter((album) => album.data.cameraSyncId === cameraSyncId)
}

export function coverPhoto(
  entityType: 'film_photo' | 'digital_photo',
  coverPhotoSyncId?: string | null,
) {
  if (!coverPhotoSyncId) return undefined
  return entityType === 'film_photo'
    ? findEntity<FilmPhotoEntity>('film_photo', coverPhotoSyncId)
    : findEntity<DigitalPhotoEntity>('digital_photo', coverPhotoSyncId)
}

export function filmTitle(film: FilmEntity) {
  const name = film.name.trim()
  const brand = film.brand.trim()
  return name.toLowerCase().startsWith(brand.toLowerCase()) ? name : `${brand} ${name}`.trim()
}

export function filmTypeLabel(type: string) {
  if (type === 'Color Negative') return '彩色负片'
  if (type === 'Slide') return '反转片'
  if (type === 'B&W') return '黑白'
  return type
}

export function formatBytes(bytes: number) {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const value = bytes / 1024 ** index
  return `${value.toFixed(index === 0 ? 0 : 1)} ${units[index]}`
}

export function formatSyncTime(value?: string | null) {
  if (!value) return '尚未同步'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString('zh-CN', { hour12: false })
}

export async function totalEntityCount() {
  return countEntities()
}

export async function cachedAssetCount() {
  return (await allAssets()).length
}
