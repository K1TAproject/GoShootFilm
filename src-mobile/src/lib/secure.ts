interface SecureBridge {
  getItem(key: string): Promise<string | null> | string | null
  setItem(key: string, value: string): Promise<void> | void
  removeItem(key: string): Promise<void> | void
}

declare global {
  interface Window {
    GoShootFilmSecure?: SecureBridge
  }
}

const CREDENTIAL_KEY = 'goshootfilm.mobile.credential'

function bridge() {
  return window.GoShootFilmSecure
}

export function usingNativeSecureStorage() {
  return Boolean(bridge())
}

export async function getCredential(): Promise<string | null> {
  const native = bridge()
  if (native) return await native.getItem(CREDENTIAL_KEY)
  return localStorage.getItem(CREDENTIAL_KEY)
}

export async function setCredential(value: string): Promise<void> {
  const native = bridge()
  if (native) {
    await native.setItem(CREDENTIAL_KEY, value)
    return
  }
  localStorage.setItem(CREDENTIAL_KEY, value)
}

export async function clearCredential(): Promise<void> {
  const native = bridge()
  if (native) {
    await native.removeItem(CREDENTIAL_KEY)
    return
  }
  localStorage.removeItem(CREDENTIAL_KEY)
}
