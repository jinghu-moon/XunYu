<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import type { ProxyConfig, ProxyItem, ProxyTestItem } from '../types'
import { fetchProxyConfig, fetchProxyStatus, proxyDel, proxySet, proxyTest, saveProxyConfig } from '../api'
import { IconCheck, IconRefresh, IconPlugConnected, IconPlugX } from '@tabler/icons-vue'
import { Button } from './button'

const items = ref<ProxyItem[]>([])
const cfg = ref<ProxyConfig>({})
const url = ref('')
const noproxy = ref('')
const only = ref('all')
const includeMsys2 = ref(true)
const testTargets = ref('proxy,8.8.8.8,1.1.1.1')
const testTimeoutMs = ref<number | null>(2000)
const testJobs = ref<number | null>(3)
const testResult = ref<ProxyTestItem[]>([])
const busy = ref(false)
const removeConfirmRemaining = ref(0)
let removeConfirmTimer: number | null = null

const CONFIRM_WINDOW_SEC = 3

function stopRemoveConfirmTimer() {
  if (removeConfirmTimer != null) {
    clearInterval(removeConfirmTimer)
    removeConfirmTimer = null
  }
}

function resetRemoveConfirm() {
  removeConfirmRemaining.value = 0
  stopRemoveConfirmTimer()
}

function armRemoveConfirm() {
  removeConfirmRemaining.value = CONFIRM_WINDOW_SEC
  stopRemoveConfirmTimer()
  removeConfirmTimer = window.setInterval(() => {
    removeConfirmRemaining.value -= 1
    if (removeConfirmRemaining.value <= 0) resetRemoveConfirm()
  }, 1000)
}

function isRemoveConfirmArmed() {
  return removeConfirmRemaining.value > 0
}

const currentDefaultUrl = computed(() => (cfg.value.defaultUrl || '').trim())
const currentNoProxy = computed(() => (cfg.value.noproxy || '').trim())
const activeProxyItems = computed(() => items.value.filter(p => p.status === 'ON' && p.address.trim()))
const matchCount = computed(() => activeProxyItems.value.filter(p => p.address.trim() === currentDefaultUrl.value).length)
const consistencyText = computed(() => {
  if (!currentDefaultUrl.value) return 'No defaultUrl'
  if (!activeProxyItems.value.length) return 'No active proxies'
  if (matchCount.value === activeProxyItems.value.length) return 'Consistent'
  return 'Drift'
})
const consistencyDetail = computed(() => {
  if (!currentDefaultUrl.value) return 'Save a defaultUrl to check consistency'
  if (!activeProxyItems.value.length) return 'Apply proxy to tools to check consistency'
  return `Matched ${matchCount.value}/${activeProxyItems.value.length} active tools`
})
const consistencyTone = computed(() => {
  if (!currentDefaultUrl.value || !activeProxyItems.value.length) return 'muted'
  if (matchCount.value === activeProxyItems.value.length) return 'ok'
  return 'warn'
})

const onlyValue = computed(() => {
  const v = only.value.trim().toLowerCase()
  if (!v || v === 'all') return includeMsys2.value ? undefined : 'cargo,git,npm'
  return includeMsys2.value ? `${v},msys2` : v
})

async function load() {
  items.value = await fetchProxyStatus()
  cfg.value = await fetchProxyConfig()
  url.value = cfg.value.defaultUrl || ''
  noproxy.value = cfg.value.noproxy || ''
}

async function onSaveConfig() {
  busy.value = true
  try {
    await saveProxyConfig({
      defaultUrl: url.value.trim() || null,
      noproxy: noproxy.value.trim() || null,
    })
    await load()
  } finally {
    busy.value = false
  }
}

async function onApply() {
  if (!url.value.trim()) {
    alert('Proxy URL is empty.')
    return
  }
  busy.value = true
  try {
    await proxySet(url.value.trim(), noproxy.value.trim(), onlyValue.value)
    await load()
  } finally {
    busy.value = false
  }
}

async function onRemove() {
  if (!isRemoveConfirmArmed()) {
    armRemoveConfirm()
    return
  }
  resetRemoveConfirm()
  busy.value = true
  try {
    await proxyDel(onlyValue.value)
    await load()
  } finally {
    busy.value = false
  }
}

