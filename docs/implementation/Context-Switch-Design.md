# XunYu 项目上下文切换方案（可落地版）

目标：将**路径 + 代理 + 环境变量 + 标签**打包为 profile，通过一条命令完成项目场景切换，显著降低上下文切换成本。

补充（2026-02）：Dashboard Web UI 迭代不改变 `ctx` 命令与配置模型。

---

## 1. 目标与范围

**目标**
- 一键切换：`xun ctx use <name>` 同时完成路径/代理/标签/环境变量设置。
- 可逆：`xun ctx off` 恢复到切换前状态（当前 shell 会话内）。
- 低侵入：不破坏现有 `z`/`pon`/`poff` 工作流。
- 可发现：`ctx list/show` 清晰展示 profile。

**非目标**
- 不管理 IDE 配置（`.vscode/settings.json` 等）。
- 不自动检测项目类型（Rust/Node/Python）。
- 不引入守护进程或后台服务。

---

## 2. 现状与问题

**现状**
- 切换项目需要多步：`cd` → `pon` → `set ENV=...` → 记住标签。
- 代理、环境变量、标签分散在不同子系统，无统一入口。
- 已有魔法行协议：`__CD__`、`__ENV_SET__`、`__ENV_UNSET__`。
  - PowerShell wrapper 已逐行解析；
  - Bash/Zsh 仍是单行匹配，需要在阶段 1 升级为逐行扫描。

**痛点**
- 每次切换项目重复 3-5 条命令。
- 容易忘记设置代理/环境变量。
- 难以回到“切换前的状态”。

---

## 3. CLI 设计（argh 友好）

### 3.1 命令族

> `argh` 不支持同层“子命令 + 位置参数”混用，激活命令统一用 `use`。

```
xun ctx set <name> [options]    # 定义/更新 profile
xun ctx use <name>              # 激活 profile
xun ctx off                     # 停用当前 profile（还原）
xun ctx list                    # 列出所有 profile
xun ctx show [name]             # 查看详情（缺省显示当前激活）
xun ctx del <name>              # 删除 profile
xun ctx rename <old> <new>      # 重命名
```

### 3.2 `ctx set` 参数

```
xun ctx set <name>
    --path <dir>                # 工作目录（新建时必填）
    --proxy <url|off|keep>      # 代理 URL / 关闭 / 保持不变
    --noproxy <hosts>           # NO_PROXY（proxy=url 时生效）
    --tag <t1,t2|->             # 默认标签（逗号分隔；"-" 表示清空）
    --env <KEY=VALUE>           # 环境变量（可重复）[阶段 2]
    --env-file <path>           # 从 .env 文件导入 [阶段 2]
```

更新语义（必须明确）：
- 未传的字段**保留原值**（merge 语义）。
- 新建 profile 且未传 `--proxy` 时，默认等价于 `--proxy keep`。
- `--proxy keep` 表示“保持当前 profile 的代理设置不变”。
- `--proxy off` 表示“显式关闭代理”。
- `--tag -` 清空标签；未传 `--tag` 不修改原标签。

示例：
```
xun ctx set work --path D:\Repo\MyProj --proxy http://127.0.0.1:7890 --tag work,rust
xun ctx set home --path D:\Personal --proxy off --tag personal
```

### 3.3 `ctx use <name>` 激活行为（输出序列）

示例输出（魔法行）：
```
__CD__:D:\Repo\MyProj
__ENV_SET__:HTTP_PROXY=http://127.0.0.1:7890
__ENV_SET__:HTTPS_PROXY=http://127.0.0.1:7890
__ENV_SET__:NO_PROXY=localhost,127.0.0.1
__ENV_SET__:XUN_DEFAULT_TAG=work,rust
__ENV_SET__:XUN_CTX=work
```

说明：
- **代理**通过复用 `pon/poff` 逻辑输出 `__ENV_SET__/__ENV_UNSET__`，不新增新协议。
- **标签/上下文**通过 `__ENV_SET__` 设置 `XUN_DEFAULT_TAG` 与 `XUN_CTX`。
- **环境变量**（阶段 2）同样通过 `__ENV_SET__` 输出。

### 3.4 `ctx off` 还原行为

```
__ENV_UNSET__:XUN_DEFAULT_TAG
__ENV_UNSET__:XUN_CTX
__ENV_SET__/__ENV_UNSET__ (来自 proxy 还原)
__CD__:<previous_dir>
```

---

## 4. 魔法行协议（复用，无新增）

仅复用已存在的三类：
| 魔法行 | 含义 |
| --- | --- |
| `__CD__:<path>` | shell 执行 `cd` |
| `__ENV_SET__:<K>=<V>` | shell 设置环境变量 |
| `__ENV_UNSET__:<K>` | shell 删除环境变量 |

**实现要求**：shell wrapper 必须逐行解析输出。

---

## 5. 数据模型

### 5.1 Profile 存储（全局）

文件：`~/.xun.ctx.json`（建议支持 `XUN_CTX_FILE` 覆盖）

