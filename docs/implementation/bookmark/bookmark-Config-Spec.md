# xun bookmark 配置规范

> **版本**：1.0 · **更新时间**：2026-03-30  
> 关联文档：bookmark-PRD.md · Bookmarks-Feature-Roadmap.md · Bookmarks-Competitor-Review.md

---

## 1. 设计目标

### 1.0 实现状态同步（2026-03-30）

- 已实现：`bookmark` section 配置读取
- 已实现：默认 `dataFile = ~/.xun.bookmark.json`
- 已实现：默认 `visitLogFile = ~/.xun.bookmark.visits.jsonl`
- 已实现：`_BM_DATA_FILE`、`_BM_VISIT_LOG_FILE`、`_BM_EXCLUDE_DIRS`
- 已实现：`_BM_DEFAULT_SCOPE`、`_BM_DEFAULT_LIST_LIMIT`、`_BM_MAXAGE`
- 已实现：`_BM_RESOLVE_SYMLINKS`、`_BM_ECHO`、`_BM_FZF_OPTS`
- 暂未纳入本轮：Dashboard 面板侧对所有 bookmark 配置项的消费

这份文档定义 `xun bookmark` 的正式配置模型，解决三个问题：

1. `bookmark` 的运行时行为配置放在哪里
2. 配置项有哪些、默认值是什么
3. CLI flag / 环境变量 / 配置文件 之间谁覆盖谁

本设计遵循两个原则：

- **复用现有全局配置体系**
- **不为 bookmark 单独再造第二套配置文件格式**

也就是说：

> `bookmark` 的持久化配置统一放入 `~/.xun.config.json` 的 `bookmark` 顶层 section 中。  
> `bookmark` 的数据文件仍独立存储，不混入配置文件。

---

## 2. 文件与职责边界

### 2.1 配置文件

| 类型 | 默认路径 | 说明 |
|---|---|---|
| 全局配置文件 | `%USERPROFILE%\\.xun.config.json` | 由 `XUN_CONFIG` 覆盖；`bookmark` 配置写入其中的 `bookmark` section |
| bookmark 主存储 | 由 `bookmark.dataFile` 决定 | 书签主数据库，不属于配置文件 |
| bookmark 访问日志 | 由 `bookmark.visitLogFile` 决定 | 高频访问日志/WAL，不属于配置文件 |

### 2.2 为什么不单独使用 `bookmark.json`

原因：

- 项目当前已有统一全局配置入口：`xun config get/set/edit`
- 现有配置系统就是围绕 `.xun.config.json` 设计的
- 再引入第二个配置文件只会增加 discoverability 和优先级复杂度

因此 `bookmark` 采用：

```jsonc
{
  "bookmark": {
    ...
  }
}
```

### 2.3 配置 vs 数据

必须明确区分：

- **配置**：决定行为策略
- **数据**：书签条目、访问记录、frecency、source、pin 等运行状态

下面这些属于**配置**：

- 默认 scope
- exclusion 列表
- auto-learn 开关
- data file 路径
- fzf 选项
- 是否解析 symlink

下面这些属于**数据**：

- 书签列表
- `visit_count`
- `last_visited`
- `frecency_score`
- `source`
- `pinned`

---

## 3. 优先级规则

### 3.1 总优先级

统一采用：

```text
CLI flags
  > 环境变量
  > 配置文件 bookmark section
  > 内建默认值
```

### 3.2 解释

1. **CLI flags**
   当前命令的一次性显式意图，优先级最高。

2. **环境变量**
   shell/session 级控制，适合临时覆盖或 init 模板注入。

3. **配置文件**
   用户长期偏好。

4. **默认值**
   程序内置保底。

### 3.3 示例

配置文件：

```jsonc
{
  "bookmark": {
    "defaultScope": "child"
  }
}
```

环境变量：

```powershell
$env:_BM_DEFAULT_SCOPE = "global"
```

命令行：

```powershell
bm z foo --base D:/work
```

最终生效顺序：

- `--base D:/work` 覆盖 `_BM_DEFAULT_SCOPE`
- `_BM_DEFAULT_SCOPE` 覆盖配置文件 `defaultScope=child`

---

## 4. 顶层配置结构

### 4.1 全局配置 JSON 结构