async function onTest() {
  if (!url.value.trim()) {
    alert('Proxy URL is empty.')
    return
  }
  busy.value = true
  try {
    const timeoutMs =
      typeof testTimeoutMs.value === 'number' && Number.isFinite(testTimeoutMs.value) ? testTimeoutMs.value : undefined
    const jobs = typeof testJobs.value === 'number' && Number.isFinite(testJobs.value) ? testJobs.value : undefined
    testResult.value = await proxyTest(url.value.trim(), testTargets.value.trim() || undefined, { timeoutMs, jobs })
  } finally {
    busy.value = false
  }
}

onMounted(load)
onBeforeUnmount(stopRemoveConfirmTimer)
</script>

<template>
  <div>
    <div class="toolbar">
      <Button size="sm" preset="secondary" :disabled="busy" :loading="busy" @click="load" style="display:flex;align-items:center;gap:var(--space-1)">
        <IconRefresh :size="16" /> Refresh
      </Button>
      <div style="flex:1" />
      <Button size="sm" preset="primary" :disabled="busy" :loading="busy" style="display:flex;align-items:center;gap:var(--space-1)" @click="onSaveConfig">
        <IconCheck :size="16" /> Save config
      </Button>
    </div>

    <div class="config-summary">
      <div class="summary-item">
        <div class="label">Current defaultUrl</div>
        <div class="value">{{ currentDefaultUrl || '-' }}</div>
      </div>
      <div class="summary-item">
        <div class="label">Current noproxy</div>
        <div class="value">{{ currentNoProxy || '-' }}</div>
      </div>
      <div class="summary-item">
        <div class="label">Consistency</div>
        <div :class="['consistency', consistencyTone]">{{ consistencyText }}</div>
        <div class="summary-hint">{{ consistencyDetail }}</div>
      </div>
    </div>

    <div class="form">
      <div class="field">
        <div class="label">Default proxy URL</div>
        <input v-model="url" placeholder="http://127.0.0.1:7897" />
      </div>
      <div class="field">
        <div class="label">No proxy</div>
        <input v-model="noproxy" placeholder="localhost,127.0.0.1,::1,.local" />
      </div>
      <div class="field" style="max-width:260px">
        <div class="label">Apply to</div>
        <select v-model="only">
          <option value="all">all</option>
          <option value="cargo,git,npm">cargo,git,npm</option>
          <option value="cargo">cargo</option>
          <option value="git">git</option>
          <option value="npm">npm</option>
        </select>
        <label class="inline-toggle">
          <input v-model="includeMsys2" type="checkbox" />
          <span>Include MSYS2</span>
        </label>
      </div>
      <div class="actions">
        <Button size="sm" preset="primary" :disabled="busy" :loading="busy" style="display:flex;align-items:center;gap:var(--space-1)" @click="onApply">
          <IconPlugConnected :size="16" /> Apply
        </Button>
        <Button
          size="sm"
          preset="danger"
          class="btn--confirm"
          :disabled="busy"
          :loading="busy"
          :title="isRemoveConfirmArmed() ? `Confirm (${removeConfirmRemaining}s)` : 'Remove'"
          style="display:flex;align-items:center;gap:var(--space-1)"
          @click="onRemove"
        >
          <IconPlugX :size="16" /> Remove
          <span v-if="isRemoveConfirmArmed()" class="btn__confirm-badge">{{ removeConfirmRemaining }}</span>
        </Button>
      </div>
    </div>

    <div class="probe">
      <div class="label">Probe targets (csv)</div>
      <div class="probe-row">
        <input v-model="testTargets" class="probe-targets" placeholder="proxy,8.8.8.8,1.1.1.1" />
        <input
          v-model.number="testTimeoutMs"
          class="probe-num"
          type="number"
          min="100"
          max="30000"
          placeholder="timeout(ms)"
        />
        <input
          v-model.number="testJobs"
          class="probe-num"
          type="number"
          min="1"
          max="16"
          placeholder="jobs"
        />
        <Button size="sm" preset="secondary" :disabled="busy" :loading="busy" @click="onTest">Test</Button>
      </div>
      <div v-if="testResult.length" class="probeTable">
        <table>
          <thead>
            <tr>
              <th>Target</th>
              <th>Status</th>
              <th>Latency</th>
              <th>Error</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in testResult" :key="r.label">
              <td>{{ r.label }}</td>
              <td :style="{ color: r.ok ? 'var(--color-success)' : 'var(--text-secondary)' }">{{ r.ok ? 'OK' : 'FAIL' }}</td>
              <td>{{ r.ok ? `${r.ms}ms` : '-' }}</td>
              <td style="color:var(--text-tertiary)">{{ r.error || '-' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="cards">
      <div v-for="p in items" :key="p.tool" class="card">
        <div class="card-title">{{ p.tool }}</div>
        <div :class="['badge', p.status === 'ON' ? 'on' : 'off']">{{ p.status }}</div>
        <div class="addr">{{ p.address || '-' }}</div>
      </div>
      <div v-if="!items.length" style="color:var(--text-tertiary)">No proxy data</div>
    </div>
  </div>
</template>

<style scoped>
.form {
  display: grid;
  grid-template-columns: 1fr 1fr 260px auto;
  gap: var(--space-3);
  align-items: end;
  margin-bottom: var(--space-5);
}
.config-summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: var(--space-3);
  margin-bottom: var(--space-5);
}
.summary-item {
  border: var(--card-border);
  border-radius: var(--card-radius);
  padding: var(--card-padding-compact);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
}
.summary-item .label {
  font: var(--type-caption);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}
.summary-item .value {
  font: var(--type-body-sm);
  color: var(--text-primary);
  word-break: break-all;
}
.summary-hint {
  margin-top: var(--space-2);
  font: var(--type-caption);
  color: var(--text-tertiary);
}
.consistency {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  font: var(--type-caption);
  font-weight: var(--weight-semibold);
}
.consistency.ok {
  background: var(--color-success-bg);
  color: var(--color-success);
  border: 1px solid var(--color-success-bg);
}
.consistency.warn {
  background: var(--color-warning-bg);
  color: var(--color-warning);
  border: 1px solid var(--color-warning-bg);
}
.consistency.muted {
  background: var(--ds-background-2);
  color: var(--text-secondary);
  border: 1px solid var(--ds-background-2);
}
.field .label {
  font: var(--type-caption);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}
.actions {
  display: flex;
  gap: var(--space-2);
}
.inline-toggle {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-2);
  font: var(--type-caption);
  color: var(--text-secondary);
}

