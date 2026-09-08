import { convertFileSrc } from '@tauri-apps/api/core'
import type { PhotoVersion } from '../types'

export const PHOTO_EXTENSIONS: Record<PhotoVersion, string[]> = {
  lab: ['png', 'jpg', 'jpeg', 'tif', 'tiff', 'webp', 'bmp', 'gif'],
  edit: ['png', 'jpg', 'jpeg', 'webp'],
}

const supportedPhotoExtensions = new Set(Object.values(PHOTO_EXTENSIONS).flat())

export function photoImageUrl(path?: string) {
  if (!path) return ''
  return path.startsWith('/') ? path : convertFileSrc(path)
}

export function isSupportedPhotoPath(path: string) {
  const extension = path.split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase()
  return extension ? supportedPhotoExtensions.has(extension) : false
}
