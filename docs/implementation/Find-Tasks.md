# xun find — 实现计划（tasks）

> 依据：[Find-Design.md](./Find-Design.md)  
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 补充（2026-02）：Dashboard Web UI 迭代不影响 find 任务清单。

---

## Phase 0：CLI 入口与路由

- [x] `src/cli.rs`：新增 `FindCmd` 与参数定义（argh，**仅单字符短选项**）
- [x] `src/cli.rs`：参数对齐 v0.3.4（`--regex-include/--regex-exclude/--fuzzy-size/--mtime/--ctime/--atime/--attribute/--case/--dry-run/--test-path`）
- [x] `src/cli.rs`：`[path...]` 为空时默认 `.`（命令执行层处理）
- [x] `src/commands/mod.rs`：注册 `find` 路由
- [x] `src/commands/find.rs`：新增命令实现入口（骨架）
- [x] `src/find/mod.rs`：新增 find 模块根入口（与 `src/commands/find.rs` 对接）

---

## Phase 1：规则解析与匹配

- [x] Glob 解析与匹配（`* ? [] \\ **`，大小写可选）
- [x] Regex 规则（全匹配；**实现时自动包裹 `^(?:...)$`**；含 `/` → full path，否则 filename）
- [x] `!` 强制 include 语义（用于反向包含）
- [x] 目录规则（末尾 `/`）
- [x] Exact vs Fuzzy 分类（无 `* ? [ ]` 为 Exact；含 `/` 仍可为 Exact）
- [x] Exact 规则索引：`exact_by_path` + `exact_by_name`（`--case` 关闭时 key/查找统一转小写）
- [x] 默认状态规则（有 include → 默认 exclude，否则 include）
- [x] 规则优先级：Exact > Fuzzy，同类后规则覆盖前规则
- [x] `**` 仅在锚定路径中跨层匹配（非锚定等价于 `*`）
- [x] 规则去重：初版不做（保留行为说明），后续可扩展

---

## Phase 2：二级过滤器

- [x] Size 过滤（`> < =`、范围、开闭区间）
- [x] Fuzzy Size 过滤（单位桶范围）
- [x] Time 过滤（`--mtime/--ctime/--atime`，前缀 + 范围；明确解析优先级）
- [x] Depth 过滤（比较与范围）
- [x] 属性过滤（`+h -r -s -l`；冲突校验 `+x` 与 `-x`）
- [x] 空文件/空目录过滤（仅显式指定时执行额外 I/O）

---

## Phase 3：遍历引擎与性能

- [x] 目录遍历（多目录输入，默认当前目录）
- [x] 仅 `.xunignore` 生效（不含内置忽略）
- [x] 线程池 + 批量子目录分发
- [x] 扩展名索引加速（fuzzy glob `*.ext`）
- [x] 输出缓冲写（减少系统调用；stdout 使用 BufWriter）
- [x] 输出路径规范：机器输出使用 `/` 分隔，表格可显示原生分隔符
- [x] 规则匹配路径使用 base_dir 下相对路径（不含 `\\?\` 前缀）

可选优化（后续）：
- [ ] MFT 快速路径（NTFS + 管理员）

---

## Phase 4：输出与交互

- [x] `--format auto|table|tsv|json`
- [x] `--count` 仅输出数量
- [x] `--dry-run` + `--test-path <path>` 规则测试（不扫描；末尾 `/` 视为目录）
- [x] `--verbose` 时显示规则命中信息
- [x] `rule_idx` 与 `Rule #N` 统一为 1-based 编号
- [x] `tsv/json` 输出包含 `size/mtime` 字段（需强制读取元数据）
- [x] stdout 结果数据走 `out_println!` 或 BufWriter；stderr 走 `ui_println!`

---

## Phase 5：测试

- [x] 单元测试：size/time/depth/attr 解析
- [x] 单元测试：glob/regex 匹配与规则优先级
- [x] 单元测试：regex 全匹配（自动包裹 ^(?:...)$）
- [x] 单元测试：Exact 规则 case-insensitive key 处理
- [x] 单元测试：`**` 非锚定等价 `*`
- [x] 集成测试：include/exclude、format、count
- [x] 边缘用例：空文件/目录、锚定路径、regex 全路径匹配、`rule_idx` 1-based
- [x] 输出一致性：tsv/json 路径分隔符为 `/`

---

## Phase 6：文档与生成

- [ ] 更新 `docs/README.md` 索引
- [ ] 更新命令示例与速查（`tools/gen_readme_commands.py` / `tools/gen_commands_md.py`）
