# Phase 10 Vendor 维护说明

> 日期：2026-03-25
> 范围：`ZIP / 7z` 方法集扩展涉及的本地 Rust crate 修复

## 当前治理方式

当前不再把本地 fork 直接写成主依赖，而是拆成两层：

1. [Cargo.toml](/D:/100_Projects/110_Daily/XunYu/Cargo.toml) 中保留 crates.io 版本声明
2. 通过 `[patch.crates-io]` 指向本地 fork
3. 真实改动沉淀到 [patches/rust-crates](/D:/100_Projects/110_Daily/XunYu/patches/rust-crates) 下的独立 patch 文件

这样做的目的，是把“项目依赖什么版本”和“我们对该版本做了什么本地修复”明确拆开，避免继续维持隐式的路径依赖黑盒。

## 为什么继续保留 `refer/` 路径

当前实现文档中已经大量引用以下目录：

1. [refer/ppmd-rust-1.4.0](/D:/100_Projects/110_Daily/XunYu/refer/ppmd-rust-1.4.0)
2. [refer/sevenz-rust2-main](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main)

因此本阶段不移动目录，只调整治理方式。这样既不破坏历史文档引用，也能把 fork 收敛为可追溯 patch。

## 当前 patched fork 列表

### `ppmd-rust 1.4.0`

- 本地 fork：[refer/ppmd-rust-1.4.0](/D:/100_Projects/110_Daily/XunYu/refer/ppmd-rust-1.4.0)
- patch 文件：[ppmd-rust-1.4.0-large-input-overflow.patch](/D:/100_Projects/110_Daily/XunYu/patches/rust-crates/ppmd-rust-1.4.0-large-input-overflow.patch)
- 修复文件：[ppmd8.rs](/D:/100_Projects/110_Daily/XunYu/refer/ppmd-rust-1.4.0/src/internal/ppmd8.rs)

修复原因：

1. `ZIP ppmd` 的真实单条目 Zip64 验证暴露了大输入问题
2. `Ppmd8Encoder` 在超大输入上会命中 `run_length += ...` 的整数溢出
3. C 原版实现依赖自然溢出语义，而 Rust debug 构建会在该位置 panic

修复内容：

1. 将 `run_length += prev_success` 改为 `wrapping_add`
2. 将 `run_length += 1` 改为 `wrapping_add(1)`

这不是“掩盖 bug”，而是显式对齐原始 C 实现的整数行为。

### `sevenz-rust2 0.20.2`

- 本地 fork：[refer/sevenz-rust2-main](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main)
- patch 文件：[sevenz-rust2-0.20.2-ppmd-flush.patch](/D:/100_Projects/110_Daily/XunYu/patches/rust-crates/sevenz-rust2-0.20.2-ppmd-flush.patch)
- 修复文件：[encoder.rs](/D:/100_Projects/110_Daily/XunYu/refer/sevenz-rust2-main/src/encoder.rs)

修复原因：

1. `7z ppmd` 的纯 Rust 写出曾出现 `Data Error`
2. 根因不是算法本体，而是 `sevenz-rust2` 在 `Encoder::Ppmd.flush()` 上直接触发了 encoder 收尾
3. 后续 `finish(false)` 又再次收尾，导致 range encoder 双重结束并多写出尾零

修复内容：

1. `Encoder::Ppmd.flush()` 不再直接结束 PPMD encoder
2. 只 flush 内层 writer
3. 真正 finish 仍由空写 `write(&[])` 路径触发

## fork 刷新与回放方式

核心要求：

1. fork 必须能由“上游源码 + patch 文件”重建
2. 不允许继续在 `refer/` 目录做无补丁记录的散改
3. 若升级上游版本，必须重新检查 patch 是否仍然必要

标准流程：

1. 用对应上游版本源码覆盖本地 `refer/ppmd-rust-1.4.0` 或 `refer/sevenz-rust2-main`
2. 进入对应 crate 根目录执行补丁：
   - `git apply "../../patches/rust-crates/ppmd-rust-1.4.0-large-input-overflow.patch"`
   - `git apply "../../patches/rust-crates/sevenz-rust2-0.20.2-ppmd-flush.patch"`
3. 回到仓库根目录执行验证：
   - `cargo check --tests --features xunbak`
   - `cargo test --test module_backup_restore --features xunbak -- --test-threads=1`
   - `cargo test --test test_xunbak --features xunbak -- --test-threads=1`

## 验证要求

若调整这两处 fork 或升级对应版本，至少重跑：

1. `cargo check --tests --features xunbak`
2. `cargo test --test module_backup_restore --features xunbak -- --test-threads=1`
3. `cargo test --test test_xunbak --features xunbak -- --test-threads=1`
4. `ZIP ppmd` 单条目 Zip64 端到端验证

## 当前结论

截至 2026-03-25：

1. 这两处 patch 仍然是必要的
2. patch 范围已经收敛到最小行为修复
3. 当前治理方式已经比“主依赖直接指向本地路径”更干净，也更利于后续回收本地 fork
