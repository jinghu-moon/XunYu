# Backup 优化路线图 — TDD 分阶段任务清单

> 依据：[Backup-Optimization-Roadmap.md](./Backup-Optimization-Roadmap.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 原则：**红-绿-重构**。每个任务先写失败测试，再写最小实现，最后重构。
> 范围：覆盖传统 `backup`、`.xunbak` 7-Zip 插件、恢复链路、校验链路、发布与运维体验。
> 说明：本清单面向“下一阶段优化”，默认不追求兼容历史 PoC 脚本行为，但避免无必要的用户可见破坏性变更。
> 实现策略：**优先参考同类型项目，在 XunYu 内手写相关功能**；不把外部 crate 当成长期唯一实现载体。

---

## Phase 0：优化基线冻结

### 0.0 参考实现边界

- [x] 确认 `zip2-master` 作为 ZIP 行为参考，而不是最终唯一实现
- [x] 确认 `sevenz-rust2-main` / `7z2600-src` 作为 7z 行为参考，而不是最终唯一实现
- [x] 确认插件继续沿用“C++ 薄壳 + Rust 核心”分层

### 0.1 现状记录

- [x] 复核当前基线文档中的测试环境和版本信息
- [x] 确认 7-Zip 安装目录探测策略
- [x] 确认 `.xunbak` 插件 PoC 当前只读能力范围
- [x] 确认传统 `backup` 当前 `diff-mode=auto|hash|meta` 行为不变

### 0.2 基线验证

- [x] **测试**：`cargo check --tests --benches` 通过
- [x] **测试**：`cargo test --test module_backup_restore` 通过
- [x] **测试**：`.xunbak` 插件系统联调脚本当前可跑通
- [x] 将验证命令与结果写入阶段备注

---

## Phase 1：7-Zip 插件安装器与 doctor

### 1.0 已有脚本原型

- [x] 已有 `build_xunbak_7z_plugin.ps1`
- [x] 已有 `install_xunbak_7z_plugin.ps1`
- [x] 已有 `uninstall_xunbak_7z_plugin.ps1`
- [x] 已有 `smoke_xunbak_7z_plugin.ps1`
- [x] 已有 `test_xunbak_7z_plugin_portable.ps1`
- [x] 已有 `test_xunbak_7z_plugin_system.ps1`
- [x] 已有 `accept_xunbak_7z_plugin.ps1`
- [x] 说明：当前缺的是统一 `xun xunbak plugin ...` CLI 封装，而不是底层脚本能力

### 1.1 CLI 与脚本边界

- [x] 新增 `xun xunbak plugin install`
- [x] 新增 `xun xunbak plugin uninstall`
- [x] 新增 `xun xunbak plugin doctor`
- [x] 明确仅支持“已安装 7-Zip”的场景，不负责分发 7-Zip 本体

### 1.2 安装路径探测

- [x] **测试**：优先探测显式传入的 `--sevenzip-home`
- [x] **测试**：可探测 `C:/A_Softwares/7-Zip`
- [x] **测试**：可探测 `C:/Program Files/7-Zip`
- [x] **测试**：未探测到 7-Zip 时返回明确错误
- [x] 实现安装目录探测逻辑

### 1.3 插件安装

- [x] **测试**：`install` 会复制 `xunbak.dll` 到 `7-Zip/Formats`
- [x] **测试**：目标 DLL 已存在时支持覆盖或明确拒绝
- [x] **测试**：缺少本地构建产物时返回明确错误
- [x] 实现安装命令

### 1.4 插件卸载

- [x] **测试**：`uninstall` 删除 `7-Zip/Formats/xunbak.dll`
- [x] **测试**：目标 DLL 不存在时返回幂等结果
- [x] 实现卸载命令

### 1.5 doctor

- [x] **测试**：`doctor` 输出 DLL 是否存在
- [x] **测试**：`doctor` 输出 `.xunbak` 文件关联状态
- [x] **测试**：`doctor` 输出 7-Zip 主程序路径和版本
- [x] **测试**：`doctor` 输出建议修复动作
- [x] 实现诊断命令

---

## Phase 2：`.xunbak` 文件关联

### 2.1 关联状态识别

- [x] **测试**：能识别 `.xunbak` 未关联
- [x] **测试**：能识别 `.xunbak` 已关联到 `7zFM.exe`
- [x] **测试**：能识别关联到非 7-Zip 的程序
- [x] 实现关联状态探测

### 2.2 建立关联

- [x] **测试**：`install --associate` 会建立 `.xunbak -> 7zFM.exe`
- [x] **测试**：重复执行不会生成脏状态
- [x] **测试**：关联失败时返回明确错误与回滚提示
- [x] 实现关联写入逻辑

### 2.3 移除关联

- [x] **测试**：`uninstall --remove-association` 移除 `.xunbak` 关联
- [x] **测试**：若关联不是 7-Zip，不会误删第三方绑定
- [x] 实现关联移除逻辑

---

## Phase 3：7-Zip 插件大文件打开路径优化

### 3.1 callback 优先

- [x] **测试**：单文件 `.xunbak` 优先走 `xunbak_open_with_callbacks`
- [x] **测试**：分卷 `.xunbak.001` 优先走 callback volume 打开
- [x] **测试**：callback 成功时不落入整文件 `ReadAll`
- [x] 改造 `OpenCore()` 打开路径

### 3.2 fallback 收敛

- [x] **测试**：小文件 callback 失败时可安全 fallback 到内存打开
- [x] **测试**：超阈值大文件 callback 失败时不做 `ReadAll`
- [x] **测试**：大文件 fallback 被拒绝时提示明确原因
- [x] 增加 fallback 阈值策略

### 3.3 打开性能

- [x] **bench**：单文件 `.xunbak` 插件打开时间
- [x] **bench**：分卷 `.xunbak.001` 插件打开时间
- [x] **bench**：大文件 callback 路径与 memory fallback 路径对比
- [x] 记录基线

---

## Phase 4：插件属性列与归档信息增强

### 4.1 item property

- [x] **测试**：显示 `Path`
- [x] **测试**：显示 `Size`
- [x] **测试**：显示 `Packed Size`
- [x] **测试**：显示 `Method`
- [x] **测试**：显示 `Modified`
- [x] **测试**：显示 `Created`
- [x] **测试**：显示 `Attributes`
- [x] 完善 `GetProperty` / `GetPropertyInfo`

### 4.2 archive property

- [x] **测试**：显示 `Read Only`
- [x] **测试**：显示 `Files`
- [x] **测试**：显示 `Volumes`
- [x] **测试**：显示 `Physical Size`
- [x] 完善 `GetArchiveProperty` / `GetArchivePropertyInfo`

### 4.3 GUI 展示验收

- [ ] **手工清单**：7-Zip GUI 中列头显示合理
- [ ] **手工清单**：`7z l -slt` 输出关键信息完整
- [ ] **手工清单**：中文路径显示无插件侧乱码

---

## Phase 5：传统 backup 的真实 `file_id`

### 5.1 扫描阶段采集

- [x] **测试**：Windows 下可采集文件 `file_id`
- [x] **测试**：采集失败时为 `None`，不影响主流程
- [x] 在 `scan.rs` 中接入真实 `file_id`

### 5.2 hash cache 接入

- [x] **测试**：`file_id` 相同且元数据相符时命中缓存
- [x] **测试**：`file_id` 变化时正确失效缓存
- [x] 在 `hash_cache` 中消费真实 `file_id`

### 5.3 rename-only 场景

- [x] **测试**：同内容 rename-only 且 `file_id` 相同，不触发重复 hash
- [x] **测试**：路径变化但 `file_id` 相同的场景能更快命中
- [x] 优化 diff / cache 命中策略

### 5.4 性能基线

- [x] **bench**：rename-only 场景接入 `file_id` 前后对比
- [x] **bench**：冷缓存小改动场景前后对比
- [x] 记录基线

---

## Phase 6：恢复链路顺序读取与 reader 复用

### 6.1 reader 复用

- [x] **测试**：多文件恢复时同一 artifact reader 可复用
- [x] **测试**：复用 reader 不改变恢复结果
- [x] 改造 restore 内部 reader 生命周期

### 6.2 顺序读取

- [ ] **测试**：`.xunbak` 多文件恢复按更优顺序读取
- [ ] **测试**：`7z` 多文件恢复不退化为明显随机读取
- [ ] 优化批量恢复的调度顺序

### 6.3 preview 快速路径

- [x] **测试**：preview 不为全部候选文件打开内容流
- [x] **测试**：同 size/mtime/属性的预览走快速判定
- [x] 优化 preview 数据获取

---

## Phase 7：传统 backup 报告增强

### 7.1 新增统计字段

- [x] **测试**：输出 `rename_only_count`
- [x] **测试**：输出 `reused_bytes`
- [x] **测试**：输出 `cache_hit_ratio`
- [x] **测试**：输出 `baseline_source`
- [x] 扩展命令行文本报告

### 7.2 JSON 输出

- [x] **测试**：JSON 输出包含以上字段
- [x] **测试**：字段在 `skipped / dry_run / ok` 三种状态下稳定存在
- [x] 扩展 JSON 视图结构

### 7.3 `.bak-meta.json`

- [x] **测试**：新增统计字段进入 `.bak-meta.json`
- [x] **测试**：旧 meta 缺字段时仍可读取
- [x] 扩展 meta 持久化结构

---

## Phase 8：`verify` 分级模式

### 8.1 CLI 模式

- [x] 新增 `quick`
- [x] 新增 `full`
- [x] 新增 `manifest-only`
- [x] 新增 `existence-only`

### 8.2 语义测试

- [x] **测试**：`manifest-only` 只校验 manifest 结构与条目映射
- [x] **测试**：`existence-only` 只校验文件是否存在
- [x] **测试**：`quick` 提供快速完整性检查
- [x] **测试**：`full` 执行完整内容校验

### 8.3 错误报告

- [x] **测试**：错误输出包含首个失败路径
- [x] **测试**：zip / 7z / xunbak 错误输出包含来源上下文
- [x] **测试**：分卷错误输出包含卷号或卷名
- [x] 优化错误文案

---

## Phase 9：导出链路元数据对齐

### 9.1 目录与压缩格式一致性

- [x] **测试**：`dir -> zip` 保留 mtime / readonly
- [x] **测试**：`dir -> 7z` 保留 mtime / readonly
- [x] **测试**：`xunbak -> dir/zip/7z` 的路径与时间元数据一致

### 9.2 sidecar 与 packed 信息

- [x] **测试**：sidecar 中格式信息与导出结果一致
- [x] **测试**：packed size / codec 信息在支持场景下可对齐
- [x] 优化 sidecar 与导出元数据

---

## Phase 10：手写 ZIP / 7z 方法集扩展

### 10.1 ZIP backend（方案 C）

- [-] **说明**：ZIP 目标方法集固定为 `stored / deflated / bzip2 / zstd / ppmd`，不包含 `lzma2`；当前已接入全部五种方法，其中 `ppmd` 走 XunYu 自己的纯 Rust 手写 writer + manual parser
- [x] **现状**：当前已实现 `stored / deflated / bzip2 / zstd / ppmd`
- [x] **测试**：保留当前 `stored / deflated` 行为不回归
- [x] **测试**：当前 ZIP backend 可写出 `bzip2`
- [x] **测试**：当前 ZIP backend 可写出 `zstd`
- [x] **测试**：手写 ZIP backend 可写出 `ppmd`
- [-] **测试**：ZIP local header / central directory / EOCD 结构可被 `zip` / 7-Zip reopen
  当前 `bzip2 / zstd` 可被 `zip` crate 与 7-Zip reopen，`ppmd` 可被 XunYu parser 与 7-Zip reopen；上游 `zip` crate `2.4.2` 本身仍缺 `ppmd` 解压
- [x] **测试**：Zip64 在手写 backend 下仍正确
  已完成单条目 `4 GiB + 1 MiB` 的 `ZIP ppmd` 端到端验证：纯 Rust 写出、stock `7-Zip 24.09` `7z t` 通过、XunYu 自己 `convert -> dir` 恢复并通过偏移 marker 对比
- [-] 在 XunYu 内实现 ZIP method routing / writer backend

### 10.2 7z method 扩展

- [x] **现状**：当前已实现 `copy / lzma2`
- [x] **测试**：`SevenZMethod` 扩展为 `copy / lzma2 / bzip2 / deflate / ppmd / zstd`
- [x] **测试**：显式 `--method bzip2` 生效
- [x] **测试**：显式 `--method deflate` 生效
- [x] **测试**：显式 `--method ppmd` 生效
- [x] **测试**：显式 `--method zstd` 生效
- [x] **测试**：method id 与 `7z2600-src` / `sevenz-rust2-main` 一致
- [x] 在 XunYu 内实现 7z method routing / writer options

### 10.3 兼容矩阵与提示

- [-] **测试**：ZIP `bzip2 / zstd / ppmd` 的 reopen 行为稳定
  当前已验证 `bzip2 / zstd` 可被 `zip` crate 与 7-Zip reopen；`ppmd` 可被 XunYu parser 与 7-Zip reopen
- [x] **测试**：7z `bzip2 / deflate / ppmd` 在 stock 7-Zip 可解
  当前 `ppmd` 已修复为纯 Rust 写出，可通过 stock `7-Zip 24.09` 的 `7z t`
- [x] **测试**：7z `zstd` 在支持外部 codec 的解压端可解
  已在隔离临时 7-Zip 副本中注入 `7-Zip-zstd` `zstd.dll`，并验证 `7z t` 通过
- [x] **测试**：doctor / 文档能对 `zstd` codec 兼容差异给出提示
- [x] 记录方法级兼容矩阵

---

## Phase 11：插件发布化

### 11.1 Release 构建

- [ ] **测试**：Release 模式构建出 `xunbak.dll`
- [ ] **测试**：Release 插件在目标 7-Zip 版本可加载
- [ ] 固化 Release 构建脚本

### 11.2 版本矩阵

- [ ] **测试清单**：7-Zip 24.x
- [ ] **测试清单**：7-Zip 26.x
- [ ] **测试清单**：便携版 / 安装版
- [ ] 记录兼容矩阵

### 11.3 发布说明

- [ ] 记录安装步骤
- [ ] 记录卸载步骤
- [ ] 记录已知限制
- [ ] 记录问题排查方法

---

## Phase 12：综合验收

### 12.1 7-Zip 插件

- [ ] **手工验收**：单文件 `.xunbak` 可直接在 7-Zip 中打开
- [ ] **手工验收**：分卷 `.xunbak.001` 可直接在 7-Zip 中打开
- [ ] **手工验收**：单文件提取内容正确
- [ ] **手工验收**：全量提取内容正确

### 12.2 传统 backup

- [ ] **手工验收**：小改动场景下 cache 命中与 hardlink 复用明显生效
- [ ] **手工验收**：rename-only 场景行为符合预期
- [ ] **手工验收**：`verify` 分级模式行为符合预期

### 12.3 回归

- [ ] `cargo check --tests --benches`
- [ ] `cargo test --lib`
- [ ] `cargo test --test module_backup_restore`
- [ ] 插件便携式联调脚本
- [ ] 插件系统级联调脚本

---

## 依赖关系

```text
Phase 0（基线冻结）
  ├─→ Phase 1（安装器/doctor）
  ├─→ Phase 3（插件打开路径优化）
  └─→ Phase 5（file_id）

Phase 1 ─→ Phase 2（文件关联）
Phase 3 ─→ Phase 4（插件属性增强） ─→ Phase 11（发布化）
Phase 5 ─→ Phase 7（报告增强）
Phase 6（恢复优化） ─┐
Phase 8（verify 分级）├─→ Phase 12（综合验收）
Phase 9（导出对齐） ─┘
Phase 10（ZIP/7z 方法扩展） ─┘
```

---

## 建议执行顺序

### 第 1 批

1. Phase 1：安装器 / uninstall / doctor
2. Phase 2：文件关联
3. Phase 12.1：插件手工验收

### 第 2 批

1. Phase 3：插件大文件打开路径优化
2. Phase 4：插件属性增强
3. Phase 10：手写 ZIP / 7z 方法集扩展
4. Phase 11：Release 发布化

### 第 3 批

1. Phase 5：传统 backup 接入真实 `file_id`
2. Phase 6：恢复链路顺序读取与 reader 复用
3. Phase 7：报告增强
4. Phase 8：verify 分级模式
5. Phase 9：导出链路元数据对齐

---

## 验证命令建议

```bash
# 基础编译
cargo check --tests --benches

# 库测试
cargo test --lib

# backup / restore 黑盒
cargo test --test module_backup_restore

# 插件构建
./scripts/build_xunbak_7z_plugin.ps1 -Config Debug

# 插件联调
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Debug
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Debug
```
