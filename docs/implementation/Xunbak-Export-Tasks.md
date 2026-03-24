# 备份产物多格式支持 — TDD 分阶段任务清单

> 依据：[Xunbak-7zip-Compat.md](./Xunbak-7zip-Compat.md)
> 标记：`[ ]` 待办　`[-]` 进行中　`[x]` 完成
> 原则：**红-绿-重构**。每个任务先写失败测试，再写最小实现，最后重构。
> 术语：
> `backup create` = 从源目录生成备份产物
> `backup restore` = 从备份产物恢复到目标目录
> `backup convert` = 在备份产物之间转换
> 产物格式 = `dir | xunbak | zip | 7z`
> 说明：本清单按**底层到消费层**排序。CLI/路由故意放到后段，避免在底层抽象未稳定前锁死外部接口。

---

## Phase 0：核心契约与共享类型

### 0.1 动作 / 产物枚举

- [x] **测试**：`BackupAction` 可表达 `create / restore / convert`
- [x] **测试**：`BackupArtifactFormat` 可解析 `dir | xunbak | zip | 7z`
- [x] **测试**：`BackupArtifactFormat` 对未知值返回稳定错误
- [x] **测试**：`zip` / `7z` / `dir` / `xunbak` 的显示名与 JSON 序列化稳定
- [x] 实现 `src/backup_formats.rs`：`BackupAction / BackupArtifactFormat`

### 0.2 共享选项结构

- [x] **测试**：`BackupCreateOptions` 仅包含 `create` 所需字段
- [x] **测试**：`BackupConvertOptions` 仅包含 `convert` 所需字段
- [x] **测试**：`BackupRestoreOptions` 仅包含 `restore` 所需字段
- [x] **测试**：`OverwriteMode / VerifySourceMode / VerifyOutputMode / ProgressMode` 序列化稳定
- [x] 实现 `src/backup_export/options.rs`：`BackupCreateOptions / BackupConvertOptions / BackupRestoreOptions`

### 0.3 错误模型与退出码

- [x] **测试**：参数错误映射为退出码 `2`
- [x] **测试**：写出失败映射为退出码 `1`
- [x] **测试**：postflight 校验失败映射为退出码 `1`
- [x] **测试**：成功映射为退出码 `0`
- [x] **测试**：JSON 结果 `status = ok | write_failed | verify_failed | preflight_failed`
- [x] 实现统一 `ExportError / ExportStatus / ExitCode` 约定

### 0.4 统一进度事件

- [x] **测试**：`ExportProgressEvent` 包含 `phase / selected_files / processed_files / bytes_in / bytes_out / throughput / elapsed_ms`
- [x] **测试**：`phase` 仅允许 `verify_source | read | compress | write | verify_output`
- [x] 实现 `src/backup_export/progress.rs`

---

## Phase 1：源抽象与选择器

### 1.1 `SourceEntry`

- [x] **测试**：`SourceEntry` 可表达源目录文件、`dir` 产物条目、`.xunbak` manifest entry、`.zip` entry、`.7z` entry
- [x] **测试**：`SourceEntry` 覆盖 `path / size / mtime / ctime / win_attributes / content_hash(optional)`
- [x] **测试**：路径统一为 `/` 分隔、相对路径
- [x] 实现 `src/backup_export/source.rs`：`SourceEntry`

### 1.2 `fs_source.rs`

- [x] **测试**：`backup create` 完全复用现有 backup 配置文件与 scan 逻辑
- [x] **测试**：`backup create` 复用 include/exclude/gitignore，不新增独立 glob 入口
- [x] **测试**：`backup create --list` 的文件集与当前 backup scan 结果一致
- [x] 实现 `src/backup_export/fs_source.rs`

### 1.3 `artifact_source.rs`

- [x] **测试**：`backup convert` 可从 `dir` 产物读取条目
- [x] **测试**：`backup convert` 可从 `.xunbak` 产物读取条目
- [x] **测试**：`backup convert` 可从 `.zip` 产物读取条目
- [x] **测试**：`backup convert` 可从 `.7z` 产物读取条目
- [x] 实现 `src/backup_export/artifact_source.rs`

### 1.4 选择器语义

