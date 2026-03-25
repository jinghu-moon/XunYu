# Backup 方法兼容矩阵

> 记录日期：2026-03-25
> 测试环境：Windows，`7-Zip 24.09 (x64)`，XunYu 当前工作树

## 结论

当前 `Phase 10` 的方法级兼容性可以分成三类：

1. 已验证稳定：
   - ZIP：`bzip2`
   - ZIP：`zstd`
   - ZIP：`ppmd`
   - 7z：`bzip2`
   - 7z：`deflate`
   - 7z：`ppmd`
2. 受解压端 codec 支持影响：
   - 7z：`zstd`
3. 当前仍有兼容边界：
   - 上游 `zip` crate `2.4.2` 仍不支持 `ppmd` 解压

## 解压端能力

本机 `7z i` 结果显示：

1. stock `7-Zip 24.09` 内建 `PPMD / BZIP2 / DEFLATE`
2. 当前安装未报告 `7z ZSTD` codec
3. 但在隔离临时 7-Zip 副本中注入 `7-Zip-zstd` release `v25.01-v1.5.7-R4` 的 `zstd.dll` 后，`7z --method zstd` 已实测可解

## 矩阵

| 格式 | 方法 | XunYu 写出 | `zip` crate reopen | stock `7-Zip 24.09` `7z t` | 备注 |
|---|---|---:|---:|---:|---|
| ZIP | `stored` | Yes | Yes | Yes | 既有能力，回归通过 |
| ZIP | `deflated` | Yes | Yes | Yes | 既有能力，回归通过 |
| ZIP | `bzip2` | Yes | Yes | Yes | 2026-03-25 实测 `Everything is Ok` |
| ZIP | `zstd` | Yes | Yes | Yes | 2026-03-25 实测 `Everything is Ok` |
| ZIP | `ppmd` | Yes | No* | Yes | 2026-03-25 已补纯 Rust 手写 writer + XunYu manual parser；stock `7-Zip 24.09` `7z t` 通过；单条目 `4 GiB + 1 MiB` Zip64 已端到端验证 |
| 7z | `copy` | Yes | N/A | Yes | 既有能力 |
| 7z | `lzma2` | Yes | N/A | Yes | 既有能力 |
| 7z | `bzip2` | Yes | N/A | Yes | 2026-03-25 实测 `Everything is Ok` |
| 7z | `deflate` | Yes | N/A | Yes | 2026-03-25 实测 `Everything is Ok` |
| 7z | `ppmd` | Yes | N/A | Yes | 2026-03-25 已修复纯 Rust `PPMD` double-flush，stock `7-Zip 24.09` `7z t` 通过 |
| 7z | `zstd` | Yes | N/A | No* / Yes** | stock `7-Zip 24.09` 默认报 `Unsupported Method`；隔离临时副本 + `7-Zip-zstd` `zstd.dll` 实测 `Everything is Ok` |

\* 当前上游 `zip` crate `2.4.2` 本身不带 `ppmd` 解压；XunYu 通过自定义 parser/decoder 读回。

\* `7z zstd` 的失败不是方法号缺失，而是当前 stock 7-Zip 安装未报告 `ZSTD` codec。

\** 正向验证方式：复制 `C:/A_Softwares/7-Zip` 到临时目录，解出 `Codecs-x64.7z` 中的 `zstd.dll` 到临时 `Codecs` 目录，再用临时 `7z.exe t` 验证 XunYu 的 `zstd` 产物。

## 当前动作

已在代码中补上：

1. `xun xunbak plugin doctor` 输出 `7z ZSTD Codec: supported | not-detected | unknown`
2. `backup convert` 的 7z 输出校验失败时，针对 `zstd` 给出“需要外部 codec”提示
3. 已修复 `PPMD` 在 `sevenz-rust2` 中被 `flush()` + `finish()` 双重收尾的问题
4. 已补 `ZIP ppmd` 的纯 Rust writer、manual parser、reader/source fallback 与 stock 7-Zip reopen 验证
5. 已完成 `7z zstd` 在“支持外部 codec 的解压端”上的正向实测
6. 已完成 `ZIP ppmd` 单条目 Zip64 的真实端到端验证

## 后续优先级

1. 保持 `7z zstd` 走“能力探测 + 明确提示”路径，而不是假定所有 7-Zip 都支持
2. 把 `PPMD` double-flush 修复补回上游回归测试，避免未来再次退化
3. 继续评估是否要把 `zip crate` 侧的 `ppmd` 解压缺口完全剥离，统一由 XunYu 自己的 parser/decoder 承担
