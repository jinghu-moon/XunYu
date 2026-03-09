# XunYu P3-P5 剩余任务清单

## 1. 结论

按 `docs/project/Dashboard-Expansion-Roadmap.md` 的硬性交付口径，`P3 / P4 / P5` 的主干能力已经基本落地，当前剩余事项主要分为两类：

- 必做收尾：修正文案、统一命名、补齐少量闭环
- 非阻塞增强：把已落地能力做得更完整、更顺手，而不是再开新一级页面

因此，这一阶段的重点不应再是“补大块功能”，而应转向“收尾治理 + 体验补强 + 口径统一”。

---

## 2. Phase 盘点

| Phase | 路线图目标 | 当前状态 | 剩余判断 |
| --- | --- | --- | --- |
| P3 路径与上下文 | `ctx / ws / recent / stats / dedup / check / gc / keys / all / fuzzy` | 当前任务组已覆盖 `ctx / recent / stats / dedup / check / gc / keys / all / fuzzy / ws`，并已接入最近任务与 Recipe | 主干已完成，剩余以文案与命名口径统一为主 |
| P4 集成与自动化 | `init / completion / complete / alias / brn` | 当前任务组已覆盖 `init / completion / __complete / alias(ls/find/which/sync) / brn` | 主干已完成，剩余以 alias 全链路补齐与安装闭环为主 |
| P5 媒体与转换 + 统计与诊断 | `img / video / cstat / 全局审计时间线 / 任务日志 / 失败日志 / 健康中心` | 当前已接入 `img / video / cstat`，统计与诊断工作台已承载最近任务、审计时间线、失败任务、危险回执、诊断中心 | 主干已完成，剩余以 Recipe 修正与高级参数补齐为主 |

---

## 3. P3：路径与上下文剩余项

### 3.1 已落地能力

- `dashboard-ui/src/workspace-tools.ts` 已存在 `pathsContextTaskGroups`
- 当前已覆盖：
  - `ctx:list / show / use / off / set / del / rename`
  - `recent`
  - `stats`
  - `check`
  - `gc` 预览与清理
  - `dedup`
  - `keys / all / fuzzy`
  - `ws`
- 本地最近任务、Recipe、跨工作台诊断跳转均已接入

### 3.2 必做收尾

1. 修复路径与上下文内置 Recipe 文案乱码
   - 目标文件：`src/commands/dashboard/handlers/recipes.rs`
   - 重点条目：`paths-context-health`
   - 影响：Recipe 面板标题、描述、步骤摘要显示异常，直接影响可用性

2. 统一 `ws` 命名口径
   - 旧版路线图和部分说明中写的是 `workspace`
   - CLI 实际子命令定义为 `ws`
   - 建议统一规则：
     - 对外说明统一写“工作区批量打开（`ws`）”
     - Dashboard 文案中统一使用“工作区批量打开（`ws`）”

### 3.3 非阻塞增强

- 为 `keys / all / fuzzy / ws` 增加更明确的结果说明与跳转引导
- 为 P3 工作台新增更多“路径体检 / 路径整理”型 Recipe
- 检查路径工作台与 Recipe 文案是否仍存在编码污染或术语不统一问题

---

## 4. P4：集成与自动化剩余项

### 4.1 已落地能力

- `dashboard-ui/src/workspace-tools.ts` 已存在 `integrationAutomationTaskGroups`
- 当前已覆盖：
  - `init`
  - `completion`
  - `__complete`
  - `alias:ls / find / which / sync`
  - `brn`
- `brn` 已纳入 guarded 模式，符合 `Preview -> Confirm -> Receipt` 路径
- 本地最近任务与 Recipe 已完成收口

### 4.2 必做收尾

1. 修复集成与自动化内置 Recipe 文案乱码
   - 目标文件：`src/commands/dashboard/handlers/recipes.rs`
   - 重点条目：`integration-shell-bootstrap`

2. 补齐“安装闭环”而不只是“输出脚本”
   - 当前 Dashboard 更偏向生成 `init` / `completion` 内容
   - 若要更贴合路线图“新用户可完成 shell 集成、补全安装”的口径，建议至少补其一：
     - 明确的安装指引面板
     - shell profile 写入向导
     - 复制命令与验证步骤的一键化流程

