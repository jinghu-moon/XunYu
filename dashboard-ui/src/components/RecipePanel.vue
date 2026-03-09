<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import {
  executeWorkspaceRecipe,
  fetchWorkspaceRecipes,
  previewWorkspaceRecipe,
  saveWorkspaceRecipe,
} from '../api'
import type { RecipeDefinition, RecipeExecutionReceipt, RecipeExecutionStepReceipt, RecipePreviewResponse, StatisticsWorkspaceLinkPayload } from '../types'
import { Button } from './button'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const props = withDefaults(
  defineProps<{
    title?: string
    description?: string
    category?: string
  }>(),
  {
    title: 'Recipe 工作流',
    description: '把高频本地流程沉淀成可预演、可确认、可复用的顺序工作流。',
    category: '',
  },
)

const recipes = ref<RecipeDefinition[]>([])
const selectedId = ref('')
const formValues = reactive<Record<string, string>>({})
const loading = ref(false)
const previewBusy = ref(false)
const executeBusy = ref(false)
const saveBusy = ref(false)
const requestError = ref('')
const statusMessage = ref('')
const preview = ref<RecipePreviewResponse | null>(null)
const receipt = ref<RecipeExecutionReceipt | null>(null)

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '请求失败，请检查全局错误提示。'
}

function matchesCategory(recipe: RecipeDefinition): boolean {
  return !props.category || recipe.category === props.category
}

const visibleRecipes = computed(() => recipes.value.filter(matchesCategory))

const selectedRecipe = computed(
  () => visibleRecipes.value.find((recipe) => recipe.id === selectedId.value) ?? visibleRecipes.value[0] ?? null,
)

const recipeCounts = computed(() => ({
  total: visibleRecipes.value.length,
  builtin: visibleRecipes.value.filter((recipe) => recipe.source === 'builtin').length,
  custom: visibleRecipes.value.filter((recipe) => recipe.source === 'custom').length,
}))

const validationError = computed(() => {
  const recipe = selectedRecipe.value
  if (!recipe) return ''
  const missing = recipe.params.filter((param) => param.required && !String(formValues[param.key] ?? '').trim())
  return missing.length ? `缺少必填项：${missing.map((param) => param.label).join('、')}` : ''
})

const canPreview = computed(() => Boolean(selectedRecipe.value && !validationError.value))


function focusRecentTasksForStep(step: RecipeExecutionStepReceipt) {
  emit('link-panel', {
    panel: 'recent-tasks',
    request: {
      status: step.status,
      dry_run: step.dry_run ? 'dry-run' : 'executed',
      search: step.target || undefined,
      action: step.action,
    },
  })
}

function focusAuditForStep(step: RecipeExecutionStepReceipt) {
  emit('link-panel', {
    panel: 'audit',
    request: {
      search: step.target || undefined,
      action: step.audit_action || undefined,
      result: step.status === 'failed' ? 'failed' : 'success',
    },
  })
}

function resetForm(recipe: RecipeDefinition | null) {
  for (const key of Object.keys(formValues)) {
    delete formValues[key]
  }
  if (!recipe) return
  for (const param of recipe.params) {
    formValues[param.key] = param.default_value || ''
  }
}

watch(
  selectedRecipe,
  (recipe) => {
    resetForm(recipe)
    requestError.value = ''
    statusMessage.value = ''
    preview.value = null
    receipt.value = null
  },
  { immediate: true },
)

async function loadRecipes(preferredId?: string) {
  loading.value = true
  requestError.value = ''
  try {
    const response = await fetchWorkspaceRecipes()
    recipes.value = response.recipes
    const nextId = preferredId || selectedId.value
    const visible = response.recipes.filter(matchesCategory)
    selectedId.value = visible.some((recipe) => recipe.id === nextId)
      ? nextId
      : visible[0]?.id || ''
  } catch (err) {
    requestError.value = errorMessage(err)
  } finally {
    loading.value = false
  }
}

function selectRecipe(id: string) {
  selectedId.value = id
}

