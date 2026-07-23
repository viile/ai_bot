<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Group, UserProfile } from '../types'
import { randomAvatar } from '../avatarGen'
import AvatarBadge from './AvatarBadge.vue'
import AvatarPicker from './AvatarPicker.vue'

const props = defineProps<{
  groups: Group[]
  activeId: string | null
  connected: boolean
  statusText: string
  statusOk: boolean
  profile: UserProfile | null
}>()

const emit = defineEmits<{
  select: [id: string]
  create: [name: string]
  remove: [id: string]
  saveProfile: [data: { nickname: string; avatar: string }]
}>()

const draft = ref('')
const creating = ref(false)
/** Group id waiting for delete confirmation (Tauri has no reliable window.confirm). */
const pendingDeleteId = ref<string | null>(null)
const editingMe = ref(false)
const meNick = ref('我')
const meAvatar = ref(randomAvatar())

watch(
  () => props.profile,
  (p) => {
    if (!p) return
    meNick.value = p.nickname
    meAvatar.value = p.avatar
  },
  { immediate: true },
)

function submitCreate() {
  const name = draft.value.trim()
  if (!name) return
  emit('create', name)
  draft.value = ''
  creating.value = false
}

function requestDelete(id: string) {
  pendingDeleteId.value = id
}

function cancelDelete() {
  pendingDeleteId.value = null
}

function confirmDelete() {
  const id = pendingDeleteId.value
  if (!id) return
  pendingDeleteId.value = null
  emit('remove', id)
}

function saveMe() {
  const nickname = meNick.value.trim() || '我'
  emit('saveProfile', { nickname, avatar: meAvatar.value })
  editingMe.value = false
}

function cancelMe() {
  if (props.profile) {
    meNick.value = props.profile.nickname
    meAvatar.value = props.profile.avatar
  }
  editingMe.value = false
}
</script>

<template>
  <aside class="sidebar">
    <header class="brand">
      <p class="brand-mark">AI群聊</p>
      <h1>多机器人协作</h1>
      <p class="brand-sub">同一条消息，同步给群里每一位角色</p>
    </header>

    <div class="me-card">
      <div class="me-row">
        <AvatarBadge :name="meNick || '我'" :color-or-url="meAvatar" :size="36" />
        <div class="me-meta">
          <strong>{{ profile?.nickname || '我' }}</strong>
          <span>你在群里的身份 · 机器人可 @你</span>
        </div>
        <button type="button" class="link-btn" @click="editingMe = !editingMe">
          {{ editingMe ? '收起' : '设置' }}
        </button>
      </div>
      <div v-if="editingMe" class="me-edit">
        <label>
          昵称
          <input v-model="meNick" maxlength="24" placeholder="例如：阿凯" />
        </label>
        <AvatarPicker v-model="meAvatar" :name="meNick || '我'" />
        <div class="me-actions">
          <button type="button" class="ghost" @click="cancelMe">取消</button>
          <button type="button" class="primary" @click="saveMe">保存</button>
        </div>
      </div>
    </div>

    <div class="status" :class="{ ok: statusOk }">
      <span class="dot" />
      <span>{{ statusText }}</span>
      <span class="ws" :title="connected ? '桌面端已连接' : '未连接'">
        {{ connected ? '桌面' : '离线' }}
      </span>
    </div>

    <div class="section-head">
      <h2>群聊</h2>
      <button type="button" class="link-btn" @click="creating = !creating">
        {{ creating ? '取消' : '新建' }}
      </button>
    </div>

    <form v-if="creating" class="create-form" @submit.prevent="submitCreate">
      <input v-model="draft" placeholder="群名称，例如：产品评审" maxlength="40" autofocus />
      <button type="submit">创建</button>
    </form>

    <ul class="group-list">
      <li v-for="g in groups" :key="g.id">
        <button
          type="button"
          class="group-item"
          :class="{ active: g.id === activeId }"
          @click="emit('select', g.id)"
        >
          <div class="group-avatars">
            <AvatarBadge
              v-for="bot in (g.bots || []).slice(0, 3)"
              :key="bot.id"
              :name="bot.nickname"
              :color-or-url="bot.avatar"
              :size="22"
            />
            <span v-if="!(g.bots && g.bots.length)" class="empty-dot" />
          </div>
          <div class="group-meta">
            <strong>{{ g.name }}</strong>
            <span>{{ (g.bots || []).length }} 个机器人</span>
          </div>
        </button>

        <div v-if="pendingDeleteId === g.id" class="del-confirm" @click.stop>
          <span>删除此群？</span>
          <button type="button" class="danger" @click="confirmDelete">删除</button>
          <button type="button" class="ghost" @click="cancelDelete">取消</button>
        </div>
        <button
          v-else
          type="button"
          class="del"
          title="删除群"
          @click.stop="requestDelete(g.id)"
        >
          ×
        </button>
      </li>
    </ul>

    <p v-if="!groups.length" class="hint">先创建一个群，再往里加 Cursor 机器人。</p>
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.5rem 1.25rem;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border-right: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(10px);
}

