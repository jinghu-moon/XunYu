# XunYu 智能补全性能基准

本页提供性能采集脚本与日志格式说明，用于验证补全延迟目标（<50ms）与批量稳定性。

补充（2026-02）：Dashboard Web UI 迭代不影响补全性能基准。

---

## 1. 脚本位置

`tools/bench-complete.ps1`

---

## 2. 使用前提

- 已构建二进制（推荐 release）：
  ```
  cargo build --release
  ```

脚本会自动优先使用：
- `target\release\xun.exe`
- 若不存在，则回退到 `target\debug\xun.exe`

---

## 3. 常用示例

**书签名补全（默认场景）**
```
.\tools\bench-complete.ps1 -CompleteArgs "z","" -Runs 100 -Warmup 5
```

**redirect profile 补全**
```
.\tools\bench-complete.ps1 -CompleteArgs "redirect","--profile","" -Runs 100
```

**带环境变量（CWD / Shell / Limit）**
```
.\tools\bench-complete.ps1 -CompleteArgs "z","" -Runs 200 `
  -Env "XUN_COMPLETE_CWD=D:\Repo\MyProj" "XUN_COMPLETE_SHELL=pwsh" "XUN_COMPLETE_LIMIT=0"
```

**输出 JSONL 日志**
```
.\tools\bench-complete.ps1 -CompleteArgs "z","" -Runs 100 -Out D:\Temp\xun_complete_bench.jsonl
```

---

## 4. 日志格式（JSON Lines）

每次运行追加一行 JSON，字段如下：

| 字段 | 含义 |
| --- | --- |
| `ts` | 时间戳（本地时间，ISO 8601） |
| `bin` | 使用的 xun 二进制路径 |
| `args` | `__complete` 参数字符串 |
| `runs` / `warmup` | 测试次数 / 预热次数 |
| `avg_ms` / `min_ms` / `max_ms` | 平均/最小/最大耗时 |
| `p50_ms` / `p95_ms` / `p99_ms` | 分位耗时 |
| `exit_nonzero` | 非 0 退出码次数 |
| `fallback_runs` | 输出 fallback 的次数 |
| `cand_min` / `cand_p50` / `cand_max` | 候选条数统计 |
| `env` | 本次设置的环境变量数组 |

示例（JSON 中路径会以 `\\` 表示反斜杠，这是 JSON 语义要求）：
```
{"ts":"2026-01-05T12:34:56","bin":"xun.exe","args":"z ","runs":100,"warmup":5,"avg_ms":6.4,"min_ms":4.1,"p50_ms":6.0,"p95_ms":9.8,"p99_ms":12.1,"max_ms":13.7,"exit_nonzero":0,"fallback_runs":0,"cand_min":12,"cand_p50":18,"cand_max":20,"env":["XUN_COMPLETE_CWD=D:\\Repo\\MyProj"]}
```

---

## 5. TSV 输出

用于快速对比与粘贴到 Excel：
```
.\tools\bench-complete.ps1 -CompleteArgs "z","" -Runs 100 -OutTsv D:\Temp\xun_complete_bench.tsv
```

TSV 字段顺序：
```
ts  bin  args  runs  warmup  avg_ms  min_ms  p50_ms  p95_ms  p99_ms  max_ms  exit_nonzero  fallback_runs  cand_min  cand_p50  cand_max  env
```

---

## 6. 性能验收建议

- 10k 条书签，单次补全 < 50ms
- 1 万条书签 + 连续 100 次补全，总耗时 < 500ms
