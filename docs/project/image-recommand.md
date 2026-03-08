## 第一轮基准（4 张测试图，-q 80）

基于 A.jpg/A.png/A.webp/A.avif 两轮跑测：

### 本格式压缩推荐

| 输入=输出          | 速度优先          | 体积优先          | 默认建议                               |
| ------------------ | ----------------- | ----------------- | -------------------------------------- |
| JPG->JPG           | turbojpeg         | mozjpeg           | 离线用 mozjpeg，实时用 turbojpeg       |
| PNG->PNG（有损）   | pngquant q60-80   | pngquant q60-80   | 允许有损就用 pngquant                  |
| PNG->PNG（无损）   | oxipng lv2        | zopflipng -m      | 默认 oxipng lv2，zopflipng 仅极限压缩  |
| WEBP->WEBP（有损） | libwebp lossy q80 | libwebp lossy q80 | 默认 libwebp lossy                     |
| WEBP->WEBP（无损） | image-rs lossless | libwebp lossless  | 追体积用 libwebp，追速度用 image-rs    |
| AVIF->AVIF         | ravif             | ravif             | 默认 ravif（明显优于 libavif）         |

### 格式转换推荐（任意输入 -> 目标格式）

| 目标格式 | 推荐                                                                            |
| -------- | ------------------------------------------------------------------------------- |
| 转 JPG   | 速度 turbojpeg，体积 mozjpeg                                                    |
| 转 PNG   | 有损 pngquant；无损 oxipng lv2；不建议常规用 zopflipng -m（太慢）              |
| 转 WEBP  | 有损 libwebp lossy q80；无损 libwebp lossless（更小）/ image-rs lossless（更快） |
| 转 AVIF  | ravif q80 sp7（速度和体积均优于 libavif）                                       |

### 总策略

1. 默认链路：JPG=mozjpeg、PNG=pngquant(有损)/oxipng lv2(无损)、WEBP=libwebp lossy、AVIF=ravif。
2. 实时场景把 JPG 切到 turbojpeg。
3. zopflipng -m 只做离线"极限压缩"专项，不放日常批处理。

---

## 第二轮基准（10 张 A4 图，-q 80，10 线程）—— JPEG 编码器深度对比

**数据集**：`image-test/input-a4-20260301-190432`，格式过滤 jpeg，limit 10
**目的**：验证 mozjpeg 能否通过运行时关闭 trellis 来替代 turbojpeg（"二合一"可行性）

### 编码器说明

| 编码器 | 说明 |
|--------|------|
| turbojpeg | libjpeg-turbo，SIMD 加速，速度基准 |
| mozjpeg | 全特性，默认开启 trellis 量化 + progressive |
| moz-fast | `set_fastest_defaults()`，mozjpeg 内置快速模式 |
| moz-manual | `jpeg_c_set_bool_param` 手工关闭 TRELLIS_QUANT / TRELLIS_QUANT_DC / TRELLIS_EOB_OPT / TRELLIS_Q_OPT |

### 速度对比（avg_ms，越低越好）

| 输入格式 | turbojpeg | moz-fast | moz-manual | mozjpeg |
|---------|----------:|---------:|-----------:|--------:|
| JPG 输入 | **47ms** | 210ms（4.5x） | 928ms（19.6x） | 1224ms（25.9x） |
| PNG 输入 | **28ms** | 130ms（4.6x） | 489ms（17.3x） | 637ms（22.5x） |
| WebP 输入 | **85ms** | 351ms（4.1x） | 1853ms（21.8x） | 2357ms（27.7x） |
| AVIF 输入 | **41ms** | 251ms（6.1x） | 1267ms（30.9x） | 1674ms（40.8x） |

### 体积对比趋势

- **moz-fast** 体积 ≈ turbojpeg（关闭 trellis 后压缩率基本一致）
- **moz-manual** 体积 < turbojpeg，但 > mozjpeg 全特性（部分 trellis 路径仍保留）
- **mozjpeg** 体积最小（第一轮数据：比 turbojpeg 小 29~51%）

### "二合一"可行性结论

> **不可行。mozjpeg 即使关闭所有 trellis 参数，速度仍比 turbojpeg 慢 4~6x。**

原因分析：
- `moz-manual`（手工关闭 4 个 trellis flag）仍慢 17~31x，说明 mozjpeg 内部还有其他优化路径比 turbojpeg 慢
- `moz-fast`（`set_fastest_defaults`）已是最激进的速度模式，仍慢 4~6x
- 速度差不来自 trellis 一个开关，而是 mozjpeg 整体编码路径与 libjpeg-turbo 的架构差异

