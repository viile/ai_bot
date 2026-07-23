<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import type { Bot } from '../types'
import { randomAvatar } from '../avatarGen'
import {
  PERSONA_TAG_GROUPS,
  composePersonaFromTags,
  type PersonaTag,
} from '../personaTags'
import AvatarBadge from './AvatarBadge.vue'
import AvatarPicker from './AvatarPicker.vue'

const props = defineProps<{
  bots: Bot[]
  open: boolean
}>()

const emit = defineEmits<{
  close: []
  create: [data: { nickname: string; avatar: string; persona: string; model: string | null }]
  update: [
    botId: string,
    data: { nickname: string; avatar: string; persona: string; model: string | null },
  ]
  remove: [botId: string]
}>()

const editingId = ref<string | null>(null)
const selectedTags = ref<Set<string>>(new Set())
const form = reactive({
  nickname: '',
  avatar: randomAvatar(),
  persona: '',
  model: '',
})

function resetForm() {
  editingId.value = null
  selectedTags.value = new Set()
  form.nickname = ''
  form.avatar = randomAvatar()
  form.persona = ''
  form.model = ''
}

watch(
  () => props.open,
  (v) => {
    if (v) resetForm()
  },
)

function startEdit(bot: Bot) {
  editingId.value = bot.id
  selectedTags.value = new Set()
  form.nickname = bot.nickname
  form.avatar = bot.avatar
  form.persona = bot.persona
  form.model = bot.model || ''
}

const selectedList = computed(() => [...selectedTags.value])

function isSelected(id: string) {
  return selectedTags.value.has(id)
}

function syncPersonaFromTags() {
  const composed = composePersonaFromTags(selectedTags.value)
  if (composed) {
    form.persona = composed
  }
}

function toggleTag(groupExclusive: boolean | undefined, groupTags: PersonaTag[], tag: PersonaTag) {
  const next = new Set(selectedTags.value)
  if (next.has(tag.id)) {
    next.delete(tag.id)
  } else {
    if (groupExclusive) {
      for (const t of groupTags) next.delete(t.id)
    }
    next.add(tag.id)
  }
  selectedTags.value = next
  syncPersonaFromTags()
}

function clearTags() {
  selectedTags.value = new Set()
}

function submit() {
  const payload = {
    nickname: form.nickname.trim() || '机器人',
    avatar: form.avatar,
    persona: form.persona.trim() || '你是一个友好的助手。',
    model: form.model.trim() || null,
  }
  if (editingId.value) {
    emit('update', editingId.value, payload)
  } else {
    emit('create', payload)
  }
  resetForm()
}
</script>

<template>
  <aside class="panel" :class="{ open }">
    <header>
      <h3>群内机器人</h3>
      <button type="button" class="icon" @click="emit('close')">×</button>
    </header>

    <ul class="bot-list">
      <li v-for="bot in bots" :key="bot.id">
        <button type="button" class="bot-row" @click="startEdit(bot)">
          <AvatarBadge :name="bot.nickname" :color-or-url="bot.avatar" :size="40" />
          <div>
            <strong>{{ bot.nickname }}</strong>
            <p>{{ bot.persona }}</p>
          </div>
        </button>
        <button type="button" class="remove" @click="emit('remove', bot.id)">删除</button>
      </li>
    </ul>

    <p v-if="!bots.length" class="hint">还没有机器人。添加后，你的消息会同步给每一位。</p>

    <form class="form" @submit.prevent="submit">
      <h4>{{ editingId ? '编辑机器人' : '添加机器人' }}</h4>

      <label>
        昵称
        <input v-model="form.nickname" maxlength="24" placeholder="例如：产品经理小周" />
      </label>

      <AvatarPicker v-model="form.avatar" :name="form.nickname || '机'" />

      <div class="tag-block">
        <div class="tag-head">
          <span>身份标签</span>
          <button
            v-if="selectedList.length"
            type="button"
            class="clear-tags"
            @click="clearTags"
          >
            清空标签
          </button>
        </div>
        <p class="tag-tip">点选后自动写入身份设定，仍可手动改文案。</p>
        <div v-for="group in PERSONA_TAG_GROUPS" :key="group.key" class="tag-group">
          <span class="tag-title">{{ group.title }}</span>
          <div class="tag-row">
            <button
              v-for="tag in group.tags"
              :key="tag.id"
              type="button"
              class="tag"
              :class="{ on: isSelected(tag.id) }"
              @click="toggleTag(group.exclusive, group.tags, tag)"
            >
              {{ tag.label }}
            </button>
          </div>
        </div>
      </div>

      <label>
        身份设定
        <textarea
          v-model="form.persona"
          rows="5"
          placeholder="描述这个角色是谁、说话风格、擅长什么…也可先点上方标签"
        />
      </label>

      <label>
        模型（可选）
        <input v-model="form.model" placeholder="留空则用默认 CURSOR_MODEL" />
      </label>

      <div class="actions">
        <button v-if="editingId" type="button" class="ghost" @click="resetForm">取消编辑</button>
        <button type="submit">{{ editingId ? '保存' : '添加进群' }}</button>
      </div>
    </form>
  </aside>
