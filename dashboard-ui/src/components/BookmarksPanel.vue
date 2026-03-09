<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import type { Bookmark } from '../types'
import {
  bookmarksBatchAddTags,
  bookmarksBatchDelete,
  bookmarksBatchRemoveTags,
  fetchBookmarks,
  upsertBookmark,
  deleteBookmark,
  renameBookmark,
} from '../api'
import { IconPlus, IconX, IconTrash, IconSearch } from '@tabler/icons-vue'
import { Button } from './button'
import { pushToast } from '../ui/feedback'
import { tagCategoryClass } from '../ui/tags'
import { downloadCsv, downloadJson } from '../ui/export'
import SkeletonTable from './SkeletonTable.vue'

const bookmarks = ref<Bookmark[]>([])
const search = ref('')
const tagFilter = ref('')
const showForm = ref(false)
const form = ref({ name: '', path: '', tags: '' })
const selected = ref<Record<string, boolean>>({})
const batchTags = ref('')
const viewMode = ref<'list' | 'group'>('list')
const sortKey = ref<'name' | 'path' | 'tags' | 'visits'>('name')
const sortDir = ref<'asc' | 'desc'>('asc')
const columns = ref({ path: true, tags: true, visits: true })
const busy = ref(false)
const editBusy = ref(false)
const editingNameKey = ref<string | null>(null)
const editingNameValue = ref('')
const editingTagsKey = ref<string | null>(null)
const editingTagsValue = ref('')
const deleteBusyName = ref<string | null>(null)
const batchDeleteBusy = ref(false)
const confirmKey = ref<string | null>(null)
const confirmRemaining = ref(0)
let confirmTimer: number | null = null

const CONFIRM_WINDOW_SEC = 3
const batchConfirmKey = 'batch-delete'
const deleteConfirmKey = (name: string) => `bookmark:${name}`

function stopConfirmTimer() {
  if (confirmTimer != null) {
    clearInterval(confirmTimer)
    confirmTimer = null
  }
}

function resetConfirm() {
  confirmKey.value = null
  confirmRemaining.value = 0
  stopConfirmTimer()
}

function armConfirm(key: string) {
  confirmKey.value = key
  confirmRemaining.value = CONFIRM_WINDOW_SEC
  stopConfirmTimer()
  confirmTimer = window.setInterval(() => {
    confirmRemaining.value -= 1
    if (confirmRemaining.value <= 0) resetConfirm()
  }, 1000)
}

function isConfirmArmed(key: string) {
  return confirmKey.value === key && confirmRemaining.value > 0
}

function resetInlineEdit() {
  editingNameKey.value = null
  editingNameValue.value = ''
  editingTagsKey.value = null
  editingTagsValue.value = ''
}

function startEditName(b: Bookmark) {
  editingTagsKey.value = null
  editingTagsValue.value = ''
  editingNameKey.value = b.name
  editingNameValue.value = b.name
}

function startEditTags(b: Bookmark) {
  editingNameKey.value = null
  editingNameValue.value = ''
  editingTagsKey.value = b.name
  editingTagsValue.value = (b.tags || []).join(', ')
}

function cancelInlineEdit() {
  resetInlineEdit()
}

async function saveEditName(b: Bookmark) {
  if (editBusy.value) return
  if (editingNameKey.value !== b.name) return
  const next = editingNameValue.value.trim()
  if (!next || next === b.name) {
    editingNameKey.value = null
    return
  }
  editBusy.value = true
  try {
    await renameBookmark(b.name, next)
    resetInlineEdit()
    await load()
  } finally {
    editBusy.value = false
  }
}

async function saveEditTags(b: Bookmark) {
  if (editBusy.value) return
  if (editingTagsKey.value !== b.name) return
  const tags = splitTags(editingTagsValue.value)
  editBusy.value = true
  try {
    await upsertBookmark(b.name, b.path, tags)
    resetInlineEdit()
    await load()
  } finally {
    editBusy.value = false
  }
}

function splitTags(raw: string): string[] {
  return raw
    .split(',')
    .map(s => s.trim())
    .filter(Boolean)
}

const tagFilters = computed(() => {
  const tags = splitTags(tagFilter.value).map(t => t.toLowerCase())
  return Array.from(new Set(tags))
})

