# Dashboard UI 重构方案

> 版本: 1.0
> 日期: 2026-05-12
> 关联: [CLI 重构方案](./CLI-Refactor-Plan.md) · [CLI 现状](./CLI-Current-State.md)
> 目标: 好看、好用、与后端 Operation Runtime 完全打通

---

## 一、现状分析

### 1.1 技术栈

| 项目 | 当前版本 | 说明 |
|------|---------|------|
| 构建 | Vite 7.3 | 快，无问题 |
| 框架 | Vue 3.5 (Composition API) | 正确选择 |
| UI 库 | PrimeVue 4.5 + @primeuix/themes | 组件丰富 |
| 图标 | @tabler/icons-vue 3.37 | 轻量 |
| 虚拟滚动 | @tanstack/vue-virtual 3.13 | 大列表性能 |
| 测试 | Vitest 3.2 + @vue/test-utils | 覆盖良好 |
| 状态管理 | **无**（组件内 ref） | ⚠️ 需引入 |
| 路由 | **无**（CapsuleTabs 切换） | 当前够用 |
| 类型 | 手写 types.ts (~60 interface) | ⚠️ 与 Rust 不同步 |
| i18n | **无**（中文硬编码） | 暂不需要 |

### 1.2 页面架构

```
┌─────────────────────────────────────────────────────────────┐
│ Header                                                       │
│ ┌──────────┐ ┌─────────────────────────────────┐ ┌───────┐ │
│ │ Title    │ │ CapsuleTabs (9 工作台)           │ │ Theme │ │
│ └──────────┘ └─────────────────────────────────┘ └───────┘ │
├─────────────────────────────────────────────────────────────┤
│ Main (动态组件)                                              │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ <component :is="activeComponent" />                     │ │
│ │                                                         │ │
│ │ 每个 Workspace 是独立 .vue 文件                          │ │
│ │ 内部再嵌套各功能面板                                     │ │
│ └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ CommandPalette (Ctrl+K)                                      │
│ GlobalFeedback (Toast)                                       │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 9 个工作台

| 工作台 | 核心面板 | 数据来源 |
|--------|---------|---------|
| 总览 | OverviewWorkspace | `/api/workspace/overview` |
| 路径与上下文 | BookmarksPanel | `/api/bookmarks` |
| 网络与代理 | ProxyPanel, PortsPanel | `/api/proxy/*`, `/api/ports` |
| 环境与配置 | EnvPanel (10+ 子面板) | `/api/env/*` (WebSocket) |
| 文件与安全 | FileGovernance, ACL, Vault | `/api/workspace/task/*` |
| 集成与自动化 | Recipe, ShellGuide | `/api/recipe/*` |
| 媒体与转换 | DiffPanel, FileManager | `/api/diff/*` |
| 桌面控制 | DesktopControl | `/api/workspace/task/*` |
| 统计与诊断 | DiagnosticsCenter, Audit | `/api/diagnostics/*`, `/api/audit/*` |

### 1.4 后端通信

| 方式 | 用途 | 端点 |
|------|------|------|
| HTTP REST | CRUD 操作 | `GET/POST/PUT/DELETE /api/*` |
| WebSocket | Env 实时变更 | `ws://localhost:9527/api/env/ws` |
| WebSocket | Diff 实时 | `ws://localhost:9527/api/diff/ws` |

**当前 API 层问题：**
- `api.ts` 28KB 单文件，所有端点平铺
- 无请求取消（AbortController）
- 无缓存/去重（同一数据多次请求）
- 无乐观更新
- 错误处理统一但粗糙（全局 toast）

### 1.5 组件规模

| 类别 | 数量 | 最大文件 |
|------|------|---------|
| Workspace 组件 | 9 | OverviewWorkspace (4KB) |
| 功能面板 | 35+ | BatchGovernancePanel (23KB) |
| 测试文件 | 30+ | BatchGovernancePanel.test (30KB) |
| 业务逻辑 (.ts) | 15+ | catalog.desktop-control (30KB) |
| UI 基础组件 | 5 | CapsuleTabs (10KB) |

---

## 二、核心问题

### 2.1 与后端的断裂

| 问题 | 影响 | 严重度 |
|------|------|--------|
| types.ts 手写，与 Rust struct 不同步 | 运行时类型错误 | **高** |
| 无统一 Operation 协议 | 每个面板自己实现确认/预览 | **高** |
| 无 StructuredValue 消费 | 列表面板手写列定义 | 中 |
| WebSocket 只用于 env/diff | 其他操作无实时反馈 | 中 |
| api.ts 单文件 28KB | 维护困难 | 中 |

### 2.2 UI/UX 问题

| 问题 | 影响 |
|------|------|
| 9 个 tab 水平排列，小屏溢出 | 移动端不可用 |
| 无面板间导航面包屑 | 深层操作迷路 |
| 确认对话框风格不统一 | 有 UnifiedConfirmDialog 但未全面使用 |
| 大面板（23KB .vue）职责过重 | 难以维护和测试 |
| 无骨架屏/加载状态统一 | 有 SkeletonTable 但覆盖不全 |
| 无键盘导航（除 Ctrl+K） | 可访问性不足 |

### 2.3 状态管理缺失

当前每个面板自己 `ref()` 管理数据：
- 切换 tab 后数据丢失（重新请求）
- 面板间无法共享数据（如 bookmark 列表）
- 无法实现跨面板联动（如从诊断跳转到具体问题）

---

## 三、目标架构

### 3.1 与后端 Operation Runtime 对齐

```
┌─────────────────────────────────────────────────────────────┐
│                    Dashboard UI                               │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ View Layer (Vue Components)                           │   │
│  │ - Workspace 容器                                      │   │
│  │ - 功能面板（消费 StructuredValue）                     │   │
│  │ - 通用组件（Table/Confirm/Preview）                   │   │
│  └────────────────────────┬─────────────────────────────┘   │
│                           │                                  │
│  ┌────────────────────────▼─────────────────────────────┐   │
│  │ Store Layer (Pinia)                                   │   │
│  │ - useOperationStore (preview/confirm/execute 状态机)  │   │
│  │ - useDataStore (StructuredValue 缓存)                 │   │
│  │ - useWsStore (WebSocket 连接管理)                     │   │
│  └────────────────────────┬─────────────────────────────┘   │
│                           │                                  │
│  ┌────────────────────────▼─────────────────────────────┐   │
│  │ API Layer                                             │   │
│  │ - 自动生成类型 (from Rust specta/ts-rs)               │   │
│  │ - 统一 WebSocket 命令协议                             │   │
│  │ - 请求去重 + 缓存 + AbortController                  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                    WebSocket / HTTP
                           │
┌──────────────────────────▼──────────────────────────────────┐
│ Rust Backend (Operation Runtime)                             │
│ - CommandSpec → StructuredValue/Table                        │
│ - Operation → Preview → Execute → OperationResult           │
│ - DashboardRenderer → WebSocket push                        │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 目录结构（重构后）

```
dashboard-ui/src/
├── api/                        # API 层（拆分）
│   ├── client.ts              # fetch/ws 基础封装
│   ├── commands.ts            # 统一命令调用（走 WebSocket 协议）
│   ├── operations.ts          # Operation 协议（preview/confirm/execute）
│   └── legacy.ts             # 旧 REST API（渐进迁移）
│
├── generated/                  # 自动生成（from Rust）
│   └── types.ts              # specta/ts-rs 生成的类型
│
├── stores/                     # Pinia 状态管理
│   ├── operation.ts           # Operation 状态机
│   ├── data.ts               # StructuredValue 缓存
│   ├── ws.ts                 # WebSocket 连接
│   └── workspace.ts          # 当前工作台状态
│
├── composables/                # 可复用逻辑
│   ├── useTable.ts           # 通用表格（消费 Table schema）
│   ├── useOperation.ts       # Operation UI 流程
│   ├── useCommand.ts         # 命令执行 + 结果展示
│   └── useSearch.ts          # 通用搜索/过滤
│
├── components/                 # 组件（精简）
│   ├── layout/               # 布局组件
│   │   ├── AppShell.vue      # 顶层壳
│   │   ├── WorkspaceNav.vue  # 工作台导航
│   │   └── PanelLayout.vue   # 面板布局容器
│   ├── shared/               # 通用业务组件
│   │   ├── DataTable.vue     # 通用表格（自动列、排序、过滤）
│   │   ├── OperationDialog.vue  # 统一 Operation 预览+确认
│   │   ├── CommandResult.vue # 命令结果展示
│   │   └── EmptyState.vue    # 空状态
│   └── workspaces/           # 各工作台（保持）
│
├── features/                   # 业务逻辑（保持）
├── styles/                     # 样式（保持）
└── main.ts
```

---

## 四、依赖更新

### 4.1 新增依赖

```json
{
  "dependencies": {
    "pinia": "^2.3.0"
  }
}
```

**不新增的：**
- 不加 vue-router（tab 切换足够，URL 状态用 `history.replaceState`）
- 不加 axios（fetch + 自定义封装更轻）
- 不加 TanStack Query（Pinia store + composable 足够）
- 不加 i18n（当前中文单语言，未来需要时再加）

### 4.2 当前依赖保持

| 依赖 | 保留理由 |
|------|---------|
| PrimeVue 4 | 组件丰富，已深度使用 |
| @tabler/icons-vue | 轻量，风格统一 |
| @tanstack/vue-virtual | 大列表必需 |

### 4.3 构建工具保持

Vite 7 + vue-tsc + Vitest — 无需变更。

---

## 五、后端链接重构

### 5.1 当前 → 目标

| 维度 | 当前 | 目标 |
|------|------|------|
| 协议 | HTTP REST 为主 + 少量 WS | **WebSocket 命令协议为主** + HTTP 兜底 |
| 类型 | 手写 types.ts | **Rust 自动生成** (specta) |
| 数据格式 | 各 API 自定义 JSON | **统一 StructuredValue/Table** |
| 操作流程 | 前端自己实现确认 | **后端 Operation.preview → 前端确认 → 后端 execute** |
| 实时性 | 仅 env/diff 有 WS | **所有操作结果实时推送** |

### 5.2 WebSocket 命令协议

```typescript
// 发送
interface WsCommand {
  id: string           // 请求 ID（用于匹配响应）
  command: string      // e.g. "bookmark.list", "backup.create"
  args: Record<string, unknown>
}

// 接收 — 查询结果
interface WsQueryResponse {
  id: string
  type: 'result'
  data: Table | Value  // StructuredValue
}

// 接收 — Operation 预览
interface WsPreviewResponse {
  id: string
  type: 'preview'
  preview: Preview     // { summary, changes, risk_level, reversible }
}

// 发送 — 确认执行
interface WsConfirm {
  id: string
  type: 'confirm'
  token: string
}

// 接收 — 操作结果
interface WsOperationResult {
  id: string
  type: 'operation_result'
  result: OperationResult
}

// 接收 — 实时事件
interface WsEvent {
  type: 'event'
  event: string        // e.g. "env.changed", "bookmark.updated"
  data: Value
}
```

### 5.3 类型自动生成

```
Rust 侧:
  #[derive(Serialize, specta::Type)]
  pub struct Table { ... }
  pub struct Preview { ... }
  pub struct OperationResult { ... }

构建时:
  cargo run --bin generate-types > dashboard-ui/src/generated/types.ts

前端侧:
  import type { Table, Preview, OperationResult } from './generated/types'
```

**消除 types.ts 手写维护。**

---

## 六、页面布局重构

### 6.1 响应式布局

```
┌─────────────────────────────────────────────────────────────┐
│ ┌─ Nav ──────────────────────────────────────────────────┐  │
│ │ Logo   [工作台 tabs - 可滚动]          [⌘K] [◐] [☰]  │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌─ Workspace ────────────────────────────────────────────┐  │
│ │                                                        │  │
│ │  ┌─ Panel Grid ─────────────────────────────────────┐  │  │
│ │  │                                                  │  │  │
│ │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐        │  │  │
│ │  │  │ Panel 1 │  │ Panel 2 │  │ Panel 3 │        │  │  │
│ │  │  └─────────┘  └─────────┘  └─────────┘        │  │  │
│ │  │                                                  │  │  │
│ │  └──────────────────────────────────────────────────┘  │  │
│ │                                                        │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌─ Status Bar ───────────────────────────────────────────┐  │
│ │ 连接状态 ● | 最近操作: backup create (2s ago) | v0.1.0 │  │
│ └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 关键 UI 改进

| 改进 | 方案 |
|------|------|
| Tab 溢出 | 可滚动 + 下拉菜单（>5 个时折叠） |
| 面板布局 | CSS Grid auto-fit，面板可折叠/展开 |
| 操作反馈 | 底部 Status Bar 显示最近操作 + WS 连接状态 |
| 深层导航 | 面包屑 + 面板标题可点击返回 |
| 键盘 | Tab/Shift+Tab 面板间切换，Enter 执行 |
| 加载状态 | 统一骨架屏（基于 PrimeVue Skeleton） |
| 空状态 | 统一 EmptyState 组件（图标 + 文案 + 操作按钮） |

### 6.3 通用 DataTable 组件

**核心创新：消费后端 `Table` schema 自动渲染。**

```vue
<!-- components/shared/DataTable.vue -->
<script setup lang="ts">
import type { Table } from '@/generated/types'

const props = defineProps<{
  table: Table
  searchable?: boolean
  sortable?: boolean
  selectable?: boolean
}>()

// 列从 table.columns 自动生成
// 排序、过滤、搜索内置
// 虚拟滚动（@tanstack/vue-virtual）自动启用（>100 行）
</script>
```

**受益：**
- BookmarksPanel 不再手写列定义
- PortsPanel 不再手写列定义
- EnvVarsTable 不再手写列定义
- 新增任何列表面板 = 0 前端列定义代码

### 6.4 统一 OperationDialog

```vue
<!-- components/shared/OperationDialog.vue -->
<script setup lang="ts">
import type { Preview } from '@/generated/types'

const props = defineProps<{
  preview: Preview | null
  loading?: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

// 根据 risk_level 自动调整：
// Low → 绿色确认按钮
// Medium → 黄色警告
// High → 红色 + 二次确认输入
// Critical → 红色 + 倒计时 + 输入确认文本
</script>
```

---

## 七、实施计划

### Phase 1: 基础设施 (2 天)

- [ ] 安装 Pinia，创建 stores/ 目录
- [ ] 创建 `stores/ws.ts` — WebSocket 连接管理
- [ ] 创建 `stores/operation.ts` — Operation 状态机
- [ ] 创建 `api/client.ts` — 统一 fetch/ws 封装
- [ ] 创建 `composables/useTable.ts` — 通用表格逻辑
- [ ] 创建 `components/shared/DataTable.vue` — 通用表格组件
- [ ] 创建 `components/shared/OperationDialog.vue` — 统一操作确认

### Phase 2: 后端对接 (与 CLI 重构 Phase 1 同步)

- [ ] Rust 侧集成 specta，生成 TypeScript 类型
- [ ] 创建 `generated/types.ts`（替代手写 types.ts）
- [ ] 实现 WebSocket 命令协议（后端 DashboardRenderer）
- [ ] 创建 `api/commands.ts` — 统一命令调用
- [ ] 创建 `api/operations.ts` — Operation 协议封装

### Phase 3: 面板迁移 (1 周)

按优先级迁移面板到新架构：

| 优先级 | 面板 | 改动 |
|--------|------|------|
| 1 | BookmarksPanel | 使用 DataTable + useCommand |
| 2 | PortsPanel | 使用 DataTable |
| 3 | ProxyPanel | 使用 useOperation |
| 4 | EnvPanel | 使用 DataTable + WS store |
| 5 | FileGovernance | 使用 OperationDialog |
| 6 | 其他面板 | 逐步迁移 |

### Phase 4: 布局优化 (2 天)

- [ ] 重构 Header → WorkspaceNav（可滚动 tabs）
- [ ] 添加 Status Bar（WS 状态 + 最近操作）
- [ ] 面板 Grid 布局（可折叠）
- [ ] 统一空状态 + 骨架屏
- [ ] 键盘导航增强

---

## 八、设计规范

### 8.1 视觉原则

| 原则 | 实现 |
|------|------|
| 信息密度高 | 紧凑模式默认，DensityToggle 已有 |
| 操作可发现 | 悬停显示操作按钮，非永远可见 |
| 状态可感知 | 颜色编码（绿=正常，黄=警告，红=错误） |
| 反馈即时 | 操作后 <200ms 内有视觉反馈 |
| 一致性 | 所有列表用 DataTable，所有危险操作用 OperationDialog |

### 8.2 交互模式

| 模式 | 适用场景 | 组件 |
|------|---------|------|
| 查看 → 操作 | 列表类（bookmark/ports/env） | DataTable + 行操作 |
| 预览 → 确认 → 执行 | 危险操作（backup/acl/delete） | OperationDialog |
| 实时监控 | env 变更、端口变化 | WS 推送 + 自动刷新 |
| 搜索 → 选择 | 命令面板、书签跳转 | CommandPalette |

### 8.3 颜色语义

```css
/* 操作风险等级 */
--risk-low: var(--green-500);
--risk-medium: var(--yellow-500);
--risk-high: var(--orange-500);
--risk-critical: var(--red-500);

/* 操作状态 */
--status-success: var(--green-500);
--status-running: var(--blue-500);
--status-warning: var(--yellow-500);
--status-error: var(--red-500);
```

---

## 九、验收标准

| 维度 | 标准 |
|------|------|
| 类型安全 | types.ts 100% 自动生成，手写为 0 |
| 操作统一 | 所有危险操作走 OperationDialog |
| 列表统一 | 所有列表走 DataTable（消费 Table schema） |
| 实时性 | 所有写操作通过 WS 推送结果 |
| 性能 | 首屏 <1s，面板切换 <100ms |
| 可访问性 | 键盘可达所有操作，ARIA 标签完整 |
| 测试 | 核心 composable 100% 覆盖 |