function currentValues(): Record<string, string> {
  return Object.fromEntries(Object.entries(formValues).map(([key, value]) => [key, String(value ?? '').trim()]))
}

async function previewSelectedRecipe() {
  const recipe = selectedRecipe.value
  if (!recipe || validationError.value) return
  previewBusy.value = true
  requestError.value = ''
  statusMessage.value = ''
  receipt.value = null
  try {
    preview.value = await previewWorkspaceRecipe({ recipe_id: recipe.id, values: currentValues() })
  } catch (err) {
    preview.value = null
    requestError.value = errorMessage(err)
  } finally {
    previewBusy.value = false
  }
}

async function confirmExecuteRecipe() {
  if (!preview.value) return
  executeBusy.value = true
  requestError.value = ''
  statusMessage.value = ''
  try {
    receipt.value = await executeWorkspaceRecipe({ token: preview.value.token, confirm: true })
    statusMessage.value = 'Recipe 执行完成，结果已进入任务中心。'
    preview.value = null
  } catch (err) {
    requestError.value = errorMessage(err)
  } finally {
    executeBusy.value = false
  }
}

async function saveSelectedRecipeCopy() {
  const recipe = selectedRecipe.value
  if (!recipe) return
  saveBusy.value = true
  requestError.value = ''
  statusMessage.value = ''
  const targetId = recipe.source === 'builtin' ? `${recipe.id}-local` : recipe.id
  const targetName = recipe.source === 'builtin' ? `${recipe.name}（本地副本）` : recipe.name
  try {
    await saveWorkspaceRecipe({
      ...recipe,
      id: targetId,
      name: targetName,
      source: 'custom',
    })
    statusMessage.value = recipe.source === 'builtin' ? '已保存本地副本。' : '本地 Recipe 已更新。'
    await loadRecipes(targetId)
  } catch (err) {
    requestError.value = errorMessage(err)
  } finally {
    saveBusy.value = false
  }
}

function formatTime(ts: number) {
  return new Date(ts * 1000).toLocaleString()
}

onMounted(() => {
  void loadRecipes()
})
</script>

