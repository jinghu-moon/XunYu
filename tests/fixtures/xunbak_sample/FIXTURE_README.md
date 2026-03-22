# xunbak 测试 Fixture

模拟真实项目结构，覆盖 `.xunbak` 容器设计文档中的各类场景。

## 目录结构

```
xunbak_sample/
├── Cargo.toml                     # 配置文件（可压缩文本）
├── README.md                      # Markdown 文档
├── empty.txt                      # 空文件（0 字节）
├── src/
│   ├── main.rs                    # Rust 源码（可压缩，~1KB）
│   ├── duplicate_a.txt            # 去重测试 A（与 docs/duplicate_b.txt 内容相同）
│   ├── core/
│   │   ├── mod.rs
│   │   └── engine.rs              # 较大源码文件（~4KB，高压缩率）
│   └── utils/
│       ├── mod.rs
│       └── helpers.rs             # 工具函数
├── config/
│   ├── settings.json              # JSON 配置
│   ├── app.toml                   # TOML 配置
│   ├── .editorconfig              # 隐藏文件（点开头）
│   ├── readonly_file.txt          # Windows READONLY 属性
│   └── hidden_file.txt            # Windows HIDDEN 属性
├── docs/
│   ├── api/
│   │   └── endpoints.md           # API 文档
│   ├── 设计说明.md                # 中文文件名文档
│   ├── sample.log                 # 2000 行日志（~160KB，重复模式，高压缩率）
│   └── duplicate_b.txt            # 去重测试 B（与 src/duplicate_a.txt 内容相同）
├── assets/
│   ├── images/
│   │   ├── photo.jpg              # 伪 JPEG（应跳过压缩）
│   │   └── icon.png               # 伪 PNG（应跳过压缩）
│   ├── archives/
│   │   ├── data.zip               # 伪 ZIP（应跳过压缩）
│   │   └── backup.7z              # 伪 7z（应跳过压缩）
│   └── random.bin                 # 64KB 随机二进制（不可压缩）
├── 中文目录/
│   ├── 说明.txt                   # 中文路径 + 中文内容
│   └── 子目录/
│       └── 配置.json              # 深层中文路径
├── path with spaces/
│   ├── readme with spaces.txt     # 路径含空格
│   └── nested folder/
│       └── notes file.txt         # 多层空格路径
├── deep/
│   └── level1/.../level4/
│       └── leaf.txt               # 5 层深嵌套
└── empty_dir/
    └── .gitkeep                   # 模拟空目录（gitkeep 占位）
```

## 覆盖的测试场景

| 场景 | 对应文件/目录 | 设计章节 |
|------|--------------|---------|
| 可压缩文本 | `src/*.rs`, `docs/*.md`, `config/*` | §12 |
| 不可压缩（已压缩） | `assets/images/*`, `assets/archives/*` | §12.5 |
| 不可压缩（随机二进制） | `assets/random.bin` | §12.5 |
| 空文件 | `empty.txt` | §13.6 |
| 内容去重 | `src/duplicate_a.txt` vs `docs/duplicate_b.txt` | §13.4 |
| 中文路径 | `中文目录/**` | §8.3 |
| 路径含空格 | `path with spaces/**` | §8.3 |
| 深层嵌套 | `deep/level1/.../leaf.txt` | 目录结构 |
| Windows 属性 (readonly) | `config/readonly_file.txt` | §8.6 |
| Windows 属性 (hidden) | `config/hidden_file.txt` | §8.6 |
| 高压缩率日志 | `docs/sample.log` (~160KB) | §12.3 |
| 点开头文件 | `config/.editorconfig` | 边界 |
| 中文文件名 | `docs/设计说明.md` | §8.3 |
| 多种文件类型 | `.rs .md .toml .json .log .txt .jpg .png .zip .7z .bin` | §12 |

## 文件统计

- 总文件数：~22
- 总大小：~250 KB
- 可压缩文件：~15（文本/源码/日志）
- 不可压缩文件：~6（图片/压缩包/随机二进制）
- 空文件：1
- 去重对：1 对（2 文件相同内容）
