<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { Bot, Message } from '../types'
import AvatarBadge from './AvatarBadge.vue'
import { insertMention, mentionDisplayNames, mentionQueryAt, splitMentions, everyoneMentionMatchesQuery, MENTION_EVERYONE_LABEL } from '../mentions'

const props = defineProps<{
  groupName: string
  messages: Message[]
  bots: Bot[]
  userNickname: string
  botCount: number
  sending: boolean
  showMembers: boolean
}>()

const emit = defineEmits<{
  send: [content: string]
  recall: [messageId: string]
  toggleMembers: []
}>()

const draft = ref('')
const scroller = ref<HTMLElement | null>(null)
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const mentionOpen = ref(false)
const mentionStart = ref(0)
const mentionQuery = ref('')
const mentionIndex = ref(0)

const knownNames = computed(() =>
  mentionDisplayNames(
    props.bots.map((b) => b.nickname),
    props.userNickname,
  ),
)

type MentionOption =
  | { kind: 'everyone'; label: string }
  | { kind: 'bot'; bot: Bot }

const mentionOptions = computed((): MentionOption[] => {
  const q = mentionQuery.value
  const ql = q.trim().toLowerCase()
  const opts: MentionOption[] = []
  for (const bot of props.bots) {
    if (!ql || bot.nickname.toLowerCase().includes(ql)) {
      opts.push({ kind: 'bot', bot })
    }
  }
  if (everyoneMentionMatchesQuery(q)) {
    opts.push({ kind: 'everyone', label: MENTION_EVERYONE_LABEL })
  }
  return opts.slice(0, 10)
})

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

watch(mentionOptions, (list) => {
  if (mentionIndex.value >= list.length) mentionIndex.value = Math.max(0, list.length - 1)
})

function partsFor(content: string) {
  return splitMentions(content, knownNames.value)
}

function syncMentionFromCaret() {
  const el = textareaRef.value
  if (!el) {
    mentionOpen.value = false
    return
  }
  const hit = mentionQueryAt(draft.value, el.selectionStart ?? draft.value.length)
  if (!hit) {
    mentionOpen.value = false
    return
  }
  const queryChanged = !mentionOpen.value || hit.start !== mentionStart.value || hit.query !== mentionQuery.value
  mentionOpen.value = true
  mentionStart.value = hit.start
  mentionQuery.value = hit.query
  // Only reset highlight when the @query itself changes — not on ArrowUp/Down keyup.
  if (queryChanged) mentionIndex.value = 0
}

function applyMentionNickname(nickname: string) {
  const el = textareaRef.value
  const caret = el?.selectionStart ?? draft.value.length
  const next = insertMention(draft.value, caret, mentionStart.value, nickname)
  draft.value = next.text
  mentionOpen.value = false
  void nextTick(() => {
    if (!textareaRef.value) return
    textareaRef.value.focus()
    textareaRef.value.setSelectionRange(next.caret, next.caret)
  })
}

function pickMentionOption(opt: MentionOption) {
  if (opt.kind === 'everyone') {
    applyMentionNickname(MENTION_EVERYONE_LABEL)
  } else {
    applyMentionNickname(opt.bot.nickname)
  }
}

function submit() {
  if (!canSend.value) return
  const text = draft.value.trim()
  draft.value = ''
  mentionOpen.value = false
  emit('send', text)
}

function onKeydown(e: KeyboardEvent) {
  if (mentionOpen.value && mentionOptions.value.length) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      mentionIndex.value = (mentionIndex.value + 1) % mentionOptions.value.length
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      mentionIndex.value =
        (mentionIndex.value - 1 + mentionOptions.value.length) % mentionOptions.value.length
      return
    }
    if (e.key === 'Enter' || e.key === 'Tab') {
      e.preventDefault()
      const opt = mentionOptions.value[mentionIndex.value]
      if (opt) pickMentionOption(opt)
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      mentionOpen.value = false
      return
    }
  }

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

function canRecall(m: Message) {
  return m.senderType === 'user' && m.status !== 'recalled'
}

function onRecall(id: string) {
  emit('recall', id)
}
</script>

