<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { fetchRecentWorkspaceTasks, runWorkspaceTask } from '../api'
import type { RecentTaskRecord, WorkspaceCapabilities, WorkspaceTaskRunResponse } from '../types'
import { Button } from './button'

type FileVaultOperation = 'enc' | 'dec' | 'verify' | 'resume'

const props = withDefaults(
  defineProps<{
    path?: string
    capabilities?: WorkspaceCapabilities | null
  }>(),
  {
    path: '',
    capabilities: null,
  },
)

const form = reactive({
  operation: 'enc' as FileVaultOperation,
  inputPath: '',
  outputPath: '',
  password: '',
  keyfilePath: '',
  recoveryKey: '',
  recoveryKeyOutput: '',
  dpapi: false,
  algorithm: 'aes256-gcm',
  kdf: 'argon2id',
  chunkSize: '262144',
})

const taskState = reactive({
  loading: false,
  error: '',
  receipt: null as WorkspaceTaskRunResponse | null,
})

const diagnostics = reactive({
  loading: false,
  error: '',
  inspect: null as Record<string, any> | null,
  verify: null as Record<string, any> | null,
})

const recentEntries = ref<RecentTaskRecord[]>([])
const recentLoading = ref(false)
const recentError = ref('')
const cleanupConfirm = ref('')

const cryptAvailable = computed(() => props.capabilities?.crypt !== false)
const currentInput = computed(() => form.inputPath.trim())
const effectiveOutput = computed(() => {
  const manual = form.outputPath.trim()
  if (manual) return manual
  if (!currentInput.value) return ''
  if (form.operation === 'enc') return `${currentInput.value}.fv`
  if (form.operation === 'dec' && currentInput.value.endsWith('.fv')) return currentInput.value.slice(0, -3)
  return `${currentInput.value}.out`
})
const canSubmit = computed(() => {
  if (!cryptAvailable.value) return false
  if (!currentInput.value) return false
  if (form.operation === 'enc' || form.operation === 'dec') return Boolean(effectiveOutput.value)
  return true
})

watch(
  () => props.path,
  (value) => {
    const next = value.trim()
    if (next && !form.inputPath.trim()) {
      form.inputPath = next
    }
  },
  { immediate: true },
)

function parseJson(raw: string): Record<string, any> | null {
  const text = raw.trim()
  if (!text) return null
  try {
    return JSON.parse(text) as Record<string, any>
  } catch {
    return null
  }
}

function appendUnlockArgs(args: string[]) {
  if (form.password.trim()) args.push('--password', form.password.trim())
  if (form.keyfilePath.trim()) args.push('--keyfile', form.keyfilePath.trim())
  if (form.recoveryKey.trim()) args.push('--recovery-key', form.recoveryKey.trim())
  if (form.dpapi) args.push('--dpapi')
}

function buildVaultArgs(operation: FileVaultOperation): string[] {
  const args = ['vault', operation, currentInput.value]
  if (operation === 'enc') {
    args.push('-o', effectiveOutput.value)
    appendUnlockArgs(args)
    if (form.recoveryKeyOutput.trim()) args.push('--emit-recovery-key', form.recoveryKeyOutput.trim())
    args.push('--algo', form.algorithm, '--kdf', form.kdf, '--chunk-size', form.chunkSize)
  } else if (operation === 'dec') {
    args.push('-o', effectiveOutput.value)
    appendUnlockArgs(args)
  } else if (operation === 'verify') {
    appendUnlockArgs(args)
  } else if (operation === 'resume') {
    appendUnlockArgs(args)
  }
  args.push('--json')
  return args
}

async function refreshRecent() {
  recentLoading.value = true
  recentError.value = ''
  try {
    const data = await fetchRecentWorkspaceTasks(12, 'files-security')
    recentEntries.value = data.entries.filter((entry) => entry.action.startsWith('filevault:'))
  } catch (error) {
    recentError.value = error instanceof Error ? error.message : '加载最近任务失败'
  } finally {
    recentLoading.value = false
  }
}

async function runOperation() {
  taskState.loading = true
  taskState.error = ''
  try {
    const receipt = await runWorkspaceTask({
      workspace: 'files-security',
      action: `filevault:${form.operation}`,
      target: currentInput.value,
      args: buildVaultArgs(form.operation),
    })
    taskState.receipt = receipt
    const payload = parseJson(receipt.process.stdout)
    if (form.operation === 'verify') diagnostics.verify = payload
    await refreshRecent()
    if (!receipt.process.success) {
      taskState.error = receipt.process.stderr || payload?.status || '任务失败'
    }
  } catch (error) {
    taskState.error = error instanceof Error ? error.message : '任务失败'
  } finally {
    taskState.loading = false
  }
}

