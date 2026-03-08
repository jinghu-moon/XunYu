# Dashboard DiffConvertPanel 导读

这篇文档专门拆 `dashboard-ui/src/components/diff/DiffConvertPanel.vue`。

`DiffConvertPanel` 是 Diff 工作台里的一个旁路工具：它不负责 diff 本身，而是负责**配置文件跨格式转换**。但它又不是纯工具脚本，而是把“原文预览、转换预览、写回目标文件”三步放在一个小面板里。

## 1. 组件定位

它主要解决三件事：

- 读取当前配置文件原文
- 预览转换后的文本
- 把转换结果写回目标格式文件

所以它不是编辑器，也不是校验器，而是一个 **转换预览器 + 写回器**。

## 2. 输入边界

组件只接收一个可选输入：

- `path?`

也就是说，它既可以：

- 被外部路径驱动自动打开并预览
- 也可以在面板内部手动输入路径再执行

这让它既适合由 `DiffFileManager` 驱动，也适合独立临时使用。

## 3. 状态模型

## 3.1 输入与结果状态

- `filePath`
- `toFormat`
- `fromFormat`
- `convertedText`
- `originalLines`
- `writtenPath`

这组状态覆盖了转换工作流的核心信息：

- 输入文件是谁
- 目标格式是什么
- 源格式被识别为什么
- 转换结果是什么
- 写回后生成了什么路径

## 3.2 过程状态

- `loading`
- `writing`
- `originalLoading`
- `error`

它把：

- 原文加载
- 预览转换
- 写回转换

明确分成了不同过程状态，而不是全部共用一个 `busy`。

## 3.3 派生状态

- `canRun`
- `resultLines`

`canRun` 只是最基本的路径非空判断；`resultLines` 则用于统计当前转换结果行数。

## 4. 核心工作流

## 4.1 原文加载：`loadOriginal(path)`

这一步会：

1. 调 `fetchFileContent({ path, offset: 0, limit: 300 })`
2. 如果是二进制文件，显示 `[binary file]`
3. 否则把返回的 `lines` 填到 `originalLines`

这说明它的原文预览是“有限行数预览”，不是完整大文件编辑器。

## 4.2 预览转换：`previewConvert()`

流程是：

1. 若路径为空则直接返回
2. 清空错误与 `writtenPath`
3. 先调用 `loadOriginal(...)`
4. 再调用 `fetchConvertFile({ preview: true })`
5. 写入：
   - `fromFormat`
   - `convertedText`

这个顺序很有意思：它不是只展示转换结果，还会先把原文预览同步加载出来，确保左右两栏都更新。

## 4.3 写回转换：`writeConvert()`

流程是：

1. 调 `fetchConvertFile({ preview: false })`
2. 把结果写回：
   - `fromFormat`
   - `convertedText`
   - `writtenPath`

这里说明“写回”并不是前端自己保存文本，而是后端实际执行转换并返回写入路径。

## 5. 外部驱动能力

组件会 `watch(props.path, ...)`：

- 当外部传入新路径时
- 自动同步 `filePath`
- 并立即执行 `previewConvert()`

这意味着它非常适合和 `DiffFileManager` 的 `openConvert(path)` 联动：

- 侧栏选中文件
- 右键或按钮打开 Convert
- Convert 面板自动载入并展示预览

## 6. 模板结构

## 6.1 顶部头部

头部只有：

- 标题 `Convert`
- 提示语 `Cross-format config conversion`

说明它被定义成一块清晰的工具面板，而不是 Diff 主流程的一部分。

## 6.2 工具条

工具条包括：

- 路径输入框
- 目标格式选择
- `Preview`
- `Write`

这里的交互语义非常明确：

- `Preview`：只看结果
- `Write`：真正落盘

## 6.3 状态提示区

工具条下面会显示：

- `error`
- `writtenPath`
- 元信息：`From / To / Lines`

这让用户能很快判断：

- 转换是否成功
- 识别到的源格式是什么
- 最终写到了哪里

## 6.4 双栏预览区

面板主体是两栏：

- `Original`
- `Converted`

并且两栏都使用非常轻量的 `pre` 展示，而不是更复杂的代码编辑器。

这符合它的定位：**转换预览，不是全文编辑。**

## 7. 组件特征总结

`DiffConvertPanel.vue` 的核心特点有 4 个：

1. **预览与写回分离**：先看，再决定是否写。
2. **原文与结果并排**：帮助快速比较转换效果。
3. **支持外部路径驱动**：可无缝接入 `DiffFileManager`。
4. **不做编辑器野心**：只做有限预览和转换，不承担全文编辑责任。

## 8. 推荐阅读顺序

建议这样读：

1. 先看状态定义和 `watch(props.path, ...)`
2. 再看 `loadOriginal()`、`previewConvert()`、`writeConvert()` 三条流程
3. 最后看双栏模板结构

读完后你会更容易理解：`DiffConvertPanel` 是 Diff 工作台里的实用旁路工具，而不是主流程的一部分。
