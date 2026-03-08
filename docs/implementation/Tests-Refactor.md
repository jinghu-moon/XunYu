# XunYu 测试重构与全自动验收方案

> 目标：在 **无需人工干预** 的前提下，完成可重复、可扩展、可并行的测试体系重构，并覆盖 `lock/protect/crypt` 验收项。  
> 范围：`tests/` 工作区拆分、E2E 自动化、性能门槛、CI 执行矩阵。

---

## 1. 外部参考（Web）

1. Rust 官方测试组织建议（单元测试 vs 集成测试）  
   `https://doc.rust-lang.org/book/ch11-03-test-organization.html`
2. `assert_cmd`：CLI 集成测试最佳实践（命令执行、退出码、stdout/stderr 断言）  
   `https://github.com/assert-rs/assert_cmd`
3. `cargo-nextest`：并行执行、重试、过滤与 CI 友好输出  
   `https://github.com/nextest-rs/nextest`
4. `insta`：快照测试（稳定输出结构比对）  
   `https://github.com/mitsuhiko/insta`
5. CLI Guidelines（人机可读 + 机器可读并存）  
   `https://clig.dev`

---

## 2. 现状问题

1. `tests/integration_tests.rs` 体量过大，职责混杂，定位失败成本高。  
2. E2E（特别是 `lock/protect`）自动化覆盖不足，仍有人工验收依赖。  
3. 部分测试基建耦合在单文件内，复用困难。  
4. 性能门槛与功能验收未形成统一执行入口。  
5. 历史配置中存在失效测试目标时，会阻断全量测试链路。

---

## 3. 重构原则

1. **零人工干预**：默认非交互，测试中不依赖 TTY 弹窗输入。  
2. **分层清晰**：基础行为 / 网络与端口 / E2E / 性能测试分文件治理。  
3. **结果可诊断**：失败信息必须包含命令、退出码、stdout、stderr。  
4. **稳定优先**：避免依赖不确定环境；必须依赖时给出隔离策略。  
5. **可并行执行**：支持 `cargo nextest` 并行与过滤运行。  

补充（2026-02）：
- Dashboard Web UI 不引入 DOM/Snapshot 自动化测试，采用手动验证清单（见 `Test-Env.md` 的 Dashboard 章节）。

---

## 4. 目录拆分方案

```text
tests/
  common/
    mod.rs                  # TestEnv、run_ok/run_err、共享工具
  test_basic.rs             # 书签/tree/bak 等基础行为
  test_proxy_net.rs         # proxy/ports/kill 网络与端口
  test_lock_e2e.rs          # lock/rm --unlock 端到端
  test_protect_e2e.rs       # protect 拦截/放行 + audit 验证
  test_dry_run_format.rs    # 危险命令 dry-run 与格式稳定性
  test_performance.rs       # lock who + 批量 rm 性能门槛
```

---

## 5. 共享测试基座设计（`tests/common/mod.rs`）

1. `TestEnv`  
   - 每个测试独立根目录（建议 `std::env::temp_dir()` 下随机目录）。  
   - 注入 `USERPROFILE/HOME` 到测试目录，隔离用户环境。  
   - 默认注入 `XUN_NON_INTERACTIVE=1`，禁用交互依赖。  
2. 进程执行辅助  
   - `run_ok`：断言成功并打印完整失败上下文。  
   - `run_err`：断言失败并打印完整失败上下文。  
   - `run_ok_status`：性能压测场景减少输出开销。  
3. 文件与性能辅助  
   - 混合文件生成器（短路径/长路径/Unicode 名称）。  
   - 时间阈值断言与环境变量覆盖（便于 CI 调优）。

---

## 6. E2E 自动化策略

### 6.1 lock E2E（`test_lock_e2e.rs`）

1. 使用子进程持有文件锁（不要由测试主进程自己持锁，避免自杀）。  
2. `xun lock who --format json` 验证锁持有 PID 命中。  
3. `xun rm --unlock --force-kill --yes` 自动解锁删除并验证目标消失。  
4. 非交互路径验证：缺少必要参数时应返回明确退出码与错误信息。

### 6.2 protect E2E（`test_protect_e2e.rs`）

1. `protect set` 建立规则后，普通 `rm` 必须拦截。  
2. `rm --force --reason "...\" --yes` 放行删除成功。  
3. 解析 `audit.jsonl`，验证动作、结果、reason 持久化正确。

### 6.3 dry-run 与格式（`test_dry_run_format.rs`）

1. `rm/mv/ren --dry-run` 后目标状态不变。  
2. `--format json` 断言字段存在且类型稳定。  
3. 对危险命令统一验证“只演练不落地”。

---

## 7. 性能验收策略（`test_performance.rs`）

默认门槛（可被环境变量覆盖）：

1. `lock who` 单文件探测 `< 200ms`。  
2. 无占用的 1k 文件批量删除 `< 5s`。

建议：

1. 将绝对门槛参数化：`XUN_TEST_LOCK_WHO_MAX_MS`、`XUN_TEST_RM_1K_MAX_MS`。  
2. 在 CI 上保留稳定默认值，在性能专用机器执行更严格门槛。  
3. 对超时失败输出“数据规模 + 实际耗时 + 机器信息”。

---

## 8. 依赖建议（仅测试域）

`[dev-dependencies]` 可选：

1. `assert_cmd`：CLI 进程断言更简洁。  
2. `predicates`：stdout/stderr 断言表达力更强。  
3. `tempfile`：临时目录管理更安全。  
4. `serial_test`：需要串行化的系统级测试可控执行。  
5. `insta`：稳定输出快照（可用于 JSON/table 回归保护）。

说明：以上仅用于测试，不影响 release 二进制打包内容。

---

## 9. CI 执行矩阵（全自动）

### 9.1 基线

```bash
cargo check
cargo check --all-features
cargo test
cargo test --all-features
```

### 9.2 推荐（并行 + 稳定）

```bash
cargo nextest run
cargo nextest run --all-features
```

### 9.3 分层回归（按文件过滤）

```bash
cargo test --test test_lock_e2e --features lock
cargo test --test test_protect_e2e --features lock,protect
cargo test --test test_performance --features lock
```

---

## 10. 分阶段落地计划

1. Phase A：抽出 `tests/common/mod.rs`，保留旧测试不动，先建立复用基座。  
2. Phase B：新增 `test_lock_e2e.rs`、`test_protect_e2e.rs`、`test_dry_run_format.rs`。  
3. Phase C：迁移基础与网络测试到 `test_basic.rs`、`test_proxy_net.rs`。  
4. Phase D：迁移性能测试到 `test_performance.rs` 并参数化门槛。  
5. Phase E：删除旧大文件，收敛到新结构并更新文档与 CI。

---

## 11. 验收标准（Definition of Done）

1. `cargo test --all-features` 无人工输入、可一次跑完。  
2. `lock/protect/dry-run` 核心验收项均有自动化测试。  
3. 测试失败时可直接定位问题命令与上下文。  
4. 测试目录结构清晰，新增用例无需改动历史大文件。  
5. CI 可并行执行，回归时长可控。