async function runInspect() {
  diagnostics.loading = true
  diagnostics.error = ''
  try {
    const receipt = await runWorkspaceTask({
      workspace: 'files-security',
      action: 'filevault:inspect',
      target: currentInput.value,
      args: ['vault', 'inspect', currentInput.value, '--json'],
    })
    diagnostics.inspect = parseJson(receipt.process.stdout)
    await refreshRecent()
  } catch (error) {
    diagnostics.error = error instanceof Error ? error.message : '结构检查失败'
  } finally {
    diagnostics.loading = false
  }
}

async function runCleanup() {
  if (cleanupConfirm.value.trim() !== 'CLEANUP') {
    taskState.error = '请输入 CLEANUP 后再执行清理。'
    return
  }
  taskState.loading = true
  taskState.error = ''
  try {
    const receipt = await runWorkspaceTask({
      workspace: 'files-security',
      action: 'filevault:cleanup',
      target: currentInput.value,
      args: ['vault', 'cleanup', currentInput.value, '--json'],
    })
    taskState.receipt = receipt
    cleanupConfirm.value = ''
    await refreshRecent()
    if (!receipt.process.success) taskState.error = receipt.process.stderr || '清理失败'
  } catch (error) {
    taskState.error = error instanceof Error ? error.message : '清理失败'
  } finally {
    taskState.loading = false
  }
}

onMounted(() => {
  void refreshRecent()
})
</script>

<template>
  <section class="filevault-panel" data-testid="filevault-panel">
    <header class="filevault-panel__header">
      <div>
        <h3 class="filevault-panel__title">FileVault v13 Foundation</h3>
        <p class="filevault-panel__desc">统一任务表单覆盖 Enc / Dec / Verify / Resume，并把 Inspect / Verify 结果汇总到诊断面板。</p>
      </div>
      <div class="filevault-panel__header-actions">
        <Button preset="secondary" :disabled="!currentInput || diagnostics.loading" @click="runInspect">结构检查</Button>
        <Button preset="secondary" :disabled="recentLoading" @click="refreshRecent">刷新状态</Button>
      </div>
    </header>

    <p v-if="!cryptAvailable" class="filevault-panel__message">当前构建未启用 crypt 能力，FileVault 工作流不可用。</p>
    <template v-else>
      <div class="filevault-panel__grid">
        <article class="filevault-panel__card">
          <h4>统一任务表单</h4>
          <div class="filevault-panel__form-grid">
            <label><span>操作</span><select v-model="form.operation" data-testid="filevault-operation"><option value="enc">Enc</option><option value="dec">Dec</option><option value="verify">Verify</option><option value="resume">Resume</option></select></label>
            <label class="filevault-panel__field-wide"><span>输入路径</span><input v-model="form.inputPath" data-testid="filevault-input" placeholder="D:/data/file.txt 或 D:/data/file.fv" /></label>
            <label v-if="form.operation === 'enc' || form.operation === 'dec'" class="filevault-panel__field-wide"><span>输出路径</span><input v-model="form.outputPath" :placeholder="effectiveOutput" /></label>
            <label><span>Password</span><input v-model="form.password" type="password" placeholder="可选" /></label>
            <label><span>Keyfile</span><input v-model="form.keyfilePath" placeholder="D:/keys/slot.key" /></label>
            <label><span>Recovery Key</span><input v-model="form.recoveryKey" placeholder="可选" /></label>
            <label><span>DPAPI</span><input v-model="form.dpapi" type="checkbox" /></label>
            <label v-if="form.operation === 'enc'"><span>算法</span><select v-model="form.algorithm"><option value="aes256-gcm">AES-256-GCM</option><option value="xchacha20-poly1305">XChaCha20-Poly1305</option></select></label>
            <label v-if="form.operation === 'enc'"><span>KDF</span><select v-model="form.kdf"><option value="argon2id">Argon2id</option><option value="pbkdf2-sha256">PBKDF2-SHA256</option></select></label>
            <label v-if="form.operation === 'enc'"><span>Chunk Size</span><input v-model="form.chunkSize" inputmode="numeric" /></label>
            <label v-if="form.operation === 'enc'" class="filevault-panel__field-wide"><span>导出 Recovery Key</span><input v-model="form.recoveryKeyOutput" placeholder="D:/keys/recovery.txt" /></label>
          </div>
          <div class="filevault-panel__toolbar">
            <Button preset="primary" :disabled="!canSubmit || taskState.loading" data-testid="filevault-run" @click="runOperation">执行 {{ form.operation }}</Button>
            <span class="filevault-panel__hint">接收者选择统一收口在同一表单中，避免 1:1 CLI 镜像式分散操作。</span>
          </div>
          <p v-if="taskState.error" class="filevault-panel__message filevault-panel__message--error">{{ taskState.error }}</p>
        </article>

        <article class="filevault-panel__card" data-testid="filevault-diagnostics">
          <h4>诊断面板</h4>
          <p class="filevault-panel__hint">`inspect` 展示布局与 footer，`verify` 展示完整性状态。</p>
          <div class="filevault-panel__diag-grid">
            <div><strong>Inspect</strong><pre>{{ diagnostics.inspect ? JSON.stringify(diagnostics.inspect, null, 2) : '暂无 inspect 结果' }}</pre></div>
            <div><strong>Verify</strong><pre>{{ diagnostics.verify ? JSON.stringify(diagnostics.verify, null, 2) : '暂无 verify 结果' }}</pre></div>
          </div>
          <p v-if="diagnostics.error" class="filevault-panel__message filevault-panel__message--error">{{ diagnostics.error }}</p>
        </article>

        <article class="filevault-panel__card" data-testid="filevault-status-list">
          <h4>状态列表</h4>
          <p class="filevault-panel__hint">最近 12 条 FileVault 工作台任务回执。</p>
          <div v-if="recentEntries.length" class="filevault-panel__status-list">
            <div v-for="entry in recentEntries" :key="entry.id" class="filevault-panel__status-item">
              <strong>{{ entry.action }}</strong><span>{{ entry.status }}</span><span>{{ entry.target || '-' }}</span><span>{{ entry.process.duration_ms }} ms</span>
            </div>
          </div>
          <p v-else-if="recentError" class="filevault-panel__message filevault-panel__message--error">{{ recentError }}</p>
          <p v-else class="filevault-panel__message">暂无 FileVault 任务记录。</p>
        </article>

        <article class="filevault-panel__card filevault-panel__card--danger" data-testid="filevault-danger-zone">
          <h4>危险操作确认</h4>
          <p class="filevault-panel__hint">清理会删除 `.fvtmp` / `.fvjournal` 临时工件。输入确认词后才能执行。</p>
          <label class="filevault-panel__field-wide"><span>确认词</span><input v-model="cleanupConfirm" placeholder="输入 CLEANUP" /></label>
          <div class="filevault-panel__toolbar">
            <Button preset="danger" :disabled="!currentInput || taskState.loading" data-testid="filevault-cleanup" @click="runCleanup">执行 Cleanup</Button>
          </div>
        </article>
      </div>
    </template>
  </section>
