<script setup lang="ts">
import type { TaskFieldDefinition, TaskFieldValue, TaskFormState } from '../features/tasks'

const props = defineProps<{
  fields: TaskFieldDefinition[]
  form: TaskFormState
}>()

const emit = defineEmits<{
  (event: 'update-field', payload: { key: string; value: TaskFieldValue }): void
}>()

function updateField(key: string, value: TaskFieldValue) {
  emit('update-field', { key, value })
}
</script>

<template>
  <div class="task-card__form">
    <label v-for="field in props.fields" :key="field.key" class="task-card__field">
      <span class="task-card__label">{{ field.label }}</span>
      <textarea
        v-if="field.type === 'textarea'"
        :data-testid="`task-field-${field.key}`"
        :value="String(props.form[field.key] ?? '')"
        class="task-card__textarea"
        :placeholder="field.placeholder"
        @input="updateField(field.key, ($event.target as HTMLTextAreaElement).value)"
      />
      <select
        v-else-if="field.type === 'select'"
        :data-testid="`task-field-${field.key}`"
        :value="String(props.form[field.key] ?? '')"
        class="task-card__input"
        @change="updateField(field.key, ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="option in field.options || []" :key="option.value" :value="option.value">
          {{ option.label }}
        </option>
      </select>
      <input
        v-else-if="field.type === 'checkbox'"
        :data-testid="`task-field-${field.key}`"
        :checked="props.form[field.key] === true"
        type="checkbox"
        class="task-card__checkbox"
        @change="updateField(field.key, ($event.target as HTMLInputElement).checked)"
      />
      <input
        v-else
        :data-testid="`task-field-${field.key}`"
        :value="String(props.form[field.key] ?? '')"
        :type="field.type === 'number' ? 'number' : 'text'"
        class="task-card__input"
        :min="field.min"
        :max="field.max"
        :placeholder="field.placeholder"
        @input="updateField(field.key, ($event.target as HTMLInputElement).value)"
      />
      <small v-if="field.help" class="task-card__help">{{ field.help }}</small>
    </label>
  </div>
</template>

<style scoped>
.task-card__form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-3);
}

.task-card__field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.task-card__label {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.task-card__input,
.task-card__textarea {
  width: 100%;
}

.task-card__textarea {
  min-height: 88px;
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  resize: vertical;
  background: var(--surface-panel);
  color: var(--text-primary);
}

.task-card__checkbox {
  width: 18px;
  height: 18px;
}

.task-card__help {
  color: var(--text-tertiary);
  font: var(--type-caption);
}
</style>