.probe {
  border: var(--card-border);
  border-radius: var(--card-radius);
  padding: var(--card-padding);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  margin-bottom: var(--space-5);
}
.probe .label {
  font: var(--type-caption);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}
.probe-row {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  flex-wrap: wrap;
}
.probe-targets {
  flex: 1 1 260px;
}
.probe-num {
  width: 120px;
}
.probeTable {
  margin-top: var(--space-4);
}

.cards {
  display: flex;
  gap: var(--space-4);
  flex-wrap: wrap;
}

.card {
  background: var(--surface-card-muted);
  border: var(--card-border);
  border-radius: var(--card-radius);
  padding: var(--card-padding);
  min-width: 180px;
  text-align: center;
  box-shadow: var(--card-shadow);
  transition: var(--transition-base);
}

.card:hover {
  transform: translateY(-2px);
  box-shadow: var(--card-shadow-hover);
  border: var(--card-border-strong);
}

.card-title {
  font: var(--type-title-sm);
  margin-bottom: var(--space-2);
  color: var(--text-primary);
}

.badge {
  display: inline-block;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  font: var(--type-caption);
  font-weight: var(--weight-semibold);
  margin-bottom: var(--space-2);
}

.badge.on {
  background: var(--color-success-bg);
  color: var(--color-success);
  border: 1px solid var(--color-success-bg);
}

.badge.off {
  background: var(--color-info-bg);
  color: var(--text-secondary);
  border: 1px solid var(--color-info-bg);
}

.addr {
  font: var(--type-body-sm);
  color: var(--text-secondary);
  word-break: break-all;
}
</style>