</template>

<style scoped>
.panel {
  width: 0;
  overflow: hidden;
  border-left: 0;
  background: rgba(255, 255, 255, 0.72);
  backdrop-filter: blur(12px);
  transition: width 0.28s ease;
  min-height: 0;
  height: 100%;
}

.panel.open {
  width: min(380px, 94vw);
  border-left: 1px solid var(--line);
  overflow: auto;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.1rem 1.1rem 0.5rem;
}

header h3 {
  margin: 0;
  font-family: var(--font-display);
}

.icon {
  border: 0;
  background: transparent;
  font-size: 1.4rem;
  color: var(--muted);
}

.bot-list {
  list-style: none;
  margin: 0;
  padding: 0.5rem 1rem;
}

.bot-list li {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.4rem;
  align-items: center;
  margin-bottom: 0.45rem;
}

.bot-row {
  display: flex;
  gap: 0.65rem;
  align-items: flex-start;
  text-align: left;
  border: 0;
  background: transparent;
  padding: 0.45rem;
  border-radius: 12px;
  color: inherit;
  min-width: 0;
}

.bot-row:hover {
  background: rgba(13, 148, 136, 0.08);
}

.bot-row strong {
  display: block;
  font-size: 0.92rem;
}

.bot-row p {
  margin: 0.15rem 0 0;
  font-size: 0.78rem;
  color: var(--muted);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.remove {
  border: 0;
  background: transparent;
  color: var(--danger);
  font-size: 0.8rem;
}

.hint {
  padding: 0 1.1rem;
  color: var(--muted);
  font-size: 0.85rem;
}

.form {
  padding: 0.5rem 1.1rem 1.4rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border-top: 1px solid var(--line);
  margin-top: 0.5rem;
}

.form h4 {
  margin: 0.4rem 0 0;
  font-size: 0.95rem;
}

label {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 0.8rem;
  color: var(--ink-soft);
  font-weight: 600;
}

input,
textarea {
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.55rem 0.7rem;
  background: #fff;
  font-weight: 400;
  color: var(--ink);
}

.avatar-field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  align-items: center;
}

.swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  padding: 0;
}

.swatch.active {
  border-color: var(--ink);
}

.upload {
  border: 1px dashed var(--line);
  border-radius: 999px;
  padding: 0.25rem 0.65rem;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
}

.avatar-field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  align-items: center;
}

.swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  padding: 0;
}

.swatch.active {
  border-color: var(--ink);
}

.upload {
  border: 1px dashed var(--line);
  border-radius: 999px;
  padding: 0.25rem 0.65rem;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
}

.tag-block {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.tag-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.clear-tags {
  border: 0;
  background: transparent;
  color: var(--muted);
  font-size: 0.75rem;
  padding: 0;
}

.tag-tip {
  margin: 0;
  font-size: 0.72rem;
  color: var(--muted);
  font-weight: 400;
}

.tag-group {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.tag-title {
  font-size: 0.72rem;
  color: var(--muted);
  font-weight: 600;
  letter-spacing: 0.04em;
}

.tag-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.tag {
  border: 1px solid var(--line);
  background: #fff;
  color: var(--ink-soft);
  border-radius: 999px;
  padding: 0.28rem 0.65rem;
  font-size: 0.75rem;
  font-weight: 600;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}

.tag:hover {
  border-color: rgba(13, 148, 136, 0.45);
}

.tag.on {
  background: var(--teal-soft);
  border-color: rgba(13, 148, 136, 0.45);
  color: var(--teal-deep);
}

.actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
}

.actions button {
  border: 0;
  border-radius: 10px;
  padding: 0.6rem 0.95rem;
  background: var(--teal);
  color: #fff;
  font-weight: 700;
}

.actions .ghost {
  background: transparent;
  color: var(--muted);
  font-weight: 600;
}
</style>
