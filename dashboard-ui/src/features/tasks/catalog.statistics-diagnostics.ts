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
