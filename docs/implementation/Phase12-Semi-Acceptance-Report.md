# Phase 12 半自动验收报告

> 日期：2026-03-27  
> 范围：`docs/implementation/Backup-Optimization-Tasks.md` Phase 12  
> 说明：本报告采用“脚本 + CLI + 自动化测试”做半自动等价验收，不把它包装成真实人工点击 GUI 的结果。

---

## 1. 验收结论

当前 Phase 12 可半自动收口：

1. `.xunbak` 7-Zip 插件单文件 / 分卷 / `PPMD` 样本的打开、列表、提取均已通过 stock `7-Zip 24.09 / 26.00`
2. 传统 backup 的小改动 / rename-only / verify 分级行为已有稳定自动化证据
3. 全量回归已通过，当前剩余更适合作为“发布后 smoke”而不是阻塞项

---

## 2. 7-Zip 插件

### 2.1 半自动等价验收项

| 原验收项 | 半自动验证方式 | 结果 |
| --- | --- | --- |
| 单文件 `.xunbak` 可直接在 7-Zip 中打开 | `7z l` / `7z l -slt` | Pass |
| 分卷 `.xunbak.001` 可直接在 7-Zip 中打开 | `7z l` / `7z l -slt` | Pass |
| 单文件提取内容正确 | `7z x` + 树哈希比对 | Pass |
| 全量提取内容正确 | `7z x` + 树哈希比对 | Pass |

### 2.2 使用的脚本

```powershell
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild
```

### 2.3 当前实测环境

1. `7-Zip 24.09 (x64)`：已通过
2. `7-Zip 26.00 (x64)`：已通过
3. 样本覆盖：
   - 单文件 `.xunbak`
   - 分卷 `.xunbak.001`
   - `PPMD` 样本 `.xunbak`
   - 中文路径 `nested/深层.txt`

参考文档：

1. [Xunbak-7z-Plugin-Release-Matrix.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7z-Plugin-Release-Matrix.md)
2. [Xunbak-7z-Plugin-Release-Guide.md](/D:/100_Projects/110_Daily/XunYu/docs/implementation/Xunbak-7z-Plugin-Release-Guide.md)

---

## 3. 传统 backup

### 3.1 小改动场景下 cache / hardlink 复用

半自动证据：

1. `cargo test --test module_backup_restore --features xunbak -- --test-threads=1`
2. 关键用例：
   - `backup_full_reuses_unchanged_files_via_hardlink`
   - `backup_full_reuses_renamed_file_via_hash_hardlink`
   - `backup_full_reuses_added_duplicate_file_via_hash_hardlink`
   - `backup_human_report_includes_enhanced_stats_fields`
   - `backup_json_includes_enhanced_stats_fields_for_ok_dry_run_and_skipped`

结论：

1. 小改动场景下复用逻辑正常
2. 报告字段已能反映 `reused_bytes / cache_hit_ratio / rename_only_count`

### 3.2 rename-only 场景

半自动证据：

1. `backup_rename_only_reports_hash_cache_hit_when_file_id_matches`
2. `logs/backup_file_id_baseline_20260325_122439.md`

结论：

1. rename-only 行为符合预期
2. `file_id on` 时，rename-only 场景明显快于 `file_id off`

### 3.3 verify 分级模式

半自动证据：

1. `cargo test --test test_xunbak --features xunbak -- --test-threads=1`
2. 关键用例：
   - `cli_verify_full_and_paranoid_levels_succeed`
   - `cli_verify_manifest_only_and_existence_only_levels_succeed`
   - `quick_verify_passes_for_valid_container`
   - `full_verify_passes_when_all_blobs_are_valid`
   - `paranoid_verify_passes_for_valid_container`

结论：

1. `quick / full / manifest-only / existence-only / paranoid` 均有稳定自动化覆盖
2. 当前行为与设计一致

---

## 4. 回归摘要

本轮验收前后已经通过：

1. `cargo check --tests --benches --features xunbak`
2. `cargo test --lib --features xunbak`
3. `cargo test --test test_xunbak --features xunbak -- --test-threads=1`
4. `cargo test --test module_backup_restore --features xunbak -- --test-threads=1`
5. `./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild`
6. `./scripts/test_xunbak_7z_plugin_system.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild`

---

## 5. 剩余说明

1. 本报告不声称替代真人肉眼检查 7-Zip GUI 列头排版
2. 但对“打开 / 列表 / 提取 / 内容正确 / 兼容版本”已经有足够的半自动等价证据
