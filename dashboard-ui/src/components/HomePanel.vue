<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { IconActivity, IconBookmarks, IconRefresh, IconServer, IconShieldCheck } from '@tabler/icons-vue'
import type { AuditResponse, Bookmark, PortsResponse, ProxyConfig, ProxyItem } from '../types'
import { fetchAudit, fetchBookmarks, fetchPorts, fetchProxyConfig, fetchProxyStatus } from '../api'
import { Button } from './button'
import SkeletonTable from './SkeletonTable.vue'
import { tagCategoryClass } from '../ui/tags'

const bookmarks = ref<Bookmark[]>([])
const ports = ref<PortsResponse | null>(null)
const proxyItems = ref<ProxyItem[]>([])
const proxyCfg = ref<ProxyConfig | null>(null)
const audit = ref<AuditResponse | null>(null)
const busy = ref(false)
const hasLoaded = ref(false)

async function load() {
  busy.value = true
  try {
    const [bm, portsRes, proxyStatus, proxyConfig, auditRes] = await Promise.all([
      fetchBookmarks(),
      fetchPorts(),
      fetchProxyStatus(),
      fetchProxyConfig(),
      fetchAudit({ limit: 5 }),
    ])
    bookmarks.value = bm
    ports.value = portsRes
    proxyItems.value = proxyStatus
    proxyCfg.value = proxyConfig
    audit.value = auditRes
  } finally {
    busy.value = false
    hasLoaded.value = true
  }
}

const isLoading = computed(() => busy.value && !hasLoaded.value)

const bookmarkCount = computed(() => bookmarks.value.length)
const bookmarkTagCount = computed(() => {
  const set = new Set<string>()
  for (const b of bookmarks.value) {
    for (const t of b.tags || []) {
      const v = t.trim()
      if (v) set.add(v)
    }
  }
  return set.size
})
const topTags = computed(() => {
  const counts = new Map<string, number>()
  for (const b of bookmarks.value) {
    for (const t of b.tags || []) {
      const v = t.trim()
      if (!v) continue
      counts.set(v, (counts.get(v) || 0) + 1)
    }
  }
  return Array.from(counts.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, 3)
    .map(([tag, count]) => ({ tag, count }))
})

const portTotal = computed(() => (ports.value ? ports.value.tcp.length + ports.value.udp.length : 0))
const portPidCount = computed(() => {
  if (!ports.value) return 0
  const set = new Set<number>()
  for (const p of ports.value.tcp) set.add(p.pid)
  for (const p of ports.value.udp) set.add(p.pid)
  return set.size
})
const portSnapshot = computed(() => {
  const list = ports.value ? [...ports.value.tcp, ...ports.value.udp] : []
  return list.sort((a, b) => a.port - b.port).slice(0, 6)
})

const proxyDefaultUrl = computed(() => (proxyCfg.value?.defaultUrl || '').trim())
const proxyNoProxy = computed(() => (proxyCfg.value?.noproxy || '').trim())
const activeProxyItems = computed(() => proxyItems.value.filter(p => p.status === 'ON' && p.address.trim()))
const proxyConsistency = computed(() => {
  if (!proxyDefaultUrl.value) return { text: 'No defaultUrl', tone: 'muted' }
  if (!activeProxyItems.value.length) return { text: 'No active proxies', tone: 'muted' }
  const matches = activeProxyItems.value.filter(p => p.address.trim() === proxyDefaultUrl.value).length
  if (matches === activeProxyItems.value.length) return { text: 'Consistent', tone: 'ok' }
  return { text: 'Drift', tone: 'warn' }
})

const recentAudits = computed(() => audit.value?.entries || [])
const auditTotal = computed(() => audit.value?.stats.total || 0)

function fmtTs(ts: number): string {
  if (!ts) return '-'
  const d = new Date(ts * 1000)
  return d.toLocaleString()
}

onMounted(load)
</script>

