<script setup lang="ts">
import { ref } from 'vue'
import type { Group } from '../types'
import AvatarBadge from './AvatarBadge.vue'

defineProps<{
  groups: Group[]
  activeId: string | null
  connected: boolean
  statusText: string
  statusOk: boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  create: [name: string]
  remove: [id: string]
}>()

const draft = ref('')
const creating = ref(false)

function submitCreate() {
  const name = draft.value.trim()
  if (!name) return
  emit('create', name)
  draft.value = ''
  creating.value = false
}
</script>

<template>
  <aside class="sidebar">
    <header class="brand">
      <p class="brand-mark">AI群聊</p>
      <h1>多机器人协作</h1>
      <p class="brand-sub">同一条消息，同步给群里每一位角色</p>
    </header>

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
        <button
          type="button"
          class="del"
          title="删除群"
          @click.stop="emit('remove', g.id)"
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
}

.group-list li:hover .del {
  opacity: 1;
}

.hint {
  margin: 0;
  font-size: 0.85rem;
  color: var(--muted);
}
</style>
