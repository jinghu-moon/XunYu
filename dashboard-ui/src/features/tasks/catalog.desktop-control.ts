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

export const desktopControlTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'desktop-overview',
    title: '桌面控制概览',
    description: '桌面控制能力的任务入口与状态检查。',
    tasks: [
      runTask({
        id: 'desktop-daemon-status',
        workspace: 'desktop-control',
        title: '守护进程状态',
        description: '查看 desktop daemon 当前状态。',
        action: 'desktop:daemon-status',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'daemon', 'status'],
      }),
    ],
  },
  {
    id: 'desktop-daemon',
    title: '守护进程',
    description: '启动、停止与重载 desktop daemon。',
    tasks: [
      runTask({
        id: 'desktop-daemon-start',
        workspace: 'desktop-control',
        title: '启动守护进程',
        description: '启动 desktop daemon，可按需选择托盘/提权。',
        action: 'desktop:daemon-start',
        feature: 'desktop',
        fields: [
          { key: 'quiet', label: '静默输出', type: 'checkbox', defaultValue: false },
          { key: 'no_tray', label: '禁用托盘', type: 'checkbox', defaultValue: false },
          { key: 'elevated', label: '必要时提权', type: 'checkbox', defaultValue: false },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'daemon', 'start']
          if (readBool(values, 'quiet')) args.push('-q')
          if (readBool(values, 'no_tray')) args.push('--no-tray')
          if (readBool(values, 'elevated')) args.push('--elevated')
          return args
        },
      }),
      runTask({
        id: 'desktop-daemon-stop',
        workspace: 'desktop-control',
        title: '停止守护进程',
        description: '停止 desktop daemon。',
        action: 'desktop:daemon-stop',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'daemon', 'stop'],
      }),
      runTask({
        id: 'desktop-daemon-reload',
        workspace: 'desktop-control',
        title: '重载配置',
        description: '通知 desktop daemon 重新加载配置。',
        action: 'desktop:daemon-reload',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'daemon', 'reload'],
      }),
    ],
  },
  {
    id: 'desktop-hotkeys',
    title: '全局热键',
    description: '绑定、解绑与查看全局热键。',
    tasks: [
      runTask({
        id: 'desktop-hotkey-bind',
        workspace: 'desktop-control',
        title: '绑定热键',
        description: '将热键绑定到动作，例如 run:wt.exe。',
        action: 'desktop:hotkey-bind',
        feature: 'desktop',
        fields: [
          { key: 'hotkey', label: '热键', type: 'text', required: true, placeholder: 'ctrl+alt+t' },
          { key: 'action', label: '动作', type: 'text', required: true, placeholder: 'run:wt.exe' },
          { key: 'app', label: '应用过滤', type: 'text', placeholder: '可选，如 code.exe' },
        ],
        target: (values) => readText(values, 'hotkey'),
        buildRunArgs: (values) => {
          const args = ['desktop', 'hotkey', 'bind', readText(values, 'hotkey'), readText(values, 'action')]
          pushOption(args, '--app', readText(values, 'app'))
          return args
        },
      }),
      runTask({
        id: 'desktop-hotkey-unbind',
        workspace: 'desktop-control',
        title: '解绑热键',
        description: '删除热键绑定。',
        action: 'desktop:hotkey-unbind',
        feature: 'desktop',
        fields: [{ key: 'hotkey', label: '热键', type: 'text', required: true }],
        target: (values) => readText(values, 'hotkey'),
        buildRunArgs: (values) => ['desktop', 'hotkey', 'unbind', readText(values, 'hotkey')],
      }),
      runTask({
        id: 'desktop-hotkey-list',
        workspace: 'desktop-control',
        title: '热键列表',
        description: '列出当前热键绑定。',
        action: 'desktop:hotkey-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'hotkey', 'list'],
      }),
    ],
  },
  {
    id: 'desktop-remaps',
    title: '键位重映射',
    description: '管理键位重映射规则。',
    tasks: [
      guardedTask({
        id: 'desktop-remap-add',
        workspace: 'desktop-control',
        title: '添加重映射',
        description: '将输入按键映射为另一组按键。',
        action: 'desktop:remap-add',
        tone: 'danger',
        feature: 'desktop',
        fields: [
          { key: 'from', label: '原始按键', type: 'text', required: true, placeholder: 'ctrl+alt+1' },
          { key: 'to', label: '目标按键', type: 'text', required: true, placeholder: 'alt+1' },
          { key: 'app', label: '应用过滤', type: 'text', placeholder: '可选，如 code.exe' },
          { key: 'exact', label: '精确匹配应用名', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'from'),
        buildPreviewArgs: (values) => {
          const args = ['desktop', 'remap', 'add', readText(values, 'from'), readText(values, 'to')]
          pushOption(args, '--app', readText(values, 'app'))
          if (readBool(values, 'exact')) args.push('--exact')
          args.push('--dry-run')
          return args
        },
        buildExecuteArgs: (values) => {
          const args = ['desktop', 'remap', 'add', readText(values, 'from'), readText(values, 'to')]
          pushOption(args, '--app', readText(values, 'app'))
          if (readBool(values, 'exact')) args.push('--exact')
          return args
        },
        previewSummary: (values) => `重映射 ${readText(values, 'from')} -> ${readText(values, 'to')}`,
      }),
      guardedTask({
        id: 'desktop-remap-remove',
        workspace: 'desktop-control',
        title: '移除重映射',
        description: '删除已有的重映射规则。',
        action: 'desktop:remap-remove',
        tone: 'danger',
        feature: 'desktop',
        fields: [
          { key: 'from', label: '原始按键', type: 'text', required: true },
          { key: 'to', label: '目标按键', type: 'text', placeholder: '可选，留空则移除全部匹配' },
        ],
        target: (values) => readText(values, 'from'),
        buildPreviewArgs: (values) => {
          const args = ['desktop', 'remap', 'remove', readText(values, 'from')]
          const to = readText(values, 'to')
          if (to) args.push(to)
          args.push('--dry-run')
          return args
        },
        buildExecuteArgs: (values) => {
          const args = ['desktop', 'remap', 'remove', readText(values, 'from')]
          const to = readText(values, 'to')
          if (to) args.push(to)
          return args
        },
        previewSummary: (values) => `移除重映射 ${readText(values, 'from')}`,
      }),
      runTask({
        id: 'desktop-remap-list',
        workspace: 'desktop-control',
        title: '重映射列表',
        description: '列出当前键位重映射规则。',
        action: 'desktop:remap-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'remap', 'list'],
      }),
      guardedTask({
        id: 'desktop-remap-clear',
        workspace: 'desktop-control',
        title: '清空重映射',
        description: '清空全部键位重映射规则。',
        action: 'desktop:remap-clear',
        tone: 'danger',
        feature: 'desktop',
        fields: [],
        target: () => 'all',
        buildPreviewArgs: () => ['desktop', 'remap', 'clear', '--dry-run'],
        buildExecuteArgs: () => ['desktop', 'remap', 'clear'],
        previewSummary: () => '清空全部重映射',
      }),
    ],
  },
  {
    id: 'desktop-snippets',
    title: '文本片段',
    description: '管理文本片段规则。',
    tasks: [
      runTask({
        id: 'desktop-snippet-add',
        workspace: 'desktop-control',
        title: '新增片段',
        description: '添加文本片段触发器。',
        action: 'desktop:snippet-add',
        feature: 'desktop',
        fields: [
          { key: 'trigger', label: '触发文本', type: 'text', required: true },
          { key: 'expand', label: '展开内容', type: 'textarea', required: true },
          { key: 'app', label: '应用过滤', type: 'text', placeholder: '可选，如 code.exe' },
          { key: 'immediate', label: '立即触发', type: 'checkbox', defaultValue: false },
          { key: 'clipboard', label: '剪贴板粘贴', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'trigger'),
        buildRunArgs: (values) => {
          const args = ['desktop', 'snippet', 'add', readText(values, 'trigger'), readText(values, 'expand')]
          pushOption(args, '--app', readText(values, 'app'))
          if (readBool(values, 'immediate')) args.push('--immediate')
          if (readBool(values, 'clipboard')) args.push('--clipboard')
          return args
        },
      }),
      runTask({
        id: 'desktop-snippet-remove',
        workspace: 'desktop-control',
        title: '移除片段',
        description: '删除指定触发文本。',
        action: 'desktop:snippet-remove',
        feature: 'desktop',
        fields: [{ key: 'trigger', label: '触发文本', type: 'text', required: true }],
        target: (values) => readText(values, 'trigger'),
        buildRunArgs: (values) => ['desktop', 'snippet', 'remove', readText(values, 'trigger')],
      }),
      runTask({
        id: 'desktop-snippet-list',
        workspace: 'desktop-control',
        title: '片段列表',
        description: '列出当前文本片段规则。',
        action: 'desktop:snippet-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'snippet', 'list'],
      }),
      runTask({
        id: 'desktop-snippet-clear',
        workspace: 'desktop-control',
        title: '清空片段',
        description: '清空全部文本片段规则。',
        action: 'desktop:snippet-clear',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'snippet', 'clear'],
      }),
    ],
  },
  {
    id: 'desktop-layouts',
    title: '窗口布局',
    description: '创建、预览并应用布局模板。',
    tasks: [
      runTask({
        id: 'desktop-layout-new',
        workspace: 'desktop-control',
        title: '新建布局',
        description: '创建新的布局模板。',
        action: 'desktop:layout-new',
        feature: 'desktop',
        fields: [
          { key: 'name', label: '布局名', type: 'text', required: true },
          {
            key: 'layout_type',
            label: '布局类型',
            type: 'select',
            defaultValue: 'grid',
            options: [{ label: 'grid', value: 'grid' }],
          },
          { key: 'rows', label: '行数', type: 'number', placeholder: '可选' },
          { key: 'cols', label: '列数', type: 'number', placeholder: '可选' },
          { key: 'gap', label: '间距', type: 'number', placeholder: '可选' },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = ['desktop', 'layout', 'new', readText(values, 'name')]
          pushOption(args, '--layout-type', readText(values, 'layout_type') || 'grid')
          pushOption(args, '--rows', readText(values, 'rows'))
          pushOption(args, '--cols', readText(values, 'cols'))
          pushOption(args, '--gap', readText(values, 'gap'))
          return args
        },
      }),
      runTask({
        id: 'desktop-layout-apply',
        workspace: 'desktop-control',
        title: '应用布局',
        description: '应用指定布局模板。',
        action: 'desktop:layout-apply',
        feature: 'desktop',
        fields: [
          { key: 'name', label: '布局名', type: 'text', required: true },
          { key: 'move_existing', label: '移动已有窗口', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = ['desktop', 'layout', 'apply', readText(values, 'name')]
          if (readBool(values, 'move_existing')) args.push('--move-existing')
          return args
        },
      }),
      runTask({
        id: 'desktop-layout-preview',
        workspace: 'desktop-control',
        title: '预览布局',
        description: '输出布局划分信息。',
        action: 'desktop:layout-preview',
        feature: 'desktop',
        fields: [{ key: 'name', label: '布局名', type: 'text', required: true }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => ['desktop', 'layout', 'preview', readText(values, 'name')],
      }),
      runTask({
        id: 'desktop-layout-list',
        workspace: 'desktop-control',
        title: '布局列表',
        description: '列出已配置的布局。',
        action: 'desktop:layout-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'layout', 'list'],
      }),
      runTask({
        id: 'desktop-layout-remove',
        workspace: 'desktop-control',
        title: '删除布局',
        description: '移除指定布局模板。',
        action: 'desktop:layout-remove',
        feature: 'desktop',
        fields: [{ key: 'name', label: '布局名', type: 'text', required: true }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => ['desktop', 'layout', 'remove', readText(values, 'name')],
      }),
    ],
  },
  {
    id: 'desktop-workspaces',
    title: '桌面工作区',
    description: '保存、启动与管理工作区。',
    tasks: [
      runTask({
        id: 'desktop-workspace-save',
        workspace: 'desktop-control',
        title: '保存工作区',
        description: '保存当前应用布局与目标。',
        action: 'desktop:workspace-save',
        feature: 'desktop',
        fields: [
          { key: 'name', label: '工作区名', type: 'text', required: true },
          { key: 'name_only', label: '仅保存应用名', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = ['desktop', 'workspace', 'save', readText(values, 'name')]
          if (readBool(values, 'name_only')) args.push('--name-only')
          return args
        },
      }),
      runTask({
        id: 'desktop-workspace-launch',
        workspace: 'desktop-control',
        title: '启动工作区',
        description: '按模板启动工作区应用。',
        action: 'desktop:workspace-launch',
        feature: 'desktop',
        fields: [
          { key: 'name', label: '工作区名', type: 'text', required: true },
          { key: 'move_existing', label: '移动已有窗口', type: 'checkbox', defaultValue: false },
          { key: 'monitor_offset', label: '显示器偏移', type: 'number', placeholder: '可选' },
        ],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => {
          const args = ['desktop', 'workspace', 'launch', readText(values, 'name')]
          if (readBool(values, 'move_existing')) args.push('--move-existing')
          pushOption(args, '--monitor-offset', readText(values, 'monitor_offset'))
          return args
        },
      }),
      runTask({
        id: 'desktop-workspace-list',
        workspace: 'desktop-control',
        title: '工作区列表',
        description: '列出已配置的工作区。',
        action: 'desktop:workspace-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'workspace', 'list'],
      }),
      runTask({
        id: 'desktop-workspace-remove',
        workspace: 'desktop-control',
        title: '删除工作区',
        description: '移除指定工作区。',
        action: 'desktop:workspace-remove',
        feature: 'desktop',
        fields: [{ key: 'name', label: '工作区名', type: 'text', required: true }],
        target: (values) => readText(values, 'name'),
        buildRunArgs: (values) => ['desktop', 'workspace', 'remove', readText(values, 'name')],
      }),
    ],
  },
  {
    id: 'desktop-windows',
    title: '窗口控制',
    description: '聚焦、移动、调整大小与置顶控制。',
    tasks: [
      runTask({
        id: 'desktop-window-focus',
        workspace: 'desktop-control',
        title: '聚焦窗口',
        description: '按应用或标题聚焦窗口。',
        action: 'desktop:window-focus',
        feature: 'desktop',
        notices: desktopWindowNotices,
        fields: [
          { key: 'app', label: '应用名', type: 'text', placeholder: '可选，如 code.exe' },
          { key: 'title', label: '窗口标题', type: 'text', placeholder: '可选' },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'window', 'focus']
          pushOption(args, '--app', readText(values, 'app'))
          pushOption(args, '--title', readText(values, 'title'))
          return args
        },
      }),
      runTask({
        id: 'desktop-window-move',
        workspace: 'desktop-control',
        title: '移动窗口',
        description: '将窗口移动到指定坐标。',
        action: 'desktop:window-move',
        feature: 'desktop',
        notices: desktopWindowNotices,
        fields: [
          { key: 'x', label: 'X 坐标', type: 'number', required: true },
          { key: 'y', label: 'Y 坐标', type: 'number', required: true },
          { key: 'app', label: '应用名', type: 'text', placeholder: '可选，如 code.exe' },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'window', 'move', '--x', readText(values, 'x'), '--y', readText(values, 'y')]
          pushOption(args, '--app', readText(values, 'app'))
          return args
        },
      }),
      runTask({
        id: 'desktop-window-resize',
        workspace: 'desktop-control',
        title: '调整大小',
        description: '调整窗口宽高。',
        action: 'desktop:window-resize',
        feature: 'desktop',
        notices: desktopWindowNotices,
        fields: [
          { key: 'width', label: '宽度', type: 'number', required: true },
          { key: 'height', label: '高度', type: 'number', required: true },
          { key: 'app', label: '应用名', type: 'text', placeholder: '可选，如 code.exe' },
        ],
        buildRunArgs: (values) => {
          const args = [
            'desktop',
            'window',
            'resize',
            '--width',
            readText(values, 'width'),
            '--height',
            readText(values, 'height'),
          ]
          pushOption(args, '--app', readText(values, 'app'))
          return args
        },
      }),
      runTask({
        id: 'desktop-window-transparent',
        workspace: 'desktop-control',
        title: '透明度',
        description: '设置窗口透明度。',
        action: 'desktop:window-transparent',
        feature: 'desktop',
        notices: desktopWindowNotices,
        fields: [
          { key: 'alpha', label: '透明度(0-255)', type: 'number', required: true, min: 0, max: 255 },
          { key: 'app', label: '应用名', type: 'text', placeholder: '可选，如 code.exe' },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'window', 'transparent', '--alpha', readText(values, 'alpha')]
          pushOption(args, '--app', readText(values, 'app'))
          return args
        },
      }),
      runTask({
        id: 'desktop-window-top',
        workspace: 'desktop-control',
        title: '置顶切换',
        description: '启用或禁用窗口置顶。',
        action: 'desktop:window-top',
        feature: 'desktop',
        notices: desktopWindowNotices,
        fields: [
          {
            key: 'mode',
            label: '置顶模式',
            type: 'select',
            required: true,
            options: [
              { label: '启用', value: 'enable' },
              { label: '禁用', value: 'disable' },
            ],
          },
          { key: 'app', label: '应用名', type: 'text', placeholder: '可选，如 code.exe' },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'window', 'top']
          if (readText(values, 'mode') === 'enable') args.push('--enable')
          if (readText(values, 'mode') === 'disable') args.push('--disable')
          pushOption(args, '--app', readText(values, 'app'))
          return args
        },
      }),
    ],
  },
  {
    id: 'desktop-theme',
    title: '主题控制',
    description: '切换与定时主题。',
    tasks: [
      runTask({
        id: 'desktop-theme-set',
        workspace: 'desktop-control',
        title: '设置主题',
        description: '切换到指定主题。',
        action: 'desktop:theme-set',
        feature: 'desktop',
        fields: [
          {
            key: 'mode',
            label: '主题模式',
            type: 'select',
            defaultValue: 'light',
            options: [
              { label: 'light', value: 'light' },
              { label: 'dark', value: 'dark' },
            ],
          },
        ],
        target: (values) => readText(values, 'mode'),
        buildRunArgs: (values) => ['desktop', 'theme', 'set', readText(values, 'mode') || 'light'],
      }),
      runTask({
        id: 'desktop-theme-toggle',
        workspace: 'desktop-control',
        title: '切换主题',
        description: '在明暗主题之间切换。',
        action: 'desktop:theme-toggle',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'theme', 'toggle'],
      }),
      runTask({
        id: 'desktop-theme-schedule',
        workspace: 'desktop-control',
        title: '主题定时',
        description: '设置明暗主题的切换时间。',
        action: 'desktop:theme-schedule',
        feature: 'desktop',
        fields: [
          { key: 'light', label: '明亮时间', type: 'text', required: true, placeholder: '08:00' },
          { key: 'dark', label: '暗色时间', type: 'text', required: true, placeholder: '20:00' },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'theme', 'schedule']
          pushOption(args, '--light', readText(values, 'light'))
          pushOption(args, '--dark', readText(values, 'dark'))
          return args
        },
      }),
      runTask({
        id: 'desktop-theme-status',
        workspace: 'desktop-control',
        title: '主题状态',
        description: '查看当前系统主题。',
        action: 'desktop:theme-status',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'theme', 'status'],
      }),
    ],
  },
  {
    id: 'desktop-awake',
    title: 'Awake',
    description: '控制唤醒模式。',
    tasks: [
      runTask({
        id: 'desktop-awake-on',
        workspace: 'desktop-control',
        title: '开启 Awake',
        description: '启动唤醒模式，可设定持续时间。',
        action: 'desktop:awake-on',
        feature: 'desktop',
        fields: [
          { key: 'duration', label: '持续时间', type: 'text', placeholder: '可选，如 45m' },
          { key: 'expire_at', label: '结束时间', type: 'text', placeholder: '可选，如 23:30' },
          { key: 'display_on', label: '保持屏幕常亮', type: 'checkbox', defaultValue: false },
        ],
        buildRunArgs: (values) => {
          const args = ['desktop', 'awake', 'on']
          pushOption(args, '--duration', readText(values, 'duration'))
          pushOption(args, '--expire-at', readText(values, 'expire_at'))
          if (readBool(values, 'display_on')) args.push('--display-on')
          return args
        },
      }),
      runTask({
        id: 'desktop-awake-off',
        workspace: 'desktop-control',
        title: '关闭 Awake',
        description: '关闭唤醒模式。',
        action: 'desktop:awake-off',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'awake', 'off'],
      }),
      runTask({
        id: 'desktop-awake-status',
        workspace: 'desktop-control',
        title: 'Awake 状态',
        description: '查看唤醒模式状态。',
        action: 'desktop:awake-status',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'awake', 'status'],
      }),
    ],
  },
  {
    id: 'desktop-color',
    title: '取色器',
    description: '拾取屏幕颜色。',
    tasks: [
      runTask({
        id: 'desktop-color-pick',
        workspace: 'desktop-control',
        title: '取色',
        description: '打开取色器并输出 HEX。',
        action: 'desktop:color-pick',
        feature: 'desktop',
        notices: desktopColorNotices,
        fields: [{ key: 'copy', label: '复制到剪贴板', type: 'checkbox', defaultValue: false }],
        buildRunArgs: (values) => {
          const args = ['desktop', 'color']
          if (readBool(values, 'copy')) args.push('--copy')
          return args
        },
      }),
    ],
  },
  {
    id: 'desktop-hosts',
    title: 'Hosts 管理',
    description: '管理 hosts 文件条目。',
    tasks: [
      guardedTask({
        id: 'desktop-hosts-add',
        workspace: 'desktop-control',
        title: '新增 Hosts',
        description: '向 hosts 添加记录。',
        action: 'desktop:hosts-add',
        tone: 'danger',
        feature: 'desktop',
        notices: desktopHostsNotices,
        fields: [
          { key: 'host', label: '域名', type: 'text', required: true },
          { key: 'ip', label: 'IP 地址', type: 'text', required: true },
        ],
        target: (values) => readText(values, 'host'),
        buildPreviewArgs: (values) => [
          'desktop',
          'hosts',
          'add',
          readText(values, 'host'),
          readText(values, 'ip'),
          '--dry-run',
        ],
        buildExecuteArgs: (values) => ['desktop', 'hosts', 'add', readText(values, 'host'), readText(values, 'ip')],
        previewSummary: (values) => `新增 hosts ${readText(values, 'host')} -> ${readText(values, 'ip')}`,
      }),
      guardedTask({
        id: 'desktop-hosts-remove',
        workspace: 'desktop-control',
        title: '移除 Hosts',
        description: '从 hosts 移除记录。',
        action: 'desktop:hosts-remove',
        tone: 'danger',
        feature: 'desktop',
        notices: desktopHostsNotices,
        fields: [{ key: 'host', label: '域名', type: 'text', required: true }],
        target: (values) => readText(values, 'host'),
        buildPreviewArgs: (values) => ['desktop', 'hosts', 'remove', readText(values, 'host'), '--dry-run'],
        buildExecuteArgs: (values) => ['desktop', 'hosts', 'remove', readText(values, 'host')],
        previewSummary: (values) => `移除 hosts ${readText(values, 'host')}`,
      }),
      runTask({
        id: 'desktop-hosts-list',
        workspace: 'desktop-control',
        title: 'Hosts 列表',
        description: '列出当前 hosts 记录。',
        action: 'desktop:hosts-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'hosts', 'list'],
      }),
    ],
  },
  {
    id: 'desktop-apps',
    title: '应用列表',
    description: '列出已安装应用。',
    tasks: [
      runTask({
        id: 'desktop-app-list',
        workspace: 'desktop-control',
        title: '已安装应用',
        description: '扫描并列出可用应用路径。',
        action: 'desktop:app-list',
        feature: 'desktop',
        fields: [],
        buildRunArgs: () => ['desktop', 'app', 'list'],
      }),
    ],
  },
  {
    id: 'desktop-run',
    title: '命令执行',
    description: '通过 desktop 运行命令。',
    tasks: [
      runTask({
        id: 'desktop-run-command',
        workspace: 'desktop-control',
        title: '运行命令',
        description: '执行指定命令行。',
        action: 'desktop:run',
        feature: 'desktop',
        fields: [{ key: 'command', label: '命令行', type: 'textarea', required: true }],
        target: (values) => readText(values, 'command'),
        buildRunArgs: (values) => ['desktop', 'run', readText(values, 'command')],
      }),
    ],
  },
]
