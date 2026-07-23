<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { Message } from '../types'
import AvatarBadge from './AvatarBadge.vue'

const props = defineProps<{
  groupName: string
  messages: Message[]
  botCount: number
  sending: boolean
  showMembers: boolean
}>()

const emit = defineEmits<{
  send: [content: string]
  toggleMembers: []
}>()

const draft = ref('')
const scroller = ref<HTMLElement | null>(null)

const canSend = computed(() => draft.value.trim().length > 0 && !props.sending)

async function scrollBottom() {
  await nextTick()
  if (scroller.value) {
    scroller.value.scrollTop = scroller.value.scrollHeight
  }
}

watch(
  () => props.messages.map((m) => `${m.id}:${m.content.length}:${m.status}`).join('|'),
  () => {
    void scrollBottom()
  },
)

function submit() {
  if (!canSend.value) return
  const text = draft.value.trim()
  draft.value = ''
  emit('send', text)
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    submit()
  }
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return ''
  }
}
</script>

<template>
  <section class="chat">
    <header class="chat-head">
      <div>
        <h2>{{ groupName }}</h2>
        <p>{{ botCount }} 个 Cursor 机器人在群里</p>
      </div>
      <button type="button" class="ghost" @click="emit('toggleMembers')">
        {{ showMembers ? '收起成员' : '管理机器人' }}
      </button>
    </header>

    <div ref="scroller" class="messages">
      <div v-if="!messages.length" class="empty">
        <p>还没有消息。说点什么，群里的机器人会一起回应。</p>
      </div>

      <article
        v-for="m in messages"
        :key="m.id"
        class="bubble-row"
        :class="m.senderType"
      >
        <AvatarBadge
          :name="m.nickname"
          :color-or-url="m.avatar || '#0d9488'"
          :size="34"
        />
        <div class="bubble-wrap">
          <div class="meta">
            <strong>{{ m.nickname }}</strong>
            <time>{{ formatTime(m.createdAt) }}</time>
          </div>
          <div class="bubble" :class="m.status">
            <template v-if="m.status === 'streaming' && !m.content">
              <span class="typing">正在输入</span>
            </template>
            <template v-else>
              <pre>{{ m.content }}</pre>
              <span v-if="m.status === 'streaming'" class="caret" />
            </template>
          </div>
        </div>
      </article>
    </div>

    <form class="composer" @submit.prevent="submit">
      <textarea
        v-model="draft"
        rows="2"
        placeholder="输入消息，Enter 发送，Shift+Enter 换行"
        @keydown="onKeydown"
      />
      <button type="submit" :disabled="!canSend">发送</button>
    </form>
  </section>
</template>

<style scoped>
.chat {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  grid-template-columns: minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.35);
}

.chat-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.1rem 1.4rem;
  border-bottom: 1px solid var(--line);
}

.chat-head h2 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 1.35rem;
}

.chat-head p {
  margin: 0.2rem 0 0;
  color: var(--muted);
  font-size: 0.85rem;
}

.ghost {
  border: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.8);
  border-radius: 999px;
  padding: 0.45rem 0.9rem;
  color: var(--ink-soft);
  font-weight: 600;
}

.messages {
  min-height: 0;
  overflow: auto;
  padding: 1.25rem 1.4rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.empty {
  margin: auto;
  text-align: center;
  color: var(--muted);
  max-width: 22rem;
}

.bubble-row {
  display: flex;
  gap: 0.7rem;
  max-width: min(720px, 92%);
  animation: rise 0.28s ease;
}

.bubble-row.user {
  align-self: flex-end;
  flex-direction: row-reverse;
}

.bubble-wrap {
  min-width: 0;
}

.meta {
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
  margin-bottom: 0.25rem;
}

.bubble-row.user .meta {
  justify-content: flex-end;
}

.meta strong {
  font-size: 0.82rem;
}

.meta time {
  font-size: 0.72rem;
  color: var(--muted);
}

.bubble {
  padding: 0.7rem 0.9rem;
  border-radius: 16px 16px 16px 4px;
  background: #fff;
  box-shadow: 0 6px 18px rgba(20, 34, 31, 0.05);
  border: 1px solid rgba(20, 34, 31, 0.05);
}

.bubble-row.user .bubble {
  border-radius: 16px 16px 4px 16px;
  background: linear-gradient(145deg, #0f766e, #0d9488);
  color: #fff;
  border: 0;
}

.bubble.error {
  background: #fff5f4;
  color: var(--danger);
}

.bubble pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  line-height: 1.5;
}

.typing {
  display: inline-flex;
  gap: 0.15rem;
  color: var(--muted);
}

.typing::after {
  content: '...';
  animation: dots 1.2s steps(4, end) infinite;
}

.caret {
  display: inline-block;
  width: 2px;
  height: 1em;
  margin-left: 2px;
  background: var(--teal);
  vertical-align: text-bottom;
  animation: blink 1s step-end infinite;
}

.composer {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.7rem;
  padding: 1rem 1.4rem 1.25rem;
  border-top: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.55);
  flex-shrink: 0;
}

.composer textarea {
  resize: none;
  border: 1px solid var(--line);
  border-radius: 14px;
  padding: 0.75rem 0.9rem;
  background: #fff;
  outline: none;
}

.composer textarea:focus {
  border-color: rgba(13, 148, 136, 0.5);
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.12);
}

.composer button {
  align-self: end;
  border: 0;
  border-radius: 14px;
  padding: 0.75rem 1.2rem;
  background: var(--teal);
  color: #fff;
  font-weight: 700;
}

.composer button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

@keyframes blink {
  50% {
    opacity: 0;
  }
}

@keyframes dots {
  0% {
    content: '';
  }
  25% {
    content: '.';
  }
  50% {
    content: '..';
  }
  75% {
    content: '...';
  }
}
</style>
