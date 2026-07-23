export type SenderType = 'user' | 'bot'

export interface Bot {
  id: string
  groupId: string
  nickname: string
  avatar: string
  persona: string
  model: string | null
  cursorChatId: string | null
}

export interface Group {
  id: string
  name: string
  createdAt: string
  botIds: string[]
  bots?: Bot[]
}

export interface Message {
  id: string
  groupId: string
  senderType: SenderType
  senderId?: string | null
  botId?: string | null
  nickname: string
  avatar?: string | null
  content: string
  createdAt: string
  status?: 'streaming' | 'done' | 'error' | 'recalled'
}

export interface CursorStatus {
  available: boolean
  loggedIn: boolean
  binary: string | null
  model?: string | null
  message: string
}

export interface UserProfile {
  nickname: string
  avatar: string
}

export { AVATAR_COLORS, randomAvatar } from './avatarGen'
