import type { TaskFieldOption, TaskNotice } from './catalog.types'

export const JSON_FORMAT = 'json'

export const desktopWindowNotices: TaskNotice[] = [
  { text: '需要目标窗口存在且可见。', tone: 'info' },
]

export const desktopHostsNotices: TaskNotice[] = [
  { text: '修改 hosts 需要管理员权限。', tone: 'warning' },
]

export const desktopColorNotices: TaskNotice[] = [
  { text: '将弹出系统取色器窗口，需要手动选取颜色。', tone: 'info' },
]

export const shellInitOptions: TaskFieldOption[] = [
  { label: 'PowerShell', value: 'powershell' },
  { label: 'Bash', value: 'bash' },
  { label: 'Zsh', value: 'zsh' },
]

export const shellCompletionOptions: TaskFieldOption[] = [
  ...shellInitOptions,
  { label: 'Fish', value: 'fish' },
]

export const aliasTypeOptions: TaskFieldOption[] = [
  { label: '全部', value: '' },
  { label: 'cmd', value: 'cmd' },
  { label: 'app', value: 'app' },
]

export const aliasModeOptions: TaskFieldOption[] = [
  { label: 'auto', value: 'auto' },
  { label: 'exe', value: 'exe' },
  { label: 'cmd', value: 'cmd' },
]

export const aliasShellOptions: TaskFieldOption[] = [
  { label: 'cmd', value: 'cmd' },
  { label: 'ps', value: 'ps' },
  { label: 'bash', value: 'bash' },
  { label: 'nu', value: 'nu' },
]

export const dedupModeOptions: TaskFieldOption[] = [
  { label: '按路径', value: 'path' },
  { label: '按名称', value: 'name' },
]

export const brnCaseOptions: TaskFieldOption[] = [
  { label: '不转换', value: '' },
  { label: 'kebab', value: 'kebab' },
  { label: 'snake', value: 'snake' },
  { label: 'pascal', value: 'pascal' },
  { label: 'upper', value: 'upper' },
  { label: 'lower', value: 'lower' },
]

export const brnExtCaseOptions: TaskFieldOption[] = [
  { label: '不转换', value: '' },
  { label: 'upper', value: 'upper' },
  { label: 'lower', value: 'lower' },
]

export const brnSortByOptions: TaskFieldOption[] = [
  { label: 'name（默认）', value: '' },
  { label: 'mtime（修改时间）', value: 'mtime' },
  { label: 'ctime（创建时间）', value: 'ctime' },
]

export const brnBracketOptions: TaskFieldOption[] = [
  { label: 'all（全部）', value: 'all' },
  { label: 'round ()', value: 'round' },
  { label: 'square []', value: 'square' },
  { label: 'curly {}', value: 'curly' },
]

export const imgFormatOptions: TaskFieldOption[] = [
  { label: 'webp', value: 'webp' },
  { label: 'jpeg', value: 'jpeg' },
  { label: 'png', value: 'png' },
  { label: 'avif', value: 'avif' },
  { label: 'svg', value: 'svg' },
]

export const imgSvgMethodOptions: TaskFieldOption[] = [
  { label: 'bezier', value: 'bezier' },
  { label: 'visioncortex', value: 'visioncortex' },
  { label: 'potrace', value: 'potrace' },
  { label: 'skeleton', value: 'skeleton' },
  { label: 'diffvg', value: 'diffvg' },
]

export const imgJpegBackendOptions: TaskFieldOption[] = [
  { label: 'auto', value: 'auto' },
  { label: 'moz', value: 'moz' },
  { label: 'turbo', value: 'turbo' },
]

export const aliasAppScanSourceOptions: TaskFieldOption[] = [
  { label: 'all', value: 'all' },
  { label: 'reg', value: 'reg' },
  { label: 'startmenu', value: 'startmenu' },
  { label: 'path', value: 'path' },
]

export const videoModeOptions: TaskFieldOption[] = [
  { label: 'balanced', value: 'balanced' },
  { label: 'fastest', value: 'fastest' },
  { label: 'smallest', value: 'smallest' },
]

export const videoEngineOptions: TaskFieldOption[] = [
  { label: 'auto', value: 'auto' },
  { label: 'cpu', value: 'cpu' },
  { label: 'gpu', value: 'gpu' },
]

export const aclRightsOptions: TaskFieldOption[] = [
  { label: 'Read', value: 'Read' },
  { label: 'Write', value: 'Write' },
  { label: 'Modify', value: 'Modify' },
  { label: 'ReadAndExecute', value: 'ReadAndExecute' },
  { label: 'FullControl', value: 'FullControl' },
]

export const aclAceTypeOptions: TaskFieldOption[] = [
  { label: 'Allow', value: 'Allow' },
  { label: 'Deny', value: 'Deny' },
]

export const aclInheritOptions: TaskFieldOption[] = [
  { label: 'BothInherit', value: 'BothInherit' },
  { label: 'ContainerOnly', value: 'ContainerOnly' },
  { label: 'ObjectOnly', value: 'ObjectOnly' },
  { label: 'None', value: 'None' },
]

export const aclInheritModeOptions: TaskFieldOption[] = [
  { label: '启用继承', value: 'enable' },
  { label: '禁用继承', value: 'disable' },
]
