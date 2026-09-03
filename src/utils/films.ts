export const FILM_TYPES = ['B&W', 'Color Negative', 'Slide'] as const

export function compactFilmTypeLabel(type: string) {
  return type === 'Color Negative' ? 'Color' : type
}

export function filmTargetStatusLabel(status?: string) {
  if (status === 'shot') return '已拍摄'
  if (status === 'unshot') return '未拍摄'
  return '未测试'
}