<template>
  <section class="recipe-panel">
    <header class="recipe-panel__header">
      <div>
        <h3 class="recipe-panel__title">{{ props.title }}</h3>
        <p class="recipe-panel__desc">{{ props.description }}</p>
      </div>
      <div class="recipe-panel__actions">
        <Button preset="secondary" :loading="loading" @click="loadRecipes(selectedId)">刷新</Button>
      </div>
    </header>

    <div class="recipe-panel__summary">
      <span class="recipe-panel__chip">总数 {{ recipeCounts.total }}</span>
      <span class="recipe-panel__chip">内置 {{ recipeCounts.builtin }}</span>
      <span class="recipe-panel__chip">本地 {{ recipeCounts.custom }}</span>
    </div>

    <p v-if="requestError" class="recipe-panel__message recipe-panel__message--error">{{ requestError }}</p>
    <p v-else-if="statusMessage" class="recipe-panel__message recipe-panel__message--ok">{{ statusMessage }}</p>

    <div class="recipe-panel__layout">
      <section class="recipe-panel__list">
        <button
          v-for="recipe in visibleRecipes"
          :key="recipe.id"
          type="button"
          class="recipe-panel__item"
          :class="{ 'is-active': recipe.id === selectedRecipe?.id }"
          :data-testid="`recipe-item-${recipe.id}`"
          @click="selectRecipe(recipe.id)"
        >
          <div class="recipe-panel__item-top">
            <strong>{{ recipe.name }}</strong>
            <span :class="['recipe-panel__badge', recipe.source === 'custom' ? 'is-custom' : 'is-builtin']">
              {{ recipe.source }}
            </span>
          </div>
          <p class="recipe-panel__item-desc">{{ recipe.description }}</p>
          <div class="recipe-panel__item-meta">
            <span>{{ recipe.category }}</span>
            <span>{{ recipe.steps.length }} steps</span>
            <span>dry-run {{ recipe.supports_dry_run ? 'on' : 'off' }}</span>
          </div>
        </button>
        <p v-if="!recipes.length && !loading" class="recipe-panel__empty">暂无 Recipe。</p>
      </section>

      <section v-if="selectedRecipe" class="recipe-panel__detail">
        <div class="recipe-panel__detail-header">
          <div>
            <h4 class="recipe-panel__detail-title">{{ selectedRecipe.name }}</h4>
            <p class="recipe-panel__detail-desc">{{ selectedRecipe.description }}</p>
          </div>
          <div class="recipe-panel__detail-meta">
            <span class="recipe-panel__chip">{{ selectedRecipe.category }}</span>
            <span class="recipe-panel__chip">{{ selectedRecipe.source }}</span>
          </div>
        </div>

        <div v-if="selectedRecipe.params.length" class="recipe-panel__form">
          <label v-for="param in selectedRecipe.params" :key="param.key" class="recipe-panel__field">
            <span class="recipe-panel__label">{{ param.label }}</span>
            <input
              :data-testid="`recipe-param-${param.key}`"
              class="recipe-panel__input"
              :value="formValues[param.key] ?? ''"
              :placeholder="param.placeholder"
              @input="formValues[param.key] = ($event.target as HTMLInputElement).value"
            />
            <small v-if="param.description" class="recipe-panel__hint">{{ param.description }}</small>
          </label>
        </div>

        <div class="recipe-panel__step-list">
          <div v-for="step in selectedRecipe.steps" :key="step.id" class="recipe-panel__step-item">
            <div class="recipe-panel__item-top">
              <strong>{{ step.title }}</strong>
              <span :class="['recipe-panel__badge', step.kind === 'guarded' ? 'is-danger' : 'is-builtin']">
                {{ step.kind }}
              </span>
            </div>
            <div class="recipe-panel__item-meta">
              <span>{{ step.workspace }}</span>
              <span>{{ step.action }}</span>
              <span>{{ step.summary }}</span>
            </div>
          </div>
        </div>

        <div class="recipe-panel__actions">
          <Button
            data-testid="preview-recipe-button"
            preset="primary"
            :disabled="!canPreview"
            :loading="previewBusy"
            @click="previewSelectedRecipe"
          >
            预演 Recipe
          </Button>
          <Button
            data-testid="save-recipe-button"
            preset="secondary"
            :loading="saveBusy"
            @click="saveSelectedRecipeCopy"
          >
            {{ selectedRecipe.source === 'builtin' ? '保存副本' : '更新本地 Recipe' }}
          </Button>
          <span v-if="validationError" class="recipe-panel__message recipe-panel__message--error">{{ validationError }}</span>
        </div>

        <section v-if="preview" class="recipe-panel__result">
          <div class="recipe-panel__detail-header">
            <div>
              <h4 class="recipe-panel__detail-title">预演结果</h4>
              <p class="recipe-panel__detail-desc">{{ preview.summary }}</p>
            </div>
            <span :class="['recipe-panel__badge', preview.ready_to_execute ? 'is-ok' : 'is-danger']">
              {{ preview.ready_to_execute ? '可确认执行' : '不可执行' }}
            </span>
          </div>
          <div class="recipe-panel__item-meta">
            <span>步骤 {{ preview.total_steps }}</span>
            <span>保护链路 {{ preview.guarded ? '已启用' : '未启用' }}</span>
            <span>过期 {{ preview.expires_in_secs }}s</span>
          </div>
          <article v-for="step in preview.steps" :key="step.id" class="recipe-panel__output-card">
            <div class="recipe-panel__item-top">
              <strong>{{ step.title }}</strong>
              <span :class="['recipe-panel__badge', step.guarded ? 'is-danger' : 'is-builtin']">{{ step.status }}</span>
            </div>
            <p class="recipe-panel__item-desc">{{ step.summary }}</p>
            <pre class="recipe-panel__output">{{ step.process.command_line }}

