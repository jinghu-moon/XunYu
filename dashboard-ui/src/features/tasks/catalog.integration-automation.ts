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

export const integrationAutomationTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'shell-bootstrap',
    title: 'Shell 集成',
    description: '通过 init / completion / __complete 形成安装、导出与验证闭环。',
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
    description: '覆盖 alias 常用治理动作，并为删除类操作启用 Triple-Guard。',
    tasks: [
      runTask({
        id: 'alias-setup',
        workspace: 'integration-automation',
        title: '初始化 alias 运行时',
        description: '安装 shim 模板与 shell 集成。',
        action: 'alias:setup',
        feature: 'alias',
        fields: [
          { key: 'no_cmd', label: '跳过 cmd', type: 'checkbox', defaultValue: false },
          { key: 'no_ps', label: '跳过 PowerShell', type: 'checkbox', defaultValue: false },
          { key: 'no_bash', label: '跳过 Bash', type: 'checkbox', defaultValue: false },
          { key: 'no_nu', label: '跳过 Nushell', type: 'checkbox', defaultValue: false },
          { key: 'core_only', label: '仅核心 Shell', type: 'checkbox', defaultValue: false },
        ],
        buildRunArgs: (values) => {
          const args = ['alias', 'setup']
          if (readBool(values, 'no_cmd')) args.push('--no-cmd')
          if (readBool(values, 'no_ps')) args.push('--no-ps')
          if (readBool(values, 'no_bash')) args.push('--no-bash')
          if (readBool(values, 'no_nu')) args.push('--no-nu')
          if (readBool(values, 'core_only')) args.push('--core-only')
          return args
        },
      }),
      runTask({
        id: 'alias-add',
        workspace: 'integration-automation',
        title: '新增命令别名',
        description: '添加普通 alias，并支持 tags / shells / force。',
        action: 'alias:add',
        feature: 'alias',
        fields: [
          { key: 'name', label: '别名', type: 'text', required: true, placeholder: 'gst' },
          { key: 'command', label: '命令串', type: 'textarea', required: true, placeholder: 'git status -sb' },
          { key: 'mode', label: '模式', type: 'select', defaultValue: 'auto', options: aliasModeOptions },
          { key: 'desc', label: '说明', type: 'text', placeholder: '可选' },
          { key: 'tag', label: '标签', type: 'textarea', placeholder: 'dev,git' },
          { key: 'shell', label: '生效 Shell', type: 'textarea', placeholder: 'cmd,ps,bash,nu' },
          { key: 'force', label: '覆盖已存在别名', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = [
            'alias',
            'add',
            readText(values, 'name'),
            readText(values, 'command'),
            '--mode',
            readText(values, 'mode') || 'auto',
          ]
          pushOption(args, '--desc', readText(values, 'desc'))
          pushRepeatableOption(args, '--tag', readText(values, 'tag'))
          pushRepeatableOption(args, '--shell', readText(values, 'shell'))
          if (readBool(values, 'force')) args.push('--force')
          return args
        },
      }),
      guardedTask({
        id: 'alias-rm',
        workspace: 'integration-automation',
        title: '删除命令别名',
        description: '先查看 alias 指向，再执行删除。',
        action: 'alias:rm',
        tone: 'danger',
        feature: 'alias',
        fields: [{ key: 'name', label: '别名', type: 'text', required: true, placeholder: 'gst' }],
        target: (values) => readText(values, 'name'),
        buildPreviewArgs: (values) => ['alias', 'which', readText(values, 'name')],
        buildExecuteArgs: (values) => ['alias', 'rm', readText(values, 'name')],
        previewSummary: (values) => `删除 alias ${readText(values, 'name')} 前先查看解析结果`,
      }),
      runTask({
        id: 'alias-ls',
        workspace: 'integration-automation',
        title: '列出别名',
        description: '按类型和标签筛选 alias。',
        action: 'alias:ls',
        feature: 'alias',
        fields: [
          { key: 'type', label: '类型', type: 'select', defaultValue: '', options: aliasTypeOptions },
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
      runTask({
        id: 'alias-export',
        workspace: 'integration-automation',
        title: '导出别名',
        description: '导出 aliases.toml 到指定文件或 stdout。',
        action: 'alias:export',
        feature: 'alias',
        fields: [{ key: 'output', label: '输出文件', type: 'text', placeholder: '可选；留空输出到 stdout' }],
        buildRunArgs: (values) => {
          const args = ['alias', 'export']
          pushOption(args, '-o', readText(values, 'output'))
          return args
        },
      }),
      runTask({
        id: 'alias-import',
        workspace: 'integration-automation',
        title: '导入别名',
        description: '从 TOML 文件导入别名定义。',
        action: 'alias:import',
        feature: 'alias',
        fields: [
          { key: 'file', label: 'TOML 文件', type: 'text', required: true, placeholder: 'D:/xun/aliases.toml' },
          { key: 'force', label: '覆盖冲突项', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'file'),
        buildRunArgs: (values) => {
          const args = ['alias', 'import', readText(values, 'file')]
          if (readBool(values, 'force')) args.push('--force')
          return args
        },
      }),
      runTask({
        id: 'alias-app-add',
        workspace: 'integration-automation',
        title: '新增应用别名',
        description: '为可执行文件注册 app alias。',
        action: 'alias:app-add',
        feature: 'alias',
        fields: [
          { key: 'name', label: '别名', type: 'text', required: true, placeholder: 'code' },
          { key: 'exe', label: '可执行文件', type: 'text', required: true, placeholder: 'C:/Program Files/Microsoft VS Code/Code.exe' },
          { key: 'args', label: '固定参数', type: 'text', placeholder: '可选' },
          { key: 'desc', label: '说明', type: 'text', placeholder: '可选' },
          { key: 'tag', label: '标签', type: 'textarea', placeholder: 'editor,dev' },
          { key: 'no_apppaths', label: '禁用 App Paths 注册', type: 'checkbox', defaultValue: false },
          { key: 'force', label: '覆盖冲突项', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = ['alias', 'app', 'add', readText(values, 'name'), readText(values, 'exe')]
          pushOption(args, '--args', readText(values, 'args'))
          pushOption(args, '--desc', readText(values, 'desc'))
          pushRepeatableOption(args, '--tag', readText(values, 'tag'))
          if (readBool(values, 'no_apppaths')) args.push('--no-apppaths')
          if (readBool(values, 'force')) args.push('--force')
          return args
        },
      }),
      guardedTask({
        id: 'alias-app-rm',
        workspace: 'integration-automation',
        title: '删除应用别名',
        description: '先查看 app alias 指向，再执行删除。',
        action: 'alias:app-rm',
        tone: 'danger',
        feature: 'alias',
        fields: [{ key: 'name', label: '应用别名', type: 'text', required: true, placeholder: 'code' }],
        target: (values) => readText(values, 'name'),
        buildPreviewArgs: (values) => ['alias', 'app', 'which', readText(values, 'name')],
        buildExecuteArgs: (values) => ['alias', 'app', 'rm', readText(values, 'name')],
        previewSummary: (values) => `删除 app alias ${readText(values, 'name')} 前先查看解析结果`,
      }),
      runTask({
        id: 'alias-app-ls',
        workspace: 'integration-automation',
        title: '列出应用别名',
        description: '以 JSON 输出当前 app alias 清单。',
        action: 'alias:app-ls',
        feature: 'alias',
        fields: [],
        buildRunArgs: () => ['alias', 'app', 'ls', '--json'],
      }),
      runTask({
        id: 'alias-app-scan',
        workspace: 'integration-automation',
        title: '扫描应用候选项',
        description: '从注册表、开始菜单或 PATH 扫描可注册应用。',
        action: 'alias:app-scan',
        feature: 'alias',
        fields: [
          { key: 'source', label: '扫描来源', type: 'select', defaultValue: 'all', options: aliasAppScanSourceOptions },
          { key: 'filter', label: '关键字过滤', type: 'text', placeholder: '可选' },
          { key: 'all', label: '加入全部扫描结果', type: 'checkbox', defaultValue: false },
          { key: 'no_cache', label: '绕过缓存', type: 'checkbox', defaultValue: false },
        ],
        buildRunArgs: (values) => {
          const args = ['alias', 'app', 'scan', '--source', readText(values, 'source') || 'all', '--json']
          pushOption(args, '--filter', readText(values, 'filter'))
          if (readBool(values, 'all')) args.push('--all')
          if (readBool(values, 'no_cache')) args.push('--no-cache')
          return args
        },
      }),
      runTask({
        id: 'alias-app-which',
        workspace: 'integration-automation',
        title: '解析应用别名',
        description: '查看 app alias 指向的可执行文件。',
        action: 'alias:app-which',
        feature: 'alias',
        fields: [{ key: 'name', label: '应用别名', type: 'text', required: true, placeholder: 'code' }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => ['alias', 'app', 'which', readText(values, 'name')],
      }),
      runTask({
        id: 'alias-app-sync',
        workspace: 'integration-automation',
        title: '同步应用别名',
        description: '只同步 app alias 相关落地。',
        action: 'alias:app-sync',
        feature: 'alias',
        fields: [],
        buildRunArgs: () => ['alias', 'app', 'sync'],
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