<template>
  <section class="chat">
    <header class="chat-head">
      <div>
        <h2>{{ groupName }}</h2>
        <p>{{ botCount }} 个 Cursor 机器人在群里 · @点名 / @所有人</p>
      </div>
      <button type="button" class="ghost" @click="emit('toggleMembers')">
        {{ showMembers ? '收起成员' : '管理机器人' }}
      </button>
    </header>

    <div ref="scroller" class="messages">
      <div v-if="!messages.length" class="empty">
        <p>还没有消息。说点什么，用 @昵称 点名，或 @所有人 让全员回复。</p>
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
            <template v-if="m.status === 'recalled'">
              <span class="recalled">已撤回</span>
            </template>
            <template v-else-if="m.status === 'streaming' && !m.content">
              <span class="typing">正在输入</span>
            </template>
            <template v-else>
              <p class="msg-body">
                <template v-for="(part, i) in partsFor(m.content)" :key="i">
                  <span v-if="part.type === 'mention'" class="mention">@{{ part.value }}</span>
                  <template v-else>{{ part.value }}</template>
                </template>
              </p>
              <span v-if="m.status === 'streaming'" class="caret" />
            </template>
          </div>
          <button
            v-if="canRecall(m)"
            type="button"
            class="recall-btn"
            title="撤回并停止回复"
            @click="onRecall(m.id)"
          >
            撤回
          </button>
        </div>
      </article>
    </div>

    <form class="composer" @submit.prevent="submit">
      <div class="composer-field">
        <ul v-if="mentionOpen && mentionOptions.length" class="mention-menu" role="listbox">
          <li
            v-for="(opt, idx) in mentionOptions"
            :key="opt.kind === 'everyone' ? 'everyone' : opt.bot.id"
            :class="{ active: idx === mentionIndex }"
            role="option"
            @mousedown.prevent="pickMentionOption(opt)"
          >
            <template v-if="opt.kind === 'everyone'">
              <span class="mention-all-icon">@</span>
              <span>所有人</span>
              <em class="mention-all-hint">全员回复</em>
            </template>
            <template v-else>
              <AvatarBadge :name="opt.bot.nickname" :color-or-url="opt.bot.avatar" :size="22" />
              <span>{{ opt.bot.nickname }}</span>
            </template>
          </li>
        </ul>
        <textarea
          ref="textareaRef"
          v-model="draft"
          rows="2"
          placeholder="输入消息，@ 点名或 @所有人；Enter 发送"
          @keydown="onKeydown"
          @input="syncMentionFromCaret"
          @click="syncMentionFromCaret"
          @keyup="syncMentionFromCaret"
        />
      </div>
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
  position: relative;
}

.bubble-row:hover .recall-btn {
  opacity: 1;
}

.recall-btn {
  position: absolute;
  top: 0;
  border: 0;
  background: transparent;
  color: var(--muted);
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.1rem 0.35rem;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.bubble-row.user .recall-btn {
  left: -0.2rem;
  transform: translateX(-100%);
}

.bubble-row.bot .recall-btn {
  display: none;
}

.recalled {
  font-size: 0.85rem;
  color: var(--muted);
  font-style: italic;
}

.bubble.recalled {
  background: transparent;
  box-shadow: none;
  border: 1px dashed var(--line);
  padding: 0.45rem 0.75rem;
}

.bubble-row.user .bubble.recalled {
  background: transparent;
  color: var(--muted);
  border: 1px dashed rgba(255, 255, 255, 0.35);
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

.msg-body {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.5;
}

.mention {
  display: inline;
  font-weight: 700;
  color: var(--teal-deep);
  background: rgba(13, 148, 136, 0.12);
  border-radius: 6px;
  padding: 0.05em 0.28em;
}

.bubble-row.user .mention {
  color: #ecfdf5;
  background: rgba(255, 255, 255, 0.22);
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

.composer-field {
  position: relative;
  min-width: 0;
}

.mention-menu {
  position: absolute;
  left: 0;
  right: 0;
  bottom: calc(100% + 6px);
  z-index: 5;
  margin: 0;
  padding: 0.35rem;
  list-style: none;
  background: #fff;
  border: 1px solid var(--line);
  border-radius: 12px;
  box-shadow: 0 12px 28px rgba(20, 34, 31, 0.12);
  max-height: 220px;
  overflow: auto;
}

.mention-menu li {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.45rem 0.55rem;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.mention-menu li:hover,
.mention-menu li.active {
  background: rgba(13, 148, 136, 0.1);
  color: var(--teal-deep);
}

.mention-all-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--teal);
  color: #fff;
  font-size: 0.75rem;
  font-weight: 800;
}

.mention-all-hint {
  margin-left: auto;
  font-style: normal;
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--muted);
}

.composer textarea {
  width: 100%;
  resize: none;
  border: 1px solid var(--line);
  border-radius: 14px;
  padding: 0.75rem 0.9rem;
  background: #fff;
  outline: none;
  box-sizing: border-box;
}

.composer textarea:focus {
  border-color: rgba(13, 148, 136, 0.5);
  box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.12);
}

.composer > button {
  align-self: end;
  border: 0;
  border-radius: 14px;
  padding: 0.75rem 1.2rem;
  background: var(--teal);
  color: #fff;
  font-weight: 700;
}

.composer > button:disabled {
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
