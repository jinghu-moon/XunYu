# XunYu Video Compress/Remux 规范（CPU/GPU）

## 1. 目标与边界

1. 仅支持本地视频处理：
   - 有损压缩：`compress`
   - 无损换封装：`remux`（`-c copy`）
2. 不在本阶段实现：网页搜索、索引、在线播放、滤镜编辑。
3. 采用外部 `ffmpeg`/`ffprobe` 驱动，不引入 `ac-ffmpeg` 多 worker 管线。

## 2. 借鉴 refer/video 的关键点

1. 预设驱动：借鉴 `refer/video/preset.rs`，将模式参数配置化，避免硬编码散落。
2. 硬件探测与回退：借鉴 `refer/video/hw_accel.rs`，统一优先级和回退链。
3. 先探测后执行：借鉴 `refer/video/scan.rs`，执行前用 `ffprobe` 判定容器/流兼容性。
4. 进度与完成信号：借鉴 `refer/video/progress.rs` 的“完成信号”思路，避免无时长素材卡住进度。
5. 批量重试策略：借鉴 `refer/video/batch.rs` 的失败清理与重试报告。

## 3. CLI 规范

```text
xun video probe -i <input>
xun video compress -i <input> -o <output> [--mode fastest|balanced|smallest] [--engine auto|cpu|gpu]
xun video remux -i <input> -o <output> [--strict]
```

默认值：
1. `mode=balanced`
2. `engine=auto`
3. `strict=true`

## 4. 编码器选择策略

### 4.1 engine=auto

1. GPU 可用时优先 GPU 编码器。
2. 目标模式为 `smallest` 时默认优先 CPU（压缩率优先）。
3. GPU 不可用或目标编码器不可用时自动回退 CPU。

回退顺序：
1. `NVENC`
2. `QSV`
3. `AMF`
4. `CPU`

### 4.2 engine=cpu / gpu

1. `cpu`：强制只使用软件编码器。
2. `gpu`：强制硬件编码；无可用硬件时直接报错，不静默降级。

## 5. 六种组合（2 容器 × 3 模式）

## 5.1 MP4 输出

1. `fastest`
   - 首选：`h264_nvenc | h264_qsv | h264_amf | h264_mf`
   - 回退：`libx264`
   - 建议参数：
     - GPU: `-c:v h264_* -cq 28 -b:v 0`
     - CPU: `-c:v libx264 -preset veryfast -crf 24`
     - 音频：`-c:a aac -b:a 128k`
2. `balanced`
   - 首选：`hevc_nvenc | hevc_qsv | hevc_amf | hevc_mf`
   - 回退：`libx265`
   - 建议参数：
     - GPU: `-c:v hevc_* -cq 29 -b:v 0`
     - CPU: `-c:v libx265 -preset medium -crf 26`
     - 音频：`-c:a aac -b:a 128k`
3. `smallest`
   - 首选：`libx265`
   - 可选：`libsvtav1`（更小但更慢）
   - 建议参数：
     - `-c:v libx265 -preset slow -crf 28`
     - 音频：`-c:a aac -b:a 96k`

## 5.2 WebM 输出

1. `fastest`
   - 编码器：`libvpx-vp9`
   - 建议参数：`-deadline realtime -cpu-used 8 -crf 36 -b:v 0`
   - 音频：`-c:a libopus -b:a 96k`
2. `balanced`
   - 编码器：`libvpx-vp9`
   - 建议参数：`-deadline good -cpu-used 4 -crf 33 -b:v 0`
   - 音频：`-c:a libopus -b:a 96k`
3. `smallest`
   - 编码器：`libsvtav1`（极限可选 `libaom-av1`）
   - 建议参数：`-preset 6 -crf 35`
   - 音频：`-c:a libopus -b:a 80k`

## 6. Remux（无损换封装）规范

核心要求：
1. 必须使用 `-map 0 -c copy`。
2. MP4 输出额外加 `-movflags +faststart`。
3. `--strict=true` 时，不兼容直接失败，不做隐式重编码。
4. “不同格式互转”在本规范中指容器互转：如 `mkv -> mp4`、`mov -> mkv`、`mp4 -> webm`（仅当流编码兼容时）。

示例：

```bash
ffmpeg -i in.mkv -map 0 -c copy -movflags +faststart out.mp4
ffmpeg -i in.mov -map 0 -c copy out.mkv
```

