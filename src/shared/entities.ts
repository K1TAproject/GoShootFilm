export interface CameraEntity {
  brand: string
  model: string
  status: string
  cameraType: 'film' | 'digital'
  format?: string | null
  sensorFormat?: string | null
  purchaseDate?: string | null
  note?: string | null
}

export interface EquipmentEntity {
  category: 'lens' | 'other'
  subtype?: string | null
  brand: string
  model: string
  mount?: string | null
  status: string
  purchaseDate?: string | null
  note?: string | null
}

export interface FilmEntity {
  brand: string
  name: string
  iso: number
  type: string
  targetStatus?: string | null
  note?: string | null
}

export interface RollEntity {
  cameraSyncId?: string | null
  filmSyncId?: string | null
  cameraBrand: string
  cameraModel: string
  filmBrand: string
  filmName: string
  filmType: string
  cameraInfo: string
  filmInfo: string
  index: number
  shotMonth?: string | null
  city?: string | null
  note?: string | null
  coverPhotoSyncId?: string | null
  coverVersion?: string | null
  photoCount: number
}

export interface FilmPhotoEntity {
  rollSyncId?: string | null
  frameNumber?: number | null
  displayName?: string | null
  labAvailable: boolean
  editAvailable: boolean
  isFavorite: boolean
}

export interface DigitalAlbumEntity {
  title: string
  cameraSyncId?: string | null
  cameraBrand?: string | null
  cameraModel?: string | null
  shotDate?: string | null
  city?: string | null
  note?: string | null
  photoCount: number
  rawCount: number
  editCount: number
  coverPhotoSyncId?: string | null
  coverVersion?: string | null
}

export interface DigitalPhotoEntity {
  albumSyncId?: string | null
  pairingKey: string
  rawAvailable: boolean
  editAvailable: boolean
  isFavorite: boolean
}
