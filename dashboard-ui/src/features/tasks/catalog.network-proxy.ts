import type {
  TaskFieldType,
  TaskFieldValue,
  TaskFormState,
  TaskFieldOption,
  TaskFieldDefinition,
  TaskNoticeTone,
  TaskNotice,
  WorkspaceTaskDefinition,
  WorkspaceTaskGroup,
  WorkspaceTabDefinition
} from './catalog-shared'

import {
  JSON_FORMAT,
  desktopWindowNotices,
  desktopHostsNotices,
  desktopColorNotices,
  shellInitOptions,
  shellCompletionOptions,
  aliasTypeOptions,
  aliasModeOptions,
  aliasShellOptions,
  dedupModeOptions,
  brnCaseOptions,
  imgFormatOptions,
  imgSvgMethodOptions,
  imgJpegBackendOptions,
  aliasAppScanSourceOptions,
  videoModeOptions,
  videoEngineOptions,
  aclRightsOptions,
  aclAceTypeOptions,
  aclInheritOptions,
  aclInheritModeOptions,
  readText,
  readBool,
  splitItems,
  splitCommand,
  pushOption,
  pushRepeatableOption,
  runTask,
  guardedTask,
  pathTarget,
  previewPath,
  moveLikeArgs
} from './catalog-shared'

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