{{ step.process.stdout || step.process.stderr || '暂无输出' }}</pre>
          </article>
          <div class="recipe-panel__actions">
            <Button
              data-testid="execute-recipe-button"
              preset="danger"
              :loading="executeBusy"
              :disabled="!preview.ready_to_execute"
              @click="confirmExecuteRecipe"
            >
              确认执行 Recipe
            </Button>
          </div>
        </section>

        <section v-if="receipt" class="recipe-panel__result">
          <div class="recipe-panel__detail-header">
            <div>
              <h4 class="recipe-panel__detail-title">执行回执</h4>
              <p class="recipe-panel__detail-desc">{{ receipt.summary }}</p>
            </div>
            <span :class="['recipe-panel__badge', receipt.status === 'succeeded' ? 'is-ok' : 'is-danger']">
              {{ receipt.status }}
            </span>
          </div>
          <div class="recipe-panel__item-meta">
            <span>已完成 {{ receipt.completed_steps }}/{{ receipt.total_steps }}</span>
            <span>保护链路 {{ receipt.guarded ? '已启用' : '未启用' }}</span>
            <span>{{ formatTime(receipt.audited_at) }}</span>
          </div>
          <article v-for="step in receipt.steps" :key="step.id" class="recipe-panel__output-card">
            <div class="recipe-panel__item-top">
              <strong>{{ step.title }}</strong>
              <span :class="['recipe-panel__badge', step.status === 'succeeded' ? 'is-ok' : 'is-danger']">
                {{ step.status }}
              </span>
            </div>
            <p class="recipe-panel__item-desc">{{ step.summary }}</p>
            <div class="recipe-panel__result-links">
              <button :data-testid="`recipe-link-recent-${step.id}`" class="recipe-panel__link" type="button" @click="focusRecentTasksForStep(step)">
                回到最近任务
              </button>
              <button :data-testid="`recipe-link-audit-${step.id}`" class="recipe-panel__link" type="button" @click="focusAuditForStep(step)">
                查看审计
              </button>
            </div>
            <pre class="recipe-panel__output">{{ step.process.command_line }}

{{ step.process.stdout || step.process.stderr || '暂无输出' }}</pre>
          </article>
        </section>
      </section>
    </div>
  </section>
</template>

<style scoped>
.recipe-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.recipe-panel__header,
.recipe-panel__item-top,
.recipe-panel__detail-header,
.recipe-panel__actions {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: center;
}

.recipe-panel__title,
.recipe-panel__detail-title {
  font: var(--type-title);
  color: var(--text-primary);
}

.recipe-panel__desc,
.recipe-panel__detail-desc,
.recipe-panel__item-desc,
.recipe-panel__item-meta,
.recipe-panel__hint,
.recipe-panel__message {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.recipe-panel__message--error {
  color: var(--color-danger);
}

.recipe-panel__message--ok {
  color: var(--color-success);
}

.recipe-panel__summary,
.recipe-panel__detail-meta,
.recipe-panel__item-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.recipe-panel__layout {
  display: grid;
  grid-template-columns: minmax(300px, 360px) minmax(0, 1fr);
  gap: var(--space-4);
}

.recipe-panel__list,
.recipe-panel__detail,
.recipe-panel__result {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
}

.recipe-panel__list,
.recipe-panel__detail,
.recipe-panel__result,
.recipe-panel__step-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.recipe-panel__item,
.recipe-panel__step-item,
.recipe-panel__output-card {
  text-align: left;
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  padding: var(--space-3);
}

.recipe-panel__item {
  cursor: pointer;
}

.recipe-panel__item.is-active {
  border-color: var(--text-secondary);
  background: var(--ds-background-2);
}

.recipe-panel__chip,
.recipe-panel__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.recipe-panel__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.recipe-panel__badge.is-danger {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.recipe-panel__badge.is-custom {
  background: var(--color-info-bg);
  color: var(--color-info);
}

.recipe-panel__form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-3);
}

.recipe-panel__field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.recipe-panel__label {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.recipe-panel__input {
  width: 100%;
}

.recipe-panel__result-links {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.recipe-panel__link {
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-primary);
  cursor: pointer;
  font: var(--type-caption);
}

.recipe-panel__output {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}

.recipe-panel__empty {
  color: var(--text-secondary);
}
</style>
