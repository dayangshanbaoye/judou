import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type PingResponse = {
  message: string
  job_id: string
}

export async function ping(payload: string): Promise<PingResponse> {
  return invoke<PingResponse>('ping', { payload })
}

export async function onPing(handler: (response: PingResponse) => void): Promise<UnlistenFn> {
  return listen<PingResponse>('ping://pong', (event) => handler(event.payload))
}
