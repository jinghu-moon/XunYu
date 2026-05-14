<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { Preview } from '../../generated/types'

const props = defineProps<{
  preview: Preview
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

const confirmedOnce = ref(false)
const criticalInput = ref('')

const isCritical = computed(() => props.preview.risk_level === 'Critical')
const isHigh = computed(() => props.preview.risk_level === 'High')
const canConfirm = computed(() => {
  if (isCritical.value) return criticalInput.value === 'CONFIRM'
  return true
})

function handleConfirm() {
  if (!canConfirm.value) return
  if (isHigh.value && !confirmedOnce.value) {
    confirmedOnce.value = true
    return
  }
  emit('confirm')
}

function handleCancel() {
  emit('cancel')
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('cancel')
}

onMounted(() => window.addEventListener('keydown', handleKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <div data-testid="dialog">
    <div data-testid="preview-description">{{ preview.description }}</div>

    <div v-if="preview.risk_level" data-testid="risk-badge">{{ preview.risk_level }}</div>

    <ul>
      <li
        v-for="(change, idx) in preview.changes"
        :key="idx"
        data-testid="change-item"
      >
        {{ change.action }} — {{ change.target }}
      </li>
    </ul>

    <input
      v-if="isCritical"
      v-model="criticalInput"
      data-testid="critical-confirm-input"
      placeholder="Type CONFIRM to proceed"
    />

    <button
      data-testid="cancel-btn"
      @click="handleCancel"
    >
      Cancel
    </button>

    <button
      data-testid="confirm-btn"
      :class="`risk-${(preview.risk_level ?? 'low').toLowerCase()}`"
      :disabled="!canConfirm"
      @click="handleConfirm"
    >
      {{ confirmedOnce ? 'Confirm' : 'Execute' }}
    </button>
  </div>
</template>
