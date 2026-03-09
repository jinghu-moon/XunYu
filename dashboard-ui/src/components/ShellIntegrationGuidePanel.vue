<script setup lang="ts">
import { computed, ref } from 'vue'

type ShellKind = 'powershell' | 'bash' | 'zsh'
type TaskPresetMap = Record<string, Partial<Record<string, string | boolean>>>

const emit = defineEmits<{
  (event: 'apply-task-presets', presets: TaskPresetMap): void
}>()

interface ShellGuideDefinition {
  key: ShellKind
  label: string
  profilePath: string
  profileCommand: string
  initCommand: string
  completionCommand: string
  verifyCommand: string
  verifyArgs: string
}

const shellGuides: ShellGuideDefinition[] = [
  {
    key: 'powershell',
    label: 'PowerShell',
    profilePath: '$PROFILE',
    profileCommand: 'xun init powershell | Out-String | Invoke-Expression',
    initCommand: 'xun init powershell',
    completionCommand: 'xun completion powershell',
    verifyCommand: 'xun __complete alias ls --j',
    verifyArgs: 'alias ls --j',
  },
  {
    key: 'bash',
    label: 'Bash',
    profilePath: '~/.bashrc',
    profileCommand: 'eval "$(xun init bash)"',
    initCommand: 'xun init bash',
    completionCommand: 'xun completion bash',
    verifyCommand: 'xun __complete alias ls --j',
    verifyArgs: 'alias ls --j',
  },
  {
    key: 'zsh',
    label: 'Zsh',
    profilePath: '~/.zshrc',
    profileCommand: 'eval "$(xun init zsh)"',
    initCommand: 'xun init zsh',
    completionCommand: 'xun completion zsh',
    verifyCommand: 'xun __complete alias ls --j',
    verifyArgs: 'alias ls --j',
  },
]

const selectedShell = ref<ShellKind>('powershell')
const copyFeedback = ref('')

const activeGuide = computed(
  () => shellGuides.find((guide) => guide.key === selectedShell.value) ?? shellGuides[0],
)

const taskPresets = computed<TaskPresetMap>(() => ({
  init: { shell: activeGuide.value.key },
  completion: { shell: activeGuide.value.key },
  complete: { args: activeGuide.value.verifyArgs },
}))

async function copyText(text: string, label: string) {
  copyFeedback.value = ''
  try {
    if (!navigator.clipboard?.writeText) {
      throw new Error('当前环境不支持 Clipboard API，请手动复制。')
    }
    await navigator.clipboard.writeText(text)
    copyFeedback.value = `已复制：${label}`
  } catch (error) {
    copyFeedback.value = error instanceof Error ? error.message : '复制失败，请手动复制。'
  }
}

function applyTaskPresets() {
  emit('apply-task-presets', taskPresets.value)
}
</script>

<template>
  <section class="shell-guide" data-testid="shell-guide-panel">
    <header class="shell-guide__header">
      <div>
        <h3 class="shell-guide__title">Shell 安装向导</h3>
        <p class="shell-guide__desc">
          先选目标 shell，再按“写入 profile → 重开终端 → 运行验证命令”的顺序完成安装闭环。
        </p>
      </div>
      <div class="shell-guide__switches" role="tablist" aria-label="选择 Shell">
        <button
          v-for="guide in shellGuides"
          :key="guide.key"
          :class="['shell-guide__switch', guide.key === selectedShell ? 'is-active' : '']"
          type="button"
          :data-testid="`shell-guide-shell-${guide.key}`"
          @click="selectedShell = guide.key"
        >
          {{ guide.label }}
        </button>
      </div>
    </header>

    <div class="shell-guide__grid">
      <article class="shell-guide__card">
        <div class="shell-guide__step">步骤 1</div>
        <h4>写入 profile</h4>
        <p>把下面这行命令写入 <code data-testid="shell-guide-profile-path">{{ activeGuide.profilePath }}</code>。</p>
        <pre data-testid="shell-guide-profile-command">{{ activeGuide.profileCommand }}</pre>
        <div class="shell-guide__actions">
          <button data-testid="shell-guide-copy-profile" type="button" @click="copyText(activeGuide.profileCommand, 'profile 片段')">复制片段</button>
          <button data-testid="shell-guide-apply-presets" type="button" @click="applyTaskPresets">填入任务卡</button>
        </div>
      </article>

      <article class="shell-guide__card">
        <div class="shell-guide__step">步骤 2</div>
        <h4>检查脚本输出</h4>
        <p>如需先查看原始输出，可运行下面两条命令，或直接填入下方任务卡。</p>
        <pre data-testid="shell-guide-init-command">{{ activeGuide.initCommand }}</pre>
        <pre data-testid="shell-guide-completion-command">{{ activeGuide.completionCommand }}</pre>
        <div class="shell-guide__actions">
          <button data-testid="shell-guide-copy-init" type="button" @click="copyText(activeGuide.initCommand, 'init 命令')">复制 init</button>
          <button data-testid="shell-guide-copy-completion" type="button" @click="copyText(activeGuide.completionCommand, 'completion 命令')">复制 completion</button>
        </div>
      </article>

      <article class="shell-guide__card">
        <div class="shell-guide__step">步骤 3</div>
        <h4>验证补全闭环</h4>
        <p>重开终端后，先用内部补全调试命令验证 `xun/xyu/xy` 的补全链路。</p>
        <pre data-testid="shell-guide-verify-command">{{ activeGuide.verifyCommand }}</pre>
        <div class="shell-guide__actions">
          <button data-testid="shell-guide-copy-verify" type="button" @click="copyText(activeGuide.verifyCommand, '验证命令')">复制验证命令</button>
        </div>
      </article>
    </div>

    <p v-if="copyFeedback" class="shell-guide__feedback" data-testid="shell-guide-copy-feedback">
      {{ copyFeedback }}
    </p>
  </section>
</template>

<style scoped>
.shell-guide {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  background: var(--surface-panel);
}

.shell-guide__header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
}

.shell-guide__title {
  margin: 0 0 var(--space-1);
  font: var(--type-title);
  color: var(--text-primary);
}

.shell-guide__desc {
  margin: 0;
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.shell-guide__switches {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.shell-guide__switch,
.shell-guide__actions button {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--surface-base);
  color: var(--text-primary);
  cursor: pointer;
  padding: 0.5rem 0.875rem;
}

.shell-guide__switch.is-active {
  border-color: var(--accent-strong);
  background: var(--accent-soft);
}

.shell-guide__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: var(--space-3);
}

.shell-guide__card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  background: var(--surface-base);
}

.shell-guide__step {
  font: var(--type-label);
  color: var(--accent-strong);
}

.shell-guide__card h4,
.shell-guide__card p {
  margin: 0;
}

.shell-guide__card p {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.shell-guide__card pre {
  margin: 0;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
  border-radius: var(--radius-sm);
  background: var(--surface-muted);
  padding: var(--space-3);
  font: var(--type-code);
}

.shell-guide__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.shell-guide__feedback {
  margin: 0;
  color: var(--text-secondary);
  font: var(--type-body-sm);
}
</style>