```json
{
  "profiles": {
    "work": {
      "path": "D:\\Repo\\MyProj",
      "proxy": { "mode": "set", "url": "http://127.0.0.1:7890", "noproxy": "localhost,127.0.0.1" },
      "tags": ["work", "rust"],
      "env": { "RUST_LOG": "info" }
    },
    "home": {
      "path": "D:\\Personal",
      "proxy": { "mode": "off" },
      "tags": ["personal"],
      "env": {}
    }
  }
}
```

### 5.2 会话态存储（每个 shell 独立）

为避免多终端互相覆盖，**激活状态与恢复信息放到会话态文件**：

- 环境变量：`XUN_CTX_STATE` 指向会话文件路径（**由 shell wrapper 生成并设置**）
- 推荐路径：`%TEMP%\xun-ctx-<shell_pid>.json`（使用 shell PID，确保同一会话复用）

示例（由 wrapper 负责设置）：

```bash
ctx() {
  if [ -z "$XUN_CTX_STATE" ]; then
    export XUN_CTX_STATE="$TEMP/xun-ctx-$$.json"
  fi
  # 必须调用“解析魔法行”的 wrapper：
  # - xun init 输出里是 `x`
  # - xun.sh 里是 `_xun_wrapper`
  if declare -F _xun_wrapper >/dev/null; then
    _xun_wrapper ctx "$@"
  else
    x ctx "$@"
  fi
}
```

```powershell
function ctx {
  if (-not $env:XUN_CTX_STATE) {
    $env:XUN_CTX_STATE = Join-Path $env:TEMP ("xun-ctx-{0}.json" -f $PID)
  }
  # 必须调用“解析魔法行”的 wrapper（xun init 输出里是 `x`）
  x ctx @args
}
```

示例：
```json
{
  "active": "work",
  "previous_dir": "D:\\Repo\\Other",
  "previous_env": {
    "XUN_DEFAULT_TAG": "ops",
    "XUN_CTX": null
  },
  "previous_proxy": {
    "url": "http://127.0.0.1:7890",
    "noproxy": "localhost,127.0.0.1"
  },
  "proxy_changed": true
}
```

说明：
- `previous_env` 保存“被 ctx 影响的键”的旧值（`null` 表示原本不存在）。
- `previous_proxy` 优先来自环境变量（HTTP_PROXY/NO_PROXY），回退 `.xun.proxy.json`。
- `proxy_changed` 用于 `ctx off` 是否需要恢复代理。

---

## 6. 执行流程（详细）

### 6.1 `xun ctx use <name>`

1. 读取 `~/.xun.ctx.json`，查找 profile。
2. 校验 `path` 存在；不存在则报错并提示修复。
3. 读取会话文件路径：
   - 若 `XUN_CTX_STATE` 未设置，报错并提示先加载 `xun init` 的 wrapper（或手动设置该变量）。
4. 记录旧状态（写入会话文件）：
   - `previous_dir = current_dir()`
   - `previous_env`：对 `XUN_DEFAULT_TAG`、`XUN_CTX` 与 profile.env 中的 key 保存旧值
   - `previous_proxy`：优先读取 `HTTP_PROXY/NO_PROXY` 环境变量；若不存在则回退 `.xun.proxy.json`
   - `proxy_changed`：profile.proxy 为 `set` 或 `off` 时置 true
5. 输出 `__CD__:<profile.path>`。
6. 处理代理：
   - `proxy.mode = set` → 复用 `pon` 逻辑（输出 env 魔法行 + 修改 git/npm/cargo）
   - `proxy.mode = off` → 复用 `poff` 逻辑
   - `proxy.mode = keep` → 不变
7. 输出 `__ENV_SET__:XUN_DEFAULT_TAG=<tags>`（若 tags 非空，否则输出 `__ENV_UNSET__`）。
8. 输出 `__ENV_SET__:XUN_CTX=<name>`。
9. （阶段 2）输出 `__ENV_SET__:<K>=<V>` 逐一设置环境变量。
10. 备注：A→B 切换时，会话文件会被覆盖；`ctx off` 只回到**最近一次** `ctx use` 前的目录与环境（不提供多层回退）。

### 6.2 `xun ctx off`

1. 读取 `XUN_CTX_STATE` 会话文件；若 **XUN_CTX_STATE 未设置或会话文件不存在** → 提示“无激活 profile”并退出 0。
2. 代理还原：
   - `proxy_changed=false` → 不变
   - `previous_proxy` 存在 → 调用 `proxy set` 逻辑恢复
   - 否则 → 调用 `proxy del`
3. 恢复 env：
   - 对 `previous_env` 每个键：有值 → `__ENV_SET__`；无值 → `__ENV_UNSET__`
4. 输出 `__CD__:<previous_dir>`（若存在且路径有效，**放在最后，避免影响后续相对路径操作**）。
   - 若路径不存在，跳过 `cd` 并在 stderr 提示。
5. 输出 `__ENV_UNSET__:XUN_CTX_STATE` 并删除会话文件。

---

## 7. Shell 集成

