<script setup lang="ts">
import { computed } from 'vue'
import type { DiffHunk, DiffLine } from '../../types'

const props = defineProps<{
  hunks: DiffHunk[]
  viewMode: 'unified' | 'split'
  kind: string
}>()

/* ── Unified view helpers ── */

interface UnifiedRow {
  type: 'hunk-header' | 'line'
  hunk?: DiffHunk
  line?: DiffLine
  oldNum?: number | null
  newNum?: number | null
}

const unifiedRows = computed<UnifiedRow[]>(() => {
  const rows: UnifiedRow[] = []
  for (const hunk of props.hunks) {
    rows.push({ type: 'hunk-header', hunk })
    let oldNum = hunk.old_start
    let newNum = hunk.new_start
    for (const line of hunk.lines) {
      if (line.tag === 'context') {
        rows.push({ type: 'line', line, oldNum, newNum })
        oldNum++
        newNum++
      } else if (line.tag === 'remove') {
        rows.push({ type: 'line', line, oldNum, newNum: null })
        oldNum++
      } else if (line.tag === 'add') {
        rows.push({ type: 'line', line, oldNum: null, newNum })
        newNum++
      }
    }
  }
  return rows
})

/* ── Side-by-side view helpers ── */

interface SplitRow {
  type: 'hunk-header' | 'line'
  hunk?: DiffHunk
  old: { num: number | null; content: string; tag: string } | null
  new: { num: number | null; content: string; tag: string } | null
}

function pairLines(lines: DiffLine[], oldStart: number, newStart: number): SplitRow[] {
  const result: SplitRow[] = []
  let removes: DiffLine[] = []
  let adds: DiffLine[] = []
  let oldNum = oldStart
  let newNum = newStart

  function flush() {
    const maxLen = Math.max(removes.length, adds.length)
    for (let i = 0; i < maxLen; i++) {
      const rm = removes[i] || null
      const ad = adds[i] || null
      result.push({
        type: 'line',
        old: rm
          ? { num: oldNum++, content: rm.content, tag: 'remove' }
          : ad
            ? { num: null, content: '', tag: 'empty' }
            : null,
        new: ad
          ? { num: newNum++, content: ad.content, tag: 'add' }
          : rm
            ? { num: null, content: '', tag: 'empty' }
            : null,
      })
    }
    removes = []
    adds = []
  }

  for (const line of lines) {
    if (line.tag === 'context') {
      flush()
      result.push({
        type: 'line',
        old: { num: oldNum++, content: line.content, tag: 'context' },
        new: { num: newNum++, content: line.content, tag: 'context' },
      })
    } else if (line.tag === 'remove') {
      removes.push(line)
    } else if (line.tag === 'add') {
      adds.push(line)
    }
  }
  flush()
  return result
}

const splitRows = computed<SplitRow[]>(() => {
  const rows: SplitRow[] = []
  for (const hunk of props.hunks) {
    rows.push({ type: 'hunk-header', hunk, old: null, new: null })
    rows.push(...pairLines(hunk.lines, hunk.old_start, hunk.new_start))
  }
  return rows
})

function hunkHeader(h: DiffHunk): string {
  let s = `@@ -${h.old_start},${h.old_count} +${h.new_start},${h.new_count} @@`
  if (h.symbol) s += ` ${h.symbol}`
  else if (h.section) s += ` ${h.section}`
  return s
}

function lineMarker(tag: string): string {
  if (tag === 'add') return '+'
  if (tag === 'remove') return '-'
  return ' '
}
</script>

