<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { IconGripVertical, IconPlus, IconTrash, IconRefresh } from '@tabler/icons-vue'
import { Button } from './button'
import type { RedirectConfig, RedirectProfile, RedirectDryRunResponse } from '../types'
import { deleteRedirectProfile, fetchRedirectProfiles, redirectDryRun, upsertRedirectProfile } from '../api'

type RuleDraft = {
  name: string
  extCsv: string
  glob: string
  regex: string
  size: string
  age: string
  dest: string
}

const cfg = ref<RedirectConfig>({ profiles: {} })
const profileName = ref('default')
const ruleDrafts = ref<RuleDraft[]>([])
const unmatchedMode = ref<'skip' | 'archive'>('skip')
const archiveAge = ref('>=30d')
const archiveDest = ref('./Others')
const onConflict = ref<'rename_new' | 'rename_date' | 'hash_dedup' | 'rename_existing' | 'trash' | 'skip' | 'overwrite'>('rename_new')
const recursive = ref(false)
const maxDepth = ref(1)
const newProfileName = ref('')
const dragIndex = ref<number | null>(null)
const lastError = ref('')
const dryError = ref('')
const savedSnapshot = ref<string | null>(null)
const savedProfile = ref<RedirectProfile | null>(null)
const lastProfileName = ref(profileName.value)
const drySource = ref('')
const dryCopy = ref(false)
const dryBusy = ref(false)
const dryResp = ref<RedirectDryRunResponse | null>(null)
const deleteProfileBusy = ref(false)
const confirmKey = ref<string | null>(null)
const confirmRemaining = ref(0)
let confirmTimer: number | null = null

const CONFIRM_WINDOW_SEC = 3
const confirmProfileKey = (name: string) => `profile:${name}`
const confirmRuleKey = (idx: number) => `rule:${idx}`

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

const profileNames = computed(() => Object.keys(cfg.value.profiles).sort())
const currentProfile = computed(() => draftToProfile())
const isDirty = computed(() => {
  if (savedSnapshot.value === null) return false
  return savedSnapshot.value !== JSON.stringify(currentProfile.value)
})
const isUnmatchedChanged = computed(() => {
  if (!savedProfile.value) return false
  return savedProfile.value.unmatched !== currentProfile.value.unmatched
})
const isOnConflictChanged = computed(() => {
  if (!savedProfile.value) return false
  return savedProfile.value.on_conflict !== currentProfile.value.on_conflict
})
const isRecursiveChanged = computed(() => {
  if (!savedProfile.value) return false
  return !!savedProfile.value.recursive !== recursive.value
})
const isMaxDepthChanged = computed(() => {
  if (!savedProfile.value) return false
  const savedDepth = Number(savedProfile.value.max_depth || 1)
  const currentDepth = Math.max(1, Number(maxDepth.value || 1))
  return savedDepth !== currentDepth
})

function normalizeCsv(csv: string): string[] {
  return csv
    .split(',')
    .map(s => s.trim())
    .filter(Boolean)
}

function profileToDraft(p: RedirectProfile) {
  const u = (p.unmatched as any) || 'skip'
  if (typeof u === 'string' && u.toLowerCase().startsWith('archive:')) {
    unmatchedMode.value = 'archive'
    const rest = u.slice('archive:'.length)
    const i1 = rest.indexOf(':')
    archiveAge.value = i1 >= 0 ? rest.slice(0, i1) || '>=30d' : '>=30d'
    archiveDest.value = i1 >= 0 ? rest.slice(i1 + 1) || './Others' : './Others'
  } else {
    unmatchedMode.value = 'skip'
  }
  onConflict.value = (p.on_conflict as any) || 'rename_new'
  recursive.value = !!(p.recursive as any)
  maxDepth.value = Number((p.max_depth as any) || 1) || 1
  ruleDrafts.value = (p.rules || []).map(r => ({
    name: r.name || '',
    extCsv: (r.match?.ext || []).join(', '),
    glob: (r.match?.glob as any) || '',
    regex: (r.match?.regex as any) || '',
    size: (r.match?.size as any) || '',
    age: (r.match?.age as any) || '',
    dest: r.dest || '',
  }))
}