- [x] **测试**：`backup create` 不接受 `--file/--glob/patterns-from`
- [x] **测试**：`backup convert --file a.txt` 仅选择指定路径
- [x] **测试**：`backup convert --glob "src/**/*.rs"` 仅选择匹配条目
- [x] **测试**：`backup convert --patterns-from patterns.txt` 批量导入选择规则
- [x] **测试**：`--file + --glob + --patterns-from` 取并集
- [x] 实现 `src/backup_export/selection.rs`

---

## Phase 2：已校验流式读取

### 2.1 `VerifiedEntryReader`

- [x] **测试**：`open_entry()` 返回 `Read` 流，而不是 `Vec<u8>`
- [ ] **测试**：大文件导出时内存峰值与 chunk size 成正比
- [x] **测试**：内容校验失败会中止导出
- [-] 实现 `VerifiedEntryReader`

### 2.2 `.xunbak` 流式读取适配

- [x] **测试**：从 `.xunbak` 读取单文件可边读边写
- [ ] **测试**：multipart entry 可流式拼接输出并保持 hash 校验
- [x] **测试**：读取顺序延续现有 `volume_index + blob_offset` 优化
- [x] 实现流式 `.xunbak` 读取 / 校验复制路径

### 2.3 `dir/zip/7z` 流式读取适配

- [x] **测试**：`dir` 产物条目可转成 `Read`
- [x] **测试**：`.zip` 产物条目可转成 `Read`
- [x] **测试**：`.7z` 产物条目可转成 `Read`
- [-] 实现其余 source adapters

---

## Phase 3：输出计划、原子写出与 sidecar

### 3.1 `output_plan.rs`

- [x] **测试**：ZIP 输出使用 `name.tmp.zip`
- [x] **测试**：单卷 7z 输出使用 `name.tmp.7z`
- [x] **测试**：分卷 7z 输出使用 `name.tmp.7z.001/.002/...`
- [x] **测试**：`dir` 输出使用临时目录
- [-] 实现 `OutputPlan`
  已覆盖：目录、单文件 `.zip`、单文件 `.7z`、单文件 `.xunbak`、分卷 `.xunbak` 新建路径

### 3.2 overwrite 行为

- [x] **测试**：`overwrite=fail` 且目标存在时拒绝
- [x] **测试**：`overwrite=replace` 对单文件产物成功原子替换
- [ ] **测试**：`overwrite=ask` 在交互模式触发确认
- [x] **测试**：`dir` 模式 `replace` 仅在目标不存在时原子 rename；目标已存在时改为 staged mirror + replace
- [-] 实现 `OverwriteMode`

### 3.3 失败清理

- [x] **测试**：写出失败时清理临时产物
- [x] **测试**：失败不污染正式目标
- [x] **测试**：首版不支持 resume，但返回明确提示
- [x] 实现失败清理策略

### 3.4 sidecar

- [x] **测试**：默认写入 `__xunyu__/export_manifest.json`
- [x] **测试**：ZIP/7z 都写 sidecar
- [x] **测试**：`dir` 模式默认也写 sidecar
- [x] **测试**：sidecar 包含 `snapshot_id/source_root/exported_at/xunyu_version`
- [x] **测试**：sidecar 包含每个文件的 `content_hash/created_time_ns/win_attributes`
- [x] **测试**：`--no-sidecar`（若开放）可禁用
- [x] 实现 sidecar 开关与默认输出

---

## Phase 4：`.xunbak` 产物接入新模型

### 4.1 `backup create --format xunbak`

- [x] **测试**：`backup create --format xunbak` 成功生成 `.xunbak`
- [x] **测试**：底层直接复用现有 `cmd_backup_container` / writer 路径
- [x] **测试**：新命令路径复用 backup 配置与选择规则
- [x] **测试**：旧命令 `xun backup --container ...` 与新命令结果一致
- [x] 将现有 `.xunbak` create 路径接入新 backup 域

### 4.2 `backup restore xunbak`

- [x] **测试**：`backup restore archive.xunbak --to out` 成功
- [x] **测试**：旧命令 `xun restore archive.xunbak --to out` 仍成功
- [x] 接入现有 `.xunbak` restore 路径

### 4.3 `backup convert xunbak -> ...`

- [x] **测试**：`xunbak -> dir`
- [x] **测试**：`xunbak -> zip`
- [x] **测试**：`xunbak -> 7z`
- [-] 打通 `.xunbak` 作为 convert 输入

---

## Phase 5：`dir` 产物