### 4.3 非阻塞增强

- `alias` 命令族继续纳入更多操作：
  - `setup`
  - `add`
  - `rm`
  - `export`
  - `import`
  - `app add / rm / ls / scan / which / sync`
- 为 `alias` 补更多治理型 Recipe，例如：
  - 初始化 alias 运行时
  - 导入并同步 alias
  - 扫描应用并生成 app alias
- 为 `brn` 结果页补更清晰的影响摘要与失败项分类

---

## 5. P5：媒体与转换 + 统计与诊断剩余项

### 5.1 已落地能力

- `dashboard-ui/src/workspace-tools.ts` 已存在：
  - `mediaConversionTaskGroups`
  - `statisticsDiagnosticsTaskGroups`
- 当前已覆盖：
  - `img`
  - `video probe`
  - `video compress`
  - `video remux`
  - `cstat`
- `StatisticsDiagnosticsWorkspace` 已承载：
  - `DiagnosticsCenterPanel`
  - `RecentTasksPanel`
  - `RecipePanel`
  - `AuditPanel`
- `DiagnosticsCenterPanel` 已聚合：
  - Doctor 概览
  - 治理预警
  - 失败任务
  - 危险回执
  - 审计时间线

### 5.2 必做收尾

1. 修复媒体工作台内置 Recipe 文案乱码
   - 目标文件：`src/commands/dashboard/handlers/recipes.rs`
   - 重点条目：`media-video-probe-compress`

2. 顺带排查相邻 Recipe 是否有同类问题
   - 例如：`proxy-diagnostics`
   - 目标不是只修一条，而是把同文件内受编码污染的内置文案一次性清干净

### 5.3 非阻塞增强

1. 补齐 `img` 高级参数的可视化
   - 当前 Dashboard 主要暴露基础输入、输出、格式、质量、尺寸等参数
   - CLI 侧仍有更多高级参数未进入 UI，例如：
     - `svg_method`
     - `svg_diffvg_iters`
     - `svg_diffvg_strokes`
     - `jpeg_backend`
     - `png_lossy`
     - `png_dither_level`
     - `webp_lossy`
     - `threads`
     - `avif_threads`

2. 继续扩展媒体与诊断 Recipe
   - `video remux` 场景化 Recipe
   - 图像目录批处理 Recipe
   - `cstat` + 审计复盘型 Recipe

3. 进一步细化统计与诊断分区语义
   - 现在已经有“失败任务 / 危险回执 / 审计时间线 / Doctor / 治理预警”
   - 后续可继续增强：
     - 更清晰的过滤器预设
     - 更明确的任务分类标签
     - 更强的跨工作台回链与复盘路径

---

## 6. 建议执行顺序

建议按“底层问题先清理，再补体验”的顺序推进：

1. 第一优先级：修复 `recipes.rs` 中 P3-P5 相关内置 Recipe 乱码
2. 第二优先级：统一 `ws` 命名口径
3. 第三优先级：补 P4 的 shell 安装闭环
4. 第四优先级：补 `alias` 全链路可视化
5. 第五优先级：补 `img` 高级参数与更多 Recipe

---

## 7. 判断标准

当以下条件同时成立时，可以认为 `P3-P5` 进入“完成且收尾干净”的状态：

- P3-P5 对应工作台不存在明显乱码或错误文案
- 路线图、Dashboard 文案、CLI 子命令名称口径一致
- P4 至少具备“生成脚本 + 安装引导”闭环
- P5 的媒体与统计诊断能力已不再只停留在基础接入，而是具备足够的高频流程模板
- 新增增强项继续遵守“工作台优先”，不回退到“每个命令一个页面”的模式

---

## 8. 对后续开发的建议

后续推进时，建议继续坚持以下原则：

- 工作台优先，不新增命令墙式导航
- 危险动作继续保持 `Preview -> Confirm -> Receipt + Audit`
- Recent Tasks 负责本地闭环，Statistics & Diagnostics 负责跨域复盘
- Recipe 负责固化高频工作流，不引入第二套复杂编排系统