常见注意：
1. `H.264/H.265 + AAC` 通常可 `mkv <-> mp4` 无损互转。
2. 若原流为 `PCM/TrueHD/某些字幕流`，`mp4` 可能不兼容，此时 `--strict` 应报错。
3. `webm` 一般要求 `VP8/VP9/AV1 + Vorbis/Opus`，不兼容流不能直接 `-c copy`。

## 7. 容器兼容规则（strict）

1. 若目标容器不支持输入流编码，则报错退出。
2. 若目标容器不支持某字幕/附件流，报错并提示使用非严格模式（后续可扩展）。
3. 不在 `remux` 模式自动执行任何转码。

## 8. 执行流程

1. `probe`：
   - 读取容器、视频编码、音频编码、时长、分辨率、帧率。
2. `plan`：
   - 根据 `mode + container + engine` 选择编码器和参数。
3. `run`：
   - 组装 `ffmpeg` 命令执行，输出进度。
4. `verify`：
   - 检查输出文件存在、时长合理、流映射合理。
   - `remux` 额外验证 codec 未变化。

## 9. 构建与运行前置检查

1. `ffmpeg -encoders` 必须包含目标编码器。
2. `ffmpeg -muxers` 必须包含目标容器。
3. `ffprobe` 可执行且可返回 JSON。

建议检查命令：

```bash
ffmpeg -hide_banner -encoders | rg "264|265|av1|vp9|aac|opus"
ffmpeg -hide_banner -muxers | rg "mp4|webm|matroska|mov"
ffprobe -hide_banner -version
```

## 10. MSYS2 编译 FFmpeg（补充）

本项目的 `video` 子命令依赖可执行文件 `ffmpeg.exe` 与 `ffprobe.exe`，因此建议在 Windows 下使用 MSYS2 MinGW64 编译一套专用构建。
编译脚本基于 `refer/video/build_ffmpeg.sh` 的参数组织方式（`--disable-everything` + 白名单启用），并按 xun 需求做最小差异化。

新增脚本：
1. `tools/video/build_ffmpeg_msys2.sh`
2. `tools/video/build_ffmpeg_msys2.ps1`

### 10.1 一键编译（推荐）

1. 在 Windows PowerShell 中执行：

```powershell
./tools/video/build_ffmpeg_msys2.ps1
```

默认使用：
1. `-Msys2Root "C:/A_Softwares/MSYS2"`
2. `-Branch "n8.0.1"`
3. `-Source ""`（未指定时走脚本默认目录）
4. `-LinkMode shared`

2. 或在 MSYS2 MinGW64 Shell 中直接执行：

```bash
bash tools/video/build_ffmpeg_msys2.sh
```

默认输出目录：
1. `~/ffmpeg-xun/bin/ffmpeg.exe`
2. `~/ffmpeg-xun/bin/ffprobe.exe`

推荐（本机实测）命令：

```powershell
./tools/video/build_ffmpeg_msys2.ps1 `
  -Msys2Root "C:/A_Softwares/MSYS2" `
  -Source "D:/100_Projects/110_Daily/Xun/refer/ffmpeg" `
  -Branch "n8.0.1" `
  -LinkMode shared `
  -Prefix "C:/A_Softwares/ffmpeg-xun-shared"

./tools/video/build_ffmpeg_msys2.ps1 `
  -Msys2Root "C:/A_Softwares/MSYS2" `
  -Source "D:/100_Projects/110_Daily/Xun/refer/ffmpeg" `
  -Branch "n8.0.1" `
  -LinkMode static `
  -Prefix "C:/A_Softwares/ffmpeg-xun-static"
```

### 10.2 编译策略说明

1. 许可证：
   - 启用 `--enable-gpl`
   - 默认 `--disable-nonfree`
   - 如需开启：`./tools/video/build_ffmpeg_msys2.ps1 -EnableNonfree`
2. 覆盖能力：
   - 压缩（默认）：`libx264/libx265/libvpx/libopus`
   - AV1（可选）：`libsvtav1`（默认关闭，需显式开启）
   - 无损转换：依赖 demux/mux 与 stream copy（`-c copy`）
   - 仅保留必要组件白名单（最小化）：`--disable-everything --disable-autodetect` + 必需 decoder/encoder/demuxer/muxer/filter/protocol
3. 硬件加速：
   - 默认尝试启用 `NVENC/AMF/QSV(oneVPL)`
   - 通过 `-DisableHw` 可关闭硬件相关启用项
