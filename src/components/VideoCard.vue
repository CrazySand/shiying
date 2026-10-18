<script setup lang="ts">
import { ref, watch } from 'vue'
import { ElButton, ElOption, ElSelect } from 'element-plus'
import { Check, Download, Film, LoaderCircle, X } from '@lucide/vue'
import type { Video } from '../types'

const props = defineProps<{ video: Video; busy: boolean }>()
defineEmits<{ download: [index: number]; dismiss: [] }>()
const selected = ref(0)
const imageFailed = ref(false)
watch(() => props.video, () => { selected.value = 0; imageFailed.value = false })
const quality = (label: string) => label.replace(/\s*·\s*推荐$/, '')
const recommended = (label: string) => /·\s*推荐$/.test(label)
</script>
<template>
  <article class="video-card">
    <ElButton text class="icon-button video-close" aria-label="移除此解析结果" title="移除结果" @click="$emit('dismiss')"><X :size="15"/></ElButton>
    <div class="thumbnail">
      <img v-if="video.thumbnail && !imageFailed" :src="video.thumbnail" alt="视频封面" @error="imageFailed = true"/>
      <Film v-else :size="30"/><span class="thumbnail-label">MP4</span>
    </div>
    <div class="video-details">
      <h3>{{ video.title }}</h3>
      <div class="video-actions">
        <ElSelect v-model="selected" class="quality-select" aria-label="下载清晰度" popper-class="quality-dropdown" placement="bottom-start" :show-arrow="false" :disabled="busy || !video.variants.length">
          <ElOption v-for="(variant, index) in video.variants" :key="variant.url" :value="index" :label="variant.label">
            <span class="quality-option"><span>{{ quality(variant.label) }}</span><span v-if="recommended(variant.label)" class="quality-recommended">推荐</span><Check v-if="selected === index" :size="14" class="quality-check"/></span>
          </ElOption>
        </ElSelect>
        <ElButton type="primary" :loading="busy" :loading-icon="LoaderCircle" :disabled="!video.variants.length" @click="$emit('download', selected)">
          <Download v-if="!busy" :size="14"/><span>{{ busy ? '添加中' : '下载' }}</span>
        </ElButton>
      </div>
    </div>
  </article>
</template>
