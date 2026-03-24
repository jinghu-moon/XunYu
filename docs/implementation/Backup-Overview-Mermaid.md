# Backup 概览 Mermaid 图

> 面向当前 XunYu 的 `backup / restore / convert / verify` 体系。
> 聚焦传统 `backup` 主链路，以及 `dir / zip / xunbak / 7z` 产物之间的关系。

---

## 1. 命令面

```mermaid
flowchart TD
    A[xun backup] --> B[默认传统 backup]
    A --> C[backup create]
    A --> D[backup restore]
    A --> E[backup convert]
    A --> F[backup list]
    A --> G[backup verify]
    A --> H[backup find]

    B --> B1[目录备份]
    B --> B2[zip 备份]
    B --> B3[xunbak 容器备份]

    C --> C1[format=dir]
    C --> C2[format=zip]
    C --> C3[format=7z]
    C --> C4[format=xunbak]

    D --> D1[按名称恢复]
    D --> D2[按路径恢复]
    D --> D3[按 file/glob 选择恢复]

    E --> E1[dir/zip/7z/xunbak 互转]
```

---

## 2. 传统 backup 主链路

```mermaid
flowchart TD
    A[读取 .xun-bak.json] --> B[解析 include/exclude/gitignore]
    B --> C[扫描源目录]
    C --> D{diff-mode}

    D -->|auto| E[优先读取上一版 .bak-manifest.json]
    D -->|hash| F[强制使用 hash manifest]
    D -->|meta| G[仅用 size+mtime baseline]

    E --> E1{上一版 manifest 存在?}
    E1 -->|是| H[scan + hash cache + hash diff]
    E1 -->|否| I[fresh full]

    F --> F1{上一版 manifest 存在?}
    F1 -->|是| H
    F1 -->|否| J[直接报错]

    G --> K[metadata diff]
    H --> L[得到 New / Modified / Reused / Unchanged / Deleted]
    I --> L
    K --> L

    L --> M[apply_diff]
    M --> M1[复制 New/Modified]
    M --> M2[hardlink 复用 Unchanged/Reused]
    M --> M3[Deleted 不写入新版本]

    M --> N[写 .bak-manifest.json]
    N --> O[写 .bak-meta.json]
    O --> P{compress?}
    P -->|false| Q[保留目录]
    P -->|true| R[打包 zip]
    Q --> S[retention]
    R --> S
    S --> T[list/find/verify/restore 消费结果]
```

---

## 3. 哈希增量数据关系

```mermaid
flowchart LR
    A[源目录文件] --> B[scan.rs]
    B --> C[content_hash]
    B --> D[mtime_ns / created_time_ns / win_attributes]
    C --> E[hash_cache]
    C --> F[.bak-manifest.json]

    F --> G[path_index]
    F --> H[content_index]

    G --> I[同路径判定<br/>Unchanged / Modified]
    H --> J[跨路径复用判定<br/>Reused / Rename]
    I --> K[diff 结果]
    J --> K

    K --> L[apply_diff]
    L --> M[新备份目录/zip]
    M --> N[restore]
    M --> O[verify]
    M --> P[list/find]
```

---

## 4. 备份结果与消费关系

```mermaid
flowchart TD
    A[backup result] --> B[目录 backup]
    A --> C[zip backup]
    A --> D[7z / xunbak artifact]

    B --> B1[.bak-manifest.json]
    B --> B2[.bak-meta.json]

    C --> C1[内嵌 .bak-manifest.json]
    C --> C2[旁路 .meta.json]

    B1 --> E[下一次增量 baseline]
    C1 --> E

    B1 --> F[backup verify]
    C1 --> F

    B --> G[restore]
    C --> G
    D --> G
```

---

## 5. 说明

1. 传统 `backup` 现在已经是 **hash 驱动增量**。
2. `diff-mode=auto|hash|meta` 控制增量判定方式。
3. `.bak-manifest.json` 是传统 backup 的权威快照元数据。
4. `hash_cache` 只是性能优化，不是真相来源。
5. `backup create / restore / convert` 统一负责多格式产物。