const allTags = computed(() => {
  const set = new Set<string>()
  for (const b of bookmarks.value) {
    for (const t of b.tags || []) {
      const v = t.trim()
      if (v) set.add(v)
    }
  }
  return Array.from(set).sort((a, b) => a.localeCompare(b))
})

const filtered = computed(() => {
  const q = search.value.toLowerCase()
  const tags = tagFilters.value
  return bookmarks.value.filter(b => {
    const matchSearch = !q
      || b.name.toLowerCase().includes(q)
      || b.path.toLowerCase().includes(q)
      || b.tags.some(t => t.toLowerCase().includes(q))
    if (!matchSearch) return false
    if (!tags.length) return true
    const own = b.tags.map(t => t.toLowerCase())
    return tags.every(t => own.includes(t))
  })
})

const sorted = computed(() => {
  const items = [...filtered.value]
  const key = sortKey.value
  const dir = sortDir.value === 'asc' ? 1 : -1
  items.sort((a, b) => compareBookmarks(a, b, key) * dir)
  return items
})

async function load() {
  busy.value = true
  try {
    bookmarks.value = await fetchBookmarks()
    selected.value = {}
    resetInlineEdit()
  } finally {
    busy.value = false
  }
}

async function onAdd() {
  if (!form.value.name || !form.value.path) return
  const tags = form.value.tags.split(',').map(t => t.trim()).filter(Boolean)
  await upsertBookmark(form.value.name, form.value.path, tags)
  form.value = { name: '', path: '', tags: '' }
  showForm.value = false
  await load()
}

async function onDelete(name: string) {
  const key = deleteConfirmKey(name)
  if (!isConfirmArmed(key)) {
    armConfirm(key)
    return
  }
  resetConfirm()
  deleteBusyName.value = name
  try {
    await deleteBookmark(name)
    await load()
  } finally {
    deleteBusyName.value = null
  }
}

const selectedNames = computed(() => {
  return Object.entries(selected.value)
    .filter(([, v]) => v)
    .map(([k]) => k)
})
const hasSelection = computed(() => selectedNames.value.length > 0)

async function onBatchDelete() {
  if (!selectedNames.value.length) return
  if (!isConfirmArmed(batchConfirmKey)) {
    armConfirm(batchConfirmKey)
    return
  }
  resetConfirm()
  batchDeleteBusy.value = true
  try {
    await bookmarksBatchDelete(selectedNames.value)
    await load()
  } finally {
    batchDeleteBusy.value = false
  }
}

async function onBatchAddTags() {
  const tags = splitTags(batchTags.value)
  if (!selectedNames.value.length || !tags.length) return
  await bookmarksBatchAddTags(selectedNames.value, tags)
  batchTags.value = ''
  await load()
}

async function onBatchRemoveTags() {
  const tags = splitTags(batchTags.value)
  if (!selectedNames.value.length || !tags.length) return
  await bookmarksBatchRemoveTags(selectedNames.value, tags)
  batchTags.value = ''
  await load()
}

function dirname(p: string): string {
  const s = p.replace(/\\/g, '/')
  const idx = s.lastIndexOf('/')
  if (idx <= 0) return ''
  return s.slice(0, idx)
}

function compareBookmarks(a: Bookmark, b: Bookmark, key: 'name' | 'path' | 'tags' | 'visits'): number {
  if (key === 'visits') {
    return (a.visits || 0) - (b.visits || 0)
  }
  if (key === 'path') {
    return a.path.toLowerCase().localeCompare(b.path.toLowerCase())
  }
  if (key === 'tags') {
    return a.tags.join(',').toLowerCase().localeCompare(b.tags.join(',').toLowerCase())
  }
  return a.name.toLowerCase().localeCompare(b.name.toLowerCase())
}

function toggleSort(key: 'name' | 'path' | 'tags' | 'visits') {
  if (sortKey.value === key) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
    return
  }
  sortKey.value = key
  sortDir.value = 'asc'
}

function sortLabel(key: 'name' | 'path' | 'tags' | 'visits'): string {
  if (sortKey.value !== key) return ''
  return sortDir.value === 'asc' ? ' (asc)' : ' (desc)'
}

