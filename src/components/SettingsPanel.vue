<script setup lang="ts">
import { reactive, ref } from 'vue'
import { ElMessage, ElButton, ElDialog, ElForm, ElFormItem, ElInput, ElSegmented, ElScrollbar, type FormInstance, type FormRules } from 'element-plus'
import { X, FolderOpen, LoaderCircle } from '@lucide/vue'
import { api } from '../api'
import type { Settings, SettingsView } from '../types'

const props = defineProps<{ settings: SettingsView }>()
const emit = defineEmits<{ close: []; saved: [value: SettingsView] }>()
const concurrencyOptions = Array.from({ length: 8 }, (_, index) => ({ label: String(index + 1), value: index + 1 }))
const visible = ref(true)
const formRef = ref<FormInstance>()
const form = reactive<Settings>({ downloadDir: props.settings.downloadDir, queryId: props.settings.queryId, maxConcurrentDownloads: props.settings.maxConcurrentDownloads ?? 2 })
const cookie = ref(props.settings.cookie || '')
const authorization = ref(props.settings.authorization || '')
const saving = ref(false)
const choosing = ref(false)

const rules: FormRules<Settings> = {
  downloadDir: [{ required: true, whitespace: true, message: '请选择保存位置', trigger: 'blur' }],
}
function close() { if (!saving.value && !choosing.value) visible.value = false }
function beforeClose(done: () => void) { if (!saving.value && !choosing.value) done() }
async function choose() {
  choosing.value = true
  try {
    const path = await api.pickDirectory()
    if (typeof path === 'string') { form.downloadDir = path; formRef.value?.clearValidate('downloadDir') }
  } catch (error) { ElMessage.error({ message: String(error), duration: 4000 }) }
  finally { choosing.value = false }
}
async function save() {
  if (saving.value || choosing.value) return
  form.downloadDir = form.downloadDir.trim()
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  saving.value = true
  try {
    emit('saved', await api.saveSettings({ ...form }, cookie.value.trim(), authorization.value.trim()))
    visible.value = false
  } catch (error) { ElMessage.error({ message: String(error), duration: 4000 }) }
  finally { saving.value = false }
}
</script>
<template>
  <ElDialog v-model="visible" title="拾影设置" class="settings-panel" modal-class="settings-overlay" width="460px" align-center :show-close="false" :before-close="beforeClose" :close-on-click-modal="!saving && !choosing" :close-on-press-escape="!saving && !choosing" @closed="emit('close')">
    <template #header="{ titleId }">
      <div class="settings-header"><h2 :id="titleId">拾影设置</h2><ElButton text class="icon-button" aria-label="关闭设置" :disabled="saving || choosing" @click="close"><X :size="19"/></ElButton></div>
    </template>
    <ElScrollbar max-height="calc(100vh - 180px)" wrap-class="settings-scroll-wrap" view-class="settings-content">

      <ElForm id="settings-form" ref="formRef" :model="form" :rules="rules" label-position="top" :disabled="saving" hide-required-asterisk @submit.prevent="save">
        <ElFormItem label="保存位置" prop="downloadDir" for="download-dir">
          <div class="directory-input"><ElInput id="download-dir" v-model="form.downloadDir" placeholder="选择下载目录"/><ElButton text class="icon-button" :loading="choosing" :loading-icon="LoaderCircle" :disabled="saving" title="选择文件夹" aria-label="选择下载目录" @click="choose"><FolderOpen v-if="!choosing" :size="18"/></ElButton></div>
        </ElFormItem>
        <ElFormItem label="数据目录" for="data-dir">
          <ElInput id="data-dir" :model-value="settings.dataDir" :title="settings.dataDir" readonly placeholder="在桌面应用中显示"/>
        </ElFormItem>
        <ElFormItem label="同时下载数量" for="max-concurrent-downloads">
          <div class="concurrency-control">
            <ElSegmented id="max-concurrent-downloads" v-model="form.maxConcurrentDownloads" class="concurrency-selector" :options="concurrencyOptions" aria-label="同时下载数量"/>
            <p class="concurrency-description">同时下载 {{ form.maxConcurrentDownloads }} 个视频，其余自动排队</p>
          </div>
        </ElFormItem>
        <ElFormItem for="cookie">
          <template #label>X 登录 Cookie</template>
          <ElInput id="cookie" v-model="cookie" class="credential-input" type="text" autocomplete="off" spellcheck="false" placeholder="粘贴包含 auth_token 和 ct0 的 Cookie"/>
        </ElFormItem>
        <ElFormItem for="authorization">
          <template #label>Authorization</template>
          <ElInput id="authorization" v-model="authorization" class="credential-input" type="text" autocomplete="off" spellcheck="false" placeholder="粘贴 Bearer Token 或 Bearer …"/>
        </ElFormItem>
      </ElForm>
    </ElScrollbar>
    <template #footer><ElButton :disabled="saving || choosing" @click="close">取消</ElButton><ElButton type="primary" native-type="submit" form="settings-form" :loading="saving" :loading-icon="LoaderCircle" :disabled="choosing">保存设置</ElButton></template>
  </ElDialog>
</template>