function draftToProfile(): RedirectProfile {
  const unmatched =
    unmatchedMode.value === 'skip'
      ? 'skip'
      : `archive:${archiveAge.value || ''}:${archiveDest.value || ''}`
  return {
    rules: ruleDrafts.value.map(r => ({
      name: r.name,
      match: {
        ext: normalizeCsv(r.extCsv),
        glob: r.glob.trim() ? r.glob.trim() : null,
        regex: r.regex.trim() ? r.regex.trim() : null,
        size: r.size.trim() ? r.size.trim() : null,
        age: r.age.trim() ? r.age.trim() : null,
      },
      dest: r.dest,
    })),
    unmatched,
    on_conflict: onConflict.value,
    recursive: recursive.value,
    max_depth: Math.max(1, Number(maxDepth.value || 1)),
  }
}

function markSavedSnapshot() {
  const snapshot = JSON.stringify(currentProfile.value)
  savedSnapshot.value = snapshot
  savedProfile.value = JSON.parse(snapshot) as RedirectProfile
}

function confirmDiscardIfDirty(): boolean {
  if (!isDirty.value) return true
  return confirm('You have unsaved changes. Discard them?')
}

async function load(force = false) {
  if (!force && !confirmDiscardIfDirty()) return
  lastError.value = ''
  try {
    cfg.value = await fetchRedirectProfiles()
    if (!cfg.value.profiles[profileName.value]) {
      profileName.value = profileNames.value[0] || 'default'
    }
    const p = cfg.value.profiles[profileName.value]
    if (p) {
      profileToDraft(p)
    } else {
      unmatchedMode.value = 'skip'
      archiveAge.value = '>=30d'
      archiveDest.value = './Others'
      onConflict.value = 'rename_new'
      recursive.value = false
      maxDepth.value = 1
      ruleDrafts.value = []
    }
    markSavedSnapshot()
    lastProfileName.value = profileName.value
  } catch (e: any) {
    lastError.value = e?.message || String(e)
  }
}

async function onSave() {
  lastError.value = ''
  try {
    await upsertRedirectProfile(profileName.value, draftToProfile())
    await load(true)
  } catch (e: any) {
    lastError.value = e?.message || String(e)
  }
}

async function onDeleteProfile() {
  if (!confirmDiscardIfDirty()) return
  const key = confirmProfileKey(profileName.value)
  if (!isConfirmArmed(key)) {
    armConfirm(key)
    return
  }
  resetConfirm()
  lastError.value = ''
  deleteProfileBusy.value = true
  try {
    await deleteRedirectProfile(profileName.value)
    profileName.value = 'default'
    await load(true)
  } catch (e: any) {
    lastError.value = e?.message || String(e)
  } finally {
    deleteProfileBusy.value = false
  }
}

function onAddRule() {
  ruleDrafts.value.push({ name: '', extCsv: '', glob: '', regex: '', size: '', age: '', dest: '' })
}

function onDeleteRule(idx: number) {
  const key = confirmRuleKey(idx)
  if (!isConfirmArmed(key)) {
    armConfirm(key)
    return
  }
  resetConfirm()
  ruleDrafts.value.splice(idx, 1)
}

async function onCreateProfile() {
  if (!confirmDiscardIfDirty()) return
  const name = newProfileName.value.trim()
  if (!name) return
  if (cfg.value.profiles[name]) {
    alert('Profile already exists.')
    return
  }
  lastError.value = ''
  try {
  await upsertRedirectProfile(name, {
    rules: [{ name: 'Images', match: { ext: ['jpg'], glob: null, regex: null, size: null, age: null }, dest: './Images' }],
    unmatched: 'skip',
    on_conflict: 'rename_new',
  })
    newProfileName.value = ''
    profileName.value = name
    await load(true)
  } catch (e: any) {
    lastError.value = e?.message || String(e)
  }
}

async function onProfileChange() {
  if (!confirmDiscardIfDirty()) {
    profileName.value = lastProfileName.value
    return
  }
  await load(true)
  lastProfileName.value = profileName.value
}

function onDragStart(idx: number, e: DragEvent) {
  dragIndex.value = idx
  try {
    e.dataTransfer?.setData('text/plain', String(idx))
    e.dataTransfer?.setDragImage(new Image(), 0, 0)
  } catch {}
}

function onDrop(idx: number) {
  const from = dragIndex.value
  dragIndex.value = null
  if (from === null || from === idx) return
  if (from < 0 || from >= ruleDrafts.value.length) return
  if (idx < 0 || idx >= ruleDrafts.value.length) return
  const arr = ruleDrafts.value.slice()
  const [item] = arr.splice(from, 1)
  arr.splice(idx, 0, item)
  ruleDrafts.value = arr
}

