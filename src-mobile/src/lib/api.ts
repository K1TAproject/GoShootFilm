import type {
  PairStatusResponse,
  SyncChangesResponse,
} from '../../../src/shared/sync'

export class ApiRequestError extends Error {
  readonly status: number
  readonly code: string

  constructor(message: string, status = 0, code = 'network_error') {
    super(message)
    this.name = 'ApiRequestError'
    this.status = status
    this.code = code
  }
}

function baseUrl(value: string) {
  return value.trim().replace(/\/+$/, '')
}

async function readJson(response: Response): Promise<Record<string, unknown>> {
  const text = await response.text()
  if (!text) return {}
  try {
    return JSON.parse(text) as Record<string, unknown>
  } catch {
    return {}
  }
}

async function requestJson<T>(
  url: string,
  init: RequestInit,
  timeoutMs: number,
): Promise<T> {
  const controller = new AbortController()
  const timer = window.setTimeout(() => controller.abort(), timeoutMs)
  try {
    const response = await fetch(url, { ...init, signal: controller.signal })
    const body = await readJson(response)
    if (!response.ok) {
      const message = typeof body.error === 'string' ? body.error : '请求失败'
      const code = typeof body.code === 'string' ? body.code : 'request_failed'
      throw new ApiRequestError(message, response.status, code)
    }
    return body as T
  } catch (error) {
    if (error instanceof ApiRequestError) throw error
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new ApiRequestError('请求超时，请检查电脑是否仍在同步', 0, 'timeout')
    }
    throw new ApiRequestError(
      '无法连接电脑，请确认手机与电脑在同一局域网，且同步服务已启动',
      0,
      'network_error',
    )
  } finally {
    window.clearTimeout(timer)
  }
}

export async function health(serverUrl: string) {
  return requestJson<{
    protocolVersion: number
    serverFingerprint: string
    appVersion: string
    pairingAvailable: boolean
  }>(`${baseUrl(serverUrl)}/api/v1/health`, { method: 'GET' }, 8_000)
}

export async function pairRequest(
  serverUrl: string,
  token: string,
  deviceName: string,
) {
  return requestJson<{ requestId: string; expiresAt: number }>(
    `${baseUrl(serverUrl)}/api/v1/pair/request`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        token,
        deviceName,
        platform: 'android',
        protocolVersion: 1,
      }),
    },
    10_000,
  )
}

export async function pairStatus(serverUrl: string, requestId: string) {
  return requestJson<PairStatusResponse>(
    `${baseUrl(serverUrl)}/api/v1/pair/status/${encodeURIComponent(requestId)}`,
    { method: 'GET' },
    10_000,
  )
}

export async function fetchChanges(
  serverUrl: string,
  credential: string,
  cursor: number,
  limit = 150,
) {
  return requestJson<SyncChangesResponse>(
    `${baseUrl(serverUrl)}/api/v1/sync/changes?cursor=${encodeURIComponent(String(cursor))}&limit=${encodeURIComponent(String(limit))}`,
    {
      method: 'GET',
      headers: { Authorization: `Bearer ${credential}` },
    },
    20_000,
  )
}

export async function fetchResource(
  serverUrl: string,
  credential: string,
  assetId: string,
): Promise<{ blob: Blob; sha256: string }> {
  const controller = new AbortController()
  const timer = window.setTimeout(() => controller.abort(), 35_000)
  try {
    const response = await fetch(
      `${baseUrl(serverUrl)}/api/v1/resources/${encodeURIComponent(assetId)}`,
      {
        method: 'GET',
        headers: { Authorization: `Bearer ${credential}` },
        signal: controller.signal,
      },
    )
    if (!response.ok) {
      const body = await readJson(response)
      throw new ApiRequestError(
        typeof body.error === 'string' ? body.error : '图片下载失败',
        response.status,
        typeof body.code === 'string' ? body.code : 'resource_failed',
      )
    }
    const blob = await response.blob()
    return {
      blob,
      sha256: response.headers.get('x-goshootfilm-sha256') ?? '',
    }
  } catch (error) {
    if (error instanceof ApiRequestError) throw error
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new ApiRequestError('图片下载超时', 0, 'timeout')
    }
    throw new ApiRequestError('图片下载失败，请检查网络', 0, 'network_error')
  } finally {
    window.clearTimeout(timer)
  }
}
