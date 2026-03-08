<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useVirtualizer, type VirtualItem } from '@tanstack/vue-virtual'
import { connectDiffWs, fetchFiles, fetchFileSearch } from '../../api'
import type { DiffWsEvent, FileEntry, FileSearchEntry } from '../../types'
import DiffFilePreview from './DiffFilePreview.vue'

const props = defineProps<{
  oldPath: string
  newPath: string
}>()

const emit = defineEmits<{
  (event: 'update:oldPath', value: string): void
  (event: 'update:newPath', value: string): void
  (event: 'runDiff'): void
  (event: 'openConvert', value: string): void
}>()

type ContextAction =
  | 'open-preview'
  | 'set-old'
  | 'set-new'
  | 'set-old-run'
  | 'set-new-run'
  | 'open-convert'

interface ContextMenuState {
  visible: boolean
  x: number
  y: number
  path: string
}

interface TreeNode {
  name: string
  path: string
  isDir: boolean
  size?: number
  expanded?: boolean
  loaded?: boolean
  loading?: boolean
  children?: TreeNode[]
}

interface TreeRow {
  node: TreeNode
  depth: number
}

type VirtualListItem =
  | { kind: 'tree'; key: string; row: TreeRow }
  | { kind: 'search'; key: string; hit: FileSearchEntry }

interface VirtualRenderRow {
  virtual: VirtualItem
  item: VirtualListItem
}

const isWindows = typeof navigator !== 'undefined' && /win/i.test(navigator.platform)
const defaultRoot = isWindows ? 'C:\\' : '/'

const currentPath = ref(defaultRoot)
const pathInput = ref(defaultRoot)
const roots = ref<TreeNode[]>([])
const selectedPath = ref('')
const loading = ref(false)
const refreshing = ref(false)
const error = ref('')
const searchTerm = ref('')
const autoRefresh = ref(false)
const autoRefreshMs = 5000
let refreshTimer: ReturnType<typeof window.setInterval> | null = null
const deepSearch = ref(false)
const searchBusy = ref(false)
const searchError = ref('')
const searchHits = ref<FileSearchEntry[]>([])
const deepSearchLimit = 300
const deepSearchDebounceMs = 250
let searchDebounceTimer: ReturnType<typeof window.setTimeout> | null = null
let searchSeq = 0
const listViewportRef = ref<HTMLElement | null>(null)
const treeRowHeight = 34
const searchRowHeight = 44
const listOverscan = 8
const convertibleExts = new Set(['toml', 'yaml', 'yml', 'json', 'json5'])
const wsConnected = ref(false)
const wsStatus = ref<'connecting' | 'connected' | 'retrying' | 'closed'>('connecting')
const previewRefreshKey = ref(0)
let wsStopFn: (() => void) | null = null
let wsReconnectTimer: ReturnType<typeof window.setTimeout> | null = null
let wsManualStop = false
let wsRefreshTimer: ReturnType<typeof window.setTimeout> | null = null

const menu = ref<ContextMenuState>({
  visible: false,
  x: 0,
  y: 0,
  path: '',
})

const canUseSelected = computed(() => Boolean(selectedPath.value.trim()))
const canRunWithSelectedAsOld = computed(() => canUseSelected.value && Boolean(props.newPath.trim()))
const canRunWithSelectedAsNew = computed(() => canUseSelected.value && Boolean(props.oldPath.trim()))
const canConvertSelected = computed(() => canUseSelected.value && isConvertiblePath(selectedPath.value))
const wsLabel = computed(() => {
  if (wsStatus.value === 'connected') return 'WS Live'
  if (wsStatus.value === 'retrying') return 'WS Reconnecting'
  if (wsStatus.value === 'connecting') return 'WS Connecting'
  return 'WS Closed'
})
const visibleRows = computed<TreeRow[]>(() => flattenTree(roots.value))
const activeRows = computed<TreeRow[]>(() => {
  const term = searchTerm.value.trim().toLowerCase()
  if (!term) return visibleRows.value
  return flattenFilteredTree(roots.value, term)
})
const useDeepSearch = computed(() => deepSearch.value && Boolean(searchTerm.value.trim()))
const activeItems = computed<VirtualListItem[]>(() => {
  if (useDeepSearch.value) {
    return searchHits.value.map((hit) => ({ kind: 'search', key: hit.path, hit }))
  }
  return activeRows.value.map((row) => ({ kind: 'tree', key: row.node.path, row }))
})

