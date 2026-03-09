<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import type { PortInfo } from '../types'
import { fetchPorts, killPid } from '../api'
import { IconRefresh, IconTimelineEventX } from '@tabler/icons-vue'
import { Button } from './button'
import SkeletonTable from './SkeletonTable.vue'
import { pushToast } from '../ui/feedback'
import { downloadCsv, downloadJson } from '../ui/export'

const props = withDefaults(defineProps<{
  disableKill?: boolean
}>(), {
  disableKill: false,
})

const tcp = ref<PortInfo[]>([])
const udp = ref<PortInfo[]>([])
const busy = ref(false)
const devOnly = ref(false)
const groupByPid = ref(false)
const protocolFilter = ref<'all' | 'tcp' | 'udp'>('all')
const processFilter = ref('')
const autoRefreshMs = ref(0)
const killBusyPid = ref<number | null>(null)
const killConfirmKey = ref<string | null>(null)
const killConfirmRemaining = ref(0)
let killConfirmTimer: number | null = null

const DEV_RANGE = [3000, 9999]
const ICON_SIZE = 18
let autoTimer: number | null = null
const CONFIRM_WINDOW_SEC = 3
const killConfirmKeyFor = (pid: number) => `pid:${pid}`

function stopKillConfirmTimer() {
  if (killConfirmTimer != null) {
    clearInterval(killConfirmTimer)
    killConfirmTimer = null
  }
}

function resetKillConfirm() {
  killConfirmKey.value = null
  killConfirmRemaining.value = 0
  stopKillConfirmTimer()
}

function armKillConfirm(key: string) {
  killConfirmKey.value = key
  killConfirmRemaining.value = CONFIRM_WINDOW_SEC
  stopKillConfirmTimer()
  killConfirmTimer = window.setInterval(() => {
    killConfirmRemaining.value -= 1
    if (killConfirmRemaining.value <= 0) resetKillConfirm()
  }, 1000)
}

function isKillConfirmArmed(key: string) {
  return killConfirmKey.value === key && killConfirmRemaining.value > 0
}

const filtered = computed(() => {
  let list = [...tcp.value, ...udp.value]
  if (devOnly.value) {
    list = list.filter(p => p.port >= DEV_RANGE[0] && p.port <= DEV_RANGE[1])
  }
  if (protocolFilter.value !== 'all') {
    list = list.filter(p => p.protocol.toLowerCase() === protocolFilter.value)
  }
  const q = processFilter.value.trim().toLowerCase()
  if (q) {
    list = list.filter(p => {
      const name = p.name.toLowerCase()
      const pid = String(p.pid)
      const path = (p.exe_path || '').toLowerCase()
      const cmdline = (p.cmdline || '').toLowerCase()
      const cwd = (p.cwd || '').toLowerCase()
      return name.includes(q) || pid.includes(q) || path.includes(q) || cmdline.includes(q) || cwd.includes(q)
    })
  }
  return list.sort((a, b) => a.port - b.port)
})

const isLoading = computed(() => busy.value && !tcp.value.length && !udp.value.length)
const skeletonColumns = computed(() => (groupByPid.value ? 3 : 5))

const grouped = computed(() => {
  const map = new Map<number, { pid: number; name: string; exe_path: string; cmdline: string; cwd: string; items: PortInfo[] }>()
  for (const p of filtered.value) {
    const entry = map.get(p.pid)
    if (entry) {
      entry.items.push(p)
    } else {
      map.set(p.pid, { pid: p.pid, name: p.name, exe_path: p.exe_path, cmdline: p.cmdline, cwd: p.cwd, items: [p] })
    }
  }
  const out = Array.from(map.values())
  for (const g of out) {
    g.items.sort((a, b) => a.port - b.port)
  }
  out.sort((a, b) => a.pid - b.pid)
  return out
})

const iconUrl = (pid: number) => `/api/ports/icon/${pid}?size=${ICON_SIZE}`
const iconFallback = (name: string) => (name || '?').trim().slice(0, 1).toUpperCase()
const procTitle = (p: { cmdline?: string; cwd?: string }) => {
  const parts: string[] = []
  if (p.cmdline) parts.push(`cmdline: ${p.cmdline}`)
  if (p.cwd) parts.push(`cwd: ${p.cwd}`)
  return parts.join('\n')
}

function onIconLoad(e: Event) {
  const img = e.target as HTMLImageElement
  img.dataset.loaded = '1'
  delete img.dataset.error
}

