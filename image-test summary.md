# 图像编码基准总结（4 图样本，q80）

## 测试范围
- 输入样本：`A.jpg`、`A.png`、`A.webp`、`A.avif`（各 1 张）
- 输入目录：`D:/100_Projects/110_Daily/Xun/image-test/input-a4-20260301-190432`
- 测试参数：`--format all --threads 10 --limit 1 -q 80 --webp-lossless-backend both`
- 对比二进制：
  - turbo 版：`reference/img-bench/target2/release/img-bench.exe`
  - moz 版：`reference/img-bench/target2-moz/release/img-bench.exe`
- 输出目录：
  - turbo：`D:/100_Projects/110_Daily/Xun/image-test/output/run-1772363102-turbo-fmt-all-q80-only--`
  - moz：`D:/100_Projects/110_Daily/Xun/image-test/output/run-1772364509-moz-fmt-all-q80-only--`

## 核心结论
- JPG 编码：
  - 耗时最短：`turbojpeg`
  - 体积最小：`mozjpeg`
  - `jpegli` 明显更慢（单图常见 30s~80s），不建议作为默认编码器
- PNG 编码：
  - 有损综合最优：`pngquant (q60-80)`
  - 无损综合最优：`oxipng lv2`（体积接近极限、耗时可控）
  - 极限最小：`zopflipng -m`，但单图可达数百秒，不适合常规批处理
- WebP 编码：
  - 有损推荐：`libwebp lossy q80`
  - 无损：体积优先 `libwebp lossless`，速度优先 `image-rs lossless`
- AVIF 编码：
  - 本次样本中 `ravif q80 sp7` 同时优于 `libavif q80 sp7`（更快且更小）

## 推荐矩阵（本格式压缩）
| 输入=输出 | 速度优先 | 体积优先 | 默认建议 |
|---|---|---|---|
| JPG->JPG | turbojpeg | mozjpeg | 离线用 mozjpeg，实时用 turbojpeg |
| PNG->PNG（有损） | pngquant | pngquant | 允许有损时首选 pngquant |
| PNG->PNG（无损） | oxipng lv2 | zopflipng -m | 默认 oxipng lv2；zopflipng 仅做极限压缩 |
| WebP->WebP（有损） | libwebp lossy | libwebp lossy | 默认 libwebp lossy |
| WebP->WebP（无损） | image-rs lossless | libwebp lossless | 按速度/体积目标二选一 |
| AVIF->AVIF | ravif | ravif | 默认 ravif |

## 推荐矩阵（格式转换）
| 目标格式 | 推荐方案 |
|---|---|
| 转 JPG | 速度：turbojpeg；体积：mozjpeg |
| 转 PNG | 有损：pngquant；无损：oxipng lv2；极限：zopflipng -m（仅离线专项） |
| 转 WebP | 有损：libwebp lossy q80；无损：libwebp lossless（更小）/image-rs lossless（更快） |
| 转 AVIF | ravif q80 sp7（本次数据优于 libavif） |

## 落地建议
1. 常规批处理默认加 `--no-slow`，避免 `zopflipng` 拉长整体时间。
2. 需要“极限 PNG 体积”时，单独跑 `--only zopflipng` 专项任务。
3. 保持 `-q 80` 作为统一默认质量；再按业务目标微调（如 `-q 75` 或 `-q 85`）。