const rowVirtualizer = useVirtualizer<HTMLElement, HTMLElement>(computed(() => ({
  count: activeItems.value.length,
  getScrollElement: () => listViewportRef.value,
  estimateSize: () => (useDeepSearch.value ? searchRowHeight : treeRowHeight),
  overscan: listOverscan,
})))

const virtualRows = computed<VirtualRenderRow[]>(() => {
  const items = activeItems.value
  return rowVirtualizer.value.getVirtualItems()
    .map((virtual) => {
      const item = items[virtual.index]
      if (!item) return null
      return { virtual, item }
    })
    .filter((row): row is VirtualRenderRow => row != null)
})

const virtualTotalSize = computed(() => rowVirtualizer.value.getTotalSize())

function normalizeDirectory(path: string): string {
  const p = path.trim()
  if (!p) return currentPath.value
  if (p === '/') return p
  if (/^[A-Za-z]:$/.test(p)) return `${p}\\`
  if (/^[A-Za-z]:[\\/]+$/.test(p)) return `${p.slice(0, 2)}\\`
  return p.replace(/[\\/]+$/, '')
}

function trimTrailingSeparators(path: string): string {
  if (path === '/') return path
  if (/^[A-Za-z]:\\$/.test(path)) return path
  if (/^[A-Za-z]:$/.test(path)) return `${path}\\`
  return path.replace(/[\\/]+$/, '')
}

function resolveSeparator(path: string): '/' | '\\' {
  return path.includes('\\') ? '\\' : '/'
}

function joinPath(base: string, name: string): string {
  const root = trimTrailingSeparators(base)
  if (root === '/') return `/${name}`
  if (/^[A-Za-z]:\\$/.test(root)) return `${root}${name}`
  const sep = resolveSeparator(root)
  return `${root}${sep}${name}`
}

function normalizePathForCompare(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+/g, '/').toLowerCase()
}

function pathExt(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? ''
  const dot = name.lastIndexOf('.')
  if (dot <= 0 || dot === name.length - 1) return ''
  return name.slice(dot + 1).toLowerCase()
}

function isConvertiblePath(path: string): boolean {
  return convertibleExts.has(pathExt(path))
}

function toTreeNodes(basePath: string, list: FileEntry[], prevMap?: Map<string, TreeNode>): TreeNode[] {
  return list.map((entry) => ({
    ...(prevMap?.get(joinPath(basePath, entry.name)) ?? {}),
    name: entry.name,
    path: joinPath(basePath, entry.name),
    isDir: entry.is_dir,
    size: entry.size,
    expanded: prevMap?.get(joinPath(basePath, entry.name))?.expanded ?? false,
    loaded: prevMap?.get(joinPath(basePath, entry.name))?.loaded ?? false,
    loading: false,
    children: prevMap?.get(joinPath(basePath, entry.name))?.children ?? [],
  }))
}

function flattenTree(nodes: TreeNode[]): TreeRow[] {
  const rows: TreeRow[] = []
  function walk(list: TreeNode[], depth: number) {
    for (const node of list) {
      rows.push({ node, depth })
      if (node.isDir && node.expanded && node.children?.length) {
        walk(node.children, depth + 1)
      }
    }
  }
  walk(nodes, 0)
  return rows
}

function flattenFilteredTree(nodes: TreeNode[], term: string): TreeRow[] {
  const rows: TreeRow[] = []
  function visit(list: TreeNode[], depth: number): boolean {
    let anyMatched = false
    for (const node of list) {
      const byName = node.name.toLowerCase().includes(term)
      const byPath = node.path.toLowerCase().includes(term)
      let childRows: TreeRow[] = []
      let childMatched = false
      if (node.children?.length) {
        childRows = []
        childMatched = visitInto(node.children, depth + 1, childRows)
      }
      const include = byName || byPath || childMatched
      if (include) {
        rows.push({ node, depth })
        rows.push(...childRows)
        anyMatched = true
      }
    }
    return anyMatched
  }
  function visitInto(list: TreeNode[], depth: number, out: TreeRow[]): boolean {
    let anyMatched = false
    for (const node of list) {
      const byName = node.name.toLowerCase().includes(term)
      const byPath = node.path.toLowerCase().includes(term)
      let childRows: TreeRow[] = []
      let childMatched = false
      if (node.children?.length) {
        childMatched = visitInto(node.children, depth + 1, childRows)
      }
      const include = byName || byPath || childMatched
      if (include) {
        out.push({ node, depth })
        out.push(...childRows)
        anyMatched = true
      }
    }
    return anyMatched
  }
  visit(nodes, 0)
  return rows
}

