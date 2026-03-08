# Dashboard EnvImportExportPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/EnvImportExportPanel.vue`。

`EnvImportExportPanel` 是 Env 工作台里最偏“批量流转”的组件。它不面向单条变量，而是面向：

- 把当前环境整体导出
- 把一段 env/json/reg/csv 文本整体导入

所以它是批量入口，不是细粒度编辑器。

## 1. 组件定位

这个组件负责两类事情：

- Export：把环境导成文件/文本格式
- Import：把一段内容按指定模式灌回环境系统

它更像 Env 域的“数据交换闸口”。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `scope: EnvScope`
- `loading?: boolean`

这说明它本身没有结果状态，只负责收集输入参数并触发动作。

### 2.2 Emits

它向外抛出：

- `export`
- `export-all`
- `import`

其中：

- `export` 会带 `scope + format`
- `export-all` 会带 `scope`
- `import` 会带 `scope + content + mode + dry_run`

真正的文件生成、导入解析、冲突处理都不在这里。

## 3. 本地状态与核心逻辑

组件内部维护了 5 个状态：

- `format`
- `mode`
- `dryRun`
- `content`
- `dragging`

### 3.1 Export 状态

`format` 支持：

- `json`
- `env`
- `reg`
- `csv`

### 3.2 Import 状态

`mode` 支持：

- `merge`
- `overwrite`

`dryRun` 默认是 `true`，这非常重要，因为它让导入默认走“先预演”的保守路径。

### 3.3 `runImport()`

导入执行前会：

1. 检查 `content` 是否为空
2. 如果当前 `scope === 'all'`，收敛到 `user`
3. emit `import({ scope, content, mode, dry_run })`

这说明批量导入这种写操作同样不接受模糊 scope。

### 3.4 `onDrop(e)`

拖拽逻辑支持两种输入来源：

- 拖一个文件进来，读取第一个文件的文本内容
- 直接拖纯文本进来

这个细节让它既能当粘贴框，也能当轻量文件导入区。

## 4. 模板结构

### 4.1 Export 工具条

第一条工具条包含：

- 格式选择器
- `Export`
- `Export ZIP`

这里的 `Export ZIP` 对应的是 `export-all`。

### 4.2 Import 工具条

第二条工具条包含：

- `merge/overwrite`
- `dry run`
- `Import`

导入模式和 dry run 都在动作前显式暴露出来。

### 4.3 内容输入区

最后是拖拽区和 textarea：

- 支持 paste
- 支持 drag `.env/.json/.reg/.csv`
- 用 `dragging` 控制激活态样式

这让整个导入体验保持在一块非常直接的文本区域里。

## 5. 架构意义

`EnvImportExportPanel` 的设计很实用：

- UI 只负责输入和动作表达
- 不在前端做复杂格式解析
- 不在组件里显示大段导入结果

这让批量流转能力被很好地隔离出来，不会污染主编辑工作流。

## 6. 组件特征总结

一句话概括 `EnvImportExportPanel.vue`：

- **它是 Env 的批量交换闸口，负责格式选择、导入模式选择和内容收集。**

最值得关注的点有四个：

- 导入默认 `dryRun = true`
- 写操作会把 `all` 收敛到 `user`
- 支持粘贴和拖拽文件两种输入方式
- 组件不展示导入结果详情，只负责发起动作

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`，确认它是纯动作面板
2. 再看 `format/mode/dryRun/content/dragging`
3. 接着看 `runImport()` 和 `onDrop()`
4. 最后看模板里的 Export 区、Import 区和 drop zone

读完后回 `EnvPanel` 看 `onExport()`、`onExportBundle()`、`onImport()` 即可。
