# Shell 集成（推荐）

通过 `xun init <shell>` 把 `xun` 接入当前 shell 后，你可以获得：

- `xun`：正式命令包装
- `xyu`：兼容命令入口
- `xy`：快捷别名
- `x` / `z` / `o` / `ctx` / `xtree` / `xr` / `redir` 等高频快捷函数

## PowerShell

```powershell
# 如果 xun.exe 不在 PATH，可先显式指定：
$env:XUN_EXE = "D:/path/to/xun.exe"

Invoke-Expression ((xun init powershell) -join "`n")
```

加载后可直接使用：

```powershell
xy --help
xyu --help
```

## Bash / Zsh

```bash
export XUN_EXE="D:/path/to/xun.exe"   # 可选：仅在 xun.exe 不在 PATH 时需要
eval "$("$XUN_EXE" init bash)"        # Zsh 使用：init zsh
```

加载后可直接使用：

```bash
xy --help
xyu --help
```

## 当前命令注入规则

- `xun`：主命令
- `xyu`：与 `xun` 等价的兼容入口
- `xy`：指向 `xun` 的快捷别名
- 其它快捷函数继续保留原有语义

## 补全说明

- `xun init <shell>` 会优先加载 `xun completion <shell>` 的动态补全脚本。
- 如果动态补全加载失败，会回退到内置静态补全。
- `xyu` 和 `xy` 共享与 `xun` 相同的主命令补全行为。
- 如果只想加载补全，不需要 wrapper / 快捷函数，可以单独运行 `xun completion <shell>`。

### PowerShell 单独加载补全

```powershell
Invoke-Expression ((xun completion powershell) -join "`n")
```

### Bash / Zsh 单独加载补全

```bash
eval "$("$XUN_EXE" completion bash)"   # Zsh 使用：completion zsh
```

### Fish 单独加载补全

```fish
source (xun completion fish | psub)
```