function buildNodeMap(nodes: TreeNode[], map: Map<string, TreeNode>) {
  for (const node of nodes) {
    map.set(node.path, node)
    if (node.children?.length) {
      buildNodeMap(node.children, map)
    }
  }
}

function hasPath(nodes: TreeNode[], path: string): boolean {
  for (const node of nodes) {
    if (node.path === path) return true
    if (node.children?.length && hasPath(node.children, path)) return true
  }
  return false
}

function formatSize(size?: number): string {
  if (size == null) return ''
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}

async function loadDirectory(path: string) {
  const normalized = normalizeDirectory(path)
  loading.value = true
  error.value = ''
  closeMenu()
  try {
    roots.value = toTreeNodes(normalized, await fetchFiles(normalized))
    currentPath.value = normalized
    pathInput.value = normalized
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
    roots.value = []
  } finally {
    loading.value = false
  }
}

async function hydrateExpandedChildren(node: TreeNode, prevMap: Map<string, TreeNode>) {
  if (!node.isDir || !node.expanded) return
  node.loading = true
  try {
    node.children = toTreeNodes(node.path, await fetchFiles(node.path), prevMap)
    node.loaded = true
    for (const child of node.children) {
      if (child.isDir && child.expanded) {
        await hydrateExpandedChildren(child, prevMap)
      }
    }
  } finally {
    node.loading = false
  }
}

async function refreshCurrentTree() {
  if (refreshing.value) return
  refreshing.value = true
  error.value = ''
  closeMenu()
  try {
    const prevMap = new Map<string, TreeNode>()
    buildNodeMap(roots.value, prevMap)
    const nextRoots = toTreeNodes(currentPath.value, await fetchFiles(currentPath.value), prevMap)
    for (const node of nextRoots) {
      if (node.isDir && node.expanded) {
        await hydrateExpandedChildren(node, prevMap)
      }
    }
    roots.value = nextRoots
    if (selectedPath.value && !hasPath(nextRoots, selectedPath.value)) {
      selectedPath.value = ''
    }
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    refreshing.value = false
  }
}

function clearWsReconnectTimer() {
  if (wsReconnectTimer != null) {
    window.clearTimeout(wsReconnectTimer)
    wsReconnectTimer = null
  }
}

function clearWsRefreshTimer() {
  if (wsRefreshTimer != null) {
    window.clearTimeout(wsRefreshTimer)
    wsRefreshTimer = null
  }
}

function scheduleWsRefresh() {
  clearWsRefreshTimer()
  wsRefreshTimer = window.setTimeout(() => {
    void refreshCurrentTree()
  }, 200)
}

function shouldRefreshForPath(path?: string): boolean {
  if (!path) return true
  const normalizedEvent = normalizePathForCompare(path)
  const normalizedCurrent = normalizePathForCompare(trimTrailingSeparators(currentPath.value))
  if (normalizedEvent === normalizedCurrent) return true
  if (normalizedEvent.startsWith(`${normalizedCurrent}/`)) return true
  if (!selectedPath.value) return false
  return normalizedEvent === normalizePathForCompare(selectedPath.value)
}

function onWsEvent(evt: DiffWsEvent) {
  if (evt.type === 'connected') {
    wsConnected.value = true
    wsStatus.value = 'connected'
    return
  }
  if (evt.type === 'refresh') {
    scheduleWsRefresh()
    if (selectedPath.value) previewRefreshKey.value += 1
    return
  }
  if (evt.type === 'file_changed') {
    if (shouldRefreshForPath(evt.path)) {
      scheduleWsRefresh()
    }
    if (evt.path && selectedPath.value) {
      const normalizedSelected = normalizePathForCompare(selectedPath.value)
      if (normalizePathForCompare(evt.path) === normalizedSelected) {
        previewRefreshKey.value += 1
      }
    }
  }
}

