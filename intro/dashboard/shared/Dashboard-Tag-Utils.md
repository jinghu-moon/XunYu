# Dashboard tags.ts 导读

这篇文档专门拆 `dashboard-ui/src/ui/tags.ts`。

它负责把书签 / 首页统计等场景中的 tag 文本映射成相对稳定的视觉类别 class。

## 1. 它解决什么问题

如果标签颜色完全写死在业务组件里，会出现两个问题：

- 相同 tag 在不同面板上颜色不一致
- 新 tag 到来时没有稳定降级策略

`tags.ts` 的目标就是：**用一套可重复的规则，把任意 tag 映射到有限类别。**

## 2. 核心设计

### 2.1 固定类别集合

文件先定义了 6 个类别：

- `lang`
- `tool`
- `env`
- `work`
- `path`
- `general`

这说明它不是做“无限色板”，而是做“有限语义桶”。

### 2.2 关键词优先

`KEYWORDS` 为每个类别配置一组关键字，比如：

- 语言类：`rust`、`python`、`ts`
- 工具类：`git`、`docker`、`cargo`
- 环境类：`dev`、`prod`、`test`

所以当 tag 包含明显语义线索时，它会优先落到对应类别。

### 2.3 前缀直达

如果 tag 本身就是 `lang:rust`、`tool:git` 这种格式，`resolveTagCategory()` 会先取前缀直接命中类别。

这给调用方留出了“显式控制颜色类别”的入口。

### 2.4 哈希兜底

对于没有明显语义的 tag：

- 先走 `hashString()`
- 再按类别数量取模
- 得到稳定类别

这很关键：它不是随机色，而是**同一 tag 每次都能得到相同类别**。

## 3. 对外暴露什么

### 3.1 `resolveTagCategory(tag)`

返回归类后的类别名。

### 3.2 `tagCategoryClass(tag)`

把类别直接包装成样式 class：

- `tag-pill--lang`
- `tag-pill--tool`
- `tag-pill--env`
- ...

这让模板层可以直接消费，不必自己拼规则。

## 4. 谁在用它

当前明确使用它的组件包括：

- `dashboard-ui/src/components/BookmarksPanel.vue`
- `dashboard-ui/src/components/HomePanel.vue`

它主要服务两个场景：

- 书签列表中的 tag pill 渲染
- 首页聚合统计中的热门 tag 展示

## 5. 为什么这个工具有价值

它把“标签文本 -> 视觉类别”这件事从业务模板里抽走后，带来三个收益：

- 样式口径统一
- 新标签自动获得可接受的视觉结果
- 业务组件不需要重复写 if / else 分类逻辑

## 6. 一句话概括

`tags.ts` 是 Dashboard 的标签语义映射器：优先按前缀和关键字分类，实在无法识别时再用哈希兜底，保证标签颜色既有语义又稳定。

## 7. 建议连读

1. `./Dashboard-Export-Utils.md`
2. `dashboard-ui/src/components/BookmarksPanel.vue`
3. `dashboard-ui/src/components/HomePanel.vue`