const allVisibleSelected = computed(() => {
  const items = sorted.value
  if (!items.length) return false
  return items.every(b => selected.value[b.name])
})

const selectAllLabel = computed(() => (allVisibleSelected.value ? 'Clear' : 'Select All'))

function onToggleSelectAll() {
  const items = sorted.value
  if (!items.length) return
  const next = { ...selected.value }
  const shouldSelect = !allVisibleSelected.value
  for (const b of items) {
    if (shouldSelect) {
      next[b.name] = true
    } else {
      delete next[b.name]
    }
  }
  selected.value = next
}

async function copyPath(path: string, opts?: { title?: string; detail?: string }) {
  const text = path.trim()
  if (!text) return
  const title = opts?.title || '路径已复制'
  const detail = opts?.detail
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      pushToast({ level: 'success', title, detail })
      return
    } catch {}
  }
  const el = document.createElement('textarea')
  el.value = text
  el.setAttribute('readonly', 'true')
  el.style.position = 'fixed'
  el.style.top = '-1000px'
  document.body.appendChild(el)
  el.select()
  try {
    document.execCommand('copy')
    pushToast({ level: 'success', title, detail })
  } finally {
    document.body.removeChild(el)
  }
}

async function onCopyPath(path: string) {
  await copyPath(path)
}

async function onOpenPath(path: string) {
  await copyPath(path, { title: '路径已复制', detail: '请粘贴到资源管理器打开。' })
}

function exportBookmarks(format: 'csv' | 'json') {
  const items = sorted.value.map(b => ({
    name: b.name,
    path: b.path,
    tags: b.tags || [],
    visits: b.visits ?? 0,
  }))
  if (!items.length) {
    pushToast({ level: 'warning', title: '暂无可导出的书签' })
    return
  }
  if (format === 'json') {
    downloadJson('bookmarks', items)
  } else {
    const rows = items.map(b => [b.name, b.path, b.tags.join('|'), b.visits])
    downloadCsv('bookmarks', ['name', 'path', 'tags', 'visits'], rows)
  }
  pushToast({ level: 'success', title: '书签已导出', detail: `${items.length} 条` })
}

const listColspan = computed(() => {
  let count = 2
  if (columns.value.path) count += 1
  if (columns.value.tags) count += 1
  if (columns.value.visits) count += 1
  count += 1
  return count
})

const grouped = computed(() => {
  const items = sorted.value
  const map = new Map<string, Bookmark[]>()
  for (const b of items) {
    const d = dirname(b.path) || '(root)'
    const arr = map.get(d) || []
    arr.push(b)
    map.set(d, arr)
  }
  const out = Array.from(map.entries())
    .map(([dir, items]) => ({ dir, items }))
    .sort((a, b) => a.dir.localeCompare(b.dir))
  return out
})

onMounted(load)
onBeforeUnmount(stopConfirmTimer)
</script>

