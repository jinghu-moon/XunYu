<script setup lang="ts">
const model = defineModel<{
  mode: string
  algorithm: string
  context: number
  ignore_space_change: boolean
  ignore_all_space: boolean
  ignore_blank_lines: boolean
  strip_trailing_cr: boolean
  force_text: boolean
}>({ required: true })
</script>

<template>
  <div class="diff-options">
    <div class="options-row">
      <label class="opt-field">
        <span class="opt-label">Mode</span>
        <select v-model="model.mode">
          <option value="auto">Auto</option>
          <option value="line">Line</option>
          <option value="ast">AST</option>
        </select>
      </label>
      <label class="opt-field">
        <span class="opt-label">Algorithm</span>
        <select v-model="model.algorithm">
          <option value="histogram">Histogram</option>
          <option value="myers">Myers</option>
          <option value="minimal">Minimal</option>
          <option value="patience">Patience</option>
        </select>
      </label>
      <label class="opt-field">
        <span class="opt-label">Context</span>
        <input
          v-model.number="model.context"
          type="number"
          min="0"
          max="50"
          class="ctx-input"
        />
      </label>
    </div>
    <div class="options-row">
      <label class="opt-check">
        <input type="checkbox" v-model="model.ignore_space_change" />
        <span>Ignore space change</span>
      </label>
      <label class="opt-check">
        <input type="checkbox" v-model="model.ignore_all_space" />
        <span>Ignore all space</span>
      </label>
      <label class="opt-check">
        <input type="checkbox" v-model="model.ignore_blank_lines" />
        <span>Ignore blank lines</span>
      </label>
      <label class="opt-check">
        <input type="checkbox" v-model="model.strip_trailing_cr" />
        <span>Strip trailing CR</span>
      </label>
      <label class="opt-check">
        <input type="checkbox" v-model="model.force_text" />
        <span>Force text</span>
      </label>
    </div>
  </div>
</template>

<style scoped>
.diff-options {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.options-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  align-items: center;
}

.opt-field {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.opt-label {
  font: var(--type-body-sm);
  color: var(--text-secondary);
  white-space: nowrap;
}

.opt-field select,
.opt-field input {
  padding: var(--comp-padding-xs);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-panel);
  color: var(--text-primary);
  font: var(--type-body-sm);
  outline: none;
  transition: border-color var(--duration-fast) ease;
}

.opt-field select:focus,
.opt-field input:focus {
  border-color: var(--text-secondary);
}

.ctx-input {
  width: 60px;
}

.opt-check {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  cursor: pointer;
  font: var(--type-body-sm);
  color: var(--text-secondary);
  white-space: nowrap;
}

.opt-check input[type="checkbox"] {
  accent-color: var(--color-info);
  cursor: pointer;
}

.opt-check:hover {
  color: var(--text-primary);
}
</style>
