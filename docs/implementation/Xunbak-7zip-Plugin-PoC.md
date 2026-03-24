# Xunbak 7-Zip Plugin PoC

## 目标

提供一个可实际试用的 `.xunbak` 7-Zip 只读插件 PoC：

- 支持单文件 `.xunbak`
- 支持分卷 `.xunbak.001/.002/...`
- 支持 7-Zip `l` / `x`
- 保持 `.xunbak` 解析逻辑继续在 Rust 侧统一维护

## 当前结构

- Rust core
  路径：[lib.rs](/D:/100_Projects/110_Daily/XunYu/crates/xunbak-7z-core/src/lib.rs)
  责任：打开容器、读 manifest、提取 blob、回调式分卷桥接、C ABI
- C++ 薄壳
  路径：[cpp/xunbak-7z-plugin](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin)
  责任：7-Zip `IInArchive`、导出函数、`IInStream`/`IArchiveOpenVolumeCallback` 适配
- 生成式头文件
  路径：[xunbak_ffi_generated.h](/D:/100_Projects/110_Daily/XunYu/cpp/xunbak-7z-plugin/xunbak_ffi_generated.h)
  来源：`cbindgen`

## 构建

```powershell
./scripts/build_xunbak_7z_plugin.ps1 -Config Debug
```

输出：

- [xunbak.dll](/D:/100_Projects/110_Daily/XunYu/build/xunbak-7z-plugin/Debug/xunbak.dll)

## 验证层级

1. 仅导出冒烟

```powershell
./scripts/smoke_xunbak_7z_plugin.ps1
```

2. 便携式联调
不修改系统安装目录

```powershell
./scripts/test_xunbak_7z_plugin_portable.ps1 -Config Debug
```

3. 系统级联调
会临时写入 `7-Zip/Formats/xunbak.dll`，脚本结束后自动卸载

```powershell
./scripts/test_xunbak_7z_plugin_system.ps1 -Config Debug
```

4. 一键验收

```powershell
./scripts/accept_xunbak_7z_plugin.ps1 -Config Debug
./scripts/accept_xunbak_7z_plugin.ps1 -Config Debug -WithSystem
```

## 当前结果

- Rust core 单测通过
- DLL 编译通过
- 便携式 7-Zip 联调通过
- 系统级 7-Zip 联调通过

## 已知限制

- 当前是只读插件，不支持通过 7-Zip 创建 `.xunbak`
- 目前只覆盖 `IInArchive` 最小可用子集
- 控制台列出中文路径时建议加 `-sccUTF-8`
  原因：这是 `7z.exe` 控制台输出字符集问题，不是插件内部路径损坏

## 建议的下一阶段

1. 补更多归档级属性
2. 补 `GetPropertyInfo/GetArchivePropertyInfo` 的显示列定义
3. 评估把便携式/系统联调纳入 CI 或半自动发布流程
