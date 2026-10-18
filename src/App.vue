<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { ElButton, ElConfigProvider, ElContainer, ElHeader, ElMain, ElMessage, ElScrollbar, ElSegmented, ElSkeleton, ElSkeletonItem, type MessageHandler } from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import { Aperture, Settings2, ArrowDownToLine, FolderOpen } from '@lucide/vue'
import LinkInput from './components/LinkInput.vue'
import VideoCard from './components/VideoCard.vue'
import DownloadList from './components/DownloadList.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import { useDownloads } from './composables/useDownloads'
import { api, desktop } from './api'
import type { SettingsView, Video } from './types'

const settings = ref<SettingsView>({ downloadDir: '', dataDir: '', queryId: '6uCvnic3m5reVuehkvHa3w', cookie: '', authorization: '', hasCookie: false, hasAuthorization: false })
const showSettings = ref(false)
const parsing = ref(false)
const videos = ref<Video[]>([])
const adding = ref<string[]>([])

const pendingInput = ref('')
const filter = ref<'all' | 'active' | 'completed'>('all')
let notification: MessageHandler | undefined
function notify(message: string, type: 'success' | 'info' | 'warning' | 'error' = 'success') {
  notification?.close()
  notification = ElMessage({ message, type, duration: type === 'error' ? 4000 : 2600, customClass: 'app-notification' })
}
onUnmounted(() => notification?.close())
const report = (message: string, type: 'warning' | 'error' = 'error') => notify(message, type)
const { tasks, cancel, reveal, remove, removing } = useDownloads(report, () => notify('已删除下载记录'))
const activeCount = computed(() => tasks.value.filter(t => t.status === 'queued' || t.status === 'running').length)
const filtered = computed(() => tasks.value.filter(t => filter.value === 'all' || (filter.value === 'active' ? ['queued', 'running'].includes(t.status) : t.status === 'completed')))
const filterOptions = computed(() => [
  { label: tasks.value.length ? '全部 ' + tasks.value.length : '全部', value: 'all' },
  { label: '进行中', value: 'active' },
  { label: '已完成', value: 'completed' },
])
const emptyLabel = computed(() => filter.value === 'active' ? '没有进行中的下载' : filter.value === 'completed' ? '没有已完成的下载' : '暂无下载')
onMounted(async () => {
  if (desktop) { try { settings.value = await api.settings() } catch (e) { report(String(e)) } }
})
async function parse(input: string) {
  if (parsing.value) return
  notification?.close()
  if (!desktop) { notify('当前为界面预览，请使用桌面应用解析下载。', 'info'); return }
  if (!settings.value.hasCookie || !settings.value.hasAuthorization) { pendingInput.value = input; showSettings.value = true; notify('请先填写 Cookie 和 Authorization。', 'warning'); return }
  parsing.value = true
  videos.value = []
  try { videos.value = await api.parse(input) } catch (e) { report(String(e)) } finally { parsing.value = false }
}
async function download(video: Video, variantIndex: number) {
  if (adding.value.includes(video.id)) return
  adding.value.push(video.id)
  try { await api.download(video, variantIndex); notify('已加入下载'); filter.value = 'all' }
  catch (e) { report(String(e)) } finally { adding.value = adding.value.filter(id => id !== video.id) }
}
function saved(value: SettingsView) {
  settings.value = value; notify('已保存')
  if (pendingInput.value && value.hasCookie && value.hasAuthorization) {
    const input = pendingInput.value
    pendingInput.value = ''
    void parse(input)
  }
}
function closeSettings() { showSettings.value = false; pendingInput.value = '' }
</script>
<template>
  <ElConfigProvider :locale="zhCn" size="small">
  <ElScrollbar class="page-scrollbar" height="100vh" wrap-class="page-scroll-wrap">
  <ElContainer direction="vertical" class="app-shell">
    <ElHeader class="app-header">
      <div class="brand"><span class="brand-mark"><Aperture :size="22" :stroke-width="1.8"/></span><h1>拾影</h1></div>
      <ElButton text class="icon-button settings-button" aria-label="设置" title="设置" :aria-expanded="showSettings" @click="showSettings = true"><Settings2 :size="19"/></ElButton>
    </ElHeader>
    <ElMain class="app-main">
      <section class="input-section" aria-label="解析视频"><LinkInput :busy="parsing" @parse="parse"/></section>
      <div v-if="!desktop" class="preview-banner">界面预览</div>
      <section v-if="parsing || videos.length" class="results-section" aria-label="解析结果" :aria-busy="parsing">
        <ElSkeleton v-if="parsing" animated class="video-loading" role="status" aria-label="正在获取视频">
          <template #template><div class="loading-preview"><ElSkeletonItem variant="image" class="loading-thumbnail"/><div class="loading-details"><ElSkeletonItem variant="text"/><ElSkeletonItem variant="text" class="loading-short"/><ElSkeletonItem variant="button"/></div></div></template>
        </ElSkeleton>
        <template v-else><ElScrollbar class="video-list" max-height="260px" wrap-class="list-scroll-wrap"><VideoCard v-for="video in videos" :key="video.id" :video="video" :busy="adding.includes(video.id)" @download="download(video, $event)" @dismiss="videos = videos.filter(item => item.id !== video.id)"/></ElScrollbar></template>
      </section>
      <section class="downloads-section" :class="{ 'has-tasks': tasks.length > 0 }" aria-label="下载列表">
        <div class="section-heading"><h2><ArrowDownToLine :size="17"/>下载<span v-if="activeCount" class="count-badge">{{ activeCount }}</span></h2>
          <ElSegmented v-model="filter" class="download-filters" :options="filterOptions" aria-label="筛选下载任务"/>
        </div>
        <DownloadList :tasks="filtered" :empty-label="emptyLabel" :removing="removing" @cancel="cancel" @reveal="reveal" @remove="remove"/>
        <div class="download-footer" :title="settings.downloadDir"><FolderOpen :size="15"/><span>{{ settings.downloadDir || '尚未设置保存位置' }}</span></div>
      </section>
    </ElMain>

  </ElContainer>
  </ElScrollbar>
  <SettingsPanel v-if="showSettings" :settings="settings" @close="closeSettings" @saved="saved"/>
  </ElConfigProvider>
</template>
