<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { ElButton, ElInput, type InputInstance } from 'element-plus'
import { ArrowRight, Link2, LoaderCircle, X } from '@lucide/vue'

const props = defineProps<{ busy: boolean }>()
const emit = defineEmits<{ parse: [input: string] }>()
const input = ref('')
const inputElement = ref<InputInstance>()
async function clear() {
  input.value = ''
  await nextTick()
  inputElement.value?.focus()
}
function submit() { if (!props.busy && input.value.trim()) emit('parse', input.value.trim()) }
</script>
<template>
  <form class="link-form" :class="{ 'is-busy': busy }" @submit.prevent="submit">
    <ElInput ref="inputElement" v-model="input" class="link-input" aria-label="X 视频链接或推文 ID" placeholder="粘贴 X 链接或推文 ID" :disabled="busy" autocomplete="off" spellcheck="false">
      <template #prefix><Link2 :size="18" class="muted"/></template>
      <template #suffix><ElButton v-if="input && !busy" text class="icon-button clear-input" aria-label="清除链接" title="清除" @click="clear"><X :size="15"/></ElButton></template>
    </ElInput>
    <ElButton type="primary" native-type="submit" class="parse-button" aria-label="解析" :title="busy ? '正在解析' : '解析视频'" :aria-busy="busy" :loading="busy" :loading-icon="LoaderCircle" :disabled="!input.trim()">
      <ArrowRight v-if="!busy" :size="19" :stroke-width="2"/>
    </ElButton>
  </form>
</template>