<template>
  <!-- Unified View -->
  <table v-if="viewMode === 'unified'" class="diff-table diff-unified">
    <tbody>
      <template v-for="(row, i) in unifiedRows" :key="i">
        <tr v-if="row.type === 'hunk-header'" class="diff-hunk-header">
          <td colspan="4">{{ hunkHeader(row.hunk!) }}</td>
        </tr>
        <tr
          v-else
          class="diff-line"
          :class="{
            'diff-line--add': row.line?.tag === 'add',
            'diff-line--remove': row.line?.tag === 'remove',
          }"
        >
          <td class="diff-num">{{ row.oldNum ?? '' }}</td>
          <td class="diff-num">{{ row.newNum ?? '' }}</td>
          <td class="diff-marker">{{ lineMarker(row.line?.tag || '') }}</td>
          <td class="diff-content"><pre>{{ row.line?.content }}</pre></td>
        </tr>
      </template>
    </tbody>
  </table>

  <!-- Side-by-side View -->
  <table v-else class="diff-table diff-split">
    <tbody>
      <template v-for="(row, i) in splitRows" :key="i">
        <tr v-if="row.type === 'hunk-header'" class="diff-hunk-header">
          <td colspan="6">{{ hunkHeader(row.hunk!) }}</td>
        </tr>
        <tr v-else class="diff-line">
          <td
            class="diff-num"
            :class="{ 'diff-cell--remove': row.old?.tag === 'remove' }"
          >{{ row.old?.num ?? '' }}</td>
          <td
            class="diff-content diff-content--half"
            :class="{
              'diff-cell--remove': row.old?.tag === 'remove',
              'diff-cell--empty': row.old?.tag === 'empty',
            }"
          ><pre>{{ row.old?.content }}</pre></td>
          <td class="diff-gutter"></td>
          <td
            class="diff-num"
            :class="{ 'diff-cell--add': row.new?.tag === 'add' }"
          >{{ row.new?.num ?? '' }}</td>
          <td
            class="diff-content diff-content--half"
            :class="{
              'diff-cell--add': row.new?.tag === 'add',
              'diff-cell--empty': row.new?.tag === 'empty',
            }"
          ><pre>{{ row.new?.content }}</pre></td>
        </tr>
      </template>
    </tbody>
  </table>
</template>

<style scoped>
.diff-table {
  width: 100%;
  border-collapse: collapse;
  font-family: var(--font-family-mono);
  font-size: var(--text-xs);
  line-height: 1.5;
  table-layout: fixed;
}

/* ── Hunk header ── */
.diff-hunk-header td {
  background: var(--color-info-bg);
  color: var(--color-info);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  padding: var(--space-1) var(--space-3);
  font-weight: var(--weight-medium);
  border-top: var(--border);
  border-bottom: var(--border);
}

/* ── Line numbers ── */
.diff-num {
  width: 48px;
  min-width: 48px;
  padding: 0 var(--space-2);
  text-align: right;
  color: var(--text-tertiary);
  user-select: none;
  vertical-align: top;
  white-space: nowrap;
}

/* ── Marker (+/-/space) ── */
.diff-marker {
  width: 18px;
  min-width: 18px;
  padding: 0 2px;
  text-align: center;
  color: var(--text-tertiary);
  user-select: none;
  vertical-align: top;
}

/* ── Content ── */
.diff-content {
  padding: 0 var(--space-2);
  vertical-align: top;
}

.diff-content pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font: inherit;
}

/* ── Line coloring ── */
.diff-line--add {
  background: var(--color-success-bg);
}

.diff-line--add .diff-marker {
  color: var(--color-success);
}

.diff-line--remove {
  background: var(--color-danger-bg);
}

.diff-line--remove .diff-marker {
  color: var(--color-danger);
}

/* ── Split view specifics ── */
.diff-content--half {
  width: 50%;
}

.diff-cell--add {
  background: var(--color-success-bg);
}

.diff-cell--remove {
  background: var(--color-danger-bg);
}

.diff-cell--empty {
  background: var(--gray-alpha-100);
}

.diff-gutter {
  width: 1px;
  min-width: 1px;
  background: var(--color-border-strong);
  padding: 0;
}

/* ── Hover ── */
.diff-line:hover {
  filter: brightness(1.1);
}
</style>
