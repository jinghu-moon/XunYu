在 **Microsoft Windows** 环境中，路径并不是只有 `C:\xxx` 这一种。Windows 的文件系统路径体系其实包含多种**命名空间（Namespace）\**和\**语义类型**。
如果你要做 **高性能路径校验组件（Path Guard）**，建议先把常见路径类型统计清楚，因为不同类型在 **解析、合法性校验、WinAPI 行为**上会有差异。

下面是**Windows 中常见路径类型的完整分类（工程实践视角）**。

------

# 1 本地绝对路径（最常见）

### 形式

```
C:\Windows\System32
D:\Projects\test.txt
E:\data\logs\
```

### 特点

- 以 **驱动器号 + `:\`** 开头
- 最常见输入形式
- `GetFileAttributesW` / `CreateFileW` 直接支持
- 长路径可能超过 `MAX_PATH=260`

### Path Guard 处理建议

- 默认允许
- 可自动补 `\\?\` 支持长路径

------

# 2 相对路径

### 形式

```
.\config.json
..\logs
data\test.txt
```

### 特点

- 依赖 **当前工作目录**
- `GetFullPathNameW` 会解析为绝对路径

### Path Guard 处理建议

由 `policy.allow_relative` 控制

```
allow_relative = false -> 直接拒绝
allow_relative = true -> GetFullPathNameW
```

------

# 3 UNC 网络路径

### 形式

```
\\server\share
\\server\share\folder\file.txt
```

### 特点

- SMB 网络共享
- 访问可能非常慢
- 并行调用 WinAPI 可能导致网络拥塞

### Path Guard 处理建议

识别规则：

```
path.starts_with("\\\\")
```

优化：

```
UNC path 并发限制 = 1~2
local path 并发 = 4~8
```

------

# 4 长路径（Win32 Extended Path）

### 形式

```
\\?\C:\very\long\path
\\?\UNC\server\share\folder
```

### 特点

- 绕过 `MAX_PATH=260`
- WinAPI 直接访问 NT path
- 不做路径规范化（`.` `..` 不解析）

### Path Guard 处理建议

当：

```
len(path) > 260
```

自动转换：

```
C:\foo -> \\?\C:\foo
\\server\share -> \\?\UNC\server\share
```

------

# 5 设备命名空间路径

### 形式

```
\\.\PhysicalDrive0
\\.\COM1
\\.\PIPE\mypipe
```

### 特点

- 访问设备或系统对象
- 不属于普通文件系统
- 可能需要管理员权限

### Path Guard 建议

默认 **禁止**

```
policy.allow_device_namespace = false
```

------

# 6 NT 内核路径（NT Namespace）

### 形式

```
\??\C:\Windows
\Device\HarddiskVolume1\Windows
```

### 特点

- Windows 内核使用
- Win32 API 很少直接使用
- 常见于调试工具

### Path Guard 建议

直接拒绝。

------

# 7 Volume GUID 路径

### 形式

```
\\?\Volume{GUID}\folder\file.txt
```

示例

```
\\?\Volume{d8e12345-...}\Windows
```

### 特点

- 直接访问卷
- 不依赖盘符

### Path Guard 建议

默认拒绝

```
policy.allow_device_namespace = false
```

------

# 8 目录 Junction / Symbolic Link

### 示例

```
C:\Users -> junction
C:\ProgramData -> junction
```

### 特点

```
FILE_ATTRIBUTE_REPARSE_POINT
```

### Path Guard 建议

```
allow_reparse = false -> 拒绝
allow_reparse = true -> 允许
```

------

# 9 ADS（NTFS Alternate Data Stream）

### 形式

```
file.txt:stream
file.txt:Zone.Identifier
```

### 示例

```
notepad.exe:Zone.Identifier
```

### 特点

- NTFS 特性
- 常被恶意软件滥用
- `:` 不只用于盘符

### Path Guard 建议

默认 **禁止**

```
allow_ads = false
```

------

# 10 Win32 保留设备名

### 形式

```
CON
PRN
AUX
NUL
COM1
COM2
...
LPT1
```

### 甚至：

```
CON.txt
PRN.log
```

仍然非法。

### Path Guard 建议

字符串阶段直接拦截。

------

# 11 特殊系统目录路径

常见系统路径：

```
C:\Windows
C:\Windows\System32
C:\Program Files
C:\Program Files (x86)
C:\Users
C:\Users\username\AppData
C:\ProgramData
```

### 特点

- 权限限制
- 可能 `AccessDenied`

### Path Guard 建议

存在性检查返回：

```
AccessDenied
```

不要误判为 `NotFound`

------

# 12 环境变量路径

### 形式

```
%TEMP%
%APPDATA%
%USERPROFILE%
```

示例

```
%TEMP%\log.txt
```

### Path Guard 建议

是否支持取决于 CLI 设计：

```
ExpandEnvironmentStringsW
```

------

# Windows 路径类型统计（工程统计）

在真实 Windows CLI 工具中，大致分布：

| 类型         | 占比   |
| ------------ | ------ |
| 本地路径     | 70–85% |
| 相对路径     | 10–20% |
| UNC 网络路径 | 1–5%   |
| 长路径       | <1%    |
| 设备命名空间 | <0.1%  |
| ADS          | <0.1%  |
| Volume GUID  | <0.01% |

------

# Path Guard 最小支持集（建议）

为了保持 **性能 + 简洁**，建议只完整支持：

### 必须支持

```
Drive absolute path
Relative path
UNC path
Long path
Reparse point detection
```

### 可选

```
ADS
Environment variable expansion
```

### 默认拒绝

```
Device namespace
NT namespace
Volume GUID
```

------

# Path Guard 推荐检测顺序（性能最优）

```
1 字符串合法性检查（零IO）
2 类型识别
3 relative -> GetFullPathNameW
4 long path prefix
5 并行存在性检查 GetFileAttributesW
6 reparse point 检测
7 open_path_with_policy (必要时)
```

------

# 一个很关键但很多人忽略的路径类型

**驱动器当前目录路径**

例如：

```
C:folder\file.txt
```

注意：

```
C:folder
```

不是

```
C:\folder
```

它表示：

```
当前 C: 盘的 working directory
```

这是 **Windows 最坑路径之一**。

建议：

```
直接拒绝
```

------

# 总结

Windows 常见路径可以归纳为 **11 类核心类型**：

1. 本地绝对路径
2. 相对路径
3. UNC 网络路径
4. 长路径 `\\?\`
5. 设备命名空间 `\\.\`
6. NT 内核路径
7. Volume GUID
8. 重解析点（symlink/junction）
9. ADS
10. 保留设备名
11. 环境变量路径

如果你做 **高性能 Path Guard**，推荐：

```
支持：1 2 3 4
检测：8
可选：9 11
拒绝：5 6 7
```

------

如果你愿意，我可以再给你一份 **Windows 路径解析完整状态机设计（工业级 CLI 的路径解析算法）**，那会是 **Rust 实现 Path Guard 的最优架构**。