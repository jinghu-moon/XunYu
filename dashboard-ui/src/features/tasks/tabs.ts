import type { WorkspaceTabDefinition } from './catalog'

export const workspaceTabs: WorkspaceTabDefinition[] = [
  { value: 'overview', label: '总览', description: '统一总览与能力入口' },
  { value: 'paths-context', label: '路径与上下文', description: '书签、上下文、路径检索' },
  { value: 'network-proxy', label: '网络与代理', description: '端口、进程、代理执行' },
  { value: 'environment-config', label: '环境与配置', description: '全局配置与环境治理' },
  { value: 'files-security', label: '文件与安全', description: '文件、备份、保护与 ACL' },
  { value: 'integration-automation', label: '集成与自动化', description: 'shell 集成、别名、批量改名' },
  { value: 'media-conversion', label: '媒体与转换', description: '图像与视频任务' },
  { value: 'desktop-control', label: '桌面控制', description: '窗口、热键、布局与工作区' },
  { value: 'statistics-diagnostics', label: '统计与诊断', description: '审计、体检、统计' },
]
