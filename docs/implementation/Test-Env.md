# XunYu 测试环境（统一执行所有命令）

目标：在**同一个隔离环境**中执行所有命令，确保不会污染真实配置或数据。

---

## 1. 一键创建测试环境

推荐（自动构建 + 重置）：
```
tools\test-env.ps1 -Build -Reset
```

仅创建环境（使用已有二进制）：
```
tools\test-env.ps1
```

进入测试 shell：
```
pwsh -NoExit -File D:\100_Projects\110_Daily\Xun\target\xun-cli-env\enter.ps1
```

可选参数：
- `-Release`：使用 `target\release\xun.exe`
- `-Bin <path>`：指定 xun 可执行文件
- `-Root <path>`：指定测试根目录
- `-NonInteractive`：写入 `XUN_NON_INTERACTIVE=1`

---

## 2. 环境隔离说明

脚本会设置以下环境变量（仅在 test shell 中生效）：
- `XUN_EXE`：xun 二进制路径
- `XUN_DB`：指向测试根目录下的 `.xun.json`
- `XUN_CONFIG`：指向测试根目录下的 `.xun.config.json`
- `XUN_CTX_FILE`：指向测试根目录下的 `.xun.ctx.json`
- `USERPROFILE` / `HOME`：指向测试根目录（避免污染系统配置）
- `XUN_COMPLETE_CWD`：补全用 cwd（默认指向 `project` 目录）
- `XUN_COMPLETE_SHELL=pwsh`
- `XUN_CTX_STATE`：ctx 会话文件（默认由 test-env 预设，也可由 wrapper 设置）

因此 `proxy set/del`、`cargo/git/npm` 配置等操作**只作用于沙箱目录**，不会影响用户真实配置。

---

## 3. 测试目录结构

默认根目录：`D:\100_Projects\110_Daily\Xun\target\xun-cli-env`

主要目录：
- `project`：用于 `set/z/open`、tree、bak 示例
- `redirect-src`：redirect 输入目录（包含 jpg/png/pdf/docx/zip）
- `redirect-out`：redirect 输出目录（规则会写入）
- `lock`：锁测试文件
- `protect`：保护测试文件
- `crypt`：加密测试文件
- `bak-src`：备份源目录
- `tree`：深层结构目录
- `import`：导入测试文件
- `out`：导出目标目录

---

## 4. 快速检查清单（覆盖所有命令）

以下命令均可在同一个测试 shell 中运行：

### 4.1 基础/全局
```
xun --version
xun --help
xun completion powershell
xun __complete z ""
```

### 4.2 书签类
```
xun list
xun z proj
xun open proj
xun set demo D:\100_Projects\110_Daily\Xun\target\xun-cli-env\project
xun del demo
xun touch project
xun rename project project2
xun gc
xun check
xun stats
xun recent
xun dedup --yes
```

### 4.3 Context Switch（ctx）
```
xun ctx set work --path D:\100_Projects\110_Daily\Xun\target\xun-cli-env\project --tag work,rust --proxy keep
xun ctx list
xun ctx show work
xun ctx use work
xun ctx off
```

### 4.4 导入导出
```
xun export -f json -o D:\100_Projects\110_Daily\Xun\target\xun-cli-env\out\export.json
xun import -f json -i D:\100_Projects\110_Daily\Xun\target\xun-cli-env\import\bookmarks.json -m merge --yes
```

### 4.5 Tree
```
xun tree D:\100_Projects\110_Daily\Xun\target\xun-cli-env\tree --size
```

### 4.6 Redirect
```
xun redirect D:\100_Projects\110_Daily\Xun\target\xun-cli-env\redirect-src --profile default --dry-run
xun redirect D:\100_Projects\110_Daily\Xun\target\xun-cli-env\redirect-src --profile default --confirm --yes
xun redirect --log --last 5
```

### 4.7 Proxy
```
xun proxy detect
xun proxy get
xun proxy set http://127.0.0.1:7890 -o git,cargo
xun proxy del -o git,cargo
xun pon --no-test
xun poff
xun pst
```

### 4.8 Bak
```
xun bak -C D:\100_Projects\110_Daily\Xun\target\xun-cli-env\bak-src -m "demo"
xun bak list
xun restore <name>
```

### 4.9 Lock / FS
```
xun lock who D:\100_Projects\110_Daily\Xun\target\xun-cli-env\lock\locked.txt
xun rm D:\100_Projects\110_Daily\Xun\target\xun-cli-env\lock\locked.txt --dry-run
xun mv D:\100_Projects\110_Daily\Xun\target\xun-cli-env\project D:\100_Projects\110_Daily\Xun\target\xun-cli-env\project-moved --dry-run
```

### 4.10 Protect
```
xun protect set D:\100_Projects\110_Daily\Xun\target\xun-cli-env\protect\protected.txt --deny delete
xun protect status
xun protect clear D:\100_Projects\110_Daily\Xun\target\xun-cli-env\protect\protected.txt
```

### 4.11 Crypt
```
xun encrypt D:\100_Projects\110_Daily\Xun\target\xun-cli-env\crypt\secret.txt -o D:\100_Projects\110_Daily\Xun\target\xun-cli-env\crypt\secret.age
xun decrypt D:\100_Projects\110_Daily\Xun\target\xun-cli-env\crypt\secret.age -o D:\100_Projects\110_Daily\Xun\target\xun-cli-env\crypt\secret.dec.txt
```

### 4.12 Dashboard
```
xun serve --port 9527
```

手动检查（浏览器打开 `http://localhost:9527`）：
- Home 概览能加载，统计卡片与最近审计列表正常显示。
- Bookmarks/Ports/Audit 导出 CSV/JSON 可下载。
- Command Palette（Ctrl/Cmd+K）可用，Density/Theme 切换生效。

---

## 5. 清理

重新生成（清空旧环境）：
```
tools\test-env.ps1 -Reset
```
