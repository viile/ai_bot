<script setup lang="ts">
defineProps<{
  name: string
  colorOrUrl: string
  size?: number
}>()

function isUrl(v: string) {
  return v.startsWith('data:') || v.startsWith('http') || v.startsWith('/')
}

function initial(name: string) {
  const t = name.trim()
  return t ? t[0]!.toUpperCase() : '?'
}
</script>

<template>
  <span
    class="avatar"
    :style="{
      width: `${size || 36}px`,
      height: `${size || 36}px`,
      fontSize: `${Math.round((size || 36) * 0.42)}px`,
      background: isUrl(colorOrUrl) ? 'transparent' : colorOrUrl,
    }"
  >
    <img v-if="isUrl(colorOrUrl)" :src="colorOrUrl" :alt="name" />
    <span v-else>{{ initial(name) }}</span>
  </span>
</template>

<style scoped>
.avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: #fff;
  font-weight: 700;
  overflow: hidden;
  flex-shrink: 0;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.25);
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
