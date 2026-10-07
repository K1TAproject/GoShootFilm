export interface Camera {
  id: number
  brand: string
  model: string
  status: string
  cameraType: 'film' | 'digital'
  format?: string
  sensorFormat?: string
  purchaseDate?: string
  note?: string
}

export interface Film {
  id: number
  brand: string
  name: string
  iso: number
  type: string
  targetStatus?: 'unshot' | 'shot'
  note?: string
}

export interface RollSummary {
  id: number
  cameraId: number
  filmId: number
  index: number
  shotMonth?: string
  city?: string
  note?: string
  cameraBrand: string
  cameraModel: string
  filmBrand: string
  filmName: string
  filmType: string
  cameraInfo: string
  filmInfo: string
  coverPath?: string
  photoCount: number
}

export interface Photo {
  id: number
  displayName?: string
  frameNumber?: number
  labScanPath?: string
  editScanPath?: string
  isFavorite: boolean
}

export type PhotoVersion = 'edit' | 'lab'
export type ImportConflictAction = 'add' | 'ask' | 'skip' | 'replace' | 'cancel'

export interface ImportDraft {
  sourcePath: string
  frameNumber?: number
}

export interface ImportAnalysisItem {
  sourcePath: string
  fileName: string
  frameNumber?: number
  existingVersion: boolean
  pairedVersion: boolean
  issue?: string
  conflictAction: ImportConflictAction
}

export interface PhotoImportEntry {
  sourcePath: string
  frameNumber: number
  conflictAction: ImportConflictAction
}

export interface ImportResult {
  importedCount: number
  updatedCount: number
  skippedCount: number
}

export interface LabPreview {
  photoId: number
  previewPath: string
}

export interface LabPreviewState {
  loading: boolean
  previewPath?: string
  error?: string
}

export interface RollDetail extends RollSummary {
  photos: Photo[]
}

export interface CameraRoll {
  id: number
  rollIndex: number
  shotMonth?: string
  city?: string
  filmBrand: string
  filmName: string
  filmIso: number
  filmInfo: string
}

export interface CameraDetail {
  camera: Camera
  rolls: CameraRoll[]
}

export interface DashboardStats {
  cameras: Camera[]
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

export interface EquipmentItem {
  id: number
  category: 'lens' | 'other'
  subtype?: string
  brand: string
  model: string
  mount?: string
  status: 'active' | 'inactive'
  purchaseDate?: string
  note?: string
}

export interface DigitalPhoto {
  id: number
  pairingKey: string
  rawPath?: string
  editPath?: string
  isFavorite: boolean
}

export interface DigitalAlbum {
  id: number
  title: string
  cameraId?: number
  shotDate?: string
  city?: string
  note?: string
  cameraBrand?: string
  cameraModel?: string
  photoCount: number
  rawCount: number
  editCount: number
  coverPhotoId?: number
  coverVersion?: 'raw' | 'edit'
}

export interface DigitalAlbumDetail extends DigitalAlbum {
  photos: DigitalPhoto[]
}

export interface DigitalImportAnalysisItem {
  sourcePath: string
  fileName: string
  pairingKey: string
  existingVersion: boolean
  pairedVersion: boolean
  issue?: string
  conflictAction: ImportConflictAction
}

export interface LibraryStatus {
  libraryPath?: string
  available: boolean
  needsMigration: boolean
  legacyDataDetected: boolean
  error?: string
}

export interface LibraryMigrationResult {
  libraryPath: string
  fileCount: number
  totalBytes: number
  migrated: boolean
}
