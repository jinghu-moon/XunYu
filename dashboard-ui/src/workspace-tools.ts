import type { WorkspaceCapabilities, WorkspaceKey } from './types'

export type TaskFieldType = 'text' | 'textarea' | 'number' | 'select' | 'checkbox'
export type TaskFieldValue = string | boolean
export type TaskFormState = Record<string, TaskFieldValue>

export interface TaskFieldOption {
  label: string
  value: string
}

export interface TaskFieldDefinition {
  key: string
  label: string
  type: TaskFieldType
  placeholder?: string
  help?: string
  defaultValue?: TaskFieldValue
  required?: boolean
  options?: TaskFieldOption[]
  min?: number
  max?: number
}

export interface WorkspaceTaskDefinition {
  id: string
  workspace: WorkspaceKey
  title: string
  description: string
  action: string
  mode: 'run' | 'guarded'
  tone?: 'default' | 'danger'
  feature?: keyof WorkspaceCapabilities
  fields: TaskFieldDefinition[]
  target?: (values: TaskFormState) => string
  buildRunArgs?: (values: TaskFormState) => string[]
  buildPreviewArgs?: (values: TaskFormState) => string[]
  buildExecuteArgs?: (values: TaskFormState) => string[]
  previewSummary?: (values: TaskFormState) => string
}

export interface WorkspaceTaskGroup {
  id: string
  title: string
  description: string
  tasks: WorkspaceTaskDefinition[]
}

export interface WorkspaceTabDefinition {
  value: WorkspaceKey
  label: string
  description: string
}

const JSON_FORMAT = 'json'

const shellInitOptions: TaskFieldOption[] = [
  { label: 'PowerShell', value: 'powershell' },
  { label: 'Bash', value: 'bash' },
  { label: 'Zsh', value: 'zsh' },
]

const shellCompletionOptions: TaskFieldOption[] = [
  ...shellInitOptions,
  { label: 'Fish', value: 'fish' },
]

const dedupModeOptions: TaskFieldOption[] = [
  { label: '按路径', value: 'path' },
  { label: '按名称', value: 'name' },
]

const brnCaseOptions: TaskFieldOption[] = [
  { label: '不转换', value: '' },
  { label: 'kebab', value: 'kebab' },
  { label: 'snake', value: 'snake' },
  { label: 'pascal', value: 'pascal' },
  { label: 'upper', value: 'upper' },
  { label: 'lower', value: 'lower' },
]

const imgFormatOptions: TaskFieldOption[] = [
  { label: 'webp', value: 'webp' },
  { label: 'jpeg', value: 'jpeg' },
  { label: 'png', value: 'png' },
  { label: 'avif', value: 'avif' },
  { label: 'svg', value: 'svg' },
]

const videoModeOptions: TaskFieldOption[] = [
  { label: 'balanced', value: 'balanced' },
  { label: 'fastest', value: 'fastest' },
  { label: 'smallest', value: 'smallest' },
]

const videoEngineOptions: TaskFieldOption[] = [
  { label: 'auto', value: 'auto' },
  { label: 'cpu', value: 'cpu' },
  { label: 'gpu', value: 'gpu' },
]

const aclRightsOptions: TaskFieldOption[] = [
  { label: 'Read', value: 'Read' },
  { label: 'Write', value: 'Write' },
  { label: 'Modify', value: 'Modify' },
  { label: 'ReadAndExecute', value: 'ReadAndExecute' },
  { label: 'FullControl', value: 'FullControl' },
]

const aclAceTypeOptions: TaskFieldOption[] = [
  { label: 'Allow', value: 'Allow' },
  { label: 'Deny', value: 'Deny' },
]

const aclInheritOptions: TaskFieldOption[] = [
  { label: 'BothInherit', value: 'BothInherit' },
  { label: 'ContainerOnly', value: 'ContainerOnly' },
  { label: 'ObjectOnly', value: 'ObjectOnly' },
  { label: 'None', value: 'None' },
]

function readText(values: TaskFormState, key: string): string {
  const value = values[key]
  return typeof value === 'string' ? value.trim() : ''
}

function readBool(values: TaskFormState, key: string): boolean {
  return values[key] === true
}