function connectWs() {
  clearWsReconnectTimer()
  wsStatus.value = 'connecting'
  wsConnected.value = false
  wsStopFn = connectDiffWs(onWsEvent, () => {
    wsConnected.value = false
    if (wsManualStop) {
      wsStatus.value = 'closed'
      return
    }
    wsStatus.value = 'retrying'
    clearWsReconnectTimer()
    wsReconnectTimer = window.setTimeout(() => {
      if (wsManualStop) return
      connectWs()
    }, 1200)
  })
}

function disconnectWs() {
  wsManualStop = true
  clearWsReconnectTimer()
  clearWsRefreshTimer()
  if (wsStopFn) {
    wsStopFn()
    wsStopFn = null
  }
  wsConnected.value = false
  wsStatus.value = 'closed'
}

function collapseAll() {
  function walk(nodes: TreeNode[]) {
    for (const node of nodes) {
      node.expanded = false
      if (node.children?.length) walk(node.children)
    }
  }
  walk(roots.value)
}

function clearSearch() {
  searchTerm.value = ''
  clearDeepSearchState()
}

function clearDeepSearchState() {
  searchSeq += 1
  searchBusy.value = false
  searchError.value = ''
  searchHits.value = []
}

function stopSearchDebounce() {
  if (searchDebounceTimer != null) {
    window.clearTimeout(searchDebounceTimer)
    searchDebounceTimer = null
  }
}

async function runDeepSearch() {
  const term = searchTerm.value.trim()
  if (!deepSearch.value || !term) {
    clearDeepSearchState()
    return
  }
  const seq = ++searchSeq
  searchBusy.value = true
  searchError.value = ''
  try {
    const res = await fetchFileSearch({
      root: currentPath.value,
      query: term,
      limit: deepSearchLimit,
    })
    if (seq !== searchSeq) return
    searchHits.value = res
  } catch (e: unknown) {
    if (seq !== searchSeq) return
    searchHits.value = []
    searchError.value = e instanceof Error ? e.message : String(e)
  } finally {
    if (seq === searchSeq) searchBusy.value = false
  }
}

function scheduleDeepSearch() {
  stopSearchDebounce()
  if (!deepSearch.value || !searchTerm.value.trim()) {
    clearDeepSearchState()
    return
  }
  searchDebounceTimer = window.setTimeout(() => {
    void runDeepSearch()
  }, deepSearchDebounceMs)
}

function resetListScroll() {
  rowVirtualizer.value.scrollToOffset(0)
  const el = listViewportRef.value
  if (el) {
    el.scrollTop = 0
  }
}

function rowClass(item: VirtualListItem) {
  if (item.kind === 'search') {
    return {
      'fm-entry--search': true,
      'fm-entry--dir': item.hit.is_dir,
      'fm-entry--selected': !item.hit.is_dir && selectedPath.value === item.hit.path,
    }
  }

  return {
    'fm-entry--tree': true,
    'fm-entry--dir': item.row.node.isDir,
    'fm-entry--selected': !item.row.node.isDir && selectedPath.value === item.row.node.path,
  }
}

function rowStyle(row: VirtualRenderRow) {
  const style: Record<string, string> = {
    position: 'absolute',
    left: '0',
    right: '0',
    top: '0',
    transform: `translateY(${row.virtual.start}px)`,
  }
  if (row.item.kind === 'tree') {
    style.paddingLeft = `calc(var(--space-3) + ${row.item.row.depth * 16}px)`
  }
  return style
}

function goUp() {
  const current = trimTrailingSeparators(currentPath.value)
  if (current === '/' || /^[A-Za-z]:\\$/.test(current)) return

  if (/^[A-Za-z]:$/.test(current)) {
    loadDirectory(`${current}\\`)
    return
  }

  const sep = resolveSeparator(current)
  const idx = current.lastIndexOf(sep)
  if (idx <= 0) {
    loadDirectory(sep === '\\' ? `${current.slice(0, 2)}\\` : '/')
    return
  }

  const parent = current.slice(0, idx)
  if (/^[A-Za-z]:$/.test(parent)) {
    loadDirectory(`${parent}\\`)
    return
  }
  loadDirectory(parent || '/')
}

