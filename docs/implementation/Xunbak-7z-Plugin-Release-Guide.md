# `.xunbak` 7-Zip 插件发布说明

> 日期：2026-03-26  
> 范围：`xunbak.dll` 的构建、安装、卸载、诊断与已知限制

---

## 1. 前提

1. 机器上已安装 7-Zip
2. 当前已验证的安装目录包括：
   - `C:/A_Softwares/7-Zip`
   - `C:/Program Files/7-Zip`
3. 当前已验证的 7-Zip 版本为 `24.09 (x64)`

---

## 2. 构建

Debug：

```powershell
./scripts/build_xunbak_7z_plugin.ps1 -Config Debug
```

Release：

```powershell
./scripts/build_xunbak_7z_plugin.ps1 -Config Release
```

产物路径：

1. `build/xunbak-7z-plugin/Debug/xunbak.dll`
2. `build/xunbak-7z-plugin/Release/xunbak.dll`

说明：

1. 当前脚本支持 `Debug | Release`
2. `cbindgen` 若失败，会降级复用已有 `cpp/xunbak-7z-plugin/xunbak_ffi_generated.h`
3. 当前构建目录不适合并行调用多个插件构建脚本

---

## 3. 安装

推荐使用 CLI：

```powershell
xun xunbak plugin install --config release
```

如需显式指定 7-Zip 目录：

```powershell
xun xunbak plugin install --config release --sevenzip-home "C:/A_Softwares/7-Zip"
```

如需同时建立当前用户 `.xunbak` 文件关联：

```powershell
xun xunbak plugin install --config release --associate
```

行为：

1. 将 `xunbak.dll` 复制到 `7-Zip/Formats`
2. 若目标已有 `xunbak.dll`，会先备份为 `xunbak.dll.bak.<timestamp>`
3. `--associate` 会把 `.xunbak` 关联到 `7zFM.exe`

---

## 4. 卸载

推荐使用 CLI：

```powershell
xun xunbak plugin uninstall
```

如需同时移除当前用户 `.xunbak` 关联：

```powershell
xun xunbak plugin uninstall --remove-association
```

行为：

1. 删除 `7-Zip/Formats/xunbak.dll`
2. 若存在历史备份 `xunbak.dll.bak.*`，会恢复最新备份
3. `--remove-association` 仅在绑定指向 7-Zip 且受当前用户管理时移除关联

---

## 5. 诊断

```powershell
xun xunbak plugin doctor
```

如需显式指定 7-Zip 目录：

```powershell
xun xunbak plugin doctor --sevenzip-home "C:/A_Softwares/7-Zip"
```

诊断会输出：

1. 7-Zip 主程序路径
2. 插件 DLL 是否已安装
3. `.xunbak` 关联状态
4. `7z zstd` 外部 codec 探测状态
5. 建议修复动作

---

## 6. 联调脚本

便携版：

```powershell
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Release
```

安装版：

```powershell
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Release -SevenZipHome "C:/A_Softwares/7-Zip"
```

验收脚本：

```powershell
./scripts/accept_xunbak_7z_plugin.ps1 -Config Release -WithSystem -SevenZipHome "C:/A_Softwares/7-Zip"
```

说明：

1. `portable` / `system` / `accept` 不建议并行运行
2. 如 Release DLL 已构建，可在后续脚本中加 `-SkipBuild`

---

## 7. 已知限制

1. 当前仅确认了 `7-Zip 24.x` 的 Release 插件联调；`26.x` 仍待补跑
2. `cbindgen` 的 `cargo metadata` duplicate-dependency 问题未根治，仍走“复用已有 header”降级
3. 构建脚本当前共用同一个 `build/xunbak-7z-plugin` 目录，不支持并发构建
4. `doctor` 能发现 `7z zstd` codec 状态，但不自动安装外部 codec

---

## 8. 排查建议

1. 插件未加载：
   - 先跑 `xun xunbak plugin doctor`
   - 确认 `7-Zip/Formats/xunbak.dll` 是否存在
   - 确认 `7z.exe` / `7zFM.exe` 是否来自预期的 7-Zip 目录

2. 构建失败：
   - 先确保没有并行运行多个插件构建/测试脚本
   - 再重试 `./scripts/build_xunbak_7z_plugin.ps1 -Config Release`

3. `.xunbak` 双击打不开：
   - 执行 `xun xunbak plugin install --associate`
   - 或执行 `xun xunbak plugin doctor` 查看当前关联状态

4. `zstd` 相关兼容问题：
   - 先看 `doctor` 输出的 codec 探测结果
   - stock 7-Zip 缺少外部 codec 时，不应假定 `7z zstd` 可解
