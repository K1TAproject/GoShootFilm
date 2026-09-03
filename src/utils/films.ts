export const FILM_TYPES = ['B&W', 'Color Negative', 'Slide'] as const

export function compactFilmTypeLabel(type: string) {
  return type === 'Color Negative' ? 'Color' : type
}
