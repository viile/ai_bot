<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import GroupSidebar from './components/GroupSidebar.vue'
import ChatPanel from './components/ChatPanel.vue'
import BotPanel from './components/BotPanel.vue'
import {
  createBot,
  createGroup,
  deleteBot,
  deleteGroup,
  fetchGroups,
  fetchMessages,
  fetchStatus,
  fetchUserProfile,
  postUserMessage,
  processPendingReplies,
  recallMessage,
  updateBot,
  updateUserProfile,
} from './api'
import { upsertMessage, useChatEvents } from './composables/useChatEvents'
import type { CursorStatus, Group, Message, UserProfile } from './types'

const groups = ref<Group[]>([])
const activeId = ref<string | null>(null)
const messages = ref<Message[]>([])
const showMembers = ref(false)
const sending = ref(false)
const status = ref<CursorStatus | null>(null)
const loading = ref(true)
const profile = ref<UserProfile | null>(null)

/** Wait for send + compose idle before asking bots to reply to the batch. */
const REPLY_IDLE_MS = 2000
let replyFlushTimer: ReturnType<typeof setTimeout> | null = null
let replyPending = false
let replyProcessing = false

function clearReplyFlushTimer() {
  if (replyFlushTimer) {
    clearTimeout(replyFlushTimer)
    replyFlushTimer = null
  }
}

function armReplyFlush() {
  if (!replyPending || !activeId.value) return
  clearReplyFlushTimer()
  replyFlushTimer = setTimeout(() => {
    void flushPendingReplies()
  }, REPLY_IDLE_MS)
}

async function flushPendingReplies() {
  replyFlushTimer = null
  const groupId = activeId.value
  if (!groupId || !replyPending || replyProcessing) return

  replyPending = false
  replyProcessing = true
  sending.value = true
  try {
    await processPendingReplies(groupId)
  } catch (err) {
    console.error(err)
    // Keep pending so a later idle can retry; also surface briefly.
    replyPending = true
    status.value = {
      available: status.value?.available ?? false,
      loggedIn: status.value?.loggedIn ?? false,
      binary: status.value?.binary ?? null,
      message: err instanceof Error ? err.message : `回复失败：${String(err)}`,
    }
  } finally {
    replyProcessing = false
    sending.value = false
    if (replyPending) armReplyFlush()
  }
}

function onComposeActivity() {
  if (replyPending) armReplyFlush()
}

const activeGroup = computed(() => groups.value.find((g) => g.id === activeId.value) || null)
const activeBots = computed(() => activeGroup.value?.bots || [])

const statusOk = computed(() => !!status.value?.available && !!status.value?.loggedIn)
const statusText = computed(() => status.value?.message || '检查 Cursor 状态中…')

async function refreshGroups(preferId?: string | null) {
  groups.value = await fetchGroups()
  const next =
    preferId && groups.value.some((g) => g.id === preferId)
      ? preferId
      : activeId.value && groups.value.some((g) => g.id === activeId.value)
        ? activeId.value
        : groups.value[0]?.id || null
  if (next !== activeId.value) {
    await selectGroup(next)
  }
}

async function selectGroup(id: string | null) {
  clearReplyFlushTimer()
  replyPending = false
  activeId.value = id
  showMembers.value = false
  messages.value = []
  if (!id) return
  messages.value = await fetchMessages(id)
  const g = groups.value.find((x) => x.id === id)
  if (g && !(g.bots && g.bots.length)) {
    showMembers.value = true
  }
}

async function onCreateGroup(name: string) {
  const g = await createGroup(name)
  await refreshGroups(g.id)
  showMembers.value = true
}

async function onRemoveGroup(id: string) {
  try {
    await deleteGroup(id)
    if (activeId.value === id) {
      activeId.value = null
      messages.value = []
    }
    await refreshGroups()
  } catch (err) {
    console.error(err)
    status.value = {
      available: status.value?.available ?? false,
      loggedIn: status.value?.loggedIn ?? false,
      binary: status.value?.binary ?? null,
      message: err instanceof Error ? err.message : `删除群失败：${String(err)}`,
    }
  }
}

async function onSend(content: string) {
  if (!activeId.value) return
  try {
    const msg = await postUserMessage(activeId.value, content)
    upsertMessage(messages.value, msg)
    replyPending = true
    armReplyFlush()
  } catch (err) {
    alert(err instanceof Error ? err.message : String(err))
  }
}

async function onRecall(messageId: string) {
  if (!activeId.value) return
  try {
    clearReplyFlushTimer()
    await recallMessage(activeId.value, messageId)
    sending.value = false
    // If more trailing user messages remain, wait for idle again.
    replyPending = true
    armReplyFlush()
  } catch (err) {
    console.error(err)
    status.value = {
      available: status.value?.available ?? false,
      loggedIn: status.value?.loggedIn ?? false,
      binary: status.value?.binary ?? null,
      message: err instanceof Error ? err.message : `撤回失败：${String(err)}`,
    }
  }
}

function removeMessagesByIds(ids: string[]) {
  if (!ids.length) return
  const set = new Set(ids)
  messages.value = messages.value.filter((m) => !set.has(m.id))
}