.brand-mark {
  margin: 0;
  font-family: var(--font-display);
  font-size: 1.65rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--teal-deep);
}

.brand h1 {
  margin: 0.15rem 0 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.brand-sub {
  margin: 0.4rem 0 0;
  font-size: 0.85rem;
  color: var(--muted);
  line-height: 1.4;
}

.me-card {
  border: 1px solid var(--line);
  border-radius: 14px;
  padding: 0.7rem 0.75rem;
  background: rgba(255, 255, 255, 0.65);
}

.me-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.me-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.me-meta strong {
  font-size: 0.92rem;
}

.me-meta span {
  font-size: 0.72rem;
  color: var(--muted);
}

.me-edit {
  margin-top: 0.7rem;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  border-top: 1px solid var(--line);
  padding-top: 0.7rem;
}

.me-edit label {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.me-edit input {
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.45rem 0.6rem;
  background: #fff;
  font-weight: 400;
  color: var(--ink);
}

.me-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.4rem;
}

.me-actions .ghost {
  border: 0;
  background: transparent;
  color: var(--muted);
  font-weight: 600;
  font-size: 0.8rem;
}

.me-actions .primary {
  border: 0;
  border-radius: 8px;
  padding: 0.35rem 0.7rem;
  background: var(--teal);
  color: #fff;
  font-weight: 700;
  font-size: 0.8rem;
}

.status {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.55rem 0.7rem;
  border-radius: 999px;
  background: rgba(180, 35, 24, 0.08);
  color: var(--danger);
  font-size: 0.8rem;
}

.status.ok {
  background: rgba(13, 148, 136, 0.12);
  color: var(--teal-deep);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
}

.ws {
  margin-left: auto;
  opacity: 0.75;
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.section-head h2 {
  margin: 0;
  font-size: 0.8rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.link-btn {
  border: 0;
  background: transparent;
  color: var(--teal-deep);
  font-weight: 600;
  padding: 0;
  font-size: 0.8rem;
}

.create-form {
  display: flex;
  gap: 0.4rem;
}

.create-form input {
  flex: 1;
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.55rem 0.7rem;
  background: #fff;
}

.create-form button {
  border: 0;
  border-radius: 10px;
  padding: 0.55rem 0.85rem;
  background: var(--teal);
  color: #fff;
  font-weight: 600;
}

.group-list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow: auto;
  flex: 1;
}

.group-list li {
  position: relative;
  margin-bottom: 0.35rem;
}

.group-item {
  width: 100%;
  display: flex;
  gap: 0.7rem;
  align-items: center;
  text-align: left;
  border: 1px solid transparent;
  border-radius: 14px;
  padding: 0.7rem 2rem 0.7rem 0.7rem;
  background: transparent;
  color: inherit;
  transition: background 0.2s ease, border-color 0.2s ease;
}

.group-item:hover {
  background: rgba(255, 255, 255, 0.7);
}

.group-item.active {
  background: #fff;
  border-color: rgba(13, 148, 136, 0.25);
  box-shadow: 0 8px 24px rgba(15, 118, 110, 0.08);
}

.group-avatars {
  display: flex;
  align-items: center;
}

.group-avatars :deep(.avatar) {
  margin-left: -6px;
  border: 2px solid #fff;
}

.group-avatars :deep(.avatar:first-child) {
  margin-left: 0;
}

.empty-dot {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--sand);
  border: 1px dashed var(--line);
}

.group-meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.group-meta strong {
  font-size: 0.95rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.group-meta span {
  font-size: 0.78rem;
  color: var(--muted);
}

.del {
  position: absolute;
  right: 0.45rem;
  top: 50%;
  transform: translateY(-50%);
  border: 0;
  background: transparent;
  color: var(--muted);
  font-size: 1.1rem;
  line-height: 1;
  padding: 0.2rem 0.35rem;
  opacity: 0;
  z-index: 1;
}

.group-list li:hover .del,
.del:focus-visible {
  opacity: 1;
}

.del-confirm {
  position: absolute;
  inset: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.35rem;
  padding: 0 0.5rem;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid rgba(180, 35, 24, 0.2);
  font-size: 0.78rem;
  color: var(--ink-soft);
}

.del-confirm .danger {
  border: 0;
  border-radius: 8px;
  padding: 0.3rem 0.55rem;
  background: var(--danger);
  color: #fff;
  font-weight: 600;
  font-size: 0.75rem;
}

.del-confirm .ghost {
  border: 0;
  border-radius: 8px;
  padding: 0.3rem 0.55rem;
  background: transparent;
  color: var(--muted);
  font-weight: 600;
  font-size: 0.75rem;
}

.hint {
  margin: 0;
  font-size: 0.85rem;
  color: var(--muted);
}
</style>
