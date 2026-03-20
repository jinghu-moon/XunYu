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
    description: 'ACL 运维、修复与加解密统一纳入任务流。',
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
      runTask({
        id: 'acl-diff',
        workspace: 'files-security',
        title: 'ACL 差异对比',
        description: '比较目标路径与参考路径的 ACL 差异统计。',
        action: 'acl:diff',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'reference', label: '参考路径', type: 'text', required: true },
          { key: 'output', label: '导出 CSV', type: 'text', placeholder: '可选' },
        ],
        target: (values) => pathTarget(values),
        buildRunArgs: (values) => {
          const args = ['acl', 'diff', '-p', readText(values, 'path'), '-r', readText(values, 'reference')]
          pushOption(args, '-o', readText(values, 'output'))
          return args
        },
      }),
      runTask({
        id: 'acl-effective',
        workspace: 'files-security',
        title: '有效权限',
        description: '查看指定用户在目标路径上的有效权限。',
        action: 'acl:effective',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'user', label: '用户', type: 'text', placeholder: '留空则使用当前用户' },
        ],
        target: (values) => pathTarget(values),
        buildRunArgs: (values) => {
          const args = ['acl', 'effective', '-p', readText(values, 'path')]
          pushOption(args, '-u', readText(values, 'user'))
          return args
        },
      }),
      runTask({
        id: 'acl-backup',
        workspace: 'files-security',
        title: '备份 ACL',
        description: '将当前 ACL 导出为 JSON 备份文件。',
        action: 'acl:backup',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'output', label: '输出文件', type: 'text', placeholder: '可选' },
        ],
        target: (values) => pathTarget(values),
        buildRunArgs: (values) => {
          const args = ['acl', 'backup', '-p', readText(values, 'path')]
          pushOption(args, '-o', readText(values, 'output'))
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
        id: 'acl-copy',
        workspace: 'files-security',
        title: '复制 ACL',
        description: '先比较差异，再用参考路径 ACL 覆盖目标。',
        action: 'acl:copy',
        tone: 'danger',
        fields: [
          { key: 'path', label: '目标路径', type: 'text', required: true },
          { key: 'reference', label: '参考路径', type: 'text', required: true },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['acl', 'diff', '-p', readText(values, 'path'), '-r', readText(values, 'reference')],
        buildExecuteArgs: (values) => ['acl', 'copy', '-p', readText(values, 'path'), '-r', readText(values, 'reference'), '-y'],
        previewSummary: (values) => `用 ${readText(values, 'reference')} 覆盖 ${pathTarget(values)} 的 ACL`,
      }),
      guardedTask({
        id: 'acl-restore',
        workspace: 'files-security',
        title: '恢复 ACL',
        description: '先验证备份文件，再恢复目标 ACL。',
        action: 'acl:restore',
        tone: 'danger',
        fields: [
          { key: 'path', label: '目标路径', type: 'text', required: true },
          { key: 'from', label: '备份文件', type: 'text', required: true },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => previewPath(values, 'from'),
        buildExecuteArgs: (values) => ['acl', 'restore', '-p', readText(values, 'path'), '--from', readText(values, 'from'), '-y'],
        previewSummary: (values) => `从 ${readText(values, 'from')} 恢复 ${pathTarget(values)} 的 ACL`,
      }),
      guardedTask({
        id: 'acl-purge',
        workspace: 'files-security',
        title: '清理 ACL 主体',
        description: '先查看显式 ACE，再按主体清理全部显式规则。',
        action: 'acl:purge',
        tone: 'danger',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'principal', label: '主体', type: 'text', required: true, placeholder: 'BUILTIN\\Users' },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['acl', 'view', '-p', readText(values, 'path'), '--detail'],
        buildExecuteArgs: (values) => ['acl', 'purge', '-p', readText(values, 'path'), '--principal', readText(values, 'principal'), '-y'],
        previewSummary: (values) => `清理 ${pathTarget(values)} 上 ${readText(values, 'principal')} 的显式 ACL`,
      }),
      guardedTask({
        id: 'acl-inherit',
        workspace: 'files-security',
        title: '切换 ACL 继承',
        description: '先查看当前继承状态，再启用或禁用继承。',
        action: 'acl:inherit',
        tone: 'danger',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'mode', label: '目标状态', type: 'select', defaultValue: 'enable', options: aclInheritModeOptions },
          { key: 'preserve', label: '禁用时保留继承 ACE', type: 'checkbox', defaultValue: true },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['acl', 'view', '-p', readText(values, 'path')],
        buildExecuteArgs: (values) => {
          const args = ['acl', 'inherit', '-p', readText(values, 'path')]
          if ((readText(values, 'mode') || 'enable') === 'enable') {
            args.push('--enable')
          } else {
            args.push('--disable', '--preserve', readBool(values, 'preserve') ? 'true' : 'false')
          }
          return args
        },
        previewSummary: (values) => `将 ${pathTarget(values)} 的 ACL 继承切换为 ${(readText(values, 'mode') || 'enable') === 'enable' ? '启用' : '禁用'}`,
      }),
      guardedTask({
        id: 'acl-owner',
        workspace: 'files-security',
        title: '修改 ACL Owner',
        description: '先查看当前 Owner，再修改为指定主体。',
        action: 'acl:owner',
        tone: 'danger',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'set', label: '新 Owner', type: 'text', required: true, placeholder: 'BUILTIN\\Administrators' },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['acl', 'view', '-p', readText(values, 'path')],
        buildExecuteArgs: (values) => ['acl', 'owner', '-p', readText(values, 'path'), '--set', readText(values, 'set'), '-y'],
        previewSummary: (values) => `将 ${pathTarget(values)} 的 Owner 修改为 ${readText(values, 'set')}`,
      }),
      guardedTask({
        id: 'acl-repair',
        workspace: 'files-security',
        title: 'ACL 强制修复',
        description: '先查看现状，再执行 take ownership + grant FullControl。',
        action: 'acl:repair',
        tone: 'danger',
        fields: [
          { key: 'path', label: '路径', type: 'text', required: true },
          { key: 'exportErrors', label: '导出失败明细', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => pathTarget(values),
        buildPreviewArgs: (values) => ['acl', 'view', '-p', readText(values, 'path'), '--detail'],
        buildExecuteArgs: (values) => {
          const args = ['acl', 'repair', '-p', readText(values, 'path')]
          if (readBool(values, 'exportErrors')) args.push('--export-errors')
          args.push('-y')
          return args
        },
        previewSummary: (values) => `强制修复 ${pathTarget(values)} 的 ACL`,
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
