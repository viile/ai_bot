import { invoke } from '@tauri-apps/api/core'
import type { Bot, CursorStatus, Group, Message, UserProfile } from './types'

export function fetchStatus() {
  return invoke<CursorStatus>('get_status')
}

export function fetchUserProfile() {
  return invoke<UserProfile>('get_user_profile')
}

export function updateUserProfile(data: { nickname?: string; avatar?: string }) {
  return invoke<UserProfile>('update_user_profile', {
    input: {
      nickname: data.nickname,
      avatar: data.avatar,
    },
  })
}

export function fetchGroups() {
  return invoke<Group[]>('list_groups')
}

export function createGroup(name: string) {
  return invoke<Group>('create_group', { name })
}

export function updateGroup(id: string, patch: { name: string }) {
  return invoke<Group>('update_group', { id, name: patch.name })
}

export function deleteGroup(id: string) {
  return invoke<void>('delete_group', { id })
}

export function fetchMessages(groupId: string) {
  return invoke<Message[]>('list_messages', { groupId })
}

export function createBot(
  groupId: string,
  data: { nickname: string; avatar: string; persona: string; model?: string | null },
) {
  return invoke<Bot>('create_bot', {
    groupId,
    input: {
      nickname: data.nickname,
      avatar: data.avatar,
      persona: data.persona,
      model: data.model || null,
    },
  })
}

export function updateBot(
  groupId: string,
  botId: string,
  data: Partial<{ nickname: string; avatar: string; persona: string; model: string | null }>,
) {
  return invoke<Bot>('update_bot', {
    groupId,
    botId,
    input: {
      nickname: data.nickname,
      avatar: data.avatar,
      persona: data.persona,
      model: data.model ?? undefined,
      clearModel: data.model === null ? true : undefined,
    },
  })
}

export function deleteBot(groupId: string, botId: string) {
  return invoke<void>('delete_bot', { groupId, botId })
}

export function postUserMessage(groupId: string, content: string) {
  return invoke<Message>('post_user_message', { groupId, content })
}

export function processPendingReplies(groupId: string) {
  return invoke<void>('process_pending_replies', { groupId })
}

export function recallMessage(groupId: string, messageId: string) {
  return invoke<Message>('recall_message', { groupId, messageId })
}