```jsonc
{
  "tree": { ... },
  "proxy": { ... },
  "redirect": { ... },
  "bookmark": {
    "version": 1,
    "dataFile": "C:/Users/dev/.xun.bookmark.json",
    "visitLogFile": "C:/Users/dev/.xun.bookmark.visits.jsonl",
    "defaultScope": "auto",
    "defaultListLimit": 20,
    "maxAge": 10000,
    "resolveSymlinks": false,
    "echo": false,
    "excludeDirs": [
      "node_modules",
      "dist",
      "build",
      "target",
      ".git",
      "tmp",
      "temp"
    ],
    "autoLearn": {
      "enabled": true,
      "importHistoryOnFirstInit": true
    },
    "fzf": {
      "minVersion": "0.51.0",
      "opts": ""
    }
  }
}
```

### 4.2 `bookmark.version`

这是**配置 section 的版本号**，不是书签数据文件的 `schema_version`。

区分如下：

- `bookmark.version`
  - 用于配置结构演进
- `bookmark data schema_version`
  - 用于书签主存储结构演进

两者不要混用。

---

## 5. 配置项清单

### 5.1 存储路径

| Key | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `bookmark.dataFile` | `string` | `%USERPROFILE%/.xun.bookmark.json` | bookmark 主存储文件 |
| `bookmark.visitLogFile` | `string` | `%USERPROFILE%/.xun.bookmark.visits.jsonl` | bookmark 访问日志 |

设计说明：

- 不再默认复用历史 `%USERPROFILE%/.xun.json`
- 允许 breaking change，bookmark vNext 使用独立数据文件名
- `visitLogFile` 默认应与 `dataFile` 放在同一目录

### 5.2 查询默认值

| Key | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `bookmark.defaultScope` | `string` | `"auto"` | `auto/global/child` |
| `bookmark.defaultListLimit` | `number` | `20` | `--list` 默认返回上限 |

约束：

- `defaultScope` 不允许是 `base` 或 `workspace`
  因为它们需要额外参数

### 5.3 数据治理

| Key | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `bookmark.maxAge` | `number` | `10000` | learned/imported 老化阈值 |
| `bookmark.resolveSymlinks` | `boolean` | `false` | 入库前是否解析 symlink/reparse point |
| `bookmark.excludeDirs` | `string[]` | 内建默认列表 | 自动学习排除目录 |

说明：

- `excludeDirs` 只影响自动学习与历史导入，不影响显式 `bm set`
- `resolveSymlinks=true` 会增加 IO 与路径解析成本，默认关闭

### 5.4 自动学习

| Key | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `bookmark.autoLearn.enabled` | `boolean` | `true` | 是否启用自动学习 |
| `bookmark.autoLearn.importHistoryOnFirstInit` | `boolean` | `true` | 首次 init 是否建议导入 shell history |

不放进配置的内容：

- hook 具体实现方式
- shell wrapper 细节

这些属于 `bm init` 生成逻辑，不属于静态配置。

### 5.5 交互体验

| Key | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `bookmark.echo` | `boolean` | `false` | 跳转前打印最终路径 |
| `bookmark.fzf.minVersion` | `string` | `"0.51.0"` | 文档/诊断用最低版本 |
| `bookmark.fzf.opts` | `string` | `""` | 追加到 fzf 调用的默认选项 |

---

## 6. 环境变量映射

环境变量仍保留，但只作为**配置覆盖层**。

| 环境变量 | 映射配置 | 说明 |
|---|---|---|
| `_BM_DATA_FILE` | `bookmark.dataFile` | 覆盖主存储文件路径 |
| `_BM_VISIT_LOG_FILE` | `bookmark.visitLogFile` | 覆盖访问日志路径 |
| `_BM_DEFAULT_SCOPE` | `bookmark.defaultScope` | 覆盖默认 scope |
| `_BM_DEFAULT_LIST_LIMIT` | `bookmark.defaultListLimit` | 覆盖默认 list 上限 |
| `_BM_MAXAGE` | `bookmark.maxAge` | 覆盖老化阈值 |
| `_BM_EXCLUDE_DIRS` | `bookmark.excludeDirs` | 覆盖排除目录列表 |
| `_BM_RESOLVE_SYMLINKS` | `bookmark.resolveSymlinks` | 覆盖 symlink 解析开关 |
| `_BM_ECHO` | `bookmark.echo` | 覆盖 echo 行为 |
| `_BM_FZF_OPTS` | `bookmark.fzf.opts` | 追加/覆盖 fzf 选项 |

