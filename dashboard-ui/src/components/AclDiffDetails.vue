<script setup lang="ts">
import { computed } from 'vue'
import type { AclDiffDetails, WorkspaceTaskDetails } from '../types'

const props = defineProps<{
  details: WorkspaceTaskDetails
}>()

interface DiffPanel {
  id: string
  title: string
  diff: AclDiffDetails
}

const columnLabels = {
  principal: '主体',
  rights: '权限',
  aceType: 'ACE 类型',
  source: '来源',
  inheritance: '继承标志',
  propagation: '传播标志',
  orphan: '孤儿',
  sid: 'SID',
}

const panels = computed<DiffPanel[]>(() => {
  if (props.details.kind === 'acl_diff') {
    return [{ id: 'current', title: 'ACL 差异明细', diff: props.details.diff }]
  }
  return [
    { id: 'before', title: '执行前差异', diff: props.details.before },
    { id: 'after', title: '执行后差异', diff: props.details.after },
  ]
})

function boolText(value: boolean) {
  return value ? '是' : '否'
}

function hasDiffText(value: boolean) {
  return value ? '仍有差异' : '已对齐'
}

function ownerDiffText(diff: AclDiffDetails) {
  if (!diff.owner_diff) return '无差异'
  return `${diff.owner_diff.target} → ${diff.owner_diff.reference}`
}

function inheritanceDiffText(diff: AclDiffDetails) {
  if (!diff.inheritance_diff) return '无差异'
  const targetState = diff.inheritance_diff.target_protected ? '已关闭继承' : '已启用继承'
  const referenceState = diff.inheritance_diff.reference_protected ? '已关闭继承' : '已启用继承'
  return `${targetState} → ${referenceState}`
}
</script>

<template>
  <section class="acl-diff-details" data-testid="acl-diff-details">
    <article
      v-for="panel in panels"
      :key="panel.id"
      class="acl-diff-details__panel"
      :data-testid="`acl-diff-panel-${panel.id}`"
    >
      <header class="acl-diff-details__header">
        <div>
          <h6 class="acl-diff-details__title">{{ panel.title }}</h6>
          <p class="acl-diff-details__subtitle">{{ panel.diff.target }} vs {{ panel.diff.reference }}</p>
        </div>
        <span :class="['acl-diff-details__badge', panel.diff.has_diff ? 'is-warn' : 'is-ok']">
          {{ hasDiffText(panel.diff.has_diff) }}
        </span>
      </header>

      <dl class="acl-diff-details__summary">
        <div class="acl-diff-details__summary-item">
          <dt>共同 ACE</dt>
          <dd>{{ panel.diff.common_count }}</dd>
        </div>
        <div class="acl-diff-details__summary-item">
          <dt>Owner 差异</dt>
          <dd>{{ ownerDiffText(panel.diff) }}</dd>
        </div>
        <div class="acl-diff-details__summary-item">
          <dt>继承状态</dt>
          <dd>{{ inheritanceDiffText(panel.diff) }}</dd>
        </div>
      </dl>

      <div class="acl-diff-details__tables">
        <section class="acl-diff-details__table-card">
          <div class="acl-diff-details__table-header">
            <h6>仅目标侧 ACE</h6>
            <span class="acl-diff-details__count">{{ panel.diff.only_in_target.length }}</span>
          </div>
          <div v-if="panel.diff.only_in_target.length" class="acl-diff-details__table-wrap">
            <table class="acl-diff-details__table">
              <thead>
                <tr>
                  <th>{{ columnLabels.principal }}</th>
                  <th>{{ columnLabels.rights }}</th>
                  <th>{{ columnLabels.aceType }}</th>
                  <th>{{ columnLabels.source }}</th>
                  <th>{{ columnLabels.inheritance }}</th>
                  <th>{{ columnLabels.propagation }}</th>
                  <th>{{ columnLabels.orphan }}</th>
                  <th>{{ columnLabels.sid }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="entry in panel.diff.only_in_target" :key="`target-${panel.id}-${entry.sid}-${entry.rights}`">
                  <td>{{ entry.principal }}</td>
                  <td>{{ entry.rights }}</td>
                  <td>{{ entry.ace_type }}</td>
                  <td>{{ entry.source }}</td>
                  <td>{{ entry.inheritance }}</td>
                  <td>{{ entry.propagation }}</td>
                  <td>{{ boolText(entry.orphan) }}</td>
                  <td>{{ entry.sid }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="acl-diff-details__empty">暂无仅目标侧 ACE。</p>
        </section>

        <section class="acl-diff-details__table-card">
          <div class="acl-diff-details__table-header">
            <h6>仅参考侧 ACE</h6>
            <span class="acl-diff-details__count">{{ panel.diff.only_in_reference.length }}</span>
          </div>
          <div v-if="panel.diff.only_in_reference.length" class="acl-diff-details__table-wrap">
            <table class="acl-diff-details__table">
              <thead>
                <tr>
                  <th>{{ columnLabels.principal }}</th>
                  <th>{{ columnLabels.rights }}</th>
                  <th>{{ columnLabels.aceType }}</th>
                  <th>{{ columnLabels.source }}</th>
                  <th>{{ columnLabels.inheritance }}</th>
                  <th>{{ columnLabels.propagation }}</th>
                  <th>{{ columnLabels.orphan }}</th>
                  <th>{{ columnLabels.sid }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="entry in panel.diff.only_in_reference" :key="`reference-${panel.id}-${entry.sid}-${entry.rights}`">
                  <td>{{ entry.principal }}</td>
                  <td>{{ entry.rights }}</td>
                  <td>{{ entry.ace_type }}</td>
                  <td>{{ entry.source }}</td>
                  <td>{{ entry.inheritance }}</td>
                  <td>{{ entry.propagation }}</td>
                  <td>{{ boolText(entry.orphan) }}</td>
                  <td>{{ entry.sid }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="acl-diff-details__empty">暂无仅参考侧 ACE。</p>
        </section>
      </div>
    </article>
  </section>
</template>

<style scoped>
.acl-diff-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.acl-diff-details__panel,
.acl-diff-details__table-card,
.acl-diff-details__summary-item {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-card);
}

.acl-diff-details__panel {
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.acl-diff-details__header,
.acl-diff-details__table-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.acl-diff-details__title,
.acl-diff-details__table-header h6 {
  font: var(--type-title-xs);
  color: var(--text-primary);
}

.acl-diff-details__subtitle,
.acl-diff-details__empty,
.acl-diff-details__table th {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.acl-diff-details__badge,
.acl-diff-details__count {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.acl-diff-details__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.acl-diff-details__badge.is-warn {
  background: var(--color-warning-bg);
  color: var(--color-warning);
}

.acl-diff-details__summary,
.acl-diff-details__tables {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
}

.acl-diff-details__summary {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.acl-diff-details__summary-item {
  padding: var(--space-3);
}

.acl-diff-details__summary-item dt {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.acl-diff-details__summary-item dd {
  margin: var(--space-1) 0 0;
  color: var(--text-primary);
  font: var(--type-body-sm);
  word-break: break-word;
}

.acl-diff-details__table-card {
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.acl-diff-details__table-wrap {
  overflow: auto;
}

.acl-diff-details__table {
  width: 100%;
  border-collapse: collapse;
}

.acl-diff-details__table th,
.acl-diff-details__table td {
  padding: var(--space-2);
  border-bottom: var(--border);
  text-align: left;
  vertical-align: top;
}

.acl-diff-details__table td {
  color: var(--text-primary);
  font: var(--type-body-sm);
  word-break: break-word;
}
</style>
