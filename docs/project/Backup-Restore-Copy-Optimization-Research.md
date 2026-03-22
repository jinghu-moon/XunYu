# Backup/Restore Copy 路径优化调研

生成时间：2026-03-22

## 1. 目标

本文档用于收敛 `backup` / `restore` 在“文件复制路径”上的优化方向，回答 4 个问题：

1. 当前瓶颈具体在哪一段。
2. 主流同类工具在本地备份/恢复场景下怎么设计。
3. 哪些方案适合 XunYu 当前的快速开发期。
4. 下一轮实施应该按什么顺序推进。

本文只讨论：

- 本地文件系统上的 `backup` / `restore`
- 目录型备份与 zip 型备份
- 拷贝、跳过、硬链接、元数据缓存、原生 Win32 copy 后端

本文不讨论：

- 远程仓库同步
- 云端对象存储
- 完整内容寻址仓库重构
- UI 层交互优化

---

## 2. 当前实测结论

基于当前代码中新增的阶段计时（`XUN_CMD_TIMING=1`）和 500 文件样本，内部阶段耗时如下。

### 2.1 backup

- `scan`: 约 `1 ms`
- `baseline`: 约 `0 ms`
- `diff`: 约 `5 ms`
- `diff-print`: 约 `4 ms`
- `copy`: 约 `60 ms`
- `report`: 约 `0 ms`
- `dispatch total`: 约 `73 ms`

结论：

- `backup` 当前的主瓶颈是 `copy`
- `scan`、`baseline`、`diff` 已不是主要矛盾

### 2.2 restore

- `collect-dir`: 约 `9 ms`
- `copy-dir`: 约 `60 ms`
- `execute`: 约 `70 ms`
- `dispatch total`: 约 `70 ms`

结论：

- `restore` 当前的主瓶颈同样是复制路径
- `source` 定位、目标目录准备、预览逻辑都不是热点

### 2.3 外部观测与内部观测差异

需要区分两类耗时：

1. **命令内部耗时**
   - 来自 `XUN_CMD_TIMING=1`
   - 只统计 `xun` 进程内部执行
2. **外部测试/benchmark 耗时**
   - 来自 `cargo test` / `cargo bench`
   - 包含新进程启动、参数解析和测试驱动开销

因此：

- 内部 `backup` 大约 `73 ms`
- 外部 `perf_backup_full_500_files` 大约 `185 ms`
- 二者不能直接横比

---

## 3. 当前代码结构对优化的启示

### 3.1 backup

关键路径：

- `src/commands/backup.rs`
- `src/commands/backup/diff.rs`

当前特征：

1. 已有上一版 baseline 概念，可判定 `new/modified/unchanged/deleted`
2. `full backup` 下，`unchanged` 文件仍会复制
3. 复制实现已具备并行能力
4. 目录备份大小已经可直接复用 `bytes_copied`

### 3.2 restore

关键路径：

- `src/commands/restore.rs`
- `src/commands/restore_core.rs`

当前特征：

1. 目录恢复和 zip 恢复已经拆开
2. 目录恢复已做预建目录与 copy job 预计算
3. zip 恢复仍主要受解压 + 写文件路径影响

---

## 4. 外部参考与可借鉴点

以下结论基于主流同类工具和官方文档。

### 4.1 rsync：本地复制优先 whole-file，不优先做块级 delta

参考：

- `rsync` manpage
- 链接：<https://download.samba.org/pub/rsync/rsync.1>

关键点：

1. 本地复制默认倾向 `--whole-file`
2. 块级 delta 更适合网络同步，不一定适合本地磁盘复制
3. `--link-dest` 为 unchanged 文件提供硬链接复用方案

对 XunYu 的启示：

1. 不要在当前阶段优先做“块级增量复制”
2. 应优先优化“是否复制”与“如何复制”
3. `--link-dest` 思路非常适合 full backup 的 unchanged 文件

### 4.2 restic：用元数据快速判 unchanged

参考：

- `restic` backup docs
- 链接：<https://restic.readthedocs.io/en/latest/040_backup.html>

关键点：

1. Windows 下会使用 `path + size + mtime` 判断文件是否变化
2. 重点是减少不必要的文件内容读取

