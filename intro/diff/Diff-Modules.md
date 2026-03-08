# Diff 模块导读

本文档专门解释 `xun diff` 及其 Dashboard Diff 子系统。这个模块的特点是：CLI 定义和执行逻辑放在同一个文件里，而底层算法和类型则拆在 `src/diff/*` 中，另外 Dashboard 侧又加了一层文件管理与预览壳层。

## 1. `diff` 在项目里的位置

从结构上看，`diff` 由四层组成：

```text
xun diff ...
  -> src/commands/diff.rs        # CLI 定义 + 执行入口
  -> src/diff/*                  # 底层 diff 引擎
  -> dashboard handlers          # /api/files /api/diff /api/convert /api/validate /ws
  -> dashboard-ui DiffPanel      # 文件管理与可视化渲染
```

和大多数命令不同，`diff` 的 CLI 结构没有放在 `src/cli/`，而是直接定义在 `src/commands/diff.rs` 里。这是它一个比较特别的点。

## 2. CLI 入口：`src/commands/diff.rs`

`DiffCmd` 当前参数包括：

- `old`
- `new`
- `--mode`
- `--diff-algorithm`
- `--format`
- `--context`
- `--max-size`
- `--ignore-space-change`
- `--ignore-all-space`
- `--ignore-blank-lines`
- `--strip-trailing-cr`
- `--text`

只看命令面就能看出这个模块同时考虑了：

- 模式选择：`auto / line / ast`
- 算法选择：`histogram / myers / minimal / patience`
- 输出形式：`text / json`
- whitespace 归一化
- 大文件限制
- 二进制按文本强制处理

这说明 `diff` 并不是“简单打印几行差异”，而是一个有明确策略面的对比引擎。

## 3. CLI 执行层：`cmd_diff(args)`

`cmd_diff` 的角色主要是：

1. 解析和校验参数
2. 读取文件内容
3. 把参数映射为 `src/diff/*` 可消费的 options
4. 调用底层引擎拿到 `DiffResult`
5. 输出 text 或 json

因此 `src/commands/diff.rs` 更像是“CLI adapter”，而不是 diff 算法本体。

## 4. 底层引擎：`src/diff/*`

`src/diff/mod.rs` 是统一调度层，下面拆成：

- `types.rs`
- `line.rs`
- `ast.rs`
- `lang.rs`
- `vue.rs`

它的设计目标很清楚：CLI 和 Dashboard 共享同一套核心 diff 逻辑。

## 5. 先看核心类型：`src/diff/types.rs`

当前最关键的类型有：

- `DiffResult`
- `DiffStats`
- `StatsUnit`
- `DiffResultKind`
- `Hunk`
- `DiffLine`
- `LineTag`
- `HunkKind`
- `AstDiffError`

这套类型体系说明：

- 输出不是自由文本，而是结构化结果
- 统计信息和渲染信息被单独建模
- line diff、AST diff、binary diff、identical 都被统一在一个结果模型里

这也是 Dashboard 能直接消费 diff 结果的基础。

## 6. 三条核心 diff 路径

### 6.1 行级 diff：`line.rs`

这是最基础的路径，适用于：

- 显式 `--mode line`
- AST 不支持时的回退
- Vue SFC 中某些 section 的局部 diff

它是最稳妥、适用范围最广的路径。

### 6.2 AST diff：`ast.rs`

AST diff 用于更“语义化”的代码比较，但并不是无限制启用。根据错误类型可见，它会在这些情况下回退：

- parse 失败
- symbol 太多
- 行数太多
- 语言不支持

这说明 AST diff 的设计目标不是“强行 AST”，而是“能用则用，不能用就优雅降级”。

### 6.3 Vue SFC diff：`vue.rs`

这是一个很有项目特色的分支。

它会先把 `.vue` 文件拆成 section：

- `template`
- `script`
- `style`