async function toggleDirectory(node: TreeNode) {
  if (!node.isDir) return
  if (node.loading) return
  if (node.expanded) {
    node.expanded = false
    return
  }
  if (node.loaded) {
    node.expanded = true
    return
  }

  node.loading = true
  error.value = ''
  try {
    node.children = toTreeNodes(node.path, await fetchFiles(node.path))
    node.loaded = true
    node.expanded = true
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    node.loading = false
  }
}

function selectNode(node: TreeNode) {
  if (node.isDir) return
  selectedPath.value = node.path
}

async function onNodeClick(node: TreeNode) {
  if (node.isDir) {
    await toggleDirectory(node)
    return
  }
  selectNode(node)
}

function setSelected(target: 'old' | 'new', runAfter = false) {
  if (!selectedPath.value) return
  if (target === 'old') emit('update:oldPath', selectedPath.value)
  else emit('update:newPath', selectedPath.value)
  if (runAfter) emit('runDiff')
}

function setSelectedAsOld() {
  setSelected('old', false)
}

function setSelectedAsNew() {
  setSelected('new', false)
}

function setSelectedAsOldAndRun() {
  if (!canRunWithSelectedAsOld.value) return
  setSelected('old', true)
}

function setSelectedAsNewAndRun() {
  if (!canRunWithSelectedAsNew.value) return
  setSelected('new', true)
}

function openSelectedInConvert() {
  if (!canConvertSelected.value) return
  emit('openConvert', selectedPath.value)
}

function openContextMenu(event: MouseEvent, path: string, isDir: boolean) {
  event.preventDefault()
  if (isDir) return
  selectedPath.value = path
  const menuWidth = 220
  const menuHeight = 220
  menu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - menuWidth - 8),
    y: Math.min(event.clientY, window.innerHeight - menuHeight - 8),
    path,
  }
}

function onContextMenu(event: MouseEvent, node: TreeNode) {
  openContextMenu(event, node.path, node.isDir)
}

async function onSearchHitClick(hit: FileSearchEntry) {
  if (hit.is_dir) {
    await loadDirectory(hit.path)
    return
  }
  selectedPath.value = hit.path
}

function onSearchHitContextMenu(event: MouseEvent, hit: FileSearchEntry) {
  openContextMenu(event, hit.path, hit.is_dir)
}

function closeMenu() {
  if (!menu.value.visible) return
  menu.value.visible = false
}

function runContextAction(action: ContextAction) {
  if (!menu.value.path) return
  if (action === 'set-old-run' && !props.newPath.trim()) {
    closeMenu()
    return
  }
  if (action === 'set-new-run' && !props.oldPath.trim()) {
    closeMenu()
    return
  }
  if (action === 'open-convert' && !isConvertiblePath(menu.value.path)) {
    closeMenu()
    return
  }
  selectedPath.value = menu.value.path
  switch (action) {
    case 'open-preview':
      previewRefreshKey.value += 1
      break
    case 'set-old':
      setSelected('old', false)
      break
    case 'set-new':
      setSelected('new', false)
      break
    case 'set-old-run':
      setSelected('old', true)
      break
    case 'set-new-run':
      setSelected('new', true)
      break
    case 'open-convert':
      emit('openConvert', menu.value.path)
      break
    default:
      break
  }
  closeMenu()
}

function onGlobalPointerDown() {
  closeMenu()
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') closeMenu()
}

function stopAutoRefresh() {
  if (refreshTimer != null) {
    window.clearInterval(refreshTimer)
    refreshTimer = null
  }
}

function startAutoRefresh() {
  stopAutoRefresh()
  refreshTimer = window.setInterval(() => {
    void refreshCurrentTree()
  }, autoRefreshMs)
}

watch(
  () => autoRefresh.value,
  (enabled) => {
    if (enabled) startAutoRefresh()
    else stopAutoRefresh()
  },
)

watch(
  () => [searchTerm.value, deepSearch.value, currentPath.value],
  () => {
    scheduleDeepSearch()
    resetListScroll()
  },
)

watch(
  () => [activeItems.value.length, useDeepSearch.value],
  async () => {
    await nextTick()
    rowVirtualizer.value.measure()
  },
)

onMounted(() => {
  wsManualStop = false
  connectWs()
  loadDirectory(currentPath.value)
  window.addEventListener('pointerdown', onGlobalPointerDown)
  window.addEventListener('keydown', onGlobalKeydown)
})