<template>
  <div>
    <div class="toolbar">
      <div style="position:relative;flex:1;display:flex;align-items:center">
        <IconSearch :size="16" style="position:absolute;left:var(--space-2);color:var(--text-tertiary)" />
        <input v-model="search" placeholder="Search..." style="width:100%;padding-left:var(--space-8)" />
      </div>
      <Button size="sm" preset="secondary" style="display:flex;align-items:center;gap:var(--space-1)"
        @click="showForm = !showForm">
        <IconX v-if="showForm" :size="16" />
        <IconPlus v-else :size="16" />
        {{ showForm ? 'Cancel' : 'Add' }}
      </Button>
      <select v-model="viewMode" style="max-width:140px">
        <option value="list">List</option>
        <option value="group">Group</option>
      </select>
    </div>
    <div class="toolbar toolbar-sub">
      <div class="toolbar-label">Columns</div>
      <label class="toggle">
        <input v-model="columns.path" type="checkbox" />
        <span>Path</span>
      </label>
      <label class="toggle">
        <input v-model="columns.tags" type="checkbox" />
        <span>Tags</span>
      </label>
      <label class="toggle">
        <input v-model="columns.visits" type="checkbox" />
        <span>Visits</span>
      </label>
      <div class="toolbar-spacer"></div>
      <div class="toolbar-label">Filter tags</div>
      <input v-model="tagFilter" class="tag-filter" list="tag-options" placeholder="tags (csv, all required)" />
      <Button size="sm" preset="secondary" @click="onToggleSelectAll">{{ selectAllLabel }}</Button>
      <div class="toolbar-group">
        <div class="toolbar-label">Export</div>
        <Button size="sm" preset="secondary" @click="exportBookmarks('csv')">CSV</Button>
        <Button size="sm" preset="secondary" @click="exportBookmarks('json')">JSON</Button>
      </div>
    </div>
    <datalist id="tag-options">
      <option v-for="t in allTags" :key="t" :value="t" />
    </datalist>

    <div v-if="hasSelection" class="batch-bar">
      <div class="batch-row">
        <div class="batch-meta">Selected: {{ selectedNames.length }}</div>
        <input v-model="batchTags" list="tag-options" placeholder="tags (csv)" class="batch-tags" />
        <Button size="sm" preset="secondary" :disabled="!batchTags.trim()" @click="onBatchAddTags">Add tags</Button>
        <Button size="sm" preset="secondary" :disabled="!batchTags.trim()" @click="onBatchRemoveTags">Remove tags</Button>
        <Button
          size="sm"
          preset="danger"
          :loading="batchDeleteBusy"
          :disabled="batchDeleteBusy || !selectedNames.length"
          @click="onBatchDelete"
        >
          <span v-if="isConfirmArmed(batchConfirmKey)">Confirm ({{ confirmRemaining }}s)</span>
          <span v-else>Delete</span>
        </Button>
      </div>
    </div>
    <div v-if="showForm" style="display:flex;gap:var(--space-2);margin-bottom:var(--space-4)">
      <input v-model="form.name" placeholder="Name" />
      <input v-model="form.path" placeholder="Path" style="flex:1" />
      <input v-model="form.tags" list="tag-options" placeholder="Tags (comma)" />
      <Button size="sm" preset="primary" @click="onAdd">Save</Button>
    </div>

    <SkeletonTable v-if="busy && !sorted.length" :rows="6" :columns="listColspan" />

    <template v-else-if="viewMode === 'list'">
      <table>
        <thead>
          <tr>
            <th style="width:36px"></th>
            <th class="sortable" @click="toggleSort('name')">Name<span class="sort-label">{{ sortLabel('name') }}</span></th>
            <th v-if="columns.path" class="sortable" @click="toggleSort('path')">Path<span class="sort-label">{{ sortLabel('path') }}</span></th>
            <th v-if="columns.tags" class="sortable" @click="toggleSort('tags')">Tags<span class="sort-label">{{ sortLabel('tags') }}</span></th>
            <th v-if="columns.visits" class="sortable" @click="toggleSort('visits')">Visits<span class="sort-label">{{ sortLabel('visits') }}</span></th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="b in sorted" :key="b.name" class="bookmark-row">
            <td><input type="checkbox" v-model="selected[b.name]" /></td>
            <td class="cell-name">
              <div v-if="editingNameKey === b.name" class="inline-edit">
                <input
                  v-model="editingNameValue"
                  class="inline-input"
                  :disabled="editBusy"
                  title="Enter to save, Esc to cancel"
                  @keydown.enter.prevent="saveEditName(b)"
                  @keydown.esc.prevent="cancelInlineEdit"
                  @blur="saveEditName(b)"
                />
              </div>
              <button v-else class="inline-editable" type="button" title="Click to edit name" @click="startEditName(b)">
                <span class="inline-text">{{ b.name }}</span>
              </button>
            </td>
            <td v-if="columns.path" class="path-cell">
              <span class="truncate" :title="b.path">{{ b.path }}</span>
            </td>
            <td v-if="columns.tags" class="cell-tags">
              <div v-if="editingTagsKey === b.name" class="inline-edit">
                <input
                  v-model="editingTagsValue"
                  class="inline-input"
                  :disabled="editBusy"
                  title="Comma separated. Enter to save, Esc to cancel"
                  @keydown.enter.prevent="saveEditTags(b)"
                  @keydown.esc.prevent="cancelInlineEdit"
                  @blur="saveEditTags(b)"
                />
              </div>
              <button
                v-else
                class="inline-editable inline-editable--tags"
                type="button"
                title="Click to edit tags"
                @click="startEditTags(b)"
              >
                <span v-if="b.tags.length" class="tag-list">
                  <span v-for="(t, idx) in b.tags" :key="`${b.name}-${t}-${idx}`" :class="['tag-pill', tagCategoryClass(t)]">
                    {{ t }}
                  </span>
                </span>
                <span v-else class="inline-placeholder">Add tags</span>
              </button>
            </td>
            <td v-if="columns.visits">{{ b.visits }}</td>
            <td class="actions-cell">
              <div class="row-actions">
                <Button size="sm" preset="secondary" title="Copy path" @click="onCopyPath(b.path)">Copy</Button>
                <Button size="sm" preset="secondary" title="Open (copy path)" @click="onOpenPath(b.path)">Open</Button>
                <Button
                  size="sm"
                  preset="danger"
                  square
                  class="btn--confirm"
                  :loading="deleteBusyName === b.name"
                  :disabled="deleteBusyName === b.name"
                  :title="isConfirmArmed(deleteConfirmKey(b.name)) ? `Confirm (${confirmRemaining}s)` : 'Delete'"
                  @click="onDelete(b.name)"
                >
                  <IconTrash :size="16" />
                  <span v-if="isConfirmArmed(deleteConfirmKey(b.name))" class="btn__confirm-badge">
                    {{ confirmRemaining }}
                  </span>
                </Button>
              </div>
            </td>
          </tr>
          <tr v-if="!filtered.length">
            <td :colspan="listColspan" style="text-align:center;color:var(--text-tertiary)">暂无书签</td>
          </tr>
        </tbody>
      </table>
    </template>

    <template v-else>
      <div v-if="!filtered.length" style="text-align:center;color:var(--text-tertiary)">暂无书签</div>
      <div v-for="g in grouped" :key="g.dir" class="group">
        <details open>
          <summary class="groupTitle">{{ g.dir }} <span class="groupCount">({{ g.items.length }})</span></summary>
          <table>
            <thead>
              <tr>
                <th style="width:36px"></th>
                <th class="sortable" @click="toggleSort('name')">Name<span class="sort-label">{{ sortLabel('name') }}</span></th>
                <th v-if="columns.path" class="sortable" @click="toggleSort('path')">Path<span class="sort-label">{{ sortLabel('path') }}</span></th>
                <th v-if="columns.tags" class="sortable" @click="toggleSort('tags')">Tags<span class="sort-label">{{ sortLabel('tags') }}</span></th>
                <th v-if="columns.visits" class="sortable" @click="toggleSort('visits')">Visits<span class="sort-label">{{ sortLabel('visits') }}</span></th>
                <th></th>
              </tr>
            </thead>
            <tbody>
          <tr v-for="b in g.items" :key="b.name" class="bookmark-row">
            <td><input type="checkbox" v-model="selected[b.name]" /></td>
            <td class="cell-name">
              <div v-if="editingNameKey === b.name" class="inline-edit">
                <input
                  v-model="editingNameValue"
                  class="inline-input"
                  :disabled="editBusy"
                  title="Enter to save, Esc to cancel"
                  @keydown.enter.prevent="saveEditName(b)"
                  @keydown.esc.prevent="cancelInlineEdit"
                  @blur="saveEditName(b)"
                />
              </div>
              <button v-else class="inline-editable" type="button" title="Click to edit name" @click="startEditName(b)">
                <span class="inline-text">{{ b.name }}</span>
              </button>
            </td>
            <td v-if="columns.path" class="path-cell">
              <span class="truncate" :title="b.path">{{ b.path }}</span>
            </td>
            <td v-if="columns.tags" class="cell-tags">
              <div v-if="editingTagsKey === b.name" class="inline-edit">
                <input
                  v-model="editingTagsValue"
                  class="inline-input"
                  :disabled="editBusy"
                  title="Comma separated. Enter to save, Esc to cancel"
                  @keydown.enter.prevent="saveEditTags(b)"
                  @keydown.esc.prevent="cancelInlineEdit"
                  @blur="saveEditTags(b)"
                />
              </div>
              <button
                v-else
                class="inline-editable inline-editable--tags"
                type="button"
                title="Click to edit tags"
                @click="startEditTags(b)"
              >
                <span v-if="b.tags.length" class="tag-list">
                  <span v-for="(t, idx) in b.tags" :key="`${b.name}-${t}-${idx}`" :class="['tag-pill', tagCategoryClass(t)]">
                    {{ t }}
                  </span>
                </span>
                <span v-else class="inline-placeholder">Add tags</span>
              </button>
            </td>
            <td v-if="columns.visits">{{ b.visits }}</td>
            <td class="actions-cell">
              <div class="row-actions">
                <Button size="sm" preset="secondary" title="Copy path" @click="onCopyPath(b.path)">Copy</Button>
                <Button size="sm" preset="secondary" title="Open (copy path)" @click="onOpenPath(b.path)">Open</Button>
                <Button
                  size="sm"
                  preset="danger"
                  square
                  class="btn--confirm"
                      :loading="deleteBusyName === b.name"
                      :disabled="deleteBusyName === b.name"
                      :title="isConfirmArmed(deleteConfirmKey(b.name)) ? `Confirm (${confirmRemaining}s)` : 'Delete'"
                      @click="onDelete(b.name)"
                    >
                      <IconTrash :size="16" />
                      <span v-if="isConfirmArmed(deleteConfirmKey(b.name))" class="btn__confirm-badge">
                        {{ confirmRemaining }}
                      </span>
                    </Button>
                  </div>
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
.group {
  margin-bottom: var(--space-4);
  border: var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.groupTitle {
  padding: var(--space-3) var(--space-4);
  cursor: pointer;
  font-size: var(--text-sm);
  color: var(--text-primary);
  background: var(--ds-background-2);
  border-bottom: var(--border);
}
.groupCount {
  color: var(--text-tertiary);
}
details > summary {
  list-style: none;
}
details > summary::-webkit-details-marker {
  display: none;
}
.toolbar-sub {
  flex-wrap: wrap;
  margin-top: calc(var(--space-2) * -1);
}
.toolbar-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-right: var(--space-2);
}
.toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: var(--space-2);
}
.toolbar-spacer {
  flex: 1 1 auto;
  min-width: var(--space-4);
}
.tag-filter {
  min-width: 220px;
}
.toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.sortable {
  cursor: pointer;
  user-select: none;
}
.sort-label {
  margin-left: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  text-transform: none;
}
.path-cell {
  max-width: 420px;
}
.truncate {
  display: inline-block;
  max-width: 420px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  vertical-align: bottom;
}
.row-actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  justify-content: flex-end;
  opacity: 0;
  pointer-events: none;
  transition: var(--transition-color);
}