4. `fdk-aac`（可选）：
   - 默认关闭
   - 如需开启：`./tools/video/build_ffmpeg_msys2.ps1 -EnableFdkAac`
5. `svt-av1`（可选）：
   - 默认关闭（脚本会显式 `--disable-libsvtav1`）
   - 如需开启：`./tools/video/build_ffmpeg_msys2.ps1 -EnableSvtAv1`
6. 链接模式：
   - 默认 `shared`：`ffmpeg.exe + 依赖 DLL`
   - 可选 `static`：单文件 `ffmpeg.exe`（体积更大，构建更慢）
   - 切换：`./tools/video/build_ffmpeg_msys2.ps1 -LinkMode static`
7. 运行时注意（Windows）：
   - 若直接在普通 PowerShell 运行 `ffmpeg.exe` 出现退出码 `-1073741511`，通常是 `mingw` 运行时 DLL 未在 `PATH`。
   - 解决方式：使用 `MSYS2 MinGW64` 环境运行，或在调用前注入 `PATH=/mingw64/bin:/usr/bin:$PATH`（通过 `bash -lc`）。

### 10.3 构建后验证

在 Windows 下建议通过 MSYS2 bash 执行验证：

```powershell
$bash = "C:/A_Softwares/MSYS2/usr/bin/bash.exe"
& $bash -lc "MSYSTEM='MINGW64' CHERE_INVOKING='1' PATH='/mingw64/bin:/usr/bin:$PATH' /c/A_Softwares/ffmpeg-xun-shared/bin/ffmpeg.exe -hide_banner -encoders | grep -E 'libx264|libx265|libsvtav1|libvpx-vp9|h264_nvenc|hevc_nvenc|h264_qsv|hevc_qsv|h264_amf|hevc_amf'"
& $bash -lc "MSYSTEM='MINGW64' CHERE_INVOKING='1' PATH='/mingw64/bin:/usr/bin:$PATH' /c/A_Softwares/ffmpeg-xun-shared/bin/ffmpeg.exe -hide_banner -muxers | grep -E 'mp4|webm|matroska|mov|mpegts'"
& $bash -lc "MSYSTEM='MINGW64' CHERE_INVOKING='1' PATH='/mingw64/bin:/usr/bin:$PATH' /c/A_Softwares/ffmpeg-xun-shared/bin/ffprobe.exe -hide_banner -version | head -n 3"
```

本机已验证：
1. `shared` 产物：`C:/A_Softwares/ffmpeg-xun-shared`
   - encoders 命中：`libx264/libx265/libvpx-vp9`
   - muxers 命中：`mp4/webm/matroska/mov/mpegts`
2. `static` 产物：`C:/A_Softwares/ffmpeg-xun-static`
   - encoders 命中：`libx264/libx265/libvpx-vp9`，并检测到 `h264_nvenc/hevc_nvenc/h264_qsv/hevc_qsv/h264_amf/hevc_amf`
   - muxers 命中：`mp4/webm/matroska/mov/mpegts`
   - 说明：若需“仅最小集”，建议按最新脚本重新构建（默认关闭 `svt-av1` 自动引入）。

## 11. XunYu 接入清单（实现落地点）

新增：
1. `src/cli/video.rs`
2. `src/commands/video/mod.rs`
3. `src/commands/video/common.rs`
4. `src/commands/video/types.rs`
5. `src/commands/video/error.rs`
6. `src/commands/video/probe.rs`
7. `src/commands/video/plan.rs`
8. `src/commands/video/ffmpeg.rs`
9. `src/commands/video/compress.rs`
10. `src/commands/video/remux.rs`

修改：
1. `src/cli.rs`（注册 `VideoCmd` + `SubCommand::Video`）
2. `src/commands/mod.rs`（注册 `video` 模块）
3. `src/commands/dispatch/misc.rs`（路由 `cmd_video`）
4. `src/commands/completion/shell_powershell.rs`（补全 `video`）
5. `src/commands/completion/shell_bash.rs`（补全 `video`）
6. `src/commands/dispatch/core.rs`（init 脚本补全 `video`）

## 12. 验收标准（DoD）

1. `cargo check --all-features` 通过。
2. `compress` 三模式在 MP4 产物可用。
3. WebM 三模式参数可执行。
4. `remux --strict` 遇到不兼容输入时明确失败。
5. `engine=auto` 正确 GPU 优先并可回退。
6. 补全脚本可提示 `video` 子命令。
