# Dashboard Diff File Preview / Viewer 关系补注

这篇文档专门补一个关系说明：`DiffFilePreview.vue`、`DiffViewer.vue`、`DiffFileManager.vue` 在 Diff 子系统里的职责其实是三段式的。

不过为了保持“一组件一文档”的粒度，这里只保留一个很短的关系索引，方便阅读时不串线：

- `DiffFileManager` 负责选文件、搜索文件、驱动预览刷新
- `DiffFilePreview` 负责看单个文件的元信息、内容和校验结果
- `DiffViewer` 负责渲染真正的 diff hunk 正文

如果你已经分别读过：

- `Dashboard-Diff-FileManager.md`
- `Dashboard-Diff-File-Preview.md`
- `Dashboard-Diff-Viewer.md`

那这一层就不需要再单独拆代码了。