function splitItems(raw: string): string[] {
  return raw
    .split(/[\n,]+/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function splitCommand(raw: string): string[] {
  const matches = raw.match(/"([^"]*)"|'([^']*)'|\S+/g) ?? []
  return matches.map((part) => part.replace(/^['"]|['"]$/g, ''))
}

function pushOption(args: string[], name: string, value: string) {
  if (value) args.push(name, value)
}

function pushRepeatableOption(args: string[], name: string, raw: string) {
  for (const item of splitItems(raw)) {
    args.push(name, item)
  }
}

function runTask(definition: Omit<WorkspaceTaskDefinition, 'mode'>): WorkspaceTaskDefinition {
  return { ...definition, mode: 'run' }
}

function guardedTask(definition: Omit<WorkspaceTaskDefinition, 'mode'>): WorkspaceTaskDefinition {
  return { ...definition, mode: 'guarded' }
}

function pathTarget(values: TaskFormState, key = 'path'): string {
  return readText(values, key)
}

function previewPath(values: TaskFormState, key = 'path'): string[] {
  const path = readText(values, key)
  const args = ['find', '--dry-run', '-f', JSON_FORMAT]
  pushOption(args, '--test-path', path)
  return args
}

function moveLikeArgs(command: 'mv' | 'ren', values: TaskFormState, dryRun: boolean): string[] {
  const src = readText(values, 'src')
  const dst = readText(values, 'dst')
  const args: string[] = [command]
  if (readBool(values, 'unlock')) args.push('--unlock')
  if (readBool(values, 'forceKill')) args.push('--force-kill')
  if (dryRun) args.push('--dry-run')
  if (!dryRun) args.push('-y')
  if (readBool(values, 'force')) args.push('--force')
  pushOption(args, '--reason', readText(values, 'reason'))
  args.push(src, dst)
  return args
}

export const workspaceTabs: WorkspaceTabDefinition[] = [
  { value: 'overview', label: '总览', description: '统一总览与能力入口' },
  { value: 'paths-context', label: '路径与上下文', description: '书签、上下文、路径检索' },
  { value: 'network-proxy', label: '网络与代理', description: '端口、进程、代理执行' },
  { value: 'environment-config', label: '环境与配置', description: '全局配置与环境治理' },
  { value: 'files-security', label: '文件与安全', description: '文件、备份、保护与 ACL' },
  { value: 'integration-automation', label: '集成与自动化', description: 'shell 集成、别名、批量改名' },
  { value: 'media-conversion', label: '媒体与转换', description: '图像与视频任务' },
  { value: 'statistics-diagnostics', label: '统计与诊断', description: '审计、体检、统计' },
]

export const pathsContextTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'ctx-core',
    title: '上下文控制',
    description: '基于 ctx 子命令管理本地上下文配置。',
    tasks: [
      runTask({
        id: 'ctx-list',
        workspace: 'paths-context',
        title: '列出上下文',
        description: '查看已定义的上下文配置。',
        action: 'ctx:list',
        fields: [],
        buildRunArgs: () => ['ctx', 'list', '-f', JSON_FORMAT],
      }),
      runTask({
        id: 'ctx-show',
        workspace: 'paths-context',
        title: '查看上下文',
        description: '查看指定上下文或当前激活上下文。',
        action: 'ctx:show',
        fields: [{ key: 'name', label: '配置名', type: 'text', placeholder: '留空则查看当前激活上下文' }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const name = readText(values, 'name')
          return ['ctx', 'show', ...(name ? [name] : []), '-f', JSON_FORMAT]
        },
      }),
      runTask({
        id: 'ctx-use',
        workspace: 'paths-context',
        title: '启用上下文',
        description: '切换到目标上下文。',
        action: 'ctx:use',
        fields: [{ key: 'name', label: '配置名', type: 'text', required: true, placeholder: '例如 work' }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => ['ctx', 'use', readText(values, 'name')],
      }),
      runTask({
        id: 'ctx-off',
        workspace: 'paths-context',
        title: '停用上下文',
        description: '清除当前激活上下文。',
        action: 'ctx:off',
        fields: [],
        buildRunArgs: () => ['ctx', 'off'],
      }),
      runTask({
        id: 'ctx-set',
        workspace: 'paths-context',
        title: '写入上下文',
        description: '定义或更新上下文配置。环境变量支持逗号或换行分隔。',
        action: 'ctx:set',
        fields: [
          { key: 'name', label: '配置名', type: 'text', required: true, placeholder: '例如 work' },
          { key: 'path', label: '工作目录', type: 'text', placeholder: 'D:/repo/project' },
          { key: 'proxy', label: '代理', type: 'text', placeholder: 'http://127.0.0.1:7890 | off | keep' },
          { key: 'noproxy', label: 'NO_PROXY', type: 'text', placeholder: 'localhost,127.0.0.1' },
          { key: 'tag', label: '默认标签', type: 'text', placeholder: 'work,repo' },
          { key: 'env', label: '环境变量', type: 'textarea', placeholder: 'KEY=VALUE' },
          { key: 'envFile', label: 'env 文件', type: 'text', placeholder: '.env.local' },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = ['ctx', 'set', readText(values, 'name')]
          pushOption(args, '--path', readText(values, 'path'))
          pushOption(args, '--proxy', readText(values, 'proxy'))
          pushOption(args, '--noproxy', readText(values, 'noproxy'))
          pushOption(args, '-t', readText(values, 'tag'))
          pushRepeatableOption(args, '--env', readText(values, 'env'))
          pushOption(args, '--env-file', readText(values, 'envFile'))
          return args
        },
      }),
      guardedTask({
        id: 'ctx-del',
        workspace: 'paths-context',
        title: '删除上下文',
        description: '删除指定上下文，先预览详情再确认。',
        action: 'ctx:del',
        tone: 'danger',
        fields: [{ key: 'name', label: '配置名', type: 'text', required: true, placeholder: '例如 work' }],
        target: (values) => readText(values, 'name'),
        buildPreviewArgs: (values) => ['ctx', 'show', readText(values, 'name'), '-f', JSON_FORMAT],
        buildExecuteArgs: (values) => ['ctx', 'del', readText(values, 'name')],
        previewSummary: (values) => `删除上下文 ${readText(values, 'name')}`,
      }),
      runTask({
        id: 'ctx-rename',
        workspace: 'paths-context',
        title: '重命名上下文',
        description: '重命名现有上下文配置。',
        action: 'ctx:rename',
        fields: [
          { key: 'old', label: '旧名称', type: 'text', required: true },
          { key: 'next', label: '新名称', type: 'text', required: true },
        ],
        target: (values) => `${readText(values, 'old')} -> ${readText(values, 'next')}`,
        buildRunArgs: (values) => ['ctx', 'rename', readText(values, 'old'), readText(values, 'next')],
      }),
    ],
  },
  {
    id: 'bookmark-health',
    title: '书签与健康',
    description: '补齐 recent / stats / check / gc / dedup 等维护能力。',
    tasks: [
      runTask({
        id: 'recent',
        workspace: 'paths-context',
        title: '最近访问',
        description: '查看最近访问的书签。',
        action: 'recent',
        fields: [
          { key: 'limit', label: '数量', type: 'number', defaultValue: '10', min: 1 },
          { key: 'tag', label: '标签过滤', type: 'text', placeholder: '可选' },
        ],
        buildRunArgs: (values) => {
          const args = ['recent', '-n', readText(values, 'limit') || '10', '-f', JSON_FORMAT]
          pushOption(args, '-t', readText(values, 'tag'))
          return args
        },
      }),
      runTask({
        id: 'bookmark-stats',
        workspace: 'paths-context',
        title: '书签统计',
        description: '查看书签统计摘要。',
        action: 'stats',
        fields: [],
        buildRunArgs: () => ['stats', '-f', JSON_FORMAT],
      }),
      runTask({
        id: 'bookmark-check',
        workspace: 'paths-context',
        title: '健康检查',
        description: '检查缺失路径、重复项和陈旧记录。',
        action: 'check',
        fields: [{ key: 'days', label: '陈旧阈值(天)', type: 'number', defaultValue: '90', min: 1 }],
        buildRunArgs: (values) => ['check', '-d', readText(values, 'days') || '90', '-f', JSON_FORMAT],
      }),
      runTask({
        id: 'gc-preview',
        workspace: 'paths-context',
        title: '扫描死链',
        description: '预览死链书签，不做删除。',
        action: 'gc:preview',
        fields: [],
        buildRunArgs: () => ['gc', '-f', JSON_FORMAT],
      }),
      guardedTask({
        id: 'gc-purge',
        workspace: 'paths-context',
        title: '清理死链',
        description: '先预览后清理所有死链书签。',
        action: 'gc:purge',
        tone: 'danger',
        fields: [],
        target: () => 'dead-links',
        buildPreviewArgs: () => ['gc', '-f', JSON_FORMAT],
        buildExecuteArgs: () => ['gc', '--purge', '-f', JSON_FORMAT],
        previewSummary: () => '清理全部死链书签',
      }),
      guardedTask({
        id: 'dedup',
        workspace: 'paths-context',
        title: '去重书签',
        description: '按路径或名称做去重，先看统计再执行。',
        action: 'dedup',
        tone: 'danger',
        fields: [{ key: 'mode', label: '模式', type: 'select', defaultValue: 'path', options: dedupModeOptions }],
        target: (values) => `mode=${readText(values, 'mode') || 'path'}`,
        buildPreviewArgs: () => ['stats', '-f', JSON_FORMAT],
        buildExecuteArgs: (values) => ['dedup', '-m', readText(values, 'mode') || 'path', '-f', JSON_FORMAT, '-y'],
        previewSummary: (values) => `执行书签去重：${readText(values, 'mode') || 'path'}`,
      }),
    ],
  },
  {
    id: 'bookmark-query',
    title: '路径检索',
    description: '以 CLI 机器输出形式接入 keys / all / fuzzy / ws。',
    tasks: [
      runTask({
        id: 'keys',
        workspace: 'paths-context',
        title: '键列表',
        description: '输出所有书签键。',
        action: 'keys',
        fields: [],
        buildRunArgs: () => ['keys'],
      }),
      runTask({
        id: 'all',
        workspace: 'paths-context',
        title: '全部书签',
        description: '按标签输出全部书签。',
        action: 'all',
        fields: [{ key: 'tag', label: '标签', type: 'text', placeholder: '可选' }],
        buildRunArgs: (values) => ['all', ...(readText(values, 'tag') ? [readText(values, 'tag')] : [])],
      }),
      runTask({
        id: 'fuzzy',
        workspace: 'paths-context',
        title: '模糊匹配',
        description: '按模式做机器可读搜索。',
        action: 'fuzzy',
        fields: [
          { key: 'pattern', label: '模式', type: 'text', required: true, placeholder: '例如 doc' },
          { key: 'tag', label: '标签', type: 'text', placeholder: '可选' },
        ],
        target: (values) => readText(values, 'pattern'),
        buildRunArgs: (values) => ['fuzzy', readText(values, 'pattern'), ...(readText(values, 'tag') ? [readText(values, 'tag')] : [])],
      }),
      runTask({
        id: 'ws',
        workspace: 'paths-context',
        title: '批量打开工作区',
        description: '按标签在 WT 中打开全部路径。',
        action: 'ws',
        fields: [{ key: 'tag', label: '标签', type: 'text', required: true, placeholder: '例如 work' }],
        target: (values) => readText(values, 'tag'),
        buildRunArgs: (values) => ['ws', readText(values, 'tag')],
      }),
    ],
  },
]

export const networkProxyTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'process-tools',
    title: '进程与端口动作',
    description: '危险动作统一走 preview -> confirm -> receipt。',
    tasks: [
      runTask({
        id: 'ps',
        workspace: 'network-proxy',
        title: '查询进程',
        description: '按名称、PID 或窗口标题查询进程。',
        action: 'ps',
        fields: [
          { key: 'pattern', label: '名称模式', type: 'text', placeholder: '可选' },
          { key: 'pid', label: 'PID', type: 'number', placeholder: '可选' },
          { key: 'win', label: '窗口标题', type: 'text', placeholder: '可选' },
        ],
        buildRunArgs: (values) => {
          const args = ['ps']
          const pattern = readText(values, 'pattern')
          const pid = readText(values, 'pid')
          const win = readText(values, 'win')
          if (pattern) args.push(pattern)
          pushOption(args, '--pid', pid)
          pushOption(args, '-w', win)
          return args
        },
      }),
      guardedTask({
        id: 'pkill',
        workspace: 'network-proxy',
        title: '结束进程',
        description: '通过 ps 预览命中目标，再执行 pkill。',
        action: 'pkill',
        tone: 'danger',
        fields: [
          { key: 'target', label: '目标', type: 'text', required: true, placeholder: '进程名、PID 或窗口标题' },
          { key: 'window', label: '按窗口标题匹配', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'target'),
        buildPreviewArgs: (values) => {
          const target = readText(values, 'target')
          if (readBool(values, 'window')) return ['ps', '-w', target]
          if (/^\d+$/.test(target)) return ['ps', '--pid', target]
          return ['ps', target]
        },
        buildExecuteArgs: (values) => {
          const args = ['pkill', readText(values, 'target'), '-f']
          if (readBool(values, 'window')) args.push('-w')
          return args
        },
        previewSummary: (values) => `结束进程 ${readText(values, 'target')}`,
      }),
      guardedTask({
        id: 'kill-ports',
        workspace: 'network-proxy',
        title: '释放端口',
        description: '先看当前端口占用，再批量 kill。',
        action: 'kill',
        tone: 'danger',
        fields: [
          { key: 'ports', label: '端口列表', type: 'text', required: true, placeholder: '3000,5173,8080' },
          { key: 'tcp', label: '仅 TCP', type: 'checkbox', defaultValue: false },
          { key: 'udp', label: '仅 UDP', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'ports'),
        buildPreviewArgs: () => ['ports', '-f', JSON_FORMAT],
        buildExecuteArgs: (values) => {
          const args = ['kill', readText(values, 'ports'), '-f']
          if (readBool(values, 'tcp')) args.push('--tcp')
          if (readBool(values, 'udp')) args.push('--udp')
          return args
        },
        previewSummary: (values) => `释放端口 ${readText(values, 'ports')}`,
      }),
    ],
  },
  {
    id: 'proxy-tools',
    title: '代理诊断与执行',
    description: '补齐 pst / proxy detect / px。',
    tasks: [
      runTask({
        id: 'pst',
        workspace: 'network-proxy',
        title: '代理状态',
        description: '查看代理状态快照。',
        action: 'pst',
        fields: [],
        buildRunArgs: () => ['pst', '-f', JSON_FORMAT],
      }),
      runTask({
        id: 'proxy-detect',
        workspace: 'network-proxy',
        title: '系统代理探测',
        description: '读取系统代理配置。',
        action: 'proxy:detect',
        fields: [],
        buildRunArgs: () => ['proxy', 'detect', '-f', JSON_FORMAT],
      }),
      runTask({
        id: 'px',
        workspace: 'network-proxy',
        title: '带代理执行命令',
        description: '使用 px 包装本地命令。命令和参数支持引号。',
        action: 'px',
        fields: [
          { key: 'url', label: '代理 URL', type: 'text', placeholder: '可选' },
          { key: 'noproxy', label: 'NO_PROXY', type: 'text', placeholder: '留空则使用 CLI 默认值' },
          { key: 'cmd', label: '命令', type: 'textarea', required: true, placeholder: '例如 cargo test --lib' },
        ],
        target: (values) => readText(values, 'cmd'),
        buildRunArgs: (values) => {
          const args = ['px']
          pushOption(args, '-u', readText(values, 'url'))
          pushOption(args, '-n', readText(values, 'noproxy'))
          return [...args, ...splitCommand(readText(values, 'cmd'))]
        },
      }),
    ],
  },
]

export const filesSecurityTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'file-discovery',
    title: '文件发现',
    description: '基于 tree / find 形成文件工作流入口。',
    tasks: [
      runTask({
        id: 'tree',
        workspace: 'files-security',
        title: '目录树',
        description: '输出目录结构或统计。',
        action: 'tree',
        fields: [
          { key: 'path', label: '路径', type: 'text', placeholder: '留空则使用当前目录' },
          { key: 'depth', label: '最大深度', type: 'number', placeholder: '可选' },
          { key: 'hidden', label: '包含隐藏文件', type: 'checkbox', defaultValue: false },
          { key: 'plain', label: '纯文本输出', type: 'checkbox', defaultValue: false },
          { key: 'statsOnly', label: '仅统计', type: 'checkbox', defaultValue: false },
          { key: 'size', label: '显示大小', type: 'checkbox', defaultValue: false },
          { key: 'include', label: '包含模式', type: 'text', placeholder: '*.rs,*.vue' },
          { key: 'exclude', label: '排除模式', type: 'text', placeholder: 'node_modules,.git' },
        ],
        target: (values) => readText(values, 'path'),
        buildRunArgs: (values) => {
          const args = ['tree']
          const path = readText(values, 'path')
          const depth = readText(values, 'depth')
          if (path) args.push(path)
          pushOption(args, '-d', depth)
          if (readBool(values, 'hidden')) args.push('--hidden')
          if (readBool(values, 'plain')) args.push('--plain')
          if (readBool(values, 'statsOnly')) args.push('--stats-only')
          if (readBool(values, 'size')) args.push('--size')
          pushRepeatableOption(args, '--include', readText(values, 'include'))
          pushRepeatableOption(args, '--exclude', readText(values, 'exclude'))
          return args
        },
      }),
      runTask({
        id: 'find',
        workspace: 'files-security',
        title: '高级查找',
        description: '按 include/exclude/扩展名/深度扫描文件。',
        action: 'find',
        fields: [
          { key: 'paths', label: '路径列表', type: 'text', placeholder: '多个路径可用逗号分隔' },
          { key: 'include', label: '包含 glob', type: 'text', placeholder: '**/*.ts,**/*.vue' },
          { key: 'exclude', label: '排除 glob', type: 'text', placeholder: 'dist,node_modules' },
          { key: 'extension', label: '扩展名', type: 'text', placeholder: 'ts,vue,rs' },
          { key: 'name', label: '名称', type: 'text', placeholder: 'README.md' },
          { key: 'depth', label: '深度过滤', type: 'text', placeholder: '0..3' },
          { key: 'count', label: '仅计数', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'paths'),
        buildRunArgs: (values) => {
          const args = ['find']
          args.push(...splitItems(readText(values, 'paths')))
          pushRepeatableOption(args, '-i', readText(values, 'include'))
          pushRepeatableOption(args, '-e', readText(values, 'exclude'))
          pushRepeatableOption(args, '--extension', readText(values, 'extension'))
          pushRepeatableOption(args, '--name', readText(values, 'name'))
          pushOption(args, '-d', readText(values, 'depth'))
          if (readBool(values, 'count')) args.push('-c')
          args.push('-f', JSON_FORMAT)
          return args
        },
      }),
    ],
  },
  {
    id: 'backup-tools',
    title: '备份与恢复',
    description: 'bak 默认 preview / apply 分离，统一走 guarded。',
    tasks: [
      runTask({
        id: 'bak-list',
        workspace: 'files-security',
        title: '列出备份',
        description: '查看当前目录下的备份集合。',
        action: 'bak:list',
        fields: [{ key: 'dir', label: '工作目录', type: 'text', placeholder: '可选' }],
        target: (values) => readText(values, 'dir'),
        buildRunArgs: (values) => {
          const args = ['bak', 'list']
          pushOption(args, '-C', readText(values, 'dir'))
          return args
        },
      }),
      guardedTask({
        id: 'bak-create',
        workspace: 'files-security',
        title: '创建备份',
        description: '预览将写入哪些内容，再创建增量备份。',
        action: 'bak:create',
        tone: 'danger',
        fields: [
          { key: 'dir', label: '工作目录', type: 'text', placeholder: '可选' },
          { key: 'msg', label: '说明', type: 'text', placeholder: '发布前快照' },
          { key: 'retain', label: '保留数量', type: 'number', placeholder: '可选' },
          { key: 'include', label: '包含路径', type: 'text', placeholder: 'src,docs' },
          { key: 'exclude', label: '排除路径', type: 'text', placeholder: 'target,node_modules' },
          { key: 'noCompress', label: '禁用压缩', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'dir'),
        buildPreviewArgs: (values) => {
          const args = ['bak', '--dry-run']
          pushOption(args, '-C', readText(values, 'dir'))
          pushOption(args, '-m', readText(values, 'msg'))
          pushOption(args, '--retain', readText(values, 'retain'))
          pushRepeatableOption(args, '--include', readText(values, 'include'))
          pushRepeatableOption(args, '--exclude', readText(values, 'exclude'))
          if (readBool(values, 'noCompress')) args.push('--no-compress')
          return args
        },
        buildExecuteArgs: (values) => {
          const args = ['bak', '-y']
          pushOption(args, '-C', readText(values, 'dir'))
          pushOption(args, '-m', readText(values, 'msg'))
          pushOption(args, '--retain', readText(values, 'retain'))
          pushRepeatableOption(args, '--include', readText(values, 'include'))
          pushRepeatableOption(args, '--exclude', readText(values, 'exclude'))
          if (readBool(values, 'noCompress')) args.push('--no-compress')
          return args
        },
        previewSummary: () => '创建增量备份',
      }),
      guardedTask({
        id: 'bak-restore',
        workspace: 'files-security',
        title: '恢复备份',
        description: '先做 dry-run，再恢复备份或单文件。',
        action: 'bak:restore',
        tone: 'danger',
        fields: [
          { key: 'name', label: '备份名', type: 'text', required: true, placeholder: 'bak-20260308-...' },
          { key: 'dir', label: '工作目录', type: 'text', placeholder: '可选' },
          { key: 'file', label: '单文件恢复', type: 'text', placeholder: 'src/main.rs' },
        ],
        target: (values) => readText(values, 'name'),
        buildPreviewArgs: (values) => {
          const args = ['bak', 'restore', readText(values, 'name'), '--dry-run']
          pushOption(args, '-C', readText(values, 'dir'))
          pushOption(args, '--file', readText(values, 'file'))
          return args
        },
        buildExecuteArgs: (values) => {
          const args = ['bak', 'restore', readText(values, 'name'), '-y']
          pushOption(args, '-C', readText(values, 'dir'))
          pushOption(args, '--file', readText(values, 'file'))
          return args
        },
        previewSummary: (values) => `恢复备份 ${readText(values, 'name')}`,
      }),
    ],
  },
  {
    id: 'file-guard',
    title: '删除 / 移动 / 保护',
    description: '危险文件操作统一走 dry-run 和回执。',
    tasks: [
      guardedTask({
        id: 'rm',
        workspace: 'files-security',
        title: '删除文件',
        description: '支持 unlock / force-kill / on-reboot。',
        action: 'rm',
        tone: 'danger',
        feature: 'fs',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'unlock', label: '删除前解锁', type: 'checkbox', defaultValue: false },
          { key: 'forceKill', label: '强制结束占用进程', type: 'checkbox', defaultValue: false },
          { key: 'onReboot', label: '重启后删除', type: 'checkbox', defaultValue: false },
          { key: 'force', label: '绕过保护', type: 'checkbox', defaultValue: false },
          { key: 'reason', label: '绕过原因', type: 'text', placeholder: '当 force=true 时建议填写' },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => {
          const path = readText(values, 'path')
          const args = ['rm']
          if (readBool(values, 'unlock')) args.push('--unlock')
          if (readBool(values, 'forceKill')) args.push('--force-kill')
          if (readBool(values, 'onReboot')) args.push('--on-reboot')
          args.push('--dry-run', '-f', JSON_FORMAT)
          if (readBool(values, 'force')) args.push('--force')
          pushOption(args, '--reason', readText(values, 'reason'))
          args.push(path)
          return args
        },
        buildExecuteArgs: (values) => {
          const path = readText(values, 'path')
          const args = ['rm']
          if (readBool(values, 'unlock')) args.push('--unlock')
          if (readBool(values, 'forceKill')) args.push('--force-kill')
          if (readBool(values, 'onReboot')) args.push('--on-reboot')
          args.push('-y', '-f', JSON_FORMAT)
          if (readBool(values, 'force')) args.push('--force')
          pushOption(args, '--reason', readText(values, 'reason'))
          args.push(path)
          return args
        },
        previewSummary: (values) => `删除 ${pathTarget(values)}`,
      }),
      guardedTask({
        id: 'mv',
        workspace: 'files-security',
        title: '移动文件',
        description: '使用 xun mv 做 dry-run 与执行。',
        action: 'mv',
        tone: 'danger',
        feature: 'lock',
        fields: [
          { key: 'src', label: '源路径', type: 'text', required: true },
          { key: 'dst', label: '目标路径', type: 'text', required: true },
          { key: 'unlock', label: '自动解锁', type: 'checkbox', defaultValue: false },
          { key: 'forceKill', label: '强制结束占用进程', type: 'checkbox', defaultValue: false },
          { key: 'force', label: '绕过保护', type: 'checkbox', defaultValue: false },
          { key: 'reason', label: '绕过原因', type: 'text' },
        ],
        target: (values) => `${readText(values, 'src')} -> ${readText(values, 'dst')}`,
        buildPreviewArgs: (values) => moveLikeArgs('mv', values, true),
        buildExecuteArgs: (values) => moveLikeArgs('mv', values, false),
        previewSummary: (values) => `移动 ${readText(values, 'src')} -> ${readText(values, 'dst')}`,
      }),
      guardedTask({
        id: 'ren',
        workspace: 'files-security',
        title: '重命名文件',
        description: '使用 xun ren 做 dry-run 与执行。',
        action: 'ren',
        tone: 'danger',
        feature: 'lock',
        fields: [
          { key: 'src', label: '源路径', type: 'text', required: true },
          { key: 'dst', label: '目标路径', type: 'text', required: true },
          { key: 'unlock', label: '自动解锁', type: 'checkbox', defaultValue: false },
          { key: 'forceKill', label: '强制结束占用进程', type: 'checkbox', defaultValue: false },
          { key: 'force', label: '绕过保护', type: 'checkbox', defaultValue: false },
          { key: 'reason', label: '绕过原因', type: 'text' },
        ],
        target: (values) => `${readText(values, 'src')} -> ${readText(values, 'dst')}`,
        buildPreviewArgs: (values) => moveLikeArgs('ren', values, true),
        buildExecuteArgs: (values) => moveLikeArgs('ren', values, false),
        previewSummary: (values) => `重命名 ${readText(values, 'src')} -> ${readText(values, 'dst')}`,
      }),
      runTask({
        id: 'lock-who',
        workspace: 'files-security',
        title: '查询占用者',
        description: '查看是谁锁住了文件。',
        action: 'lock:who',
        feature: 'lock',
        fields: [{ key: 'path', label: '路径', type: 'text', required: true }],
        target: (values) => pathTarget(values),
        buildRunArgs: (values) => ['lock', 'who', '-f', JSON_FORMAT, readText(values, 'path')],
      }),
      runTask({
        id: 'protect-status',
        workspace: 'files-security',
        title: '保护状态',
        description: '查询当前保护规则。',
        action: 'protect:status',
        feature: 'protect',
        fields: [{ key: 'path', label: '路径前缀', type: 'text', placeholder: '可选' }],
        target: (values) => pathTarget(values),
        buildRunArgs: (values) => ['protect', 'status', '-f', JSON_FORMAT, ...(readText(values, 'path') ? [readText(values, 'path')] : [])],
      }),
      guardedTask({
        id: 'protect-set',
        workspace: 'files-security',
        title: '设置保护',
        description: '先查看 status，再写入保护规则。',
        action: 'protect:set',
        tone: 'danger',
        feature: 'protect',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'deny', label: '拒绝动作', type: 'text', defaultValue: 'delete,move,rename' },
          { key: 'require', label: '绕过要求', type: 'text', defaultValue: 'force,reason' },
          { key: 'systemAcl', label: '同步系统 ACL', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['protect', 'status', '-f', JSON_FORMAT, readText(values, 'path')],
        buildExecuteArgs: (values) => {
          const args = ['protect', 'set', readText(values, 'path')]
          pushOption(args, '--deny', readText(values, 'deny'))
          pushOption(args, '--require', readText(values, 'require'))
          if (readBool(values, 'systemAcl')) args.push('--system-acl')
          return args
        },
        previewSummary: (values) => `设置保护 ${pathTarget(values)}`,
      }),
      guardedTask({
        id: 'protect-clear',
        workspace: 'files-security',
        title: '清除保护',
        description: '先查看 status，再移除保护规则。',
        action: 'protect:clear',
        tone: 'danger',
        feature: 'protect',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'systemAcl', label: '同步清除系统 ACL', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['protect', 'status', '-f', JSON_FORMAT, readText(values, 'path')],
        buildExecuteArgs: (values) => {
          const args = ['protect', 'clear', readText(values, 'path')]
          if (readBool(values, 'systemAcl')) args.push('--system-acl')
          return args
        },
        previewSummary: (values) => `清除保护 ${pathTarget(values)}`,
      }),
    ],
  },
  {
    id: 'acl-crypto',
    title: 'ACL 与加解密',
    description: 'ACL / Encrypt / Decrypt 全部纳入统一 guard 流。',
    tasks: [
      runTask({
        id: 'acl-view',
        workspace: 'files-security',
        title: '查看 ACL',
        description: '查看路径的 ACL 摘要或详细 ACE。',
        action: 'acl:view',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'detail', label: '详细模式', type: 'checkbox', defaultValue: false },
          { key: 'export', label: '导出 CSV', type: 'text', placeholder: '可选' },
        ],
        target: (values) => pathTarget(values),
        buildRunArgs: (values) => {
          const args = ['acl', 'view', '-p', readText(values, 'path')]
          if (readBool(values, 'detail')) args.push('--detail')
          pushOption(args, '--export', readText(values, 'export'))
          return args
        },
      }),
      guardedTask({
        id: 'acl-add',
        workspace: 'files-security',
        title: '新增 ACL 规则',
        description: '通过 view 预览现状，再添加显式 ACE。',
        action: 'acl:add',
        tone: 'danger',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'principal', label: '主体', type: 'text', required: true, placeholder: 'BUILTIN\\Users' },
          { key: 'rights', label: '权限', type: 'select', defaultValue: 'Read', options: aclRightsOptions },
          { key: 'aceType', label: '类型', type: 'select', defaultValue: 'Allow', options: aclAceTypeOptions },
          { key: 'inherit', label: '继承', type: 'select', defaultValue: 'BothInherit', options: aclInheritOptions },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['acl', 'view', '-p', readText(values, 'path'), '--detail'],
        buildExecuteArgs: (values) => [
          'acl', 'add', '-p', readText(values, 'path'), '--principal', readText(values, 'principal'), '--rights', readText(values, 'rights') || 'Read', '--ace-type', readText(values, 'aceType') || 'Allow', '--inherit', readText(values, 'inherit') || 'BothInherit', '-y',
        ],
        previewSummary: (values) => `为 ${pathTarget(values)} 添加 ACL`,
      }),
      guardedTask({
        id: 'encrypt',
        workspace: 'files-security',
        title: '加密文件',
        description: '先验证路径，再执行 EFS 或 age 公钥加密。',
        action: 'encrypt',
        tone: 'danger',
        feature: 'crypt',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'efs', label: '使用 EFS', type: 'checkbox', defaultValue: false },
          { key: 'to', label: '收件公钥', type: 'textarea', placeholder: '多个 key 可换行或逗号分隔' },
          { key: 'out', label: '输出路径', type: 'text', placeholder: '可选' },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => previewPath(values),
        buildExecuteArgs: (values) => {
          const args = ['encrypt']
          if (readBool(values, 'efs')) args.push('--efs')
          pushRepeatableOption(args, '--to', readText(values, 'to'))
          pushOption(args, '-o', readText(values, 'out'))
          args.push(readText(values, 'path'))
          return args
        },
        previewSummary: (values) => `加密 ${pathTarget(values)}`,
      }),
      guardedTask({
        id: 'decrypt',
        workspace: 'files-security',
        title: '解密文件',
        description: '先验证路径，再执行 EFS 或 identity 解密。',
        action: 'decrypt',
        tone: 'danger',
        feature: 'crypt',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'efs', label: '使用 EFS', type: 'checkbox', defaultValue: false },
          { key: 'identity', label: '身份文件', type: 'textarea', placeholder: '多个文件可换行或逗号分隔' },
          { key: 'out', label: '输出路径', type: 'text', placeholder: '可选' },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => previewPath(values),
        buildExecuteArgs: (values) => {
          const args = ['decrypt']
          if (readBool(values, 'efs')) args.push('--efs')
          pushRepeatableOption(args, '-i', readText(values, 'identity'))
          pushOption(args, '-o', readText(values, 'out'))
          args.push(readText(values, 'path'))
          return args
        },
        previewSummary: (values) => `解密 ${pathTarget(values)}`,
      }),
    ],
  },
]

