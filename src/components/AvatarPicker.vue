<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  AVATAR_STYLES,
  avatarBatch,
  mixedAvatarBatch,
  randomAvatar,
  type AvatarStyleId,
} from '../avatarGen'
import AvatarBadge from './AvatarBadge.vue'

const props = defineProps<{
  modelValue: string
  name?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const styleId = ref<AvatarStyleId | 'mix'>('mix')
const batchKey = ref(Math.random().toString(36).slice(2, 8))

const options = computed(() => {
  if (styleId.value === 'mix') return mixedAvatarBatch(16, batchKey.value)
  return avatarBatch(styleId.value, 16, batchKey.value)
})

watch(
  () => props.modelValue,
  (v) => {
    if (!v) emit('update:modelValue', randomAvatar())
  },
  { immediate: true },
)

function reshuffle() {
  batchKey.value = Math.random().toString(36).slice(2, 10)
}

function pick(url: string) {
  emit('update:modelValue', url)
}

function surprise() {
  const url = randomAvatar(styleId.value === 'mix' ? undefined : styleId.value)
  emit('update:modelValue', url)
}

function onFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  if (file.size > 800_000) {
    alert('头像请小于 800KB')
    return
  }
  const reader = new FileReader()
  reader.onload = () => emit('update:modelValue', String(reader.result))
  reader.readAsDataURL(file)
}
</script>

<template>
  <div class="picker">
    <div class="picker-head">
      <span>头像</span>
      <AvatarBadge :name="name || '头'" :color-or-url="modelValue" :size="36" />
    </div>

    <div class="styles">
      <button
        type="button"
        class="style-chip"
        :class="{ on: styleId === 'mix' }"
        @click="styleId = 'mix'"
      >
        混合
      </button>
      <button
        v-for="s in AVATAR_STYLES"
        :key="s.id"
        type="button"
        class="style-chip"
        :class="{ on: styleId === s.id }"
        @click="styleId = s.id"
      >
        {{ s.label }}
      </button>
    </div>

    <div class="grid">
      <button
        v-for="(url, i) in options"
        :key="`${batchKey}-${i}`"
        type="button"
        class="opt"
        :class="{ active: modelValue === url }"
        @click="pick(url)"
      >
        <AvatarBadge :name="name || '头'" :color-or-url="url" :size="36" />
      </button>
    </div>

    <div class="actions">
      <button type="button" class="ghost" @click="reshuffle">换一批</button>
      <button type="button" class="ghost" @click="surprise">随机一个</button>
      <label class="upload">
        上传图片
        <input type="file" accept="image/*" hidden @change="onFile" />
      </label>
    </div>
  </div>
</template>

<style scoped>
.picker {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.picker-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.styles {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
}

.style-chip {
  border: 1px solid var(--line);
  background: #fff;
  color: var(--ink-soft);
  border-radius: 999px;
  padding: 0.22rem 0.55rem;
  font-size: 0.72rem;
  font-weight: 600;
}

.style-chip.on {
  background: var(--teal-soft);
  border-color: rgba(13, 148, 136, 0.45);
  color: var(--teal-deep);
}

.grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.35rem;
}

.opt {
  aspect-ratio: 1;
  border: 2px solid transparent;
  border-radius: 50%;
  padding: 0;
  overflow: hidden;
  background: transparent;
}

.opt.active {
  border-color: var(--ink);
  box-shadow: 0 0 0 2px rgba(13, 148, 136, 0.25);
}

.opt :deep(.avatar) {
  width: 100% !important;
  height: 100% !important;
  font-size: 0.7rem !important;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  align-items: center;
}

.ghost {
  border: 1px solid var(--line);
  background: #fff;
  border-radius: 999px;
  padding: 0.28rem 0.65rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--ink-soft);
}

.upload {
  border: 1px dashed var(--line);
  border-radius: 999px;
  padding: 0.28rem 0.65rem;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  color: var(--ink-soft);
}
</style>
