<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import type { Table, Value } from '../../generated/types'

const VIRTUAL_THRESHOLD = 100
const ROW_HEIGHT = 36
const OVERSCAN = 10

const props = withDefaults(
  defineProps<{
    table: Table
    searchable?: boolean
    selectable?: boolean
  }>(),
  { searchable: false, selectable: false },
)

const emit = defineEmits<{
  'selection-change': [rows: Record<string, Value>[]]
}>()

const searchQuery = ref('')
const sortColumn = ref<string | null>(null)
const sortAsc = ref(true)
const selectedIndices = ref<Set<number>>(new Set())
const parentRef = ref<HTMLElement | null>(null)

const useVirtual = computed(() => filteredRows.value.length > VIRTUAL_THRESHOLD)

const virtualizer = useVirtualizer({
  count: computed(() => filteredRows.value.length),
  getScrollElement: () => parentRef.value,
  estimateSize: () => ROW_HEIGHT,
  overscan: OVERSCAN,
})

const virtualRows = computed(() =>
  useVirtual.value ? virtualizer.value.getVirtualItems() : [],
)
const totalSize = computed(() =>
  useVirtual.value ? virtualizer.value.getTotalSize() : 0,
)

const filteredRows = computed(() => {
  let rows = props.table.rows
  const q = searchQuery.value.toLowerCase()
  if (q) {
    rows = rows.filter((row) =>
      props.table.columns.some((col) => {
        const val = row[col.name]
        return val != null && String(val).toLowerCase().includes(q)
      }),
    )
  }
  if (sortColumn.value) {
    const col = sortColumn.value
    const dir = sortAsc.value ? 1 : -1
    rows = [...rows].sort((a, b) => {
      const va = a[col]
      const vb = b[col]
      if (va == null && vb == null) return 0
      if (va == null) return 1
      if (vb == null) return -1
      if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir
      return String(va).localeCompare(String(vb)) * dir
    })
  }
  return rows
})

function toggleSort(columnName: string) {
  if (sortColumn.value === columnName) {
    sortAsc.value = !sortAsc.value
  } else {
    sortColumn.value = columnName
    sortAsc.value = true
  }
}

function toggleRow(index: number) {
  if (selectedIndices.value.has(index)) {
    selectedIndices.value.delete(index)
  } else {
    selectedIndices.value.add(index)
  }
  selectedIndices.value = new Set(selectedIndices.value)
  emit(
    'selection-change',
    [...selectedIndices.value].map((i) => filteredRows.value[i]),
  )
}

function toggleAll() {
  if (selectedIndices.value.size === filteredRows.value.length) {
    selectedIndices.value = new Set()
  } else {
    selectedIndices.value = new Set(filteredRows.value.map((_, i) => i))
  }
  emit(
    'selection-change',
    [...selectedIndices.value].map((i) => filteredRows.value[i]),
  )
}
</script>

<template>
  <div>
    <input
      v-if="searchable"
      v-model="searchQuery"
      class="search-input"
      data-testid="search-input"
      placeholder="Search..."
    />

    <!-- Virtual scroll mode (>100 rows) -->
    <template v-if="useVirtual">
      <table class="virtual-header">
        <thead>
          <tr>
            <th v-if="selectable" class="checkbox-cell" />
            <th
              v-for="col in table.columns"
              :key="col.name"
              :class="{ sortable: col.sortable }"
              @click="col.sortable ? toggleSort(col.name) : undefined"
            >
              {{ col.name }}
              <span v-if="sortColumn === col.name" class="sort-indicator">
                {{ sortAsc ? '▲' : '▼' }}
              </span>
            </th>
          </tr>
        </thead>
      </table>
      <div
        ref="parentRef"
        data-testid="virtual-container"
        class="virtual-scroll"
      >
        <div :style="{ height: `${totalSize}px`, position: 'relative' }">
          <table class="virtual-body">
            <tbody>
              <tr
                v-for="vRow in virtualRows"
                :key="vRow.index"
                :style="{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: `${vRow.size}px`,
                  transform: `translateY(${vRow.start}px)`,
                }"
              >
                <td v-if="selectable" class="checkbox-cell">
                  <input
                    type="checkbox"
                    :checked="selectedIndices.has(vRow.index)"
                    @change="toggleRow(vRow.index)"
                  />
                </td>
                <td v-for="col in table.columns" :key="col.name">
                  {{ filteredRows[vRow.index][col.name] }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- Normal mode (<=100 rows) -->
    <table v-else class="data-table">
      <thead>
        <tr>
          <th v-if="selectable" class="checkbox-cell">
            <input
              type="checkbox"
              :checked="selectedIndices.size === filteredRows.length && filteredRows.length > 0"
              @change="toggleAll"
            />
          </th>
          <th
            v-for="col in table.columns"
            :key="col.name"
            :class="{ sortable: col.sortable }"
            @click="col.sortable ? toggleSort(col.name) : undefined"
          >
            {{ col.name }}
            <span v-if="sortColumn === col.name" class="sort-indicator">
              {{ sortAsc ? '▲' : '▼' }}
            </span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, idx) in filteredRows" :key="idx">
          <td v-if="selectable" class="checkbox-cell">
            <input
              type="checkbox"
              :checked="selectedIndices.has(idx)"
              @change="toggleRow(idx)"
            />
          </td>
          <td v-for="col in table.columns" :key="col.name">
            {{ row[col.name] }}
          </td>
        </tr>
      </tbody>
    </table>

    <div v-if="filteredRows.length === 0" data-testid="empty-state">No data</div>
  </div>
</template>

<style scoped>
.search-input {
  width: 100%;
  max-width: 320px;
  padding: var(--space-2) var(--space-3);
  padding-left: var(--space-8);
  margin-bottom: var(--space-3);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-panel);
  color: var(--text-primary);
  font: var(--type-body-sm);
  outline: none;
  transition: border-color var(--duration-fast) ease;
}

.search-input:focus {
  border-color: var(--color-info);
  box-shadow: 0 0 0 1px var(--color-info);
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th {
  cursor: default;
  user-select: none;
  white-space: nowrap;
  position: relative;
}

.data-table th.sortable {
  cursor: pointer;
}

.data-table th.sortable:hover {
  color: var(--text-primary);
}

.data-table tbody tr:nth-child(even) {
  background: var(--surface-card-muted);
}

.data-table tbody tr:hover {
  background: var(--color-info-bg);
}

.data-table td {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sort-indicator {
  display: inline-block;
  margin-left: var(--space-1);
  font-size: 0.75em;
  opacity: 0.6;
}

.checkbox-cell {
  width: 36px;
  text-align: center;
}

.virtual-scroll {
  height: 400px;
  overflow: auto;
}

.virtual-header {
  table-layout: fixed;
  width: 100%;
}

.virtual-body {
  table-layout: fixed;
  width: 100%;
  border-collapse: collapse;
}

.virtual-body tr {
  display: table-row;
}

.virtual-body tr:hover {
  background: var(--color-info-bg);
}
</style>