### JPEG 最终选型确认

| 场景 | 库 | 理由 |
|------|----|------|
| 体积优先（离线批处理） | **mozjpeg** | 比 turbojpeg 小 29~51%，速度不是瓶颈 |
| 速度优先（实时/CI） | **turbojpeg** | 4~6x 速度差，mozjpeg 任何模式均无法弥合 |

**现已支持单二进制并存：默认构建 `img-moz`，启用 `img-turbo` 后通过运行时动态加载 `turbojpeg.dll`，缺失时可回退 `mozjpeg`（`auto` 模式）。**

---

## 第三轮基准（xun img，10 张固定样本，-q 80，12 线程）

**测试日期**：2026-03-02  
**输入样本**：`image-test/baseline-input-10-20260302`（共 10 张，jpg/png/webp/avif = 3/3/2/2）  
**执行二进制**：`target/release/xun.exe`（`cargo build --release --features "img-moz"`）  
**统一参数**：`-q 80 -t 12 --overwrite`

### 执行命令

```powershell
# JPEG（moz 后端）
xun.exe img -i "D:/100_Projects/110_Daily/Xun/image-test/baseline-input-10-20260302" -o "D:/100_Projects/110_Daily/Xun/image-test/baseline-output-20260302/jpeg" -f jpeg -q 80 -t 12 --overwrite --jpeg-backend moz

# PNG（有损，pngquant）
xun.exe img -i "D:/100_Projects/110_Daily/Xun/image-test/baseline-input-10-20260302" -o "D:/100_Projects/110_Daily/Xun/image-test/baseline-output-20260302/png" -f png -q 80 -t 12 --overwrite --png-lossy true

# WebP（有损）
xun.exe img -i "D:/100_Projects/110_Daily/Xun/image-test/baseline-input-10-20260302" -o "D:/100_Projects/110_Daily/Xun/image-test/baseline-output-20260302/webp" -f webp -q 80 -t 12 --overwrite --webp-lossy true

# AVIF（ravif）
xun.exe img -i "D:/100_Projects/110_Daily/Xun/image-test/baseline-input-10-20260302" -o "D:/100_Projects/110_Daily/Xun/image-test/baseline-output-20260302/avif" -f avif -q 80 -t 12 --overwrite
```

### 结果汇总

| 目标格式 | 成功/失败 | 输入总大小 | 输出总大小 | 节省空间 | 吞吐量 | 平均单图耗时 | 总耗时 |
|---|---:|---:|---:|---:|---:|---:|---:|
| JPEG (moz) | 10 / 0 | 141.6 MB | 7.3 MB | 94.9% | 55.5 MB/s | 1428.8 ms | 2.55s |
| PNG (lossy) | 10 / 0 | 141.6 MB | 53.9 MB | 61.9% | 29.7 MB/s | 3387.1 ms | 4.77s |
| WebP (lossy) | 10 / 0 | 141.6 MB | 5.2 MB | 96.3% | 40.3 MB/s | 2007.5 ms | 3.51s |
| AVIF (ravif) | 10 / 0 | 141.6 MB | 4.0 MB | 97.2% | 4.8 MB/s | 25120.2 ms | 29.44s |

### 基准结论（本轮）

1. 速度：`JPEG > WebP > PNG >> AVIF`（同样本、同线程条件下）。
2. 体积：`AVIF` 最小，其次 `WebP`，`JPEG` 次之，`PNG(lossy)` 最大。
3. 作为后续优化基线：优先关注 `AVIF` 路径总耗时，其次是 `PNG` 路径吞吐。

---

## 第四轮矩阵基准（20 组：4 输入 × 5 输出）

**测试日期**：2026-03-02  
**输入集**：`image-test/matrix-input-10-20260302`（每种输入格式各 10 张）  
**输入格式**：`jpg / png / webp / avif`  
**输出配置**：`jpeg-moz / jpeg-turbo / png-lossy / webp-lossy / avif`  
**统一参数**：`-q 80 -t 12 --overwrite`  
**可执行文件**：`target/release/xun.exe`（`cargo build --release --features "img-moz,img-turbo"`）

### 结果明细

