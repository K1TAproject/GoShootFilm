import type {
  MobileAssetDescriptor,
  SyncChange,
  SyncEntityType,
  SyncStats,
} from '../../../src/shared/sync'

const DB_NAME = 'goshootfilm-mobile'
const DB_VERSION = 1

export interface EntityRecord {
  key: string
  type: SyncEntityType
  syncId: string
  revision: number
  data: Record<string, unknown>
  savedAt: number
}

export interface AssetRecord extends MobileAssetDescriptor {
  entityKey: string
  blob?: Blob
  downloadedAt: number | null
  lastAccess: number
}

export interface CacheUsage {
  totalBytes: number
  thumbnailBytes: number
  displayBytes: number
}

let databasePromise: Promise<IDBDatabase> | null = null

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'))
  })
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve()
    transaction.onerror = () => reject(transaction.error ?? new Error('IndexedDB transaction failed'))
    transaction.onabort = () => reject(transaction.error ?? new Error('IndexedDB transaction aborted'))
  })
}

function openDatabase(): Promise<IDBDatabase> {
  if (databasePromise) return databasePromise
  databasePromise = new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)
    request.onupgradeneeded = () => {
      const database = request.result
      if (!database.objectStoreNames.contains('entities')) {
        const entities = database.createObjectStore('entities', { keyPath: 'key' })
        entities.createIndex('byType', 'type')
      }
      if (!database.objectStoreNames.contains('assets')) {
        const assets = database.createObjectStore('assets', { keyPath: 'id' })
        assets.createIndex('byEntity', 'entityKey')
        assets.createIndex('byAssetType', 'assetType')
        assets.createIndex('byLastAccess', 'lastAccess')
      }
      if (!database.objectStoreNames.contains('meta')) {
        database.createObjectStore('meta', { keyPath: 'key' })
      }
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('IndexedDB open failed'))
  })
  return databasePromise
}

export function entityKey(entityType: SyncEntityType, syncId: string) {
  return `${entityType}:${syncId}`
}

export async function applyChanges(
  changes: SyncChange[],
  cursor: number,
  stats: SyncStats,
): Promise<void> {
  const database = await openDatabase()
  const transaction = database.transaction(['entities', 'assets', 'meta'], 'readwrite')
  const entities = transaction.objectStore('entities')
  const assets = transaction.objectStore('assets')
  const meta = transaction.objectStore('meta')

  for (const change of changes) {
    const key = entityKey(change.entityType, change.syncId)
    if (change.deleted) {
      entities.delete(key)
      const assetKeys = await requestResult(assets.index('byEntity').getAllKeys(key))
      for (const assetKey of assetKeys) assets.delete(assetKey)
      continue
    }
    entities.put({
      key,
      type: change.entityType,
      syncId: change.syncId,
      revision: change.revision,
      data: change.data ?? {},
      savedAt: Date.now(),
    } satisfies EntityRecord)

    for (const descriptor of change.assets ?? []) {
      const current = (await requestResult(assets.get(descriptor.id))) as AssetRecord | undefined
      const hashChanged = current?.sha256 !== descriptor.sha256
      const record: AssetRecord = {
        ...descriptor,
        entityKey: key,
        blob: hashChanged ? undefined : current?.blob,
        downloadedAt: hashChanged ? null : current?.downloadedAt ?? null,
        lastAccess: current?.lastAccess ?? 0,
      }
      assets.put(record)
    }
  }

  meta.put({ key: 'cursor', value: cursor })
  meta.put({ key: 'stats', value: stats })
  meta.put({ key: 'lastSyncAt', value: new Date().toISOString() })
  await transactionDone(transaction)
}

export async function getAllEntities(): Promise<EntityRecord[]> {
  const database = await openDatabase()
  const transaction = database.transaction('entities', 'readonly')
  return requestResult(transaction.objectStore('entities').getAll())
}

export async function getEntity(
  entityType: SyncEntityType,
  syncId: string,
): Promise<EntityRecord | undefined> {
  const database = await openDatabase()
  const transaction = database.transaction('entities', 'readonly')
  return requestResult(transaction.objectStore('entities').get(entityKey(entityType, syncId)))
}