onBeforeUnmount(() => {
  disconnectWs()
  stopAutoRefresh()
  stopSearchDebounce()
  window.removeEventListener('pointerdown', onGlobalPointerDown)
  window.removeEventListener('keydown', onGlobalKeydown)
})
</script>

<template>
  <div class="fm">
    <div class="fm-header">
      <div class="fm-title-wrap">
        <h3 class="fm-title">File Manager</h3>
        <span class="fm-subtitle">Right click a file to set diff side</span>
      </div>
      <div class="fm-header-actions">
        <span class="fm-ws-state" :class="{ 'fm-ws-state--live': wsConnected }">{{ wsLabel }}</span>
        <button class="fm-btn fm-btn--muted" @click="goUp">Up</button>
        <button class="fm-btn fm-btn--muted" :disabled="refreshing" @click="refreshCurrentTree">
          {{ refreshing ? 'Refreshing...' : 'Refresh' }}
        </button>
      </div>
    </div>

    <div class="fm-path-row">
      <input
        v-model="pathInput"
        type="text"
        class="fm-path-input"
        placeholder="Directory path..."
        @keydown.enter="loadDirectory(pathInput)"
      />
      <button class="fm-btn fm-btn--primary" @click="loadDirectory(pathInput)">Open</button>
    </div>

    <div class="fm-tools-row">
      <input
        v-model="searchTerm"
        type="text"
        class="fm-path-input"
        placeholder="Filter loaded tree by name or path..."
      />
      <label class="fm-auto-refresh">
        <input v-model="deepSearch" type="checkbox" />
        <span>Deep Search</span>
      </label>
      <button class="fm-btn fm-btn--muted" :disabled="!searchTerm" @click="clearSearch">Clear</button>
      <button class="fm-btn fm-btn--muted" @click="collapseAll">Collapse</button>
      <label class="fm-auto-refresh">
        <input v-model="autoRefresh" type="checkbox" />
        <span>Auto Refresh (5s)</span>
      </label>
    </div>

    <div class="fm-active">
      <div class="fm-picked">
        <span class="fm-picked-label">Selected</span>
        <span class="fm-picked-path">{{ selectedPath || '-' }}</span>
      </div>
      <div class="fm-picked-actions">
        <button class="fm-btn fm-btn--muted" :disabled="!canUseSelected" @click="setSelectedAsOld">
          Set as Old
        </button>
        <button class="fm-btn fm-btn--muted" :disabled="!canUseSelected" @click="setSelectedAsNew">
          Set as New
        </button>
        <button
          class="fm-btn fm-btn--accent"
          :disabled="!canRunWithSelectedAsOld"
          @click="setSelectedAsOldAndRun"
        >
          Use as Old + Diff
        </button>
        <button
          class="fm-btn fm-btn--accent"
          :disabled="!canRunWithSelectedAsNew"
          @click="setSelectedAsNewAndRun"
        >
          Use as New + Diff
        </button>
        <button
          class="fm-btn fm-btn--muted"
          :disabled="!canConvertSelected"
          @click="openSelectedInConvert"
        >
          Open Convert
        </button>
      </div>
    </div>

    <DiffFilePreview :path="selectedPath" :refresh-key="previewRefreshKey" />

    <div class="fm-list-wrap">
      <div v-if="loading" class="fm-status">Loading...</div>
      <div v-else-if="error" class="fm-status fm-status--error">{{ error }}</div>
      <div v-else-if="useDeepSearch && searchBusy" class="fm-status">Searching recursively...</div>
      <div v-else-if="useDeepSearch && searchError" class="fm-status fm-status--error">{{ searchError }}</div>
      <div v-else-if="activeItems.length === 0" class="fm-status">No matched entries</div>
      <div v-else class="fm-list-viewport" ref="listViewportRef">
        <ul class="fm-list fm-list--virtual" :style="{ height: `${virtualTotalSize}px` }">
          <li
            v-for="row in virtualRows"
            :key="row.item.key"
            class="fm-entry"
            :class="rowClass(row.item)"
            :style="rowStyle(row)"
            @click="
              row.item.kind === 'search'
                ? onSearchHitClick(row.item.hit)
                : onNodeClick(row.item.row.node)
            "
            @contextmenu="
              row.item.kind === 'search'
                ? onSearchHitContextMenu($event, row.item.hit)
                : onContextMenu($event, row.item.row.node)
            "
          >
            <template v-if="row.item.kind === 'search'">
              <span class="fm-twist fm-twist--placeholder"></span>
              <span class="fm-icon">{{ row.item.hit.is_dir ? 'DIR' : 'FILE' }}</span>
              <div class="fm-name-wrap">
                <span class="fm-name">{{ row.item.hit.name }}</span>
                <span class="fm-subpath">{{ row.item.hit.path }}</span>
              </div>
              <span
                v-if="!row.item.hit.is_dir && props.oldPath === row.item.hit.path"
                class="fm-badge fm-badge--old"
              >
                OLD
              </span>
              <span
                v-if="!row.item.hit.is_dir && props.newPath === row.item.hit.path"
                class="fm-badge fm-badge--new"
              >
                NEW
              </span>
              <span v-if="!row.item.hit.is_dir" class="fm-size">{{ formatSize(row.item.hit.size) }}</span>
            </template>
            <template v-else>
              <span
                class="fm-twist"
                :class="{ 'fm-twist--placeholder': !row.item.row.node.isDir }"
              >
                <template v-if="row.item.row.node.isDir">
                  {{ row.item.row.node.loading ? '…' : row.item.row.node.expanded ? '▾' : '▸' }}
                </template>
              </span>
              <span class="fm-icon">{{ row.item.row.node.isDir ? 'DIR' : 'FILE' }}</span>
              <span class="fm-name">{{ row.item.row.node.name }}</span>
              <span
                v-if="!row.item.row.node.isDir && props.oldPath === row.item.row.node.path"
                class="fm-badge fm-badge--old"
              >
                OLD
              </span>
              <span
                v-if="!row.item.row.node.isDir && props.newPath === row.item.row.node.path"
                class="fm-badge fm-badge--new"
              >
                NEW
              </span>
              <span v-if="!row.item.row.node.isDir" class="fm-size">{{ formatSize(row.item.row.node.size) }}</span>
            </template>
          </li>
        </ul>
      </div>
    </div>

    <Teleport to="body">
      <ul
        v-if="menu.visible"
        class="fm-menu"
        :style="{ top: `${menu.y}px`, left: `${menu.x}px` }"
        @pointerdown.stop
      >
        <li class="fm-menu-item" @click="runContextAction('open-preview')">Preview</li>
        <li class="fm-menu-divider"></li>
        <li class="fm-menu-item" @click="runContextAction('set-old')">Set as Old</li>
        <li class="fm-menu-item" @click="runContextAction('set-new')">Set as New</li>
        <li class="fm-menu-divider"></li>
        <li
          class="fm-menu-item"
          :class="{ disabled: !props.newPath.trim() }"
          @click="runContextAction('set-old-run')"
        >
          Use as Old + Run Diff
        </li>
        <li
          class="fm-menu-item"
          :class="{ disabled: !props.oldPath.trim() }"
          @click="runContextAction('set-new-run')"
        >
          Use as New + Run Diff
        </li>
        <li class="fm-menu-divider"></li>
        <li
          class="fm-menu-item"
          :class="{ disabled: !isConvertiblePath(menu.path) }"
          @click="runContextAction('open-convert')"
        >
          Open Convert
        </li>
      </ul>
    </Teleport>
  </div>