function isRuleChanged(idx: number) {
  if (!savedProfile.value) return false
  const base = savedProfile.value.rules?.[idx]
  const current = currentProfile.value.rules?.[idx]
  if (!base || !current) return true
  return JSON.stringify(base) !== JSON.stringify(current)
}

const validationErrors = computed(() => {
  const errs: string[] = []
  if (!profileName.value.trim()) errs.push('profile name is empty')
  if (unmatchedMode.value === 'archive') {
    if (!archiveDest.value.trim()) errs.push('archive dest is empty')
    if (!archiveAge.value.trim()) errs.push('archive age is empty')
  }
  if (recursive.value) {
    if (!Number.isFinite(Number(maxDepth.value)) || Number(maxDepth.value) < 1) errs.push('max depth must be >= 1')
  }
  ruleDrafts.value.forEach((r, idx) => {
    const hasExt = normalizeCsv(r.extCsv).length > 0
    const hasGlob = !!r.glob.trim()
    const hasRegex = !!r.regex.trim()
    const hasSize = !!r.size.trim()
    const hasAge = !!r.age.trim()
    if (!hasExt && !hasGlob && !hasRegex && !hasSize && !hasAge) errs.push(`rule[${idx + 1}] match is empty`)
    if (!r.dest.trim()) errs.push(`rule[${idx + 1}] dest is empty`)
  })
  return errs
})

function isRuleInvalid(r: RuleDraft) {
  const hasExt = normalizeCsv(r.extCsv).length > 0
  const hasGlob = !!r.glob.trim()
  const hasRegex = !!r.regex.trim()
  const hasSize = !!r.size.trim()
  const hasAge = !!r.age.trim()
  return !hasExt && !hasGlob && !hasRegex && !hasSize && !hasAge
}

function onBeforeUnload(e: BeforeUnloadEvent) {
  if (!isDirty.value) return
  e.preventDefault()
  e.returnValue = ''
}

async function runDryRun() {
  dryError.value = ''
  const source = drySource.value.trim()
  if (!source) {
    dryError.value = 'source path is empty'
    return
  }
  dryBusy.value = true
  try {
    dryResp.value = await redirectDryRun({
      source,
      profile: currentProfile.value,
      copy: dryCopy.value,
    })
  } catch (e: any) {
    dryError.value = e?.message || String(e)
  } finally {
    dryBusy.value = false
  }
}

function dryRowClass(result: string) {
  const key = result?.toLowerCase()
  if (key === 'failed') return 'dry-row-failed'
  if (key === 'skipped') return 'dry-row-skipped'
  if (key === 'dry_run') return 'dry-row-ok'
  return ''
}

watch(isDirty, dirty => {
  if (dirty) {
    window.addEventListener('beforeunload', onBeforeUnload)
  } else {
    window.removeEventListener('beforeunload', onBeforeUnload)
  }
})

onMounted(load)
onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', onBeforeUnload)
  stopConfirmTimer()
})
</script>

