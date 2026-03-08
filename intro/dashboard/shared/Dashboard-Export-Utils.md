# Dashboard export.ts 导读

这篇文档专门拆 `dashboard-ui/src/ui/export.ts`。

它负责 Dashboard 中最常见的前端导出动作：把当前结果集转换成文本内容，并触发浏览器下载。

## 1. 它解决什么问题

多个面板都有“导出 CSV / JSON”的需求。如果每个面板都自己：

- 拼 CSV
- 造 Blob
- 建下载链接
- 命名文件

就会形成大量重复代码。

`export.ts` 的职责就是把这些重复动作收口成一套最小公共工具。

## 2. 这套工具的分层

### 2.1 `toCsv(headers, rows)`

这是纯转换函数：

- 输入表头和二维数据行
- 对每个值做 CSV 转义
- 输出完整 CSV 文本

它不涉及 DOM，也不关心下载。

### 2.2 `downloadTextFile(filename, content, mime)`

这是浏览器下载底座：

- 用 `Blob` 包装文本
- 用 `URL.createObjectURL()` 生成临时地址
- 创建 `<a>` 标签触发下载
- 下载后移除节点并释放 URL

也就是说，它是所有文本导出的真正执行器。

### 2.3 `downloadCsv(prefix, headers, rows)`

这是 CSV 的业务包装层：

- 调 `toCsv()` 生成内容
- 生成带时间戳的文件名
- 调 `downloadTextFile()` 完成下载

### 2.4 `downloadJson(prefix, data)`

这是 JSON 的业务包装层：

- 把对象格式化成带缩进 JSON
- 生成带时间戳的文件名
- 调 `downloadTextFile()` 下载

## 3. 为什么要有时间戳文件名

`timestampSlug()` 的作用不是装饰，而是解决两个很实际的问题：

- 避免重复下载时覆盖同名文件
- 让导出文件天然带时间上下文，方便回溯

所以这个工具默认更偏“运维台账”而不是“最终报表导出”。

## 4. 谁在用它

当前明确使用这套工具的组件包括：

- `dashboard-ui/src/components/BookmarksPanel.vue`
- `dashboard-ui/src/components/AuditPanel.vue`
- `dashboard-ui/src/components/PortsPanel.vue`

这几个面板共同特点是：

- 有当前筛选结果集
- 需要前端即时导出
- 不需要后端再单独提供导出文件接口

这说明 `export.ts` 主要服务于“当前前端视图结果导出”。

## 5. 它刻意不做什么

这份工具保持得很克制，没有做这些事：

- 不做复杂表格 schema 映射
- 不做流式导出
- 不处理二进制文件
- 不感知业务领域对象

这符合 KISS / YAGNI：它只负责**文本内容导出**这一层公共能力。

## 6. 一句话概括

`export.ts` 是 Dashboard 的轻量文本导出工具层：上层面板只关心“导出什么”，它负责“怎样转成文件并下载”。

## 7. 建议连读

1. `./Dashboard-Tag-Utils.md`
2. `./Dashboard-Feedback-Store.md`
3. `dashboard-ui/src/components/BookmarksPanel.vue`
4. `dashboard-ui/src/components/AuditPanel.vue`
5. `dashboard-ui/src/components/PortsPanel.vue`