</template>

<style scoped>
.fm {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: var(--surface-card-muted);
}

.fm-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-2);
}

.fm-title-wrap {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fm-title {
  font: var(--type-body-sm);
  color: var(--text-primary);
}

.fm-subtitle {
  font: var(--type-caption);
  color: var(--text-tertiary);
}

.fm-header-actions {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}

.fm-ws-state {
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: 0 var(--space-1);
  color: var(--text-secondary);
  background: var(--surface-panel);
  font: var(--type-caption);
  white-space: nowrap;
}

.fm-ws-state--live {
  color: var(--color-success);
  background: var(--color-success-bg);
  border-color: var(--color-success);
}

.fm-path-row {
  display: flex;
  gap: var(--space-2);
}

.fm-tools-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.fm-path-input {
  flex: 1;
  min-width: 0;
  font-family: var(--font-family-mono);
}

.fm-auto-refresh {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-secondary);
  font: var(--type-body-sm);
  white-space: nowrap;
}

.fm-auto-refresh input {
  accent-color: var(--color-info);
}

.fm-active {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-panel);
}

.fm-picked {
  display: grid;
  grid-template-columns: 56px 1fr;
  gap: var(--space-2);
  align-items: center;
}

.fm-picked-label {
  font: var(--type-caption);
  color: var(--text-secondary);
}