对 XunYu 的启示：

1. 现有 `baseline + size + modified` 路线是正确的
2. 当前最应该继续做的是“让 unchanged 文件不再走复制”
3. 暂时不需要引入更复杂的内容级变更判定

### 4.3 Borg：文件缓存优先，避免重复扫描/重复读取

参考：

- Borg FAQ
- Borg internals
- 链接：
  - <https://borgbackup.readthedocs.io/en/stable/faq.html>
  - <https://borgbackup.readthedocs.io/en/2.0.0b19/internals/data-structures.html>

关键点：

1. 有明确的 files cache 设计
2. 命中缓存后，文件甚至无需再次完整读取或重分块
3. cache scope 与 TTL 会影响性能与正确性平衡

对 XunYu 的启示：

1. “项目级文件缓存”是中期值得做的方向
2. 但它的复杂度明显高于 hardlink unchanged
3. 不适合作为快速开发期第一刀

### 4.4 Kopia：内容寻址仓库 + pack files

参考：

- Kopia architecture
- 链接：<https://kopia.io/docs/advanced/architecture/>

关键点：

1. 内容按 hash 去重
2. 小块聚合成 pack files
3. 从存储模型层面规避“重复 copy 目录树”

对 XunYu 的启示：

1. 这是长期上限最高的路线
2. 但这已经不是 copy 路径优化，而是仓库模型重构
3. 现阶段不应直接切到这条路

### 4.5 Windows 官方：CopyFile2 / CopyFileEx

参考：

- Microsoft Learn `CopyFile2`
- Microsoft Learn `CopyFileEx`
- 链接：
  - <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfile2>
  - <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfileexa>

关键点：

1. 是 Win32 官方 copy API
2. 支持更好的进度与控制能力
3. 对 Windows-first 项目比只用 `std::fs::copy` 更有进一步优化空间

对 XunYu 的启示：

1. 可以在 `backup` / `restore` 中引入可切换的 native copy backend
2. 先做 A/B backend，不要一次性替换默认实现
3. 用现有 benchmark 比较 `fs::copy` 与 `CopyFile2`

---

## 5. 对 XunYu 当前最合适的方案

### 5.1 第一优先级：full backup 下 unchanged 文件 hardlink 化

方案描述：

1. 在目录型 `full backup` 中
2. 若文件在 diff 中被判为 `Unchanged`
3. 且上一版是目录备份
4. 且目标与上一版位于同一卷
5. 则优先对上一版文件执行硬链接，而不是再次复制

来源参考：

- `rsync --link-dest`

优点：

1. 直接减少 copy 时间
2. 直接减少磁盘占用
3. 不改现有 CLI 契约
4. 对当前目录型快照模型很友好

风险：

1. 仅适用于目录型备份
2. 跨卷会失败，需要 fallback 到 copy
3. 需要明确后续备份不会修改历史快照文件

适合度：

- **高**

### 5.2 第二优先级：引入 Win32 native copy backend

方案描述：

1. 对 `backup copy`
2. 对 `restore_from_dir` / `restore_many_from_dir`
3. 对 `restore_from_zip` / `restore_many_from_zip` 的落盘写出阶段
4. 提供 `std::fs::copy` 与 `CopyFile2` 双 backend

优点：

1. 对当前结构侵入相对可控
2. 能直接对准现有瓶颈
3. 可结合阶段计时和 benchmark 做 A/B

风险：

1. 平台绑定增强
2. 需要额外错误映射与兼容测试
3. 不保证在所有样本上都显著优于 `fs::copy`

适合度：

- **高**

### 5.3 第三优先级：无变更时跳过创建新快照

方案描述：

1. 若 diff 结果为 `+0 ~0 -0`
2. 则允许在显式配置或开关下直接跳过生成新版本

优点：

1. 性能提升非常直接
2. 可以减少目录与文件数量膨胀
3. 对大量“重复手动备份”场景尤其有效

风险：

1. 改变用户对“每次运行都生成版本”的预期
2. 需要设计好 CLI 契约，例如：
   - `--skip-if-unchanged`
   - 或配置项

适合度：

- **中高**

### 5.4 中期方案：项目级 file cache

方案描述：

