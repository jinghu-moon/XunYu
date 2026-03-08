<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { fetchWorkspaceOverviewSummary } from '../../api'
import type { WorkspaceCapabilities, WorkspaceOverviewSummary } from '../../types'
import HomePanel from '../HomePanel.vue'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const props = defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()

const summary = ref<WorkspaceOverviewSummary | null>(null)

async function load() {
  summary.value = await fetchWorkspaceOverviewSummary()
}

onMounted(() => {
  void load()
})
</script>

<template>
  <WorkspaceFrame title="总览" description="统一查看工作台能力、关键指标、最近任务和现有 Home 面板摘要。">
    <template #summary>
      <div class="overview-kpis">
        <div class="overview-kpi">
          <span>Bookmarks</span>
          <strong>{{ summary?.bookmarks ?? '-' }}</strong>
        </div>
        <div class="overview-kpi">
          <span>Ports</span>
          <strong>{{ summary ? summary.tcp_ports + summary.udp_ports : '-' }}</strong>
        </div>
        <div class="overview-kpi">
          <span>Env Vars</span>
          <strong>{{ summary?.env_total_vars ?? '-' }}</strong>
        </div>
        <div class="overview-kpi">
          <span>Recent Tasks</span>
          <strong>{{ summary?.recent_tasks ?? '-' }}</strong>
        </div>
        <div class="overview-kpi">
          <span>Failed Tasks</span>
          <strong>{{ summary?.failed_tasks ?? '-' }}</strong>
        </div>
        <div class="overview-kpi">
          <span>Dry Runs</span>
          <strong>{{ summary?.dry_run_tasks ?? '-' }}</strong>
        </div>
      </div>
    </template>

    <RecentTasksPanel title="最近任务" description="查看最新运行、预演和危险动作执行结果。" :limit="12" />

    <section class="overview-section">
      <h3 class="overview-section__title">工作台能力</h3>
      <div class="overview-capabilities">
        <span v-for="name in summary?.workspaces || []" :key="name" class="overview-chip">{{ name }}</span>
      </div>
      <div v-if="props.capabilities" class="overview-capabilities overview-capabilities--feature">
        <span v-for="(enabled, key) in props.capabilities" :key="key" :class="['overview-chip', enabled ? 'is-enabled' : 'is-disabled']">
          {{ key }}: {{ enabled ? 'on' : 'off' }}
        </span>
      </div>
    </section>

    <HomePanel />
  </WorkspaceFrame>
</template>

<style scoped>
.overview-kpis {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-2);
}

.overview-kpi {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.overview-kpi span {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.overview-kpi strong {
  color: var(--text-primary);
  font: var(--type-title-sm);
}

.overview-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.overview-section__title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.overview-capabilities {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.overview-capabilities--feature {
  margin-top: var(--space-2);
}

.overview-chip {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.overview-chip.is-enabled {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.overview-chip.is-disabled {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}
</style>