.fm-picked-path {
  font: var(--type-body-sm);
  color: var(--text-primary);
  font-family: var(--font-family-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fm-picked-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.fm-list-wrap {
  border: var(--border);
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--surface-panel);
}

.fm-list-viewport {
  max-height: 300px;
  overflow: auto;
}

.fm-status {
  padding: var(--space-4);
  text-align: center;
  font: var(--type-body-sm);
  color: var(--text-secondary);
}

.fm-status--error {
  color: var(--color-danger);
}

.fm-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.fm-list--virtual {
  position: relative;
}

.fm-entry {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  cursor: pointer;
  border-bottom: var(--border);
  user-select: none;
}

.fm-entry:last-child {
  border-bottom: none;
}

.fm-entry--tree {
  min-height: 34px;
}

.fm-entry--search {
  min-height: 44px;
}

.fm-entry:hover {
  background: var(--gray-alpha-100);
}

.fm-entry--selected {
  background: var(--color-info-bg);
}

.fm-entry--dir {
  font-weight: var(--weight-medium);
}

.fm-twist {
  width: 14px;
  color: var(--text-tertiary);
  font: var(--type-caption);
  text-align: center;
  flex-shrink: 0;
}

.fm-twist--placeholder {
  opacity: 0.3;
}

.fm-icon {
  min-width: 36px;
  color: var(--text-tertiary);
  font: var(--type-caption);
}

.fm-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font: var(--type-body-sm);
}

.fm-name-wrap {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fm-subpath {
  color: var(--text-tertiary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fm-size {
  color: var(--text-tertiary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
}

.fm-badge {
  padding: 1px var(--space-1);
  border-radius: var(--radius-full);
  font: var(--type-caption);
  border: 1px solid transparent;
}

.fm-badge--old {
  color: var(--color-warning);
  background: var(--color-warning-bg);
  border-color: var(--color-warning);
}

.fm-badge--new {
  color: var(--color-success);
  background: var(--color-success-bg);
  border-color: var(--color-success);
}

.fm-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-1);
  padding: var(--comp-padding-xs);
  border: var(--border);
  border-radius: var(--radius-sm);
  font: var(--type-body-sm);
  line-height: 1;
  cursor: pointer;
  transition: background var(--duration-fast) ease, color var(--duration-fast) ease, opacity var(--duration-fast) ease;
}

.fm-btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

.fm-btn--muted {
  background: var(--surface-panel);
  color: var(--text-secondary);
}

.fm-btn--muted:hover:not(:disabled) {
  background: var(--gray-alpha-100);
  color: var(--text-primary);
}

.fm-btn--primary {
  background: var(--color-info-bg);
  color: var(--color-info);
}

.fm-btn--primary:hover:not(:disabled) {
  background: var(--color-info);
  color: var(--ds-background-1);
}

.fm-btn--accent {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.fm-btn--accent:hover:not(:disabled) {
  background: var(--color-success);
  color: var(--ds-background-1);
}

.fm-menu {
  position: fixed;
  z-index: var(--z-modal);
  min-width: 220px;
  list-style: none;
  margin: 0;
  padding: var(--space-1);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-panel);
  box-shadow: var(--shadow-md);
}

.fm-menu-item {
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  font: var(--type-body-sm);
  color: var(--text-primary);
  cursor: pointer;
}

.fm-menu-item:hover {
  background: var(--gray-alpha-100);
}

.fm-menu-item.disabled {
  color: var(--text-tertiary);
}

.fm-menu-item.disabled:hover {
  background: transparent;
}

.fm-menu-divider {
  height: 1px;
  margin: var(--space-1) 0;
  background: var(--color-border-strong);
}

@media (max-width: 768px) {
  .fm {
    padding: var(--space-2);
  }

  .fm-header {
    flex-direction: column;
    align-items: stretch;
  }

  .fm-header-actions {
    justify-content: flex-end;
  }

  .fm-path-row {
    flex-direction: column;
  }

  .fm-tools-row {
    align-items: stretch;
  }

  .fm-picked {
    grid-template-columns: 1fr;
  }
}
</style>