### 6.1 路径变量命名修正

为保持语义清晰，建议正式采用：

- `_BM_DATA_FILE`
- `_BM_VISIT_LOG_FILE`

而不是之前文档中的 `_BM_DATA_DIR`。

原因：

- 这里控制的是**文件路径**
- 不是“目录路径”

如果后续确实需要“只指定目录、文件名由程序推导”，再单独增加：

- `_BM_DATA_DIR`

但本版不建议同时支持两套含义相近的变量。

### 6.2 列表型变量解析

`_BM_EXCLUDE_DIRS` 解析规则：

- Windows：用 `;` 分隔
- POSIX：用 `:`

示例：

```powershell
$env:_BM_EXCLUDE_DIRS = "node_modules;dist;build;target;.git"
```

```bash
export _BM_EXCLUDE_DIRS="node_modules:dist:build:target:.git"
```

---

## 7. 配置命令设计

既然复用现有全局配置体系，就不再单独设计 `bm config`。

统一使用：

```bash
xun config get bookmark.defaultScope
xun config set bookmark.defaultScope '"child"'
xun config set bookmark.autoLearn.enabled true
xun config set bookmark.excludeDirs '["node_modules","dist","build"]'
xun config edit
```

### 7.1 为什么不新增 `bm config`

原因：

- 当前已有 `xun config`
- 它支持 dot-path 读写
- 再造 `bm config` 只会形成重复入口

### 7.2 `bm init` 与配置的关系

`bm init` 读取：

- `bookmark` section
- 对应环境变量

然后生成 shell wrapper。

也就是说：

- 配置决定 init 模板默认行为
- init 本身不持久化配置

---

## 8. 默认配置样例

### 8.1 最小样例

```jsonc
{
  "bookmark": {
    "version": 1
  }
}
```

### 8.2 推荐样例

```jsonc
{
  "bookmark": {
    "version": 1,
    "dataFile": "C:/Users/dev/.xun.bookmark.json",
    "visitLogFile": "C:/Users/dev/.xun.bookmark.visits.jsonl",
    "defaultScope": "auto",
    "defaultListLimit": 20,
    "maxAge": 10000,
    "resolveSymlinks": false,
    "echo": false,
    "excludeDirs": [
      "node_modules",
      "dist",
      "build",
      "target",
      ".git",
      "tmp",
      "temp"
    ],
    "autoLearn": {
      "enabled": true,
      "importHistoryOnFirstInit": true
    },
    "fzf": {
      "minVersion": "0.51.0",
      "opts": "--height 40% --reverse"
    }
  }
}
```

---

## 9. 验收标准

### 9.1 功能验收

| 场景 | 要求 |
|---|---|
| 配置读取 | 未设置环境变量时，能从 `bookmark` section 读取默认值 |
| 优先级覆盖 | CLI flags > env vars > config file > defaults |
| 路径覆盖 | `dataFile` / `visitLogFile` 可被环境变量覆盖 |
| exclusion 生效 | `excludeDirs` 对自动学习和历史导入生效 |
| init 读取配置 | `bm init` 生成的脚本能反映配置默认值 |
| 无重复入口 | bookmark 配置统一通过 `xun config` 读写 |

### 9.2 设计验收

| 项目 | 要求 |
|---|---|
| 配置文件数量 | 不新增第二个 bookmark 专用配置文件 |
| 配置/数据边界 | 配置不存放书签条目数据 |
| 配置版本 | `bookmark.version` 与数据 `schema_version` 明确分离 |
| 命名一致性 | 统一使用 `_BM_*` 环境变量前缀 |

---

## 10. 最终结论

`xun bookmark` 的配置系统应当：

- **复用现有 `.xun.config.json`**
- **使用 `bookmark` 顶层 section**
- **继续支持 `_BM_*` 环境变量作为覆盖层**
- **不再单独设计 `bm config`**
- **将书签条目数据与运行配置严格分离**

一句话：

> **bookmark 配置属于 `xun` 全局配置的一部分，bookmark 数据则属于独立状态存储，两者不要混在一起。**