| # | 输入 | 输出 | 成功 | 吞吐 (MB/s) | 节省空间 | 总耗时 (ms) |
|---:|---|---|---:|---:|---:|---:|
| 1 | jpg  | jpeg-moz   | 10/10 | 45.67  | 92.21% | 1966.6 |
| 2 | jpg  | jpeg-turbo | 10/10 | 136.58 | 86.42% | 657.6 |
| 3 | jpg  | png-lossy  | 10/10 | 17.90  | 25.70% | 5018.8 |
| 4 | jpg  | webp-lossy | 10/10 | 43.10  | 95.43% | 2084.2 |
| 5 | jpg  | avif       | 10/10 | 3.35   | 96.72% | 26851.8 |
| 6 | png  | jpeg-moz   | 10/10 | 168.44 | 99.40% | 690.0 |
| 7 | png  | jpeg-turbo | 10/10 | 544.50 | 97.63% | 213.4 |
| 8 | png  | png-lossy  | 10/10 | 48.28  | 87.75% | 2407.5 |
| 9 | png  | webp-lossy | 10/10 | 99.99  | 99.65% | 1162.4 |
| 10 | png | avif       | 10/10 | 7.57   | 99.89% | 15360.4 |
| 11 | webp | jpeg-moz   | 10/10 | 49.14  | 91.61% | 2900.7 |
| 12 | webp | jpeg-turbo | 10/10 | 103.59 | 82.30% | 1375.9 |
| 13 | webp | png-lossy  | 10/10 | 25.25  | 52.88% | 5646.0 |
| 14 | webp | webp-lossy | 10/10 | 35.79  | 95.94% | 3982.9 |
| 15 | webp | avif       | 10/10 | 3.17   | 97.16% | 45001.7 |
| 16 | avif | jpeg-moz   | 10/10 | 115.15 | 95.02% | 2228.6 |
| 17 | avif | jpeg-turbo | 10/10 | 201.53 | 92.77% | 1273.5 |
| 18 | avif | png-lossy  | 10/10 | 58.78  | 68.41% | 4366.3 |
| 19 | avif | webp-lossy | 10/10 | 95.99  | 94.81% | 2673.6 |
| 20 | avif | avif       | 10/10 | 8.28   | 95.63% | 30991.2 |

### 本轮结论

1. JPEG 仍保持 `turbo > moz` 的稳定速度优势（约 1.7x 到 3.2x），`moz` 体积更小。
2. AVIF 目标格式在四类输入上均最慢，但空间节省通常最高。
3. 20 组任务全部成功（无失败/无跳过），可作为后续性能优化的固定回归基线。

### 明细文件

- CSV：`image-test/matrix-results-20260302-113811.csv`
- 日志目录：`image-test/matrix-logs-20260302-113811`
- 输出目录：`image-test/matrix-output-20260302-113811`

---

## 第五轮矩阵基准（应用代码优化后，同样 20 组）

**测试日期**：2026-03-02  
**输入集**：`image-test/matrix-input-10-20260302`（固定样本，非随机）  
**样本选择规则**：按文件名排序后，`jpg/png/webp/avif` 各取前 10 张。  
**输出配置**：`jpeg-moz / jpeg-turbo / png-lossy / webp-lossy / avif`  
**统一参数**：`-q 80 -t 12 --overwrite`

**本轮优化点（不改编码器参数）**：
1. 普通路径改为单次读文件后内存解码（避免 `metadata + open` 双访问）。
2. 并行阶段采用线程本地读缓冲复用（减少重复分配）。
3. 进度条改为批次更新（每 8 张更新一次，降低锁竞争）。
4. 线程数默认上限增加 `<= 文件数` 限制，避免过度并行。

### 结果明细

