# Phase 10 Vendor 维护说明

> 日期：2026-03-25
> 范围：`ZIP / 7z` 方法集扩展相关的本地 vendor 修复

## 当前 vendor 依赖

### `ppmd-rust`

- 位置：[refer/ppmd-rust-1.4.0](/D:/100_Projects/110_Daily/XunYu/refer/ppmd-rust-1.4.0)
- 主工程依赖声明：[Cargo.toml](/D:/100_Projects/110_Daily/XunYu/Cargo.toml)

## 为什么要 vendor

`ZIP ppmd` 的真实单条目 Zip64 验证暴露了一个大输入问题：

1. `Ppmd8Encoder` 在超大输入上会命中 `run_length += ...` 的整数溢出
2. C 原版实现依赖自然溢出语义
3. Rust debug 构建会在该位置 panic

因此当前必须本地维护最小修复，才能让：

1. 单条目 `> 4 GiB` 的 `ZIP ppmd` 创建成功
2. 后续 `cargo test` / debug 构建稳定通过

## 当前本地修复

修复文件：

- [ppmd8.rs](/D:/100_Projects/110_Daily/XunYu/refer/ppmd-rust-1.4.0/src/internal/ppmd8.rs)

修复内容：

1. 将 `run_length += prev_success` 改为 `wrapping_add`
2. 将 `run_length += 1` 改为 `wrapping_add(1)`

这样做的目的不是“掩盖 bug”，而是对齐原始 C 实现的整数行为。

## 另一处本地修复

`7z ppmd` 的纯 Rust写出曾出现 `Data Error`。根因不是算法本体，而是 `sevenz-rust2` 在 PPMD encoder 上：

1. 先 `flush()`
2. 再 `finish()`

导致 range encoder 被双重收尾，多写出 5 个尾零。

修复文件：

- [encoder.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder.rs)

修复内容：

1. `Encoder::Ppmd.flush()` 不再直接触发 range encoder 收尾
2. 只在空写 `write(&[])` 触发真正 finish

## 后续维护原则

1. 仅保留最小必要 patch，不在 vendor 中做无关重构
2. 所有 vendor patch 都要有对应回归测试
3. 如果上游接受修复，应优先回收本地 patch
4. 若升级 vendor 版本，必须先重跑：
   - `module_backup_restore`
   - `test_xunbak`
   - `ZIP ppmd` 单条目 Zip64 端到端验证

## 当前结论

截至 2026-03-25：

1. vendor patch 是必要的
2. patch 范围已经控制在最小实现修复
3. `Phase 10` 的主线功能已经依赖这些 patch 稳定工作
