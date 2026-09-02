export interface Camera {
  id: number
  brand: string
  model: string
  status: string
  format?: string
  purchaseDate?: string
  note?: string
}

export interface Film {
  id: number
  brand: string
  name: string
  iso: number
  type: string
  targetStatus?: string
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
  cameraInfo: string
  filmInfo: string
  coverPath?: string
  photoCount: number
}

export interface Photo {
  id: number
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

export interface LabOriginal {
  photoId: number
  path: string
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
}
