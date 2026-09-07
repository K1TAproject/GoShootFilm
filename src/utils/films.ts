import filmCatalogCsv from '../data/film-catalog.csv?raw'

export const FILM_TYPES = ['B&W', 'Color Negative', 'Slide'] as const
export const FILM_TARGET_STATUSES = ['unshot', 'shot'] as const
export const FILM_PLACEHOLDER_PATH = '/film-stocks/placeholder.svg'

function filmImageKey(brand: string, name: string) {
  return `${brand.trim()}\u0000${name.trim()}`.replace(/\s+/g, ' ').toLocaleLowerCase()
}

function catalogImagePaths() {
  const paths: Record<string, string> = {}
  for (const line of filmCatalogCsv.split(/\r?\n/).slice(2)) {
    if (!line.trim()) continue
    const [brand, name, , , image, legacyBrand, legacyName] = line.split(';')
    if (!brand || !name || !image) continue
    const path = `/film-stocks/${image}`
    paths[filmImageKey(brand, name)] = path
    if (legacyBrand && legacyName) paths[filmImageKey(legacyBrand, legacyName)] = path
  }
  return Object.freeze(paths)
}

const FILM_IMAGE_PATHS: Readonly<Record<string, string>> = catalogImagePaths()

export function filmDisplayName(brand: string, name: string) {
  const cleanBrand = brand.trim().replace(/\s+/g, ' ')
  const cleanName = name.trim().replace(/\s+/g, ' ')
  if (!cleanBrand) return cleanName
  if (!cleanName) return cleanBrand
  const lowerBrand = cleanBrand.toLocaleLowerCase()
  const lowerName = cleanName.toLocaleLowerCase()
  return lowerName === lowerBrand || lowerName.startsWith(`${lowerBrand} `)
    ? cleanName
    : `${cleanBrand} ${cleanName}`
}

export function filmImagePath(brand: string, name: string) {
  return FILM_IMAGE_PATHS[filmImageKey(brand, name)] ?? FILM_PLACEHOLDER_PATH
}

export function useFilmImageFallback(event: Event) {
  const image = event.target as HTMLImageElement
  if (image.dataset.fallbackApplied === 'true') return
  image.dataset.fallbackApplied = 'true'
  image.onerror = null
  image.src = FILM_PLACEHOLDER_PATH
}

export function compactFilmTypeLabel(type: string) {
  return type === 'Color Negative' ? 'Color' : type
}

export function filmTargetStatusLabel(status?: string) {
  return status === 'shot' ? '已拍摄' : '未拍摄'
}