1. 为 `backup` 引入独立 cache 文件
2. 持久化最近一次扫描到的 `path/size/mtime`
3. 多轮备份时优先命中 cache

优点：

1. 更接近 Borg/restic 的成熟设计
2. 在大目录、多轮备份下收益会更明显

风险：

1. cache 失效策略复杂
2. 容易引入“判定不更新”类 bug
3. 快速开发期实现成本偏高

适合度：

- **中**

### 5.5 长期路线：内容寻址仓库

方案描述：

1. 不再把每个快照落成完整目录
2. 文件内容按 hash 存入对象/pack
3. 快照只保留元数据索引

优点：

1. 去重能力最强
2. 空间和多版本复制开销最优

风险：

1. 不是局部优化，而是架构重写
2. 恢复路径、校验路径、保留策略都要重做

适合度：

- **低（当前阶段）**

---

## 6. 不建议当前阶段优先做的事

### 6.1 不建议先做块级 delta copy

原因：

1. 更适合网络同步
2. 本地盘场景往往得不偿失
3. 当前热点不是 diff 算法，而是 copy 路径

### 6.2 不建议直接切换到内容寻址仓库

原因：

1. 风险过高
2. 无法作为“当前 copy 路径优化”的一部分交付
3. 会拖慢当前快速开发节奏

### 6.3 不建议继续在 scan/diff 上深挖微优化

原因：

1. 当前阶段计时已经说明：
   - `scan` 很小
   - `diff` 很小
2. 再挤这部分收益有限

---

## 7. 推荐实施顺序

### 阶段 A：保守收益型

1. 保留现有阶段计时
2. 引入 `CopyFile2` backend 做 A/B
3. 用 benchmark 和 `XUN_CMD_TIMING=1` 对比

验收目标：

1. `backup copy` 明显下降
2. `restore copy-dir` 或 zip 写出阶段下降

### 阶段 B：结构收益型

1. 在目录型 full backup 下，对 unchanged 文件优先 hardlink
2. 失败时 fallback 到 copy

验收目标：

1. full backup 的 `copy` 明显下降
2. 磁盘占用下降
3. 不影响 restore 正确性

### 阶段 C：行为优化型

1. 设计 `--skip-if-unchanged`
2. 或者引入配置项

验收目标：

1. 无变化目录可快速返回
2. 不破坏现有用户预期

### 阶段 D：中期能力型

1. 设计 file cache
2. 再决定是否推进内容寻址仓库

---

## 8. 建议的实现优先级

| 优先级 | 方案 | 收益 | 风险 | 推荐度 |
| --- | --- | --- | --- | --- |
| P0 | `CopyFile2` backend A/B | 中 | 中 | 高 |
| P0 | full backup unchanged hardlink | 高 | 中 | 高 |
| P1 | `--skip-if-unchanged` | 中 | 中 | 中高 |
| P2 | 项目级 file cache | 中高 | 中高 | 中 |
| P3 | 内容寻址仓库 | 很高 | 很高 | 低 |

---

## 9. 当前结论

一句话结论：

> 对当前 XunYu 来说，`backup/restore` 的 copy 路径优化最应该先做的不是更复杂的 diff，而是 **Win32 原生 copy backend** 和 **unchanged 文件 hardlink 化**。

理由：

1. 当前内部瓶颈已经定位到 copy
2. 主流工具在本地场景里并不优先做块级 delta
3. `rsync --link-dest` 的思路和当前目录型快照模型高度契合
4. `CopyFile2` 是 Windows-first 项目最自然的下一步

---

## 10. 参考资料

1. rsync manpage  
   <https://download.samba.org/pub/rsync/rsync.1>
2. restic backup docs  
   <https://restic.readthedocs.io/en/latest/040_backup.html>
3. Borg FAQ  
   <https://borgbackup.readthedocs.io/en/stable/faq.html>
4. Borg internals  
   <https://borgbackup.readthedocs.io/en/2.0.0b19/internals/data-structures.html>
5. Kopia architecture  
   <https://kopia.io/docs/advanced/architecture/>
6. Microsoft Learn: CopyFile2  
   <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfile2>
7. Microsoft Learn: CopyFileEx  
   <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfileexa>