### 5.1 `backup create --format dir`

- [x] **测试**：生成目录型备份产物
- [x] **测试**：目录结构、内容、mtime、win_attributes 与源一致
- [x] **测试**：输出目标不得等于源工作目录
- [x] 实现 `dir_writer.rs` facade

### 5.2 `backup restore dir`

- [x] **测试**：`backup restore dir-artifact --to out`
- [ ] **测试**：preview / confirm / path_guard 行为与现有 restore 一致
- [x] **测试**：`dir` 模式 `--file/--glob` 语义与现有 restore 一致
- [x] 明确复用 restore core

### 5.3 `backup convert dir -> ...`

- [x] **测试**：`dir -> zip`
- [x] **测试**：`dir -> xunbak`
- [x] **测试**：`dir -> 7z`

---

## Phase 6：ZIP 产物（B1）

### 6.1 ZIP writer 基线

- [x] **测试**：`backup create --format zip` 生成标准 ZIP，可被 `zip` crate reopen
- [x] **测试**：显式目录项被写入
- [x] **测试**：可被 7-Zip 24.x 打开 / `t` 校验
- [x] 实现 `zip_writer.rs`

### 6.2 ZIP 元数据映射

- [x] **测试**：UTF-8 路径写入正确
- [x] **测试**：mtime 正确写入
- [x] **测试**：Unix 权限位写入；Windows 属性不承诺完整保真
- [x] **测试**：超大文件启用 Zip64

### 6.3 ZIP 压缩策略

- [x] **测试**：默认 `--method deflated`
- [x] **测试**：已压缩/不可压缩扩展名使用 `stored`
- [x] **测试**：显式 `--method stored` 生效
- [x] **测试**：ZIP 模式拒绝 `lzma2/bzip2/ppmd`
- [x] 复用现有 `should_skip_compress()` 规则

### 6.4 `backup restore zip`

- [x] **测试**：`backup restore archive.zip --to out`
- [x] **测试**：内容与 `backup create --format zip` 产物一致
- [x] **测试**：`restore --file/--glob` 对 ZIP 继续有效
- [x] 复用/整理现有 ZIP restore 路径

### 6.5 `backup convert zip -> ...`

- [x] **测试**：`zip -> dir`
- [x] **测试**：`zip -> xunbak`
- [x] **测试**：`zip -> 7z`

---

## Phase 7：公共 preflight / postflight / preview / progress / JSON

### 7.1 preflight

- [x] **测试**：`backup convert` 默认先做 `.xunbak quick verify`
- [x] **测试**：`--verify-source off` 可跳过
- [x] **测试**：`backup create` 不错误触发 `.xunbak quick verify`
- [x] 实现 `src/backup_export/verify.rs`

### 7.2 postflight

- [x] **测试**：ZIP 导出后用 `zip` crate reopen 校验 central directory
- [x] **测试**：7z 导出后用内部 reader/reopen smoke test
- [x] **测试**：postflight 失败时保留输出但返回 `verify_failed`
- [x] **测试**：B3 fallback 下 `7z t` 失败归类为 `verify_failed`

### 7.3 preview / dry-run / list

- [x] **测试**：`--dry-run` 不创建任何输出
- [x] **测试**：`--list` 输出将被纳入备份/转换的条目
- [ ] **测试**：`overwrite=ask` 时在交互模式显示 preview 并确认
- [x] **测试**：`--patterns-from` 生效（convert 路径）

### 7.4 统一进度

- [x] **测试**：`ExportProgressEvent` 在 `verify_source/read/compress/write/verify_output` 阶段都可发出
- [x] **测试**：终端模式下进度输出节流
- [x] **测试**：JSON 模式下返回完整 summary
- [x] 实现 `progress.rs`

### 7.5 JSON 结果模型

- [x] **测试**：`--json` 返回 `action/format/source/destination/status/selected/skipped/bytes_in/bytes_out/overwrite_count/verify_source/verify_output/duration_ms/outputs`
- [x] **测试**：`--list --json` 返回 item 列表 + summary
- [x] **测试**：`--dry-run --json` 返回 summary 但不写文件
  已覆盖：create / convert 的主要 JSON 结果字段、`preflight_failed` / `verify_failed` 状态、`verify_source` / `verify_output` / `duration_ms` / `outputs` / `bytes_out`

---

