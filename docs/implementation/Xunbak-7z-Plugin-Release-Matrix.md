# `.xunbak` 7-Zip 插件 Release 验证矩阵

> 日期：2026-03-26  
> 范围：`xunbak.dll` Release 构建与 7-Zip 加载/列出/提取验证  
> 说明：当前结论基于本地 Windows 环境，已覆盖 stock `7-Zip 24.09 (x64)` 与 stock `7-Zip 26.00 (x64)`

---

## 1. 验证环境

1. 项目根目录：`D:/100_Projects/110_Daily/XunYu`
2. 7-Zip 安装目录：`C:/A_Softwares/7-Zip`
3. 7-Zip 版本：
   - `24.09 (x64)`
   - `26.00 (x64)`
4. 插件构建配置：`Release`

---

## 2. 执行命令

```powershell
./scripts/build_xunbak_7z_plugin.ps1 -Config Release
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Release
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild
```

说明：

1. `portable` 和 `system` 脚本不应并行跑，因为当前共用同一个 CMake/MSBuild 输出目录
2. `system` 复测时使用了 `-SkipBuild`，避免重复占用 `build/xunbak-7z-plugin`
3. `cbindgen` 仍可能打印“复用已有 header”警告，但当前不阻塞 Release 构建

---

## 3. 当前矩阵

| 维度 | 7-Zip 24.09 | 7-Zip 26.00 | 备注 |
| --- | --- | --- | --- |
| Release `xunbak.dll` 构建 | Pass | Pass | `build/xunbak-7z-plugin/Release/xunbak.dll` 已产出 |
| 便携版 7-Zip (`portable`) | Pass | Pass | `list / -slt / extract` 全通过 |
| 安装版 7-Zip (`system`) | Pass | Pass | `list / -slt / extract` 全通过 |
| 单文件 `.xunbak` | Pass | Pass | `Files = 3`、`Method = Copy|ZSTD` 正常 |
| 分卷 `.xunbak.001` | Pass | Pass | `Volumes = 2` 正常，提取通过 |
| `PPMD` 样本 `.xunbak` | Pass | Pass | `Method = PPMD` 正常显示并可提取 |

---

## 4. 已验证行为

1. Release 插件可被 stock `7-Zip 24.09 / 26.00` 加载
2. 单文件和分卷 `.xunbak` 都可正常 `l / l -slt / x`
3. `PPMD` codec 在 Release 插件下可正常显示为 `Method = PPMD`
4. 中文路径 `nested/深层.txt` 在 `24.09 / 26.00` 下显示和提取正常

## 4.1 2026-03-27 26.00 实测

执行命令：

```powershell
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip" -SkipBuild
```

关键结果：

1. `7z.exe` 文件版本：`26.00`
2. 便携版与安装版脚本都通过
3. `Type = XUNBAK`、`Volumes = 2`、`Method = PPMD` 在 `7-Zip 26.00` 下均正常显示
4. 单文件 / 分卷 / `PPMD` 样本提取均返回 `Everything is Ok`

---

## 5. 当前限制

1. `build_xunbak_7z_plugin.ps1` 目前还不是并发安全脚本
2. `cbindgen` 的 `cargo metadata` duplicate-dependency 问题仍未根治，仍依赖“复用已有 header”的降级路径
3. 目前已验证到 `7-Zip 26.00`；若后续出现 `26.x` 其他 patch 版本，仍建议追加矩阵回归
