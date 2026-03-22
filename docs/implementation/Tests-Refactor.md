# XunYu 测试体系重构

> 状态说明（2026-03-22）：本文件描述当前已落地的测试架构，不再对应历史 `tests/test_*.rs` 平铺结构。

## 1. 目标

本轮测试体系重构有 4 个明确目标：

1. 将测试分为 `通用（general）`、`模块（modules）`、`特殊（special）` 三类。
2. 将共享能力统一收敛到 `tests/support/`，避免测试入口之间互相拖带。
3. 关闭 Cargo 的自动测试发现，改用显式 `[[test]]` 目标矩阵。
4. 让 feature 测试、性能测试、白盒测试都有独立入口，便于精准回归。

## 2. 目录结构

```text
tests/
  support/
    mod.rs
  general/
    cli_core.rs
    cli_core_cases/
    ctx.rs
    delete.rs
    dry_run.rs
    find.rs
    naming.rs
  modules/
    backup_restore.rs
    backup_restore_cases/
    acl.rs
    acl_cases/
    alias.rs
    alias_cases/
    batch_rename.rs
    crypt_e2e.rs
    dashboard.rs
    diff.rs
    filevault_formats.rs
    filevault_v13.rs
    lock_e2e.rs
    protect_e2e.rs
    proxy.rs
    redirect_e2e.rs
    redirect_tools.rs
    redirect_undo.rs
    redirect_watch.rs
  special/
    acl_stress.rs
    alias_perf.rs
    filevault_performance.rs
    path_guard_bench.rs
    path_guard_integration.rs
    path_guard_trace.rs
    path_guard_unit.rs
    performance.rs
    redirect_watch_core.rs
```

补充说明：

1. 原始用例文件已经物理迁到 `general/cli_core_cases/`、`modules/backup_restore_cases/`、`modules/acl_cases/`、`modules/alias_cases/`。
2. `general/modules/special` 目录下的是新的 Cargo test target 入口，不再承载大量具体断言实现。
3. `tests/support/mod.rs` 是唯一共享测试基座，负责 `TestEnv`、命令执行、性能辅助、Windows 测量工具等。

## 3. 三类测试的职责边界

### 3.1 通用（general）

用于验证跨模块、默认能力、CLI 基本约定。

典型内容：

1. 书签、列表、树、标签、导入导出。
2. `ctx`、`find`、`delete`、命名命令。
3. `dry-run` 这类跨命令的一致性行为。

### 3.2 模块（modules）

用于验证单个业务模块的功能正确性。

典型内容：

1. `backup/restore`
2. `acl`
3. `alias`
4. `redirect`
5. `crypt/filevault`
6. `proxy`
7. `dashboard`
8. `batch_rename`
9. `diff`
10. `lock/protect`

原则：

1. 模块测试只覆盖模块自身能力。
2. 某个模块需要 feature 时，用 `required-features` 显式声明，不再依赖“跑到一半才因 feature 不匹配失败”。

### 3.3 特殊（special）

用于承载不适合放进常规模块回归链的测试。

典型内容：

1. 性能测试
2. 压力测试
3. trace / bench / 内部白盒测试
4. 资源占用观测

原则：

1. 特殊测试必须有独立目标，不拖慢默认回归。
2. `#[ignore]` 只用于需要显式触发的性能类场景，不再用来掩盖结构混乱。

## 4. Cargo 策略

当前 `Cargo.toml` 已启用：

1. `autotests = false`
2. 所有集成测试通过 `[[test]]` 显式注册
3. feature 模块使用 `required-features`

这样做的直接收益：

1. 测试入口是可枚举、可治理的。
2. `alias`、`redirect`、`crypt` 这类 feature 模块不会误伤默认测试链。
3. 后续新增测试时，只要决定属于哪一类，再新增一个明确 target 即可。

## 5. 推荐运行方式

### 5.1 默认回归

```bash
cargo test --test general_cli_core
cargo test --test general_ctx
cargo test --test module_backup_restore
cargo test --test module_acl
```

### 5.2 feature 模块回归

```bash
cargo test --test module_alias --features alias
cargo test --test module_redirect_e2e --features redirect
cargo test --test module_protect_e2e --features lock,protect
cargo test --test module_filevault_v13 --features crypt
```

### 5.3 特殊测试

```bash
cargo test --test special_performance -- --ignored --nocapture
cargo test --test special_alias_perf --features alias -- --nocapture
cargo test --test special_filevault_performance --features crypt -- --ignored --nocapture
```

### 5.4 backup / restore 精准回归

```bash
pwsh -File "tools/test-safe.ps1" -Preset backup
pwsh -File "tools/test-safe.ps1" -Preset restore
```

### 5.5 三类测试批量运行

```bash
pwsh -File "tools/test-suite.ps1" -Scope general
pwsh -File "tools/test-suite.ps1" -Scope modules
pwsh -File "tools/test-suite.ps1" -Scope special
pwsh -File "tools/test-suite.ps1" -Scope all -KeepGoing
```

说明：

1. 默认 `Runner=auto`，若本机安装了 `cargo-nextest` 会优先使用，否则回退到 `cargo test`。
2. `special` 默认会带上 `ignored` 测试一起执行。
3. `module_alias` / `special_alias_perf` 会先自动构建 `alias-shim`。

### 5.6 nextest 配置

仓库已提供 `.config/nextest.toml`，主要策略如下：

1. `profile.default` 默认排除 `special_*` 测试。
2. `profile.ci` 默认跑全量。
3. `lock/protect/redirect` 这类 Windows 资源敏感测试放入串行测试组。
4. `special` 中的重型测试放入独占测试组。

若本机尚未安装：

```bash
cargo install cargo-nextest --locked
```

## 6. 支撑脚本

### 6.1 `tools/test-safe.ps1`

用于精准执行测试，并在 `LNK1104` 时自动尝试排查锁定者。

当前预设：

1. `backup` -> `module_backup_restore`
2. `restore` -> `module_backup_restore`
3. `alias` -> `module_alias`，并自动构建 `alias-shim`

### 6.2 `tools/find-locker.ps1`

用于定位测试产物被占用的进程。

当前已兼容新测试目标命名：

1. `general_*`
2. `module_*`
3. `special_*`

### 6.3 `tools/test-suite.ps1`

用于按测试类别批量执行目标：

1. `general`
2. `modules`
3. `special`
4. `default`（`general + modules`）
5. `all`

可选执行器：

1. `cargo`
2. `nextest`
3. `auto`

## 7. 后续建议

下一阶段可以继续做 3 件事：

1. 给 `general/modules/special` 增加按类型批量运行脚本。
2. 引入 `cargo nextest` 配置文件，固化 CI 与本地一致的并行策略。
3. 针对高重复测试逐步引入 `rstest` 或快照测试，而不是在本轮同时大面积改断言风格。