export const integrationAutomationTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'shell-bootstrap',
    title: 'Shell 集成',
    description: '初始化 wrapper、导出补全脚本与内部补全调试。',
    tasks: [
      runTask({
        id: 'init',
        workspace: 'integration-automation',
        title: '生成 init',
        description: '输出 shell 初始化脚本。',
        action: 'init',
        fields: [{ key: 'shell', label: 'Shell', type: 'select', defaultValue: 'powershell', options: shellInitOptions }],
        buildRunArgs: (values) => ['init', readText(values, 'shell') || 'powershell'],
      }),
      runTask({
        id: 'completion',
        workspace: 'integration-automation',
        title: '生成补全',
        description: '输出 shell completion 脚本。',
        action: 'completion',
        fields: [{ key: 'shell', label: 'Shell', type: 'select', defaultValue: 'powershell', options: shellCompletionOptions }],
        buildRunArgs: (values) => ['completion', readText(values, 'shell') || 'powershell'],
      }),
      runTask({
        id: 'complete',
        workspace: 'integration-automation',
        title: '补全调试',
        description: '调用内部 __complete 入口调试补全结果。',
        action: '__complete',
        fields: [{ key: 'args', label: '预分词参数', type: 'textarea', required: true, placeholder: '例如 alias ls --j' }],
        target: (values) => readText(values, 'args'),
        buildRunArgs: (values) => ['__complete', ...splitCommand(readText(values, 'args'))],
      }),
    ],
  },
  {
    id: 'alias-tools',
    title: '别名与同步',
    description: '将 alias 族命令纳入 Dashboard。',
    tasks: [
      runTask({
        id: 'alias-ls',
        workspace: 'integration-automation',
        title: '列出别名',
        description: '按类型和标签筛选 alias。',
        action: 'alias:ls',
        feature: 'alias',
        fields: [
          { key: 'type', label: '类型', type: 'select', defaultValue: '', options: [{ label: '全部', value: '' }, { label: 'cmd', value: 'cmd' }, { label: 'app', value: 'app' }] },
          { key: 'tag', label: '标签', type: 'text', placeholder: '可选' },
        ],
        buildRunArgs: (values) => {
          const args = ['alias', 'ls', '--json']
          pushOption(args, '--type', readText(values, 'type'))
          pushOption(args, '--tag', readText(values, 'tag'))
          return args
        },
      }),
      runTask({
        id: 'alias-find',
        workspace: 'integration-automation',
        title: '查找别名',
        description: '按关键字模糊匹配 alias。',
        action: 'alias:find',
        feature: 'alias',
        fields: [{ key: 'keyword', label: '关键字', type: 'text', required: true }],
        target: (values) => readText(values, 'keyword'),
        buildRunArgs: (values) => ['alias', 'find', readText(values, 'keyword')],
      }),
      runTask({
        id: 'alias-which',
        workspace: 'integration-automation',
        title: '解析别名',
        description: '查看 alias 指向与 shim 信息。',
        action: 'alias:which',
        feature: 'alias',
        fields: [{ key: 'name', label: '别名', type: 'text', required: true }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => ['alias', 'which', readText(values, 'name')],
      }),
      runTask({
        id: 'alias-sync',
        workspace: 'integration-automation',
        title: '同步别名',
        description: '同步 shim、应用路径和 shell 配置。',
        action: 'alias:sync',
        feature: 'alias',
        fields: [],
        buildRunArgs: () => ['alias', 'sync'],
      }),
    ],
  },
  {
    id: 'rename-tools',
    title: '批量改名',
    description: 'brn 默认 dry-run，执行时统一走 guarded。',
    tasks: [
      guardedTask({
        id: 'brn',
        workspace: 'integration-automation',
        title: '批量重命名',
        description: '支持 regex / case / prefix / suffix / strip-prefix / seq。默认先预览。',
        action: 'brn',
        tone: 'danger',
        feature: 'batch_rename',
        fields: [
          { key: 'path', label: '扫描目录', type: 'text', defaultValue: '.' },
          { key: 'regex', label: 'Regex', type: 'text', placeholder: '可选' },
          { key: 'replace', label: 'Replace', type: 'text', placeholder: '用于 --regex' },
          { key: 'case', label: 'Case', type: 'select', defaultValue: '', options: brnCaseOptions },
          { key: 'prefix', label: '前缀', type: 'text', placeholder: '可选' },
          { key: 'suffix', label: '后缀', type: 'text', placeholder: '可选' },
          { key: 'stripPrefix', label: '移除前缀', type: 'text', placeholder: '可选' },
          { key: 'seq', label: '追加序号', type: 'checkbox', defaultValue: false },
          { key: 'start', label: '起始值', type: 'number', defaultValue: '1' },
          { key: 'pad', label: '补零位数', type: 'number', defaultValue: '3' },
          { key: 'ext', label: '扩展名过滤', type: 'text', placeholder: 'ts,tsx,vue' },
          { key: 'recursive', label: '递归扫描', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'path') || '.',
        buildPreviewArgs: (values) => {
          const args = ['brn', readText(values, 'path') || '.']
          pushOption(args, '--regex', readText(values, 'regex'))
          pushOption(args, '--replace', readText(values, 'replace'))
          pushOption(args, '--case', readText(values, 'case'))
          pushOption(args, '--prefix', readText(values, 'prefix'))
          pushOption(args, '--suffix', readText(values, 'suffix'))
          pushOption(args, '--strip-prefix', readText(values, 'stripPrefix'))
          if (readBool(values, 'seq')) args.push('--seq')
          if (readBool(values, 'seq')) pushOption(args, '--start', readText(values, 'start') || '1')
          if (readBool(values, 'seq')) pushOption(args, '--pad', readText(values, 'pad') || '3')
          pushRepeatableOption(args, '--ext', readText(values, 'ext'))
          if (readBool(values, 'recursive')) args.push('-r')
          return args
        },
        buildExecuteArgs: (values) => {
          const args = ['brn', readText(values, 'path') || '.']
          pushOption(args, '--regex', readText(values, 'regex'))
          pushOption(args, '--replace', readText(values, 'replace'))
          pushOption(args, '--case', readText(values, 'case'))
          pushOption(args, '--prefix', readText(values, 'prefix'))
          pushOption(args, '--suffix', readText(values, 'suffix'))
          pushOption(args, '--strip-prefix', readText(values, 'stripPrefix'))
          if (readBool(values, 'seq')) args.push('--seq')
          if (readBool(values, 'seq')) pushOption(args, '--start', readText(values, 'start') || '1')
          if (readBool(values, 'seq')) pushOption(args, '--pad', readText(values, 'pad') || '3')
          pushRepeatableOption(args, '--ext', readText(values, 'ext'))
          if (readBool(values, 'recursive')) args.push('-r')
          args.push('--apply', '-y')
          return args
        },
        previewSummary: (values) => `批量重命名 ${readText(values, 'path') || '.'}`,
      }),
    ],
  },
]

export const mediaConversionTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'image-tools',
    title: '图像处理',
    description: 'img 统一以任务卡形式接入。',
    tasks: [
      runTask({
        id: 'img',
        workspace: 'media-conversion',
        title: '图像转换',
        description: '压缩或转换图片目录。',
        action: 'img',
        feature: 'img',
        fields: [
          { key: 'input', label: '输入', type: 'text', required: true },
          { key: 'output', label: '输出目录', type: 'text', required: true },
          { key: 'format', label: '格式', type: 'select', defaultValue: 'webp', options: imgFormatOptions },
          { key: 'quality', label: '质量', type: 'number', defaultValue: '80', min: 1, max: 100 },
          { key: 'mw', label: '最大宽度', type: 'number', placeholder: '可选' },
          { key: 'mh', label: '最大高度', type: 'number', placeholder: '可选' },
          { key: 'overwrite', label: '覆盖输出', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['img', '-i', readText(values, 'input'), '-o', readText(values, 'output'), '-f', readText(values, 'format') || 'webp', '-q', readText(values, 'quality') || '80']
          pushOption(args, '--mw', readText(values, 'mw'))
          pushOption(args, '--mh', readText(values, 'mh'))
          if (readBool(values, 'overwrite')) args.push('--overwrite')
          return args
        },
      }),
    ],
  },
  {
    id: 'video-tools',
    title: '视频处理',
    description: 'probe / compress / remux 全部在本地控制台内完成。',
    tasks: [
      runTask({
        id: 'video-probe',
        workspace: 'media-conversion',
        title: '视频探测',
        description: '读取媒体元数据。',
        action: 'video:probe',
        fields: [
          { key: 'input', label: '输入文件', type: 'text', required: true },
          { key: 'ffprobe', label: 'ffprobe 路径', type: 'text', placeholder: '可选' },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['video', 'probe', '-i', readText(values, 'input')]
          pushOption(args, '--ffprobe', readText(values, 'ffprobe'))
          return args
        },
      }),
      runTask({
        id: 'video-compress',
        workspace: 'media-conversion',
        title: '视频压缩',
        description: '按 mode / engine 做转码压缩。',
        action: 'video:compress',
        fields: [
          { key: 'input', label: '输入文件', type: 'text', required: true },
          { key: 'output', label: '输出文件', type: 'text', required: true },
          { key: 'mode', label: '模式', type: 'select', defaultValue: 'balanced', options: videoModeOptions },
          { key: 'engine', label: '引擎', type: 'select', defaultValue: 'auto', options: videoEngineOptions },
          { key: 'ffmpeg', label: 'ffmpeg 路径', type: 'text', placeholder: '可选' },
          { key: 'overwrite', label: '覆盖输出', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['video', 'compress', '-i', readText(values, 'input'), '-o', readText(values, 'output'), '--mode', readText(values, 'mode') || 'balanced', '--engine', readText(values, 'engine') || 'auto']
          pushOption(args, '--ffmpeg', readText(values, 'ffmpeg'))
          if (readBool(values, 'overwrite')) args.push('--overwrite')
          return args
        },
      }),
      runTask({
        id: 'video-remux',
        workspace: 'media-conversion',
        title: '无损封装转换',
        description: '做 remux 容器迁移。',
        action: 'video:remux',
        fields: [
          { key: 'input', label: '输入文件', type: 'text', required: true },
          { key: 'output', label: '输出文件', type: 'text', required: true },
          { key: 'strict', label: '严格模式', type: 'checkbox', defaultValue: true },
          { key: 'ffmpeg', label: 'ffmpeg 路径', type: 'text', placeholder: '可选' },
          { key: 'ffprobe', label: 'ffprobe 路径', type: 'text', placeholder: '可选' },
          { key: 'overwrite', label: '覆盖输出', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['video', 'remux', '-i', readText(values, 'input'), '-o', readText(values, 'output'), '--strict', readBool(values, 'strict') ? 'true' : 'false']
          pushOption(args, '--ffmpeg', readText(values, 'ffmpeg'))
          pushOption(args, '--ffprobe', readText(values, 'ffprobe'))
          if (readBool(values, 'overwrite')) args.push('--overwrite')
          return args
        },
      }),
    ],
  },
]

export const statisticsDiagnosticsTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'statistics-tools',
    title: '代码统计与清理线索',
    description: 'cstat 进入统计与诊断工作台。',
    tasks: [
      runTask({
        id: 'cstat',
        workspace: 'statistics-diagnostics',
        title: '目录统计',
        description: '扫描空文件、大文件、重复文件和临时文件。',
        action: 'cstat',
        feature: 'cstat',
        fields: [
          { key: 'path', label: '路径', type: 'text', defaultValue: '.' },
          { key: 'empty', label: '空文件', type: 'checkbox', defaultValue: false },
          { key: 'large', label: '大文件阈值(行)', type: 'number', placeholder: '可选' },
          { key: 'dup', label: '重复文件', type: 'checkbox', defaultValue: false },
          { key: 'tmp', label: '临时文件', type: 'checkbox', defaultValue: false },
          { key: 'all', label: '全部检查', type: 'checkbox', defaultValue: false },
          { key: 'ext', label: '扩展名过滤', type: 'text', placeholder: 'rs,ts,vue' },
          { key: 'depth', label: '最大深度', type: 'number', placeholder: '可选' },
          { key: 'output', label: '导出 JSON', type: 'text', placeholder: 'report.json' },
        ],
        target: (values) => readText(values, 'path') || '.',
        buildRunArgs: (values) => {
          const args = ['cstat', readText(values, 'path') || '.', '-f', JSON_FORMAT]
          if (readBool(values, 'empty')) args.push('--empty')
          pushOption(args, '--large', readText(values, 'large'))
          if (readBool(values, 'dup')) args.push('--dup')
          if (readBool(values, 'tmp')) args.push('--tmp')
          if (readBool(values, 'all')) args.push('--all')
          pushRepeatableOption(args, '--ext', readText(values, 'ext'))
          pushOption(args, '--depth', readText(values, 'depth'))
          pushOption(args, '-o', readText(values, 'output'))
          return args
        },
      }),
    ],
  },
]