function onIconError(e: Event) {
  const img = e.target as HTMLImageElement
  img.dataset.error = '1'
  delete img.dataset.loaded
}

function exportPorts(format: 'csv' | 'json') {
  const items = filtered.value.map(p => ({
    port: p.port,
    pid: p.pid,
    name: p.name,
    protocol: p.protocol,
    exe_path: p.exe_path || '',
    cmdline: p.cmdline || '',
    cwd: p.cwd || '',
  }))
  if (!items.length) {
    pushToast({ level: 'warning', title: 'No ports to export' })
    return
  }
  if (format === 'json') {
    downloadJson('ports', items)
  } else {
    const rows = items.map(p => [p.port, p.pid, p.name, p.protocol, p.exe_path, p.cmdline, p.cwd])
    downloadCsv('ports', ['port', 'pid', 'name', 'protocol', 'exe_path', 'cmdline', 'cwd'], rows)
  }
  pushToast({ level: 'success', title: '已导出端口记录', detail: `${items.length} 条` })
}

async function load() {
  if (busy.value) return
  busy.value = true
  try {
    const data = await fetchPorts()
    tcp.value = data.tcp
    udp.value = data.udp
  } finally {
    busy.value = false
  }
}

async function onKillPid(pid: number, name: string) {
  if (props.disableKill) return
  const key = killConfirmKeyFor(pid)
  if (!isKillConfirmArmed(key)) {
    armKillConfirm(key)
    return
  }
  resetKillConfirm()
  killBusyPid.value = pid
  try {
    await killPid(pid)
    await load()
  } finally {
    killBusyPid.value = null
  }
}

function stopAutoRefresh() {
  if (autoTimer != null) {
    clearInterval(autoTimer)
    autoTimer = null
  }
}

function startAutoRefresh(ms: number) {
  stopAutoRefresh()
  if (ms > 0) {
    autoTimer = window.setInterval(() => {
      void load()
    }, ms)
  }
}

watch(autoRefreshMs, ms => startAutoRefresh(ms))

onMounted(() => {
  void load()
  startAutoRefresh(autoRefreshMs.value)
})

onBeforeUnmount(() => {
  stopAutoRefresh()
  stopKillConfirmTimer()
})
</script>

