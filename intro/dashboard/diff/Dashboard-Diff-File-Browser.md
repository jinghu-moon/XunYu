# Dashboard Diff FileBrowser 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/FileBrowser.vue`。

`FileBrowser` 是 Diff 子系统里的独立文件选择浮层。当前主链并没有直接挂载它，但从实现上看，它已经是一个完整可用的目录浏览器。

## 1. 组件定位

这个组件负责：

- 以模态浮层浏览目录
- 进入子目录
- 返回父目录
- 选中文件后把完整路径回传给父层

所以它是 Diff 侧的文件选择器，而不是文件内容预览器。

## 2. 输入输出边界

### 2.1 Props

它接收：

- `initialPath?: string`

默认起点会落到：

- `C:\`

### 2.2 Emits

它向外抛出：

- `select(path)`
- `close()`

所以父层可以把它作为一个典型的“选择后回填路径”的辅助浮层。

## 3. 本地状态与逻辑

### 3.1 本地状态

- `currentPath`
- `entries`
- `loading`
- `error`

### 3.2 `loadDir(path)`

这是主入口：

- 调 `fetchFiles(path)`
- 成功后更新 `entries` 和 `currentPath`
- 失败后更新错误信息并清空列表

### 3.3 `goUp()`

返回上级目录时会：

- 自动判断路径分隔符 `/` 或 `\`
- 处理 Windows 盘符根路径边界

这说明它对 Windows 路径语义有显式照顾。

### 3.4 `onClick(entry)`

- 如果是目录，继续下钻
- 如果是文件，拼出完整路径并 emit `select`

### 3.5 `onBackdropClick()`

只有点在真正的背景遮罩上，才会关闭模态。

## 4. 模板结构

模板是标准模态框结构：

- 外层遮罩 `fb-backdrop`
- 主体 `fb-modal`
- 头部：上级目录按钮、当前路径、关闭按钮
- 内容区：loading / error / empty / list 四态

列表里的每一项会显示：

- 简单图标
- 名称
- 文件大小（仅文件）

## 5. 架构意义

虽然当前它还没有挂进主 Diff 工作流，但这个组件本身边界非常完整：

- 目录加载独立封装
- 选择结果通过事件回传
- 模态行为自洽

因此它更像一个已准备好的通用辅助组件。

## 6. 组件特征总结

一句话概括 `FileBrowser.vue`：

- **它是一个可独立挂载的目录浏览模态，用于选择文件并回传路径。**

## 7. 推荐阅读顺序

建议这样读：

1. 先看 `props + emits`
2. 再看 `loadDir()`、`goUp()`、`onClick()`
3. 最后看模态结构与四态内容区