然后按 tag + lang 配对，对每个 section 再走 AST 或 line diff，最后再把行号偏移回原始 SFC 文件。

这说明：

- Vue 文件不是被当成纯文本硬比
- 也不是简单整文件 AST diff
- 而是“先按 SFC 结构拆段，再分段比较”

对于这个仓库的 Dashboard 前端来说，这条分支非常实用。

## 7. 模式调度：`src/diff/mod.rs`

`mod.rs` 里可以读出几个重要策略：

- 先做二进制检测
- 先做 whitespace 预处理
- `auto` 模式下按文件类型与语言能力决定走 line / ast / vue
- 遇到 AST 不可用时自动回退到 line diff

这说明它更偏“可靠的统一入口”，而不是把判断逻辑扔给 CLI 调用方。

## 8. Dashboard 对应层

`diff` 在 Dashboard 中并不只是一个结果面板，而是一个完整文件管理工作台。

### 8.1 后端 API

当前主要接口包括：

- `/api/files`
- `/api/files/search`
- `/api/info`
- `/api/content`
- `/api/diff`
- `/api/convert`
- `/api/validate`
- `/ws`

这说明 Dashboard Diff 不只是展示对比结果，还包含：

- 文件树浏览
- 深度搜索
- 文件信息与内容分块读取
- 配置格式转换
- 配置校验
- WebSocket 文件变更通知

### 8.2 前端组件树

对应组件包括：

- `DiffPanel.vue`
- `diff/DiffOptions.vue`
- `diff/DiffFileManager.vue`
- `diff/DiffFilePreview.vue`
- `diff/DiffConvertPanel.vue`
- `diff/DiffViewer.vue`
- `diff/CodeDiffPanel.vue`
- `diff/LineDiffPanel.vue`
- `diff/ConfigDiffTree.vue`
- `diff/DiffStats.vue`
- `diff/FileBrowser.vue`

其中：

- `DiffPanel` 是编排层
- `DiffFileManager` 是文件工作台
- `DiffViewer` 是底层渲染器
- `ConfigDiffTree` 是配置语义树渲染器

## 9. 一个值得注意的设计点

`DiffCmd` 定义放在 `src/commands/diff.rs`，而不是 `src/cli/diff.rs`。

这意味着：

- `diff` 可能是后加进来的模块，尚未完全对齐其他命令族的目录约定
- 或者作者有意把“CLI adapter + 文本输出”放在同一处，以减少上下跳转

无论原因是什么，这个模块的阅读路径和其他命令略有不同，需要特别记住。

## 10. 推荐阅读顺序

### 10.1 想先看 CLI

1. `src/commands/diff.rs`
2. `src/diff/types.rs`
3. `src/diff/mod.rs`

### 10.2 想先看算法调度

1. `src/diff/mod.rs`
2. `src/diff/lang.rs`
3. `src/diff/line.rs`
4. `src/diff/ast.rs`
5. `src/diff/vue.rs`

### 10.3 想先看 Dashboard 工作台

1. `src/commands/dashboard/mod.rs`
2. `dashboard-ui/src/components/DiffPanel.vue`
3. `dashboard-ui/src/components/diff/DiffFileManager.vue`
4. `dashboard-ui/src/components/diff/DiffFilePreview.vue`
5. `dashboard-ui/src/components/diff/DiffViewer.vue`

## 11. 当前实现上的几个观察

- `diff` 的底层引擎和 Dashboard 工作台是明显分层的，核心算法没有混进 UI。
- Vue SFC 专项 diff 是这个项目非常有价值的一条定制路径。
- `DiffResult` 这种结构化输出让 CLI 和 Web 共用引擎成为可能。
- Dashboard Diff 的能力边界远大于 CLI `xun diff`，它已经是一个“小型文件分析工作台”。
- `FileBrowser.vue` 当前像备用组件，而 `DiffFileManager.vue` 才是主路径上的文件交互中心。