<template>
  <div class="home">
    <div class="home-header">
      <div class="home-title">Overview</div>
      <Button size="sm" preset="secondary" :disabled="busy" class="home-refresh" @click="load">
        <IconRefresh :size="16" /> Refresh
      </Button>
    </div>

    <div v-if="isLoading" class="kpi-grid">
      <div v-for="n in 4" :key="n" class="kpi-card kpi-card--skeleton">
        <div class="skeleton-line w-40" />
        <div class="skeleton-line w-70" />
        <div class="skeleton-line w-60" />
      </div>
    </div>

    <div v-else class="kpi-grid">
      <div class="kpi-card">
        <div class="kpi-label"><IconBookmarks :size="16" /> Bookmarks</div>
        <div class="kpi-value">{{ bookmarkCount }}</div>
        <div class="kpi-meta">
          Tags {{ bookmarkTagCount }}
          <span v-if="topTags.length" class="kpi-sep">·</span>
          <span v-for="t in topTags" :key="t.tag" :class="['tag-pill', tagCategoryClass(t.tag)]">
            {{ t.tag }} · {{ t.count }}
          </span>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-label"><IconServer :size="16" /> Ports</div>
        <div class="kpi-value">{{ portTotal }}</div>
        <div class="kpi-meta">
          TCP {{ ports?.tcp.length || 0 }} · UDP {{ ports?.udp.length || 0 }} · PID {{ portPidCount }}
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-label"><IconShieldCheck :size="16" /> Proxy</div>
        <div class="kpi-value">{{ activeProxyItems.length }}/{{ proxyItems.length }}</div>
        <div class="kpi-meta">
          <span :class="['kpi-status', proxyConsistency.tone]">{{ proxyConsistency.text }}</span>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-label"><IconActivity :size="16" /> Audits</div>
        <div class="kpi-value">{{ auditTotal }}</div>
        <div class="kpi-meta">Recent {{ recentAudits.length }}</div>
      </div>
    </div>

    <div class="home-grid">
      <section class="panel">
        <div class="panel-title">Ports snapshot</div>
        <SkeletonTable v-if="isLoading" :rows="6" :columns="4" />
        <table v-else>
          <thead>
            <tr>
              <th class="col-port">Port</th>
              <th class="col-pid">PID</th>
              <th>Process</th>
              <th class="col-proto">Proto</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in portSnapshot" :key="`${p.port}-${p.pid}-${p.protocol}`">
              <td>{{ p.port }}</td>
              <td>{{ p.pid }}</td>
              <td class="muted">{{ p.name }}</td>
              <td class="muted">{{ p.protocol }}</td>
            </tr>
            <tr v-if="!portSnapshot.length">
              <td colspan="4" class="empty">No ports</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section class="panel">
        <div class="panel-title">Proxy health</div>
        <div class="kv">
          <div class="kv-row">
            <div class="kv-key">defaultUrl</div>
            <div class="kv-value">{{ proxyDefaultUrl || '-' }}</div>
          </div>
          <div class="kv-row">
            <div class="kv-key">noproxy</div>
            <div class="kv-value">{{ proxyNoProxy || '-' }}</div>
          </div>
          <div class="kv-row">
            <div class="kv-key">consistency</div>
            <div :class="['kv-pill', proxyConsistency.tone]">{{ proxyConsistency.text }}</div>
          </div>
        </div>
        <div class="proxy-list">
          <div v-for="p in proxyItems" :key="p.tool" class="proxy-item">
            <div class="proxy-tool">{{ p.tool }}</div>
            <div :class="['proxy-status', p.status === 'ON' ? 'ok' : 'muted']">{{ p.status }}</div>
            <div class="proxy-addr">{{ p.address || '-' }}</div>
          </div>
          <div v-if="!proxyItems.length" class="empty">No proxy data</div>
        </div>
      </section>

      <section class="panel panel-span">
        <div class="panel-title">Recent audits</div>
        <SkeletonTable v-if="isLoading" :rows="5" :columns="5" />
        <table v-else>
          <thead>
            <tr>
              <th class="col-time">Time</th>
              <th class="col-action">Action</th>
              <th>Target</th>
              <th class="col-result">Result</th>
              <th>Reason</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(e, idx) in recentAudits" :key="idx">
              <td class="muted">{{ fmtTs(e.timestamp) }}</td>
              <td>{{ e.action }}</td>
              <td class="muted">{{ e.target }}</td>
              <td :class="['result', e.result === 'failed' ? 'bad' : 'ok']">{{ e.result }}</td>
              <td class="muted">{{ e.reason || '-' }}</td>
            </tr>
            <tr v-if="!recentAudits.length">
              <td colspan="5" class="empty">No audits</td>
            </tr>
          </tbody>
        </table>
      </section>
    </div>
  </div>