export async function getMeta<T>(key: string, fallback: T): Promise<T> {
  const database = await openDatabase()
  const transaction = database.transaction('meta', 'readonly')
  const row = (await requestResult(transaction.objectStore('meta').get(key))) as
    | { key: string; value: T }
    | undefined
  return row?.value ?? fallback
}

export async function setMeta(key: string, value: unknown): Promise<void> {
  const database = await openDatabase()
  const transaction = database.transaction('meta', 'readwrite')
  transaction.objectStore('meta').put({ key, value })
  await transactionDone(transaction)
}

export async function assetsForEntity(
  entityType: SyncEntityType,
  syncId: string,
): Promise<AssetRecord[]> {
  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readonly')
  return requestResult(
    transaction.objectStore('assets').index('byEntity').getAll(entityKey(entityType, syncId)),
  )
}

export async function getAsset(id: string): Promise<AssetRecord | undefined> {
  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readonly')
  return requestResult(transaction.objectStore('assets').get(id))
}

export async function putAsset(asset: AssetRecord): Promise<void> {
  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readwrite')
  transaction.objectStore('assets').put(asset)
  await transactionDone(transaction)
}

export async function touchAsset(id: string): Promise<void> {
  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readwrite')
  const store = transaction.objectStore('assets')
  const row = (await requestResult(store.get(id))) as AssetRecord | undefined
  if (row) {
    row.lastAccess = Date.now()
    store.put(row)
  }
  await transactionDone(transaction)
}

export async function allAssets(): Promise<AssetRecord[]> {
  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readonly')
  return requestResult(transaction.objectStore('assets').getAll())
}

export async function cacheUsage(): Promise<CacheUsage> {
  const assets = await allAssets()
  let thumbnailBytes = 0
  let displayBytes = 0
  for (const asset of assets) {
    if (!asset.blob) continue
    if (asset.assetType === 'thumbnail') {
      thumbnailBytes += asset.blob.size
    } else {
      displayBytes += asset.blob.size
    }
  }
  return {
    totalBytes: thumbnailBytes + displayBytes,
    thumbnailBytes,
    displayBytes,
  }
}

export async function clearDisplayCache(activeIds: Set<string>): Promise<number> {
  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readwrite')
  const store = transaction.objectStore('assets')
  const assets = await requestResult(store.getAll()) as AssetRecord[]
  let freed = 0
  for (const asset of assets) {
    if (asset.assetType === 'thumbnail' || activeIds.has(asset.id)) continue
    if (asset.blob) freed += asset.blob.size
    store.delete(asset.id)
  }
  await transactionDone(transaction)
  return freed
}

export async function enforceCacheLimit(limitBytes: number, activeIds: Set<string>): Promise<number> {
  if (limitBytes <= 0) return 0
  const assets = await allAssets()
  const display = assets
    .filter((asset) => asset.assetType !== 'thumbnail' && asset.blob && !activeIds.has(asset.id))
    .sort((left, right) => left.lastAccess - right.lastAccess)
  let total = assets
    .filter((asset) => asset.assetType !== 'thumbnail' && asset.blob)
    .reduce((sum, asset) => sum + (asset.blob?.size ?? 0), 0)
  if (total <= limitBytes) return 0

  const database = await openDatabase()
  const transaction = database.transaction('assets', 'readwrite')
  const store = transaction.objectStore('assets')
  let removed = 0
  for (const asset of display) {
    if (total <= limitBytes) break
    total -= asset.blob?.size ?? 0
    store.delete(asset.id)
    removed += 1
  }
  await transactionDone(transaction)
  return removed
}

export async function resetSyncData(): Promise<void> {
  const database = await openDatabase()
  const transaction = database.transaction(['entities', 'assets', 'meta'], 'readwrite')
  transaction.objectStore('entities').clear()
  transaction.objectStore('assets').clear()
  const meta = transaction.objectStore('meta')
  meta.delete('cursor')
  meta.delete('stats')
  meta.delete('lastSyncAt')
  await transactionDone(transaction)
}

export async function countEntities(): Promise<number> {
  const database = await openDatabase()
  const transaction = database.transaction('entities', 'readonly')
  return requestResult(transaction.objectStore('entities').count())
}