不新增协议，仅保证 wrapper **逐行解析**魔法行（Bash/Zsh 需升级）：

```bash
while IFS= read -r line; do
  case "$line" in
    __CD__:*)        cd "${line#__CD__:}" ;;
    __ENV_SET__:* )  export "${line#__ENV_SET__:}" ;;
    __ENV_UNSET__:* ) unset "${line#__ENV_UNSET__:}" ;;
    * ) printf '%s\n' "$line" ;;
  esac
done <<< "$out"
```

PowerShell 同理（`-split "`n"` 逐行处理）。

新增别名建议：`ctx()`（包装“解析魔法行”的 wrapper：`x` 或 `_xun_wrapper`）。

---

## 8. 补全集成

`xun __complete` 路由增加 `ctx`：

| 位置 | 候选源 | directive |
| --- | --- | --- |
| `ctx <TAB>` | 子命令（use/set/off/list/show/del/rename） | nofilecomp |
| `ctx use <TAB>` | profile 名 | nofilecomp |
| `ctx del <TAB>` | profile 名 | nofilecomp |
| `ctx show <TAB>` | profile 名 | nofilecomp |
| `ctx rename <TAB>` | profile 名 | nofilecomp |
| `ctx set <name> --path <TAB>` | 目录补全 | filterdirs |
| `ctx set <name> --tag <TAB>` | 已有标签列表 | nofilecomp |
| `ctx set <name> --proxy <TAB>` | `off` / `keep` | nofilecomp |

---

## 9. 与现有功能交互

### 9.1 `XUN_DEFAULT_TAG`

`list`/`z`/`recent` 在未显式传 `--tag` 时，若检测到 `XUN_DEFAULT_TAG` 则默认使用该标签。

### 9.2 代理

`ctx use/off` 复用 `pon/poff` 逻辑（包含 git/npm/cargo 配置与 env 设置）。  
为保持切换速度，默认**不做连通性检测**；如需检测，可手动执行 `xun proxy test`。  
还原时优先以**当前环境变量**为准（HTTP_PROXY/NO_PROXY），再回退到 `.xun.proxy.json`。

---

## 10. 实施阶段

### 阶段 1：路径 + 代理 + 标签

- [ ] `CtxStore`/`CtxProfile`/`CtxSession` 数据模型
- [ ] `ctx set` / `ctx del` / `ctx list` / `ctx show` / `ctx rename`
- [ ] `ctx use` / `ctx off`（会话文件 + 代理复用 + 魔法行输出）
- [ ] `XUN_DEFAULT_TAG` 对 `list/z/recent` 生效
- [ ] Bash/Zsh wrapper 升级为逐行扫描（多行魔法行输出）
- [ ] shell wrapper 新增 `ctx` 别名，并设置 `XUN_CTX_STATE`
- [ ] 补全集成（ctx 路由）

### 阶段 2：通用环境变量

- [ ] `--env KEY=VALUE`（可重复）
- [ ] `--env-file <path>` 导入 `.env`
- [ ] 激活时输出 `__ENV_SET__` 行
- [ ] 停用时按 `previous_env` 还原

---

## 11. 错误处理

| 场景 | 行为 |
| --- | --- |
| profile 不存在 | 报错 + 提示 `xun ctx list` |
| profile.path 不存在 | 报错 + 提示 `xun ctx set <name> --path <new>` |
| profile 名是保留字 | 报错 + 列出保留字 |
| `ctx off` 无激活 profile | 提示并退出 0 |
| `XUN_CTX_STATE` 未设置 | 报错 + 提示先加载 `xun init` 或手动设置 |
| `~/.xun.ctx.json` 损坏 | 视为空，不崩溃 |

保留字：`set`、`use`、`off`、`list`、`show`、`del`、`delete`、`rename`、`help`。

---

## 12. 性能约束

- `ctx use` < 100ms（文件读写 + 魔法行输出）。
- 不引入网络请求。

---

## 13. 测试要点

**功能测试**
- `ctx set` → `ctx list` → `ctx show` → `ctx del` CRUD。
- `ctx use` 输出正确魔法行序列。
- `ctx off` 恢复 `previous_dir` 与环境变量。
- 保留字拒绝、路径不存在报错。

**集成测试**
- 激活后 `XUN_CTX` 与 `XUN_DEFAULT_TAG` 正确设置。
- A→B 切换时旧值被记录并可恢复。

**边缘场景**
- 路径含空格/中文。
- 连续激活同一 profile（幂等）。
- `ctx off` 后再 `ctx off`（无操作）。

---

## 14. Open Questions（可选）

1. 是否需要 `ctx edit`（直接打开 `~/.xun.ctx.json`）？
2. 是否需要 `ctx back`（支持多层回退）？

---

## 15. 参考设计（主流 CLI 习惯）

- `kubectl config use-context` / `set-context`：明确区分“激活”与“定义”，命令分层清晰。
- `direnv`：环境变量由 shell 层生效，核心逻辑保持轻量。
- `aws --profile` / `gcloud config configurations activate`：配置与激活分离。