.bookmark-row:hover .row-actions,
.bookmark-row:focus-within .row-actions {
  opacity: 1;
  pointer-events: auto;
}

.bookmark-row:hover {
  background: var(--ds-background-2);
}

.actions-cell {
  width: 1%;
  white-space: nowrap;
}

.cell-name,
.cell-tags {
  max-width: 260px;
}

.inline-edit {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.inline-input {
  width: 100%;
  min-width: 140px;
}

.inline-editable {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  padding: 2px var(--space-2);
  border-radius: var(--radius-sm);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-primary);
  font: var(--type-body-sm);
  cursor: text;
  max-width: 100%;
  justify-content: flex-start;
  text-align: left;
}

.inline-editable:hover,
.inline-editable:focus-visible {
  border-color: var(--color-border-strong);
  background: var(--ds-background-2);
}

.inline-editable:focus-visible {
  outline: var(--focus-ring-width) solid var(--text-primary);
  outline-offset: var(--focus-ring-offset);
}

.inline-text {
  display: inline-block;
  max-width: 240px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.inline-placeholder {
  color: var(--text-tertiary);
}

.inline-editable--tags {
  align-items: center;
}

.tag-list {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  flex-wrap: wrap;
  max-width: 100%;
}

.batch-bar {
  position: sticky;
  bottom: 0;
  margin-bottom: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-1);
  box-shadow: var(--shadow-sm);
  z-index: var(--z-sticky);
}

.batch-row {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}

.batch-meta {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.batch-tags {
  flex: 1 1 auto;
}
</style>
