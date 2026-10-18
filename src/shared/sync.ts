export const SYNC_PROTOCOL_VERSION = 1

export type SyncEntityType =
  | 'camera'
  | 'equipment'
  | 'film'
  | 'roll'
  | 'film_photo'
  | 'digital_album'
  | 'digital_photo'

export type MobileAssetType = 'thumbnail' | 'edit_display' | 'original_preview'
export type PhotoVersion = 'lab' | 'edit' | 'raw'

export interface SyncStats {
  cameraCount: number
  filmCount: number
  shotFilmCount: number
  rollCount: number
  photoCount: number
  favoritePhotoCount: number
  equipmentCount: number
  digitalAlbumCount: number
  digitalPhotoCount: number
  archivedPhotoCount: number
}

export interface MobileAssetDescriptor {
  id: string
  entityType: SyncEntityType
  entitySyncId: string
  version: PhotoVersion
  assetType: MobileAssetType
  sha256: string | null
  byteSize: number | null
  width: number | null
  height: number | null
  available: boolean
}

export interface SyncChange {
  entityType: SyncEntityType
  syncId: string
  revision: number
  deleted: boolean
  data?: Record<string, unknown>
  assets?: MobileAssetDescriptor[]
}

export interface SyncChangesResponse {
  protocolVersion: number
  cursor: number
  resetRequired: boolean
  hasMore: boolean
  serverTime: string
  stats: SyncStats
  changes: SyncChange[]
}

export interface PairStatusResponse {
  status:
    | 'pending'
    | 'approved'
    | 'delivered'
    | 'credentialExpired'
    | 'denied'
    | 'expired'
  requestId?: string
  deviceId?: string
  credential?: string
  serverFingerprint?: string
  protocolVersion?: number
}

export interface MobileSyncStatus {
  running: boolean
  address: string | null
  port: number | null
  fingerprint: string
  protocolVersion: number
  pairedDeviceCount: number
  pendingRequestCount: number
  defaultPort: number
  localOnly: boolean
}

export interface PairingSession {
  qrSvg: string
  payload: string
  url: string
  expiresAt: number
  fingerprint: string
  serverName: string
}

export interface PairingRequest {
  id: string
  deviceName: string
  platform: string
  createdAt: string
  expiresAt: string
}

export interface MobileDevice {
  id: string
  name: string
  platform: string
  pairedAt: string
  lastSyncAt: string | null
}

export interface PairingQrPayload {
  v: number
  url: string
  token: string
  fingerprint: string
  expiresAt: number
  serverName: string
}