async function onSaveProfile(data: { nickname: string; avatar: string }) {
  try {
    profile.value = await updateUserProfile(data)
  } catch (err) {
    console.error(err)
    status.value = {
      available: status.value?.available ?? false,
      loggedIn: status.value?.loggedIn ?? false,
      binary: status.value?.binary ?? null,
      message: err instanceof Error ? err.message : `保存身份失败：${String(err)}`,
    }
  }
}

async function onCreateBot(data: {
  nickname: string
  avatar: string
  persona: string
  model: string | null
}) {
  if (!activeId.value) return
  await createBot(activeId.value, data)
  await refreshGroups(activeId.value)
}

async function onUpdateBot(
  botId: string,
  data: { nickname: string; avatar: string; persona: string; model: string | null },
) {
  if (!activeId.value) return
  await updateBot(activeId.value, botId, data)
  await refreshGroups(activeId.value)
}

async function onRemoveBot(botId: string) {
  if (!activeId.value) return
  try {
    await deleteBot(activeId.value, botId)
    await refreshGroups(activeId.value)
  } catch (err) {
    console.error(err)
    status.value = {
      available: status.value?.available ?? false,
      loggedIn: status.value?.loggedIn ?? false,
      binary: status.value?.binary ?? null,
      message: err instanceof Error ? err.message : `删除机器人失败：${String(err)}`,
    }
  }
}

function handleChatEvent(payload: Record<string, unknown>) {
  const type = String(payload.type || '')
  const groupId = payload.groupId as string | undefined

  if (type === 'group_updated') {
    void refreshGroups(activeId.value)
    return
  }

  if (!groupId || groupId !== activeId.value) return

  if (type === 'message_recalled') {
    const removed = (payload.removedIds as string[] | undefined) || []
    removeMessagesByIds(removed)
    // Also drop any in-flight typing bubbles.
    messages.value = messages.value.filter((m) => m.status !== 'streaming')
    if (payload.message) {
      upsertMessage(messages.value, payload.message as Message)
    }
    sending.value = false
    return
  }

  if (type === 'message_removed') {
    const removed = (payload.removedIds as string[] | undefined) || []
    const single = payload.messageId as string | undefined
    removeMessagesByIds([...removed, ...(single ? [single] : [])])
    return
  }

  if (type === 'message' && payload.message) {
    upsertMessage(messages.value, payload.message as Message)
    return
  }

  if (type === 'bot_typing' && payload.message) {
    upsertMessage(messages.value, payload.message as Message)
    return
  }

  if ((type === 'bot_done' || type === 'bot_error') && payload.message) {
    upsertMessage(messages.value, payload.message as Message)
  }
}

const { connected } = useChatEvents(handleChatEvent)

onMounted(async () => {
  try {
    status.value = await fetchStatus()
    profile.value = await fetchUserProfile()
    await refreshGroups()
  } catch (err) {
    status.value = {
      available: false,
      loggedIn: false,
      binary: null,
      message: err instanceof Error ? err.message : '无法连接桌面后端',
    }
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="app-shell">
    <GroupSidebar
      :groups="groups"
      :active-id="activeId"
      :connected="connected"
      :status-text="statusText"
      :status-ok="statusOk"
      :profile="profile"
      @select="selectGroup"
      @create="onCreateGroup"
      @remove="onRemoveGroup"
      @save-profile="onSaveProfile"
    />

    <main class="main">
      <div v-if="loading" class="placeholder">加载中…</div>

      <div v-else-if="!activeGroup" class="placeholder hero">
        <p class="mark">AI群聊</p>
        <h2>把多个角色请进同一个群</h2>
        <p>创建群聊，为每位机器人设定头像、昵称与身份，一条消息同步给所有人。</p>
      </div>

      <template v-else>
        <ChatPanel
          :group-name="activeGroup.name"
          :messages="messages"
          :bots="activeBots"
          :user-nickname="profile?.nickname || '我'"
          :bot-count="activeBots.length"
          :sending="sending"
          :show-members="showMembers"
          @send="onSend"
          @recall="onRecall"
          @compose-activity="onComposeActivity"
          @toggle-members="showMembers = !showMembers"
        />
        <BotPanel
          :bots="activeBots"
          :open="showMembers"
          @close="showMembers = false"
          @create="onCreateBot"
          @update="onUpdateBot"
          @remove="onRemoveBot"
        />
      </template>
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: minmax(260px, 300px) minmax(0, 1fr);
  height: 100%;
  min-height: 0;
  max-width: 1400px;
  margin: 0 auto;
  border-left: 1px solid var(--line);
  border-right: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.28);
  box-shadow: 0 20px 60px rgba(20, 34, 31, 0.08);
  overflow: hidden;
}

.main {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  position: relative;
}

.placeholder {
  grid-column: 1 / -1;
  display: grid;
  place-content: center;
  text-align: center;
  padding: 2rem;
  color: var(--muted);
}

.hero .mark {
  margin: 0;
  font-family: var(--font-display);
  font-size: clamp(2.4rem, 5vw, 3.4rem);
  font-weight: 700;
  color: var(--teal-deep);
  letter-spacing: -0.03em;
}

.hero h2 {
  margin: 0.5rem 0 0;
  font-size: 1.15rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.hero p:last-child {
  margin: 0.75rem auto 0;
  max-width: 28rem;
  line-height: 1.55;
}

@media (max-width: 860px) {
  .app-shell {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr;
    height: auto;
    min-height: 100%;
  }

  .main {
    min-height: 70vh;
  }
}
</style>