| # | 输入 | 输出 | 成功 | 吞吐 (MB/s) | 节省空间 | 总耗时 (ms) |
|---:|---|---|---:|---:|---:|---:|
| 1 | jpg  | jpeg-moz   | 10/10 | 44.82  | 92.21% | 2004.1 |
| 2 | jpg  | jpeg-turbo | 10/10 | 133.75 | 86.42% | 671.6 |
| 3 | jpg  | png-lossy  | 10/10 | 17.62  | 25.70% | 5097.2 |
| 4 | jpg  | webp-lossy | 10/10 | 41.98  | 95.43% | 2139.6 |
| 5 | jpg  | avif       | 10/10 | 3.02   | 96.72% | 29733.1 |
| 6 | png  | jpeg-moz   | 10/10 | 162.36 | 99.40% | 715.8 |
| 7 | png  | jpeg-turbo | 10/10 | 523.37 | 97.63% | 222.1 |
| 8 | png  | png-lossy  | 10/10 | 47.53  | 87.75% | 2445.1 |
| 9 | png  | webp-lossy | 10/10 | 100.31 | 99.65% | 1158.7 |
| 10 | png | avif       | 10/10 | 6.60   | 99.89% | 17613.6 |
| 11 | webp | jpeg-moz   | 10/10 | 47.97  | 91.61% | 2971.1 |
| 12 | webp | jpeg-turbo | 10/10 | 97.70  | 82.30% | 1458.8 |
| 13 | webp | png-lossy  | 10/10 | 23.89  | 52.88% | 5966.6 |
| 14 | webp | webp-lossy | 10/10 | 34.96  | 95.94% | 4076.8 |
| 15 | webp | avif       | 10/10 | 2.83   | 97.16% | 50337.8 |
| 16 | avif | jpeg-moz   | 10/10 | 113.11 | 95.02% | 2268.8 |
| 17 | avif | jpeg-turbo | 10/10 | 197.67 | 92.77% | 1298.3 |
| 18 | avif | png-lossy  | 10/10 | 56.18  | 68.41% | 4568.4 |
| 19 | avif | webp-lossy | 10/10 | 94.75  | 94.81% | 2708.7 |
| 20 | avif | avif       | 10/10 | 7.37   | 95.63% | 34818.2 |

### 与第四轮对比（同样本、同参数）

1. 20 组全部成功，功能无回归。
2. 大多数用例耗时小幅上升（约 1%~6%），AVIF 目标用例上升更明显（约 10%~15%）。
3. 当前结论：该轮优化在本机当次跑测中未体现出速度收益，建议后续按固定 CPU 频率/空闲环境重复 3 轮取中位数再定最终结论。

### 明细文件

- CSV：`image-test/matrix-results-optimized-20260302-122551.csv`
- 日志目录：`image-test/matrix-logs-optimized-20260302-122551`
- 输出目录：`image-test/matrix-output-optimized-20260302-122551`

---

## 第六轮矩阵基准（分阶段耗时统计，20 组）

**测试日期**：2026-03-02  
**输入集**：`image-test/matrix-input-10-20260302`（固定样本，非随机）  
**输出配置**：`jpeg-moz / jpeg-turbo / png-lossy / webp-lossy / avif`  
**统一参数**：`-q 80 -t 12 --overwrite`

### 全局阶段占比（20 组合计）

| 阶段 | 耗时 (ms) | 占比 |
|---|---:|---:|
| 读文件 | 0 | 0.00% |
| 解码 | 145430 | 8.56% |
| PNG优化 | 0 | 0.00% |
| 编码 | 1552625 | 91.42% |
| 写出 | 198 | 0.01% |
| 合计 | 1698253 | 100% |

### 按输出配置的阶段分布

| 输出配置 | 平均总耗时 (ms) | 解码占比 | 编码占比 | 写出占比 |
|---|---:|---:|---:|---:|
| jpeg-moz | 1838.9 | 41.92% | 58.05% | 0.03% |
| jpeg-turbo | 892.3 | 88.21% | 11.67% | 0.12% |
| png-lossy | 4529.8 | 17.06% | 82.90% | 0.04% |
| webp-lossy | 2567.0 | 29.98% | 70.00% | 0.02% |
| avif | 34701.4 | 2.17% | 97.83% | 0.00% |

### 分析结论

1. 主要瓶颈是 **编码阶段**（整体 91.42%）。
2. `avif` 输出几乎完全受编码阶段支配（97.83%），应优先优化 AVIF 编码链路。
3. I/O 不是当前瓶颈（写出仅 0.01%）。
4. 读文件阶段当前为 0，是因为普通路径使用 `image::open`（读文件时间被计入解码阶段）；如需拆分读/解码，需要改为显式 `fs::read + load_from_memory` 的统计模式。

### 明细文件

- CSV：`image-test/matrix-results-staged-20260302-123949.csv`
- 日志目录：`image-test/matrix-logs-staged-20260302-123949`
- 输出目录：`image-test/matrix-output-staged-20260302-123949`

---

## 第七轮矩阵对比（编码优化实现后，20 组）

**测试日期**：2026-03-02  
**输入集**：`image-test/matrix-input-10-20260302`（固定样本，非随机）  
**统一参数**：`-q 80 -t 12 --overwrite`  
**构建核验**：

