# xunbak 7-Zip Plugin PoC

## 目标

这个目录提供 `.xunbak` 的 7-Zip 只读格式插件 PoC：

- 单文件 `.xunbak` 可 `l` / `x`
- 分卷 `.xunbak.001/.002/...` 可 `l` / `x`
- Rust 负责 `.xunbak` 解析、校验、解压
- C++ 只负责 7-Zip `IInArchive`/导出函数/流桥接

## 目录

- [CMakeLists.txt](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/CMakeLists.txt)
  CMake 构建入口
- [xunbak_exports.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_exports.cpp)
  `CreateObject` / `GetHandlerProperty2` / `GetIsArc`
- [xunbak_handler.cpp](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_handler.cpp)
  `IInArchive` 实现
- [xunbak_ffi.h](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_ffi.h)
  C++ 侧稳定入口头；内部转引 [xunbak_ffi_generated.h](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_ffi_generated.h)
- [xunbak_ffi_generated.h](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_ffi_generated.h)
  由 `cbindgen` 从 Rust core 自动生成

## 构建

```powershell
./scripts/build_xunbak_7z_plugin.ps1 -Config Debug
```

构建脚本会先执行：

```powershell
./scripts/generate_xunbak_ffi_header.ps1
```

产物：

- [xunbak.dll](/D:/100_Projects/110_Daily/XunYu/build/xunbak-7z-plugin/Debug/xunbak.dll)

## 冒烟

仅检查 DLL 可加载和导出存在：

```powershell
./scripts/smoke_xunbak_7z_plugin.ps1
```

## 便携式联调

不会修改系统 7-Zip 安装目录：

```powershell
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Debug
```

## 系统联调

会临时写入 `7-Zip/Formats/xunbak.dll`，脚本结束后自动卸载并恢复备份：

```powershell
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Debug
```

仅安装：

```powershell
./scripts/install_xunbak_7z_plugin.ps1 -Config Debug
```

卸载：

```powershell
./scripts/uninstall_xunbak_7z_plugin.ps1
```

## 已知限制

- 当前是只读插件，不支持通过 7-Zip 创建 `.xunbak`
- 目前只实现 `IInArchive` 的最小可用子集，更多归档级属性仍可继续补
- 控制台列出中文路径时，建议对 `7z.exe` 加 `-sccUTF-8`
  原因：`7z.exe` 默认 console charset 不一定是 UTF-8，GUI/真实解压路径不受此问题影响
- 当前采用“双头文件”模式：
  - `xunbak_ffi_generated.h` 由 `cbindgen` 生成
  - `xunbak_ffi.h` 只是稳定 include 入口，避免 C++ 代码直接依赖生成文件名

## 当前验证结论

- Rust core 单元测试通过
- DLL 编译通过
- 便携式 7-Zip 联调通过
- 系统级 7-Zip 联调通过
