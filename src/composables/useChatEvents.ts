import { ref, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Message } from '../types'

type Handler = (payload: Record<string, unknown>) => void

export function useChatEvents(onEvent: Handler) {
  const connected = ref(true)
  let unlisten: UnlistenFn | null = null
  let cancelled = false

  void listen<Record<string, unknown>>('chat-event', (event) => {
    onEvent(event.payload)
  }).then((fn) => {
    if (cancelled) {
      fn()
      return
    }
    unlisten = fn
  })

  onUnmounted(() => {
    cancelled = true
    unlisten?.()
  })

  return { connected }
}

export function upsertMessage(list: Message[], message: Message) {
  const idx = list.findIndex((m) => m.id === message.id)
  if (idx >= 0) {
    list[idx] = { ...list[idx], ...message }
  } else {
    list.push(message)
  }
}