</template>

<style scoped>
.filevault-panel { border: var(--card-border); border-radius: var(--card-radius); background: var(--surface-card); box-shadow: var(--card-shadow); padding: var(--card-padding); display: flex; flex-direction: column; gap: var(--space-4); }
.filevault-panel__header, .filevault-panel__toolbar, .filevault-panel__header-actions { display: flex; gap: var(--space-3); align-items: center; justify-content: space-between; }
.filevault-panel__header-actions { justify-content: flex-end; }
.filevault-panel__title { font: var(--type-title-sm); }
.filevault-panel__desc, .filevault-panel__hint { color: var(--text-secondary); font: var(--type-body-sm); }
.filevault-panel__grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-4); }
.filevault-panel__card { border: var(--border); border-radius: var(--radius-md); padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-3); }
.filevault-panel__card--danger { border-color: rgba(220, 38, 38, 0.35); }
.filevault-panel__form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-3); }
.filevault-panel__field-wide { grid-column: 1 / -1; }
.filevault-panel__form-grid label { display: flex; flex-direction: column; gap: var(--space-1); font: var(--type-body-sm); }
.filevault-panel__diag-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-3); }
.filevault-panel pre { margin: 0; padding: var(--space-3); border-radius: var(--radius-sm); background: var(--surface-page); overflow: auto; font: var(--type-code-sm, var(--type-body-sm)); white-space: pre-wrap; }
.filevault-panel__status-list { display: flex; flex-direction: column; gap: var(--space-2); }
.filevault-panel__status-item { display: grid; grid-template-columns: 1.1fr .6fr 1.6fr .5fr; gap: var(--space-2); font: var(--type-body-sm); }
.filevault-panel__message { color: var(--text-secondary); font: var(--type-body-sm); }
.filevault-panel__message--error { color: var(--color-danger-600, #dc2626); }
@media (max-width: 1200px) { .filevault-panel__grid, .filevault-panel__diag-grid, .filevault-panel__form-grid { grid-template-columns: 1fr; } .filevault-panel__status-item { grid-template-columns: 1fr; } }
</style>