<template>
  <div>
    <div class="toolbar">
      <select v-model="profileName" @change="onProfileChange">
        <option v-for="n in profileNames" :key="n" :value="n">{{ n }}</option>
      </select>
      <Button size="sm" preset="secondary" title="Reload" square @click="load">
        <IconRefresh :size="16" />
      </Button>
      <Button size="sm" preset="primary" :disabled="validationErrors.length > 0" @click="onSave">Save</Button>
      <Button
        size="sm"
        preset="danger"
        square
        class="btn--confirm"
        :loading="deleteProfileBusy"
        :disabled="deleteProfileBusy"
        :title="isConfirmArmed(confirmProfileKey(profileName)) ? `Confirm (${confirmRemaining}s)` : 'Delete profile'"
        @click="onDeleteProfile"
      >
        <IconTrash :size="16" />
        <span v-if="isConfirmArmed(confirmProfileKey(profileName))" class="btn__confirm-badge">
          {{ confirmRemaining }}
        </span>
      </Button>
      <div style="flex:1" />
      <input v-model="newProfileName" placeholder="New profile name" style="max-width:220px" />
      <Button size="sm" preset="secondary" style="display:flex;align-items:center;gap:var(--space-1)" @click="onCreateProfile">
        <IconPlus :size="16" />
        Create
      </Button>
    </div>

    <div v-if="isDirty" class="dirtyBanner">Unsaved changes</div>
    <div v-if="validationErrors.length" class="validation">
      <div class="validationTitle">Fix before saving</div>
      <ul class="validationList">
        <li v-for="e in validationErrors" :key="e">{{ e }}</li>
      </ul>
    </div>
    <div v-if="lastError" class="errorBanner">{{ lastError }}</div>
    <div v-if="dryError" class="errorBanner">{{ dryError }}</div>

    <div style="display:flex;gap:var(--space-3);margin-bottom:var(--space-4);align-items:center">
      <div>
        <div style="font-size:var(--text-xs);color:var(--text-secondary);margin-bottom:var(--space-1)">Unmatched</div>
        <select v-model="unmatchedMode" :class="{ 'changed-field': isUnmatchedChanged }">
          <option value="skip">skip</option>
          <option value="archive">archive</option>
        </select>
      </div>
      <div v-if="unmatchedMode === 'archive'">
        <div style="font-size:var(--text-xs);color:var(--text-secondary);margin-bottom:var(--space-1)">Archive age</div>
        <input v-model="archiveAge" :class="{ 'changed-field': isUnmatchedChanged }" placeholder=">=30d" style="width:120px" />
      </div>
      <div v-if="unmatchedMode === 'archive'">
        <div style="font-size:var(--text-xs);color:var(--text-secondary);margin-bottom:var(--space-1)">Archive dest</div>
        <input v-model="archiveDest" :class="{ 'changed-field': isUnmatchedChanged }" placeholder="./Others" style="width:160px" />
      </div>
      <div>
        <div style="font-size:var(--text-xs);color:var(--text-secondary);margin-bottom:var(--space-1)">On conflict</div>
        <select v-model="onConflict" :class="{ 'changed-field': isOnConflictChanged }">
          <option value="rename_new">rename_new</option>
          <option value="rename_date">rename_date</option>
          <option value="hash_dedup">hash_dedup</option>
          <option value="rename_existing">rename_existing</option>
          <option value="trash">trash</option>
          <option value="skip">skip</option>
          <option value="overwrite">overwrite</option>
        </select>
      </div>
      <div>
        <div style="font-size:var(--text-xs);color:var(--text-secondary);margin-bottom:var(--space-1)">Recursive</div>
        <select v-model="recursive" :class="{ 'changed-field': isRecursiveChanged }">
          <option :value="false">false</option>
          <option :value="true">true</option>
        </select>
      </div>
      <div v-if="recursive">
        <div style="font-size:var(--text-xs);color:var(--text-secondary);margin-bottom:var(--space-1)">Max depth</div>
        <input v-model.number="maxDepth" :class="{ 'changed-field': isMaxDepthChanged }" type="number" min="1" style="width:90px" />
      </div>
      <div style="flex:1" />
      <Button size="sm" preset="secondary" style="display:flex;align-items:center;gap:var(--space-1)" @click="onAddRule">
        <IconPlus :size="16" />
        Add rule
      </Button>
    </div>

    <div class="dry-panel">
      <div class="dry-title">Dry run</div>
      <div class="dry-toolbar">
        <input v-model="drySource" placeholder="Source path (local)" style="flex:1" />
        <label class="toggle">
          <input type="checkbox" v-model="dryCopy" /> Copy
        </label>
        <Button size="sm" preset="primary" :disabled="dryBusy" :loading="dryBusy" @click="runDryRun">Run</Button>
      </div>
      <div v-if="dryResp" class="dry-stats">
        <div class="stat">
          <div class="k">Total</div>
          <div class="v">{{ dryResp.stats.total }}</div>
        </div>
        <div class="stat">
          <div class="k">Dry run</div>
          <div class="v">{{ dryResp.stats.dry_run }}</div>
        </div>
        <div class="stat">
          <div class="k">Skipped</div>
          <div class="v">{{ dryResp.stats.skipped }}</div>
        </div>
        <div class="stat">
          <div class="k">Failed</div>
          <div class="v">{{ dryResp.stats.failed }}</div>
        </div>
      </div>
      <table v-if="dryResp">
        <thead>
          <tr>
            <th style="width:110px">Result</th>
            <th style="width:120px">Action</th>
            <th style="width:160px">Rule</th>
            <th>Source</th>
            <th>Destination</th>
            <th>Reason</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(r, idx) in dryResp.results" :key="idx" :class="dryRowClass(r.result)">
            <td>{{ r.result }}</td>
            <td>{{ r.action }}</td>
            <td>{{ r.rule || '-' }}</td>
            <td style="color:var(--text-secondary)">{{ r.src }}</td>
            <td style="color:var(--text-secondary)">{{ r.dst || '-' }}</td>
            <td style="color:var(--text-tertiary)">{{ r.reason || '-' }}</td>
          </tr>
          <tr v-if="dryResp && dryResp.results.length === 0">
            <td colspan="6" style="text-align:center;color:var(--text-tertiary)">No dry-run results</td>
          </tr>
        </tbody>
      </table>
    </div>

    <table>
      <thead>
        <tr>
          <th style="width:36px"></th>
          <th style="width:160px">Name</th>
          <th style="width:260px">Ext (csv)</th>
          <th style="width:240px">Glob</th>
          <th style="width:240px">Regex</th>
          <th style="width:160px">Size</th>
          <th style="width:140px">Age</th>
          <th>Dest</th>
          <th style="width:60px"></th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(r, idx) in ruleDrafts"
          :key="idx"
          class="ruleRow"
          :class="{ dragging: dragIndex === idx, changed: isRuleChanged(idx) }"
          @dragover.prevent
          @drop.prevent="onDrop(idx)"
        >
          <td>
            <span
              class="dragHandle"
              draggable="true"
              title="Drag to reorder"
              @dragstart="onDragStart(idx, $event)"
              @dragend="dragIndex = null"
            >
              <IconGripVertical :size="16" />
            </span>
          </td>
          <td><input v-model="r.name" placeholder="Rule name" /></td>
          <td><input v-model="r.extCsv" :class="{ invalid: isRuleInvalid(r) }" placeholder="jpg, png, ..." /></td>
          <td><input v-model="r.glob" :class="{ invalid: isRuleInvalid(r) }" placeholder="**/*.jpg" /></td>
          <td><input v-model="r.regex" :class="{ invalid: isRuleInvalid(r) }" placeholder="^\\d{4}-\\d{2}" /></td>
          <td><input v-model="r.size" :class="{ invalid: isRuleInvalid(r) }" placeholder=">10MB" /></td>
          <td><input v-model="r.age" :class="{ invalid: isRuleInvalid(r) }" placeholder=">30d" /></td>
          <td><input v-model="r.dest" :class="{ invalid: !r.dest.trim() }" placeholder="./Images" style="width:100%" /></td>
          <td>
            <Button
              size="sm"
              preset="danger"
              square
              class="btn--confirm"
              :title="isConfirmArmed(confirmRuleKey(idx)) ? `Confirm (${confirmRemaining}s)` : 'Delete rule'"
              @click="onDeleteRule(idx)"
            >
              <IconTrash :size="16" />
              <span v-if="isConfirmArmed(confirmRuleKey(idx))" class="btn__confirm-badge">
                {{ confirmRemaining }}
              </span>
            </Button>
          </td>
        </tr>
        <tr v-if="!ruleDrafts.length">
          <td colspan="9" style="text-align:center;color:var(--text-tertiary)">No rules</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.dragHandle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  cursor: grab;
  user-select: none;
}
.dragHandle:hover {
  background: var(--ds-background-2);
  color: var(--text-secondary);
}
.dragging .dragHandle {
  cursor: grabbing;
}

.dirtyBanner {
  margin-bottom: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-warning);
  background: var(--color-warning-bg);
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.changed-field {
  border-color: var(--color-warning);
  background: var(--color-warning-bg);
}

.ruleRow.changed {
  background: var(--color-warning-bg);
}

.invalid {
  border-color: rgba(255, 79, 79, 0.65);
}

.validation {
  border: var(--border);
  background: var(--ds-background-1);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  margin-bottom: var(--space-4);
}
.validationTitle {
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  margin-bottom: var(--space-2);
}
.validationList {
  margin-left: var(--space-5);
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.errorBanner {
  margin-bottom: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
  border: var(--border);
  background: rgba(255, 79, 79, 0.08);
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.dry-panel {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  margin-bottom: var(--space-4);
  background: var(--ds-background-1);
}
.dry-title {
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  margin-bottom: var(--space-2);
}
.dry-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}
.dry-stats {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}
.stat {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
  background: var(--ds-background-2);
}
.stat .k {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}
.stat .v {
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
}
.dry-row-ok {
  background: rgba(64, 170, 80, 0.08);
}
.dry-row-skipped {
  background: rgba(255, 193, 7, 0.08);
}
.dry-row-failed {
  background: rgba(255, 79, 79, 0.08);
}
</style>