<template>
  <div>
    <div class="toolbar">
      <Button
        size="sm"
        preset="secondary"
        :disabled="busy"
        :loading="busy"
        @click="load"
        style="display:flex;align-items:center;gap:var(--space-1)"
      >
        <IconRefresh :size="16" /> Refresh
      </Button>
      <div class="toolbar-field">
        <span class="toolbar-label">Auto refresh</span>
        <select v-model.number="autoRefreshMs">
          <option :value="0">Off</option>
          <option :value="2000">2s</option>
          <option :value="5000">5s</option>
          <option :value="10000">10s</option>
          <option :value="30000">30s</option>
        </select>
      </div>
      <div class="toolbar-group">
        <span class="toolbar-label">Export</span>
        <Button size="sm" preset="secondary" @click="exportPorts('csv')">CSV</Button>
        <Button size="sm" preset="secondary" @click="exportPorts('json')">JSON</Button>
      </div>
      <div class="toolbar-spacer" />
      <label class="toggle">
        <input type="checkbox" v-model="groupByPid" /> Group by PID
      </label>
    </div>
    <div class="toolbar">
      <input v-model="processFilter" placeholder="Filter by process or PID..." style="flex:1" />
      <select v-model="protocolFilter" style="max-width:140px">
        <option value="all">All</option>
        <option value="tcp">TCP</option>
        <option value="udp">UDP</option>
      </select>
      <label class="toggle">
        <input type="checkbox" v-model="devOnly" /> Dev ports only
      </label>
    </div>
    <SkeletonTable v-if="isLoading" :rows="6" :columns="skeletonColumns" />

    <template v-else-if="!groupByPid">
      <table>
        <thead>
          <tr>
            <th>Port</th>
            <th>PID</th>
            <th>Process</th>
            <th>Protocol</th>
            <th v-if="!props.disableKill"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in filtered" :key="`${p.port}-${p.pid}-${p.protocol}`">
            <td>{{ p.port }}</td>
            <td>{{ p.pid }}</td>
            <td>
              <div class="proc-cell" :title="procTitle(p)">
                <div class="proc-icon-wrap">
                  <img
                    class="proc-icon-img"
                    :src="iconUrl(p.pid)"
                    :alt="p.name"
                    loading="lazy"
                    decoding="async"
                    @load="onIconLoad"
                    @error="onIconError"
                  />
                  <span class="proc-icon-fallback">{{ iconFallback(p.name) }}</span>
                </div>
                <span>{{ p.name }}</span>
              </div>
            </td>
            <td>{{ p.protocol }}</td>
            <td v-if="!props.disableKill">
              <Button
                size="sm"
                preset="danger"
                square
                class="btn--confirm"
                :loading="killBusyPid === p.pid"
                :disabled="busy || killBusyPid === p.pid"
                :title="isKillConfirmArmed(killConfirmKeyFor(p.pid)) ? `Confirm (${killConfirmRemaining}s)` : 'Kill Process'"
                @click="onKillPid(p.pid, p.name)"
              >
                <IconTimelineEventX :size="16" />
                <span v-if="isKillConfirmArmed(killConfirmKeyFor(p.pid))" class="btn__confirm-badge">
                  {{ killConfirmRemaining }}
                </span>
              </Button>
            </td>
          </tr>
          <tr v-if="!filtered.length">
            <td :colspan="props.disableKill ? 4 : 5" style="text-align:center;color:var(--text-tertiary)">No ports</td>
          </tr>
        </tbody>
      </table>
    </template>

    <template v-else>
      <div v-if="!grouped.length" style="text-align:center;color:var(--text-tertiary)">No ports</div>
      <div v-for="g in grouped" :key="g.pid" class="pid-group">
        <details open>
          <summary class="pid-header">
            <div class="proc-cell" :title="procTitle(g)">
              <div class="proc-icon-wrap">
                <img
                  class="proc-icon-img"
                  :src="iconUrl(g.pid)"
                  :alt="g.name"
                  loading="lazy"
                  decoding="async"
                  @load="onIconLoad"
                  @error="onIconError"
                />
                <span class="proc-icon-fallback">{{ iconFallback(g.name) }}</span>
              </div>
              <div class="pid-meta">
                <div class="pid-title">{{ g.name }}</div>
                <div class="pid-sub">PID {{ g.pid }} 路 {{ g.items.length }} ports</div>
              </div>
            </div>
          </summary>
          <table>
            <thead>
              <tr>
                <th>Port</th>
                <th>Protocol</th>
                <th v-if="!props.disableKill"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="p in g.items" :key="`${p.port}-${p.protocol}`">
                <td>{{ p.port }}</td>
                <td>{{ p.protocol }}</td>
                <td v-if="!props.disableKill">
                  <Button
                    size="sm"
                    preset="danger"
                    square
                    class="btn--confirm"
                    :loading="killBusyPid === p.pid"
                    :disabled="busy || killBusyPid === p.pid"
                    :title="isKillConfirmArmed(killConfirmKeyFor(p.pid)) ? `Confirm (${killConfirmRemaining}s)` : 'Kill Process'"
                    @click="onKillPid(p.pid, p.name)"
                  >
                    <IconTimelineEventX :size="16" />
                    <span v-if="isKillConfirmArmed(killConfirmKeyFor(p.pid))" class="btn__confirm-badge">
                      {{ killConfirmRemaining }}
                    </span>
                  </Button>
                </td>
              </tr>
            </tbody>
          </table>
        </details>
      </div>
    </template>
  </div>
</template>

<style scoped>
.toolbar-field {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.toolbar-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}
.toolbar-spacer {
  flex: 1;
}
.toggle {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--text-sm);
}
.proc-cell {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.proc-icon-wrap {
  width: var(--icon-md);
  height: var(--icon-md);
  position: relative;
  flex: 0 0 auto;
}

.proc-icon-img {
  width: 100%;
  height: 100%;
  border-radius: var(--radius-sm);
  background: var(--ds-color-2);
  object-fit: contain;
  display: block;
}

.proc-icon-img[data-error='1'] {
  display: none;
}

.proc-icon-img[data-loaded='1'] + .proc-icon-fallback {
  display: none;
}

.proc-icon-fallback {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: var(--ds-color-2);
  color: var(--text-tertiary);
  font-size: 10px;
  font-weight: var(--weight-medium);
  text-transform: uppercase;
}
.pid-group {
  border: var(--border);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-4);
  overflow: hidden;
}
.pid-header {
  padding: var(--space-3) var(--space-4);
  background: var(--ds-background-2);
  border-bottom: var(--border);
  cursor: pointer;
  list-style: none;
}
.pid-header::-webkit-details-marker {
  display: none;
}
.pid-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.pid-title {
  font-size: var(--text-sm);
  color: var(--text-primary);
}
.pid-sub {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
</style>