```powershell
cargo check
cargo check --features "img-moz,img-turbo"
cargo build --release --features "img-moz,img-turbo"
```

> 说明：`cargo build` 可通过；当前环境会出现 `build.rs` 的 DLL 拷贝提示（未找到动态库目录时跳过），不影响已有可执行文件测试。

### 本轮实现的编码优化点

1. TurboJPEG 编码器改为线程内复用 `Compressor`，并使用 `compress_to_owned` 减少额外复制。
2. PNG 有损路径改为线程内复用 `imagequant` attributes。
3. AVIF 编码增加内部线程控制（外层并行大于 1 时，内部编码线程设为 1）。
4. 编码阶段统计拆分为 `pixel_convert_ms` 与 `codec_ms`，用于更细粒度定位瓶颈。
5. `imagequant` 依赖关闭默认特性（`default-features = false`）。

### 与第六轮（分阶段基线）对比

**对比文件**：

- 基线：`image-test/matrix-results-staged-20260302-123949.csv`
- 优化后：`image-test/matrix-results-encopt-20260302-130430.csv`

| 指标 | 第六轮基线 | 第七轮优化后 | 变化 |
|---|---:|---:|---:|
| 平均总耗时 (ms/组) | 8905.87 | 10195.30 | **+14.48%** |
| 平均吞吐 (MB/s) | 90.06 | 86.46 | **-3.99%** |
| 成功组数 | 20 / 20 | 20 / 20 | 0 |
| 输出字节完全一致组数 | - | 12 / 20 | - |

### 按输出配置对比（4 输入求平均）

| 输出配置 | 基线平均总耗时 (ms) | 优化后平均总耗时 (ms) | 变化 | 吞吐变化 |
|---|---:|---:|---:|---:|
| jpeg-moz | 1838.88 | 1945.52 | +5.80% | -4.86% |
| jpeg-turbo | 892.32 | 903.60 | +1.26% | -4.30% |
| png-lossy | 4529.75 | 4718.35 | +4.16% | -3.75% |
| webp-lossy | 2567.00 | 2570.60 | +0.14% | -0.99% |
| avif | 34701.40 | 40838.40 | **+17.69%** | **-15.13%** |

### 第七轮阶段占比（20 组合计）

| 阶段 | 耗时 (ms) | 占比 |
|---|---:|---:|
| 解码 | 147730 | 7.40% |
| 像素转换 | 19369 | 0.97% |
| 编解码器 | 1830147 | 91.62% |
| 写出 | 207 | 0.01% |
| 合计 | 1997453 | 100% |

### 输出字节变化（8/20 组）

- `jpg -> png-lossy`: `69975970 -> 69981810`
- `jpg -> avif`: `3089060 -> 3031770`
- `png -> png-lossy`: `14931350 -> 14921970`
- `png -> avif`: `132130 -> 108190`
- `webp -> png-lossy`: `70417280 -> 70414710`
- `webp -> avif`: `4244950 -> 4276110`
- `avif -> png-lossy`: `85002120 -> 85009830`
- `avif -> avif`: `11766800 -> 11752140`

### 结论

1. 这轮“编码侧优化”在当前机器和固定样本下没有带来整体收益，表现为总耗时上升、吞吐下降。
2. 主要退化集中在 AVIF 目标输出（`+17.69%`），与编码器阶段主导（`91.62%`）一致。
3. 由于 8 组输出字节发生变化，说明这轮优化包含了影响编码路径结果的改动，后续应做开关化 AB 测试逐项验证收益。

### 明细文件

- CSV：`image-test/matrix-results-encopt-20260302-130430.csv`
- 日志目录：`image-test/matrix-logs-encopt-20260302-130430`
- 输出目录：`image-test/matrix-output-encopt-20260302-130430`

---

## 第八轮 AB 验证（单变量，定位负收益）

**测试日期**：2026-03-02  
**输入集**：`image-test/matrix-input-10-20260302`（固定样本）  
**范围**：只测受影响链路（`jpeg-turbo / png-lossy / avif`），每个链路覆盖 4 种输入格式（各 10 张）  
**可执行文件**：`target/release-img/xun.exe`（`cargo build --profile release-img --features "img-moz,img-turbo"`）  
**AB 明细文件**：