## Phase 8：7z 产物单卷写出（B2.1）

### 8.1 `7z` 核心结构

- [x] **测试**：最小 `.7z` 归档可被 7-Zip 24.x 打开
- [x] **测试**：单文件归档可被 7-Zip / 内部 reader 解压并内容一致
- [x] **测试**：目录 + 多文件归档可被 7-Zip / 内部 reader 打开并保持层级
- [x] 实现纯 Rust `.7z` 写出模块（`sevenz_io.rs` / `sevenz_segmented.rs`）

### 8.2 最小方法集

- [x] **测试**：首版仅支持 `copy|lzma2`
- [x] **测试**：默认 `lzma2 + non-solid`
- [x] **测试**：显式 `--method copy` 生效
- [x] **测试**：`bzip2/ppmd` 在首版作为参数错误或 future flag
- [x] 实现 7z 方法选择与参数校验

### 8.3 7z 元数据与 sidecar

- [x] **测试**：7z 产物保留文件路径、mtime、ctime、windows attributes（若格式支持）
- [x] **测试**：sidecar 默认同样写入 `__xunyu__/export_manifest.json`

### 8.4 `backup create --format 7z`

- [x] **测试**：`backup create --format 7z` 成功
- [x] **测试**：non-solid 默认生效

---

## Phase 9：7z 分卷写出（B2.2）

### 9.1 `SegmentedWriter`

- [x] **测试**：逻辑连续流可切成 `.001/.002/.003`
- [x] **测试**：初始化预留 header、结束回写 header 的 seek 场景成立
- [x] **测试**：`logical_position()` 返回全局偏移
- [x] **测试**：`finish()` 返回卷路径列表并关闭全部句柄
- [x] 实现 `src/backup_export/sevenz_segmented.rs`

### 9.2 分卷命名与输出计划

- [x] **测试**：`project.7z -> project.7z.001/.002/...`
- [x] **测试**：临时分卷命名 `project.tmp.7z.001/...`
- [x] **测试**：批量 rename 成功
- [x] 实现 7z split output plan / volume naming

### 9.3 分卷兼容矩阵

- [x] **自动化测试**：本地 reopen / CRC 检查通过
- [x] **手工验证清单**：7-Zip 24.x 从 `.001` 打开成功
- [ ] **手工验证清单**：7-Zip 26.x 从 `.001` 打开成功
- [ ] **手工验证清单**：NanaZip 从 `.001` 打开成功
- [ ] **记录**：Windows Explorer 预期不支持

---

## Phase 10：7z 读取与恢复

### 10.1 7z reader

- [x] **测试**：`.7z` 产物条目可转成 `Read`
- [x] **测试**：`backup convert` 可从 `.7z` 读取条目
- [x] **测试**：`backup restore archive.7z --to out`
- [x] 实现 `7z` 读取/恢复路径

### 10.2 `backup convert 7z -> ...`

- [x] **测试**：`7z -> dir`
- [x] **测试**：`7z -> xunbak`
- [x] **测试**：`7z -> zip`

---

## Phase 11：CLI / 路由 / 兼容别名（消费层）

### 11.1 新命令树

- [x] **测试**：CLI 解析 `xun backup create -C src --format zip -o out.zip`
- [x] **测试**：CLI 解析 `xun backup restore archive.xunbak --to out`
- [x] **测试**：CLI 解析 `xun backup convert archive.xunbak --format zip -o out.zip`
- [x] 实现 `BackupCreateCmd / BackupRestoreCmd / BackupConvertCmd / BackupSubCommand`
- [x] 实现 `SubCommand::Backup` 统一入口

### 11.2 参数校验与错误提示

- [x] **测试**：`xun backup create --format zip --method lzma2` 返回明确参数错误
- [x] **测试**：`xun backup convert --format dir --split-size 2g` 返回明确参数错误
- [x] **测试**：错误提示包含 `Fix:` 风格修复建议

### 11.3 兼容别名

- [x] **测试**：旧命令 `xun backup ...`（无子命令）仍视作 `backup create`
- [x] **测试**：旧命令 `xun restore ...` 仍视作 `backup restore`
- [x] **测试**：顶层 `export` 仍保留给现有书签导出
- [x] 实现兼容别名和迁移提示

---

## Phase 12：方案 A — 7-Zip 插件 PoC（后置）