</template>

<style scoped>
.home {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.home-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.home-title {
  font: var(--type-title);
  color: var(--text-primary);
}

.home-refresh {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}

.kpi-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-3);
}

.kpi-card {
  border: var(--card-border);
  border-radius: var(--card-radius);
  padding: var(--card-padding-compact);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.kpi-card--skeleton {
  min-height: 110px;
}

.kpi-label {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  font: var(--type-caption);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
}

.kpi-value {
  font: var(--type-title-lg);
  font-weight: var(--weight-bold);
  color: var(--text-primary);
}

.kpi-meta {
  font: var(--type-caption);
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.kpi-sep {
  color: var(--text-tertiary);
}

.kpi-status {
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  font-weight: var(--weight-semibold);
}
.kpi-status.ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}
.kpi-status.warn {
  background: var(--color-warning-bg);
  color: var(--color-warning);
}
.kpi-status.muted {
  background: var(--ds-background-2);
  color: var(--text-secondary);
}

.home-grid {
  display: grid;
  grid-template-columns: 2fr 1.2fr;
  gap: var(--space-4);
}

.panel {
  border: var(--card-border);
  border-radius: var(--card-radius);
  padding: var(--card-padding);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.panel-span {
  grid-column: 1 / -1;
}

.panel-title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.kv {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.kv-row {
  display: grid;
  grid-template-columns: 110px 1fr;
  gap: var(--space-2);
  align-items: center;
}

.kv-key {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
}

.kv-value {
  font-size: var(--text-sm);
  color: var(--text-primary);
  word-break: break-all;
}

.kv-pill {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--weight-semibold);
}
.kv-pill.ok { background: var(--color-success-bg); color: var(--color-success); }
.kv-pill.warn { background: var(--color-warning-bg); color: var(--color-warning); }
.kv-pill.muted { background: var(--ds-background-2); color: var(--text-secondary); }

.proxy-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.proxy-item {
  display: grid;
  grid-template-columns: 90px 60px 1fr;
  gap: var(--space-2);
  align-items: center;
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  background: var(--ds-background-2);
}

.proxy-tool {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.proxy-status {
  font-size: var(--text-xs);
  font-weight: var(--weight-semibold);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-wide);
}
.proxy-status.ok { color: var(--color-success); }
.proxy-status.muted { color: var(--text-tertiary); }

.proxy-addr {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  word-break: break-all;
}

.col-port { width: 80px; }
.col-pid { width: 90px; }
.col-proto { width: 90px; }
.col-time { width: 160px; }
.col-action { width: 160px; }
.col-result { width: 120px; }

.result {
  font-weight: var(--weight-semibold);
}
.result.ok { color: var(--text-secondary); }
.result.bad { color: var(--color-danger); }

.muted {
  color: var(--text-secondary);
}

.empty {
  text-align: center;
  color: var(--text-tertiary);
}

.skeleton-line {
  position: relative;
  height: var(--space-4);
  border-radius: var(--radius-sm);
  background: var(--ds-background-2);
  overflow: hidden;
}
.skeleton-line::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    var(--ds-background-2) 0%,
    var(--ds-background-1) 50%,
    var(--ds-background-2) 100%
  );
  transform: translateX(-100%);
  animation: skeleton-shimmer 1.2s ease-in-out infinite;
}

.w-40 { width: 40%; }
.w-60 { width: 60%; }
.w-70 { width: 70%; }

@keyframes skeleton-shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
</style>