- 主结果：`image-test/ab-results-releaseimg-20260302-135519.csv`
- 日志：`image-test/ab-logs-releaseimg-20260302-135519`
- 输出：`image-test/ab-output-releaseimg-20260302-135519`

### 变量与结果（相对 baseline）

| 变量 | 观测结果 | 结论 |
|---|---|---|
| `avif_threads = 1` | `avif` 平均总耗时 **+9.74%**，吞吐 **-9.60%** | 明确负收益，默认不应强制 1 线程 |
| 禁用 turbo 复用 | `jpeg-turbo` 平均总耗时 `-0.90%`（接近噪声） | 复用无显著收益，可删简化逻辑 |
| 禁用 pngquant 复用 | `png-lossy` 平均总耗时 `-0.65%`（接近噪声） | 复用无显著收益，可删简化逻辑 |

### imagequant 默认特性 AB（构建变量）

**测试文件**：

- `default-features = false` 基线：`image-test/ab-results-releaseimg-20260302-135519.csv`
- `default-features = true`：`image-test/ab-results-imagequant-default-20260302-140506.csv`
- 复测：`image-test/ab-results-imagequant-default-r2-20260302-140547.csv`

两次均值（仅 `png-lossy` 四组）相对 `default-features = false`：

- 平均总耗时：`-3.10%`
- 平均吞吐：`+4.49%`

> 备注：输出字节会变化（4/4 组均变化），但仍在同质量参数下，属于编码实现差异，不是错误。

### 最终落地决策

1. `AVIF` 默认使用内部线程 `auto`（`None`），不再强制 `1`。
2. 删除 `turbo` 与 `pngquant` 的线程本地复用代码（保留 `turbojpeg compress_to_owned`）。
3. `imagequant` 采用默认特性（移除 `default-features = false`）。
4. 增加性能构建 profile：`release-img`（速度优先），与现有 `release`（体积优先）并存。
5. CLI 新增 `--avif-threads`，在需要时可显式指定。

---

## 第九轮最终回归（全量 20 组，最终代码）

**测试日期**：2026-03-02  
**输入集**：`image-test/matrix-input-10-20260302`（固定样本，4 输入格式 × 各 10 张）  
**输出配置**：`jpeg-moz / jpeg-turbo / png-lossy / webp-lossy / avif`  
**构建**：`cargo build --profile release-img --features "img-moz,img-turbo"`  
**统一参数**：`-q 80 -t 12 --overwrite`

### 本轮结果

- 结果 CSV：`image-test/final20-20260302-150546/results.csv`
- 日志目录：`image-test/final20-20260302-150546/logs`
- 输出目录：`image-test/final20-20260302-150546/output`
- 成功率：`20/20`

| 输出配置 | 平均总耗时 (ms) | 平均吞吐 (MB/s) | 平均节省空间 |
|---|---:|---:|---:|
| jpeg-moz | 1488.07 | 114.67 | 94.56% |
| jpeg-turbo | 578.38 | 353.36 | 89.78% |
| png-lossy | 2791.78 | 57.81 | 58.68% |
| webp-lossy | 1881.40 | 86.11 | 96.46% |
| avif | 22818.58 | 7.31 | 97.35% |

### 与第六轮基线对比（`matrix-results-staged-20260302-123949.csv`）

全局平均：

- 总耗时：`-29.86%`
- 吞吐：`+49.14%`

按输出配置：

| 输出配置 | 总耗时变化 | 吞吐变化 |
|---|---:|---:|
| jpeg-moz | -17.25% | +23.60% |
| jpeg-turbo | -34.59% | +69.89% |
| png-lossy | -38.18% | +62.75% |
| webp-lossy | -24.36% | +34.21% |
| avif | -34.93% | +55.28% |

### 输出一致性

与第六轮基线逐用例比对（20 组）：

- `output_bytes` 一致：`20/20`
- 说明：本轮性能提升未引入输出体积回归。

### 可合入改动（基于 AB + 全量回归）

1. 合入：`release-img` 性能 profile（速度优先构建）。
2. 合入：AVIF 默认内部线程改为 `auto(None)`，并保留 `--avif-threads` 可控开关。
3. 合入：`imagequant` 使用默认特性（在本机数据集下对 `png-lossy` 有稳定收益）。
4. 合入：`turbojpeg compress_to_owned`（减少不必要复制路径）。
5. 不合入：`turbo` / `pngquant` 线程本地复用（收益接近噪声，增加复杂度，已回退）。