### 12.1 PoC 范围

- [ ] **测试**：DLL 可被目标 7-Zip 版本加载
- [ ] **测试**：单文件 `.xunbak` 可被 7-Zip 列出
- [ ] **测试**：单文件 `.xunbak` 可被 7-Zip 解压
- [ ] **限制**：PoC 不支持分卷、不支持写入、不支持富属性

### 12.2 正式版壳层迁移

- [ ] **测试**：C++ 薄壳 + Rust staticlib 的 C ABI 往返正确
- [ ] **测试**：版本矩阵 24.x / 26.x 通过
- [ ] **测试**：packed size 显示正确（取 `stored_size` 而非 `blob_len`）

---

## Phase 13：端到端与性能

### 13.1 端到端矩阵

- [x] **测试**：`create dir -> restore`
- [x] **测试**：`create xunbak -> restore`
- [x] **测试**：`create zip -> restore`
- [x] **测试**：`create 7z -> restore`
- [x] **测试**：`convert xunbak -> zip`
- [x] **测试**：`convert xunbak -> 7z`
- [x] **测试**：`convert zip -> dir`
- [x] **测试**：`convert 7z -> dir`

### 13.2 边界场景

- [x] **测试**：中文路径
- [x] **测试**：空格路径
- [x] **测试**：空文件
- [x] **测试**：深层目录
- [ ] **测试**：超大文件（ZIP64 / 7z 大文件）
- [x] **测试**：已压缩文件走 `stored`/`copy`

### 13.3 性能基线

- [x] **bench**：`backup create --format xunbak`
- [x] **bench**：`backup create --format zip`
- [x] **bench**：`backup create --format 7z`
- [x] **bench**：`backup convert xunbak -> zip`
- [x] **bench**：`backup convert xunbak -> 7z`
- [x] **bench**：`backup restore xunbak`
- [x] **bench**：`backup restore zip`
- [x] **bench**：`backup restore 7z`
- [x] 记录基线到 `logs/`

---

## 手工兼容验证清单

- [x] 7-Zip 24.x 打开 ZIP
- [x] 7-Zip 24.x 打开单卷 7z
- [x] 7-Zip 24.x 打开分卷 7z `.001`
- [ ] 7-Zip 26.x 打开 ZIP
- [ ] 7-Zip 26.x 打开单卷 7z
- [ ] 7-Zip 26.x 打开分卷 7z `.001`
- [ ] NanaZip 打开单卷 7z
- [ ] NanaZip 打开分卷 7z `.001`
- [ ] Windows Explorer 打开 ZIP
- [ ] Windows Explorer 不能打开 `.7z/.7z.001`（记录为预期限制）

---

## 依赖关系

```text
Phase 0（核心契约）
  └── Phase 1（源抽象）
       ├── Phase 2（流式读取）
       ├── Phase 3（输出计划）
       ├── Phase 4（xunbak 接入）
       └── Phase 5（dir）

Phase 2 + Phase 3
  └── Phase 6（zip）
  └── Phase 8（7z 单卷）

Phase 8
  └── Phase 9（7z 分卷）

Phase 8 + Phase 9
  └── Phase 10（7z reader / restore）

Phase 4 + Phase 5 + Phase 6 + Phase 10
  └── Phase 11（CLI / 路由 / 别名）

Phase 11
  └── Phase 13（E2E / perf）

Phase 12（7-Zip 插件）
  └── 在主线稳定后再做
```

---

## 建议执行顺序

1. Phase 0：核心契约
2. Phase 1：源抽象
3. Phase 2：流式读取
4. Phase 3：输出规划与原子写出
5. Phase 4：`.xunbak` 接入新模型
6. Phase 5：`dir`
7. Phase 6：ZIP
8. Phase 7：公共 pre/postflight / preview / progress / JSON
9. Phase 8：7z 单卷
10. Phase 9：7z 分卷
11. Phase 10：7z reader / restore
12. Phase 11：CLI / 路由 / 兼容别名
13. Phase 12：7-Zip 插件 PoC
14. Phase 13：E2E / perf

---

## 测试运行建议

```bash
# 先做编译级验证
cargo build --lib --features xunbak

# CLI/集成测试
cargo test --features xunbak

# 兼容格式专项（后续拆分）
cargo test --test test_xunbak_export --features xunbak

# 性能基线
cargo bench
```
