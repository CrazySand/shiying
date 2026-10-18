<script setup lang="ts">
import { ref } from 'vue'
import { FolderOpen, X, Film, Download, Trash2 } from '@lucide/vue'
import { ElButton, ElEmpty, ElImage, ElMessageBox, ElProgress, ElScrollbar, ElTag } from 'element-plus'
import type { DownloadTask } from '../types'

defineProps<{ tasks: DownloadTask[]; emptyLabel: string; removing: string[] }>()
const emit = defineEmits<{ cancel: [id: string]; reveal: [id: string]; remove: [id: string] }>()
const confirming = ref(false)
async function confirmRemove(id: string) {
  if (confirming.value) return
  confirming.value = true
  try {
    await ElMessageBox.confirm('删除此记录？视频文件会保留。', '删除下载记录', {
      type: 'warning',
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      confirmButtonClass: 'el-button--danger',
      closeOnClickModal: false,
    })
  } catch {
    return
  } finally {
    confirming.value = false
  }
  emit('remove', id)
}
const labels = { queued: '排队中', running: '下载中', completed: '已完成', failed: '失败', cancelled: '已取消' }
const tagTypes: Record<DownloadTask['status'], 'primary' | 'success' | 'warning' | 'danger' | 'info'> = { queued: 'warning', running: 'primary', completed: 'success', failed: 'danger', cancelled: 'info' }
function bytes(value: number): string {
  if (value < 1024) return value + ' B'
  if (value < 1024 ** 2) return (value / 1024).toFixed(1) + ' KB'
  return (value / 1024 ** 2).toFixed(1) + ' MB'
}
const percentage = (task: DownloadTask) => task.total ? Math.max(0, Math.min(100, task.downloaded / task.total * 100)) : 40
</script>
<template>
  <ElEmpty v-if="!tasks.length" class="empty-state" :description="emptyLabel" :image-size="43"><template #image><div class="empty-icon"><Download :size="23"/></div></template></ElEmpty>
  <ElScrollbar v-else class="task-list" max-height="300px" wrap-class="list-scroll-wrap">
    <article v-for="task in tasks" :key="task.id" class="task-row">
      <ElImage class="task-thumbnail" :src="task.thumbnail || undefined" fit="cover" alt="视频预览图" loading="lazy">
        <template #placeholder><div class="task-thumbnail-fallback"><Film :size="21"/></div></template>
        <template #error><div class="task-thumbnail-fallback"><Film :size="21"/></div></template>
      </ElImage>
      <div class="task-main">
        <div class="task-heading"><strong :title="task.title || task.filename">{{ task.title || task.filename }}</strong></div>
        <div v-if="task.title && task.title !== task.filename" class="task-filename" :title="task.filename">{{ task.filename }}</div>
        <ElProgress v-if="task.status === 'running'" class="task-progress" aria-label="下载进度" :percentage="percentage(task)" :show-text="false" :stroke-width="4" :indeterminate="!task.total" :aria-valuenow="task.total ? Math.round(percentage(task)) : undefined"/>
        <p v-if="task.error" class="task-error">{{ task.error }}</p>
        <p v-else-if="task.status === 'running'">{{ bytes(task.downloaded) }}{{ task.total ? ' / ' + bytes(task.total) : '' }}<span class="speed">{{ bytes(task.speed) }}/s</span></p>
        <p v-else-if="task.status === 'completed'" :title="task.path">{{ bytes(task.downloaded) }}</p>
        <p v-else>{{ task.status === 'queued' ? '等待下载' : '可重新添加下载' }}</p>
      </div>
      <div class="task-aside">
        <ElTag class="task-status" :type="tagTypes[task.status]" effect="light" :disable-transitions="true">{{ labels[task.status] }}</ElTag>
        <div class="task-actions">
      <ElButton v-if="task.status === 'completed'" text class="icon-button reveal-button" aria-label="在文件夹中显示" title="在文件夹中显示" @click="$emit('reveal', task.id)"><FolderOpen :size="18"/></ElButton>
      <ElButton v-if="task.status === 'queued' || task.status === 'running'" text class="icon-button" aria-label="取消下载" title="取消下载" @click="$emit('cancel', task.id)"><X :size="18"/></ElButton>
      <ElButton v-if="!['queued', 'running'].includes(task.status)" text class="icon-button delete-task-button" aria-label="删除下载记录" title="删除记录，保留文件" :disabled="removing.includes(task.id) || confirming" @click="confirmRemove(task.id)"><Trash2 :size="17"/></ElButton>
        </div>
      </div>
    </article>
  </ElScrollbar>
</template>
