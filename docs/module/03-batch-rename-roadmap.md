# batch-rename 功能规划

> 本文档记录 `xun brn` 批量重命名模块的待实现功能，含接口设计、使用示例与优先级评估。
> 当前已实现：`--regex`、`--case`、`--prefix`、`--suffix`、`--strip-prefix`、`--seq`、`--undo`。
> 参考工具：f2、rnr、ReNamer、PowerRename、Krename。

---

## 设计原则

- **多操作组合**：支持多操作同时指定（如 `--case kebab --from " " --to "_"`），按声明顺序依次应用。限制单一模式会逼用户多次调用，反而更危险。
- **两阶段执行**：apply 前必须完成全量冲突检测和环形依赖检测，通过后再一次性执行所有重命名，不允许部分成功后遇冲突退出（类似数据库两阶段提交）。
- **dry-run 优先**：所有操作默认预览，`--apply` 才执行，`--apply -y` 跳过确认。
- **明确反馈**：跳过的文件必须有反馈，但反馈级别取决于操作语义：①「要求匹配」类操作（`--strip-suffix`、`--strip-prefix`）——无匹配时输出 Warning，因为无匹配表示不符合预期；②「全目录扫描替换」类操作（`--from`/`--to`、`--regex`）——无匹配时**静默跳过**，dry-run 列表中标注「无变化」即可，不输出 Warning，因为目录中有些文件不含目标字符串是完全正常的。
- **幂等 undo**：每次 apply 覆盖 undo 文件，`--undo` 可安全重复执行。
- **模板优先级**：`--template` 是独立的最终命名阶段，不接收前序操作的中间结果。`{stem}` 始终指**原始词干**（未经任何操作处理的文件名主干）；若需要用处理后的词干，应在模板变量中显式指定（如 `{lower}`、`{upper}`）。若同时指定 `--case kebab --template "{stem}{ext}"`，kebab 转换**不会**影响 `{stem}` 的值，结果等同于只用 `--template`。
- **运行时意外**：预检阶段无法发现文件被占用（Windows `ERROR_SHARING_VIOLATION`）等运行时错误。遇到此类错误时，记录 Warning，继续处理其余文件，执行结束后汇报失败列表，不中断整批操作。
- **两阶段预检清单**：① 冲突检测 ② 环形依赖检测 ③ 输出名非法字符检测（`\ / : * ? " < > |`）④ Windows 保留名检测（`CON`、`NUL`、`COM1`–`COM9`、`LPT1`–`LPT9` 等，无论加何种扩展名均非法）

---

## 优先级定义

| 级别 | 说明 |
|------|------|
| P1 | 高频需求，优先实现 |
| P2 | 中频需求，次优先 |
| P3 | 低频/进阶需求，按需实现 |

---

## 一、字符处理类

### 1.1 字面量替换 `--from` / `--to` P1

将词干中的指定字符串替换为另一个字符串，无需正则表达式。使用双参数风格避免分隔符冲突（单参数 `:` 分隔符无法处理含冒号的字符串，如时间戳 `12:30`）。支持多组，按声明顺序依次应用。

**接口设计：**

```
xun brn <dir> --from "<old>" --to "<new>" [--from "<old2>" --to "<new2>" ...]
```

**示例：**

```
# 空格替换为下划线
xun brn . --from " " --to "_"
  my file.txt  ->  my_file.txt

# 替换含冒号的字符串（双参数风格无歧义）
xun brn . --from "12:30" --to "1230"
  record_12:30.txt  ->  record_1230.txt

# 多组替换（顺序应用）
xun brn . --from " " --to "_" --from "(" --to "" --from ")" --to ""
  my file (2024).txt  ->  my_file_2024.txt
```

**与 `--regex` 的区别：** 字面匹配，无需转义特殊字符，适合日常使用。

**配对规则：** 每个 `--from` 必须紧跟一个 `--to`，数量不匹配时报错退出。`--from A --from B --to C` 是非法输入。

**no-op 行为：** 若文件名不包含 `--from` 指定的字符串，该文件**静默跳过**（dry-run 输出中标注「无变化」即可）。不输出 Warning——目录中有些文件不含指定字符串是完全正常的，无需提示。

---

### 1.2 删除指定字符 `--remove-chars` P3

从词干中删除所有出现的指定字符集合。

**接口设计：**

```
xun brn <dir> --remove-chars "<chars>"
```

**示例：**

```
# 删除方括号和圆括号
xun brn . --remove-chars "[]()"
  file[1](copy).txt  ->  file1copy.txt

# 删除所有数字
xun brn . --remove-chars "0123456789"
  track01.mp3  ->  track.mp3
```

---

### 1.3 删除括号及其内容 `--strip-brackets` P2

删除词干中成对括号（含括号内的全部内容）。使用枚举式语法，避免字符串参数被误解为字符集合。删除后自动 trim 首尾空白。

**接口设计：**

```
xun brn <dir> --strip-brackets round|square|curly [--strip-brackets ...]
```

**示例：**

```
xun brn . --strip-brackets round
  song (2024) (official).mp3  ->  song.mp3

xun brn . --strip-brackets round --strip-brackets square
  movie [BluRay] (2024).mkv  ->  movie.mkv
```

---

### 1.4 去除首尾空白与特殊字符 `--trim` P2

去除词干首尾的指定字符。不指定时默认 trim 空格。

**接口设计：**

```
xun brn <dir> --trim ["<chars>"]
```

**示例：**

```
xun brn . --trim
  " file ".txt  ->  file.txt

xun brn . --trim " _-"
  "_-file-_".txt  ->  file.txt
```

---

### 1.5 移除词干后缀 `--strip-suffix` P1

与 `--strip-prefix` 对称，移除词干末尾的指定字符串。无该后缀的文件输出 Warning 并跳过。

**接口设计：**

```
xun brn <dir> --strip-suffix "<suffix>"
```

**示例：**

```
xun brn . --strip-suffix "_v2"
  file_v2.txt  ->  file.txt
  other.txt    ->  (skipped, warning)
```

---

### 1.6 截取词干 `--slice` P2

按字符位置截取词干，支持正负下标（负数从末尾计）。替代原来拟定的 `--drop-prefix-n` / `--drop-suffix-n`（两者完全重叠，合并为一个参数）。

**接口设计：**

```
xun brn <dir> --slice "<start>[:<end>]"
```

**示例：**

```
# 取前8个字符
xun brn . --slice ":8"
  very_long_filename.txt  ->  very_lon.txt

# 去掉前4个字符（相当于 --drop-prefix-n 4）
xun brn . --slice "4:"
  001_file.txt  ->  file.txt

# 去掉最后3个字符（相当于 --drop-suffix-n 3）
xun brn . --slice ":-3"
  file_v2.txt  ->  file_v.txt
```

> `--drop-prefix-n N` 等价于 `--slice "N:"`，`--drop-suffix-n N` 等价于 `--slice ":-N"`，不单独实现。

---

## 二、大小写类

### 2.1 扩展名大小写规范化 `--ext-case` P1

单独对扩展名进行大小写转换，不影响词干。

**接口设计：**

```
xun brn <dir> --ext-case upper|lower
```

**示例：**

```
xun brn . --ext-case lower
  photo.JPG   ->  photo.jpg
  image.JPEG  ->  image.jpeg
```

---

### 2.2 词首字母大写 `--case title` P2

在现有 `--case` 的基础上增加 `title` 样式。

**示例：**

```
xun brn . --case title
  hello world file.txt  ->  Hello World File.txt
```

---

## 三、编号类

### 3.1 序号位置与模式扩展 P1

扩展现有 `--seq`，补全前置序号和纯序号模式。

**新增参数：**

```
xun brn <dir> --seq [--seq-pos prefix|suffix] [--seq-only] [--start N] [--pad N]
```

| 参数 | 说明 | 默认 |
|------|------|------|
| `--seq-pos prefix` | 序号放词干前：`001_file.txt` | suffix |
| `--seq-pos suffix` | 序号放词干后：`file_001.txt`（当前行为）| — |
| `--seq-only` | 纯序号，不保留词干：`001.jpg` | false |

**示例：**

```
# 前置序号
xun brn . --seq --seq-pos prefix --pad 3
  photo.jpg  ->  001_photo.jpg

# 纯序号（整理照片）
xun brn . --seq --seq-only --pad 4
  DSC001.jpg  ->  0001.jpg
```

---

### 3.2 已有编号规范化 `--normalize-seq` P2

**保留文件原有顺序**，只统一补零宽度。与 `--renumber`（重新排序）不同。

操作对象为词干中**最后一组连续数字**（如 `track_1_v2` 处理 `2`，`ep01s02` 处理 `2`）。文件名含多组数字时，仅最后一组参与规范化。

**接口设计：**

```
xun brn <dir> --normalize-seq [--pad N]
```

**示例：**

```
xun brn . --normalize-seq --pad 3
  track_1.mp3      ->  track_001.mp3
  track_10.mp3     ->  track_010.mp3
  track_100.mp3    ->  track_100.mp3
  ep01s2.mp4       ->  ep01s002.mp4   # 只处理最后一组数字 "2"
```

---

### 3.3 按时间排序编号 `--seq-by` P2

`--seq` 的排序依据，默认 `name`（文件名自然排序，即 `1 < 2 < 10` 而非字典序 `1 < 10 < 2`），可改为按修改/创建时间。

**接口设计：**

```
xun brn <dir> --seq --seq-by name|mtime|ctime
```

| 值 | 说明 |
|-----|------|
| `name`（默认）| 文件名自然排序（Human Sorting）：`1,2,...,10` 而非 `1,10,2` |
| `mtime` | 按最近修改时间升序 |
| `ctime` | 按创建时间升序 |

---

### 3.4 重新编号 `--renumber` P3

对文件名中已有的数字序号重新排序编号（与 `--normalize-seq` 区别：这里会改变数字的相对大小）。

**接口设计：**

```
xun brn <dir> --renumber [--start N] [--pad N]
```

**示例：**

```
xun brn . --renumber --start 1 --pad 3
  track_3.mp3   ->  track_001.mp3
  track_7.mp3   ->  track_002.mp3
  track_15.mp3  ->  track_003.mp3
```

---

## 四、位置操作类

### 4.1 指定位置插入字符串 `--insert-at` P1

在词干的指定字符位置插入字符串。使用双参数风格，避免插入内容本身含冒号（如时间戳 `12:30`）时产生歧义。支持正负下标和具名关键词（避免 `-0` 语义不直观）。

**接口设计：**

```
xun brn <dir> --insert-at <pos> --insert-str "<str>"
```

`<pos>` 取值：
- 正整数：从词干开头第 N 位后插入
- 负整数：从词干末尾第 N 位前插入
- `start`：词干最前面（等价于 `--prefix`）
- `end`：词干最后面（等价于 `--suffix`）

**示例：**

```
# 在第4位后插入分隔符
xun brn . --insert-at 4 --insert-str "_"
  20240315photo.jpg  ->  2024_0315photo.jpg

# 插入含冒号的字符串（双参数风格无歧义）
xun brn . --insert-at 8 --insert-str "T12:30"
  20240315.log  ->  20240315T12:30.log

# 在末尾插入（用具名关键词，语义清晰）
xun brn . --insert-at end --insert-str "_final"
  report.docx  ->  report_final.docx

# 在开头插入（等价于 --prefix）
xun brn . --insert-at start --insert-str "IMG_"
  photo.jpg  ->  IMG_photo.jpg
```

---

## 五、扩展名操作类

### 5.1 批量改扩展名 `--ext-from` / `--ext-to` P1

将指定扩展名批量替换为另一个扩展名，匹配时忽略大小写。使用双参数风格与 `--from`/`--to` 保持一致，减少记忆负担。

**接口设计：**

```
xun brn <dir> --ext-from <old> --ext-to <new>
```

**示例：**

```
xun brn . --ext-from jpeg --ext-to jpg
  photo.jpeg  ->  photo.jpg
  image.JPEG  ->  image.jpg

xun brn . --ext-from txt --ext-to md
  readme.txt  ->  readme.md
```

---

### 5.2 添加扩展名 `--add-ext` P3

为无扩展名文件追加扩展名。

**接口设计：**

```
xun brn <dir> --add-ext "<ext>"
```

**示例：**

```
xun brn . --add-ext txt
  Makefile  ->  Makefile.txt
```

---

## 六、元数据类

### 6.1 插入文件日期 `--insert-date` P2

将文件的修改/创建时间以指定格式插入词干。本工具仅支持 Windows，`ctime` 即 Windows 文件创建时间（非 POSIX inode 变更时间）。

**本功能是 `--template` 的语法糖**，等价写法：

```
# --insert-date 等价于：
xun brn . --template "{date}_{stem}{ext}"

# --date-source ctime --date-pos suffix 等价于：
xun brn . --template "{stem}_{ctime}{ext}"
```

**接口设计：**

```
xun brn <dir> --insert-date [--date-source mtime|ctime] [--date-fmt <fmt>] [--date-pos prefix|suffix]
```

| 参数 | 说明 | 默认 |
|------|------|------|
| `--date-source` | mtime（最近修改时间）/ ctime（文件创建时间，Windows 专属语义）| mtime |
| `--date-fmt` | strftime 格式 | `%Y%m%d` |
| `--date-pos` | prefix / suffix | prefix |

**示例：**

```
xun brn . --insert-date
  photo.jpg  ->  20240315_photo.jpg

xun brn . --insert-date --date-source ctime --date-fmt "%Y-%m-%d" --date-pos suffix
  report.docx  ->  report_2024-03-15.docx
```

---

### 6.2 EXIF 日期重命名 `--exif-date` P3

读取图片 EXIF 中的拍摄时间重命名，需要额外依赖（exif 解析库）。

**接口设计：**

```
xun brn <dir> --exif-date [--date-fmt <fmt>] --ext jpg,jpeg,heic
```

**示例：**

```
xun brn . --exif-date --ext jpg
  DSC_001.jpg  ->  20240315_143022.jpg
```

---

## 七、模板类

### 7.1 模板命名 `--template` P1

使用模板字符串自由组合各种变量，一条命令覆盖多数复杂命名需求。**升为 P1**：模板是最高价值功能，早期实现后 `--seq-only`、`--insert-date`、`--seq-pos` 等均可成为语法糖，避免重复维护。

**支持的模板变量：**

| 变量 | 说明 | 示例值 |
|------|------|--------|
| `{stem}` | 原词干 | `photo` |
| `{ext}` | 扩展名（含点）| `.jpg` |
| `{n}` | 序号（受 `--start`、`--pad` 控制）| `001` |
| `{n:step=2}` | 步进序号（如 2,4,6...）| `002` |
| `{date}` | 文件修改日期（受 `--date-fmt` 控制）| `20240315` |
| `{mtime}` | 修改时间（独立格式，不受全局 `--date-fmt` 限制）| `20240315_143022` |
| `{ctime}` | 创建时间（Windows 专属：文件创建时间）| `20240315` |
| `{parent}` | 父目录名 | `vacation` |
| `{upper}` | 词干大写 | `PHOTO` |
| `{lower}` | 词干小写 | `photo` |
| `{size}` | 文件大小（字节）| `204800` |
| `{hash:8}` | 文件内容哈希前N位（去重场景）⚠️ 需读取全文件内容，大文件批量时性能显著下降；与 `--apply` 组合时哈希在预检阶段一次性计算完成，执行阶段不重复读取 | `a3f2b1c4` |

**接口设计：**

```
xun brn <dir> --template "<模板字符串>" [--start N] [--pad N] [--date-fmt <fmt>]
```

**示例：**

```
# 日期_序号_原词干
xun brn . --template "{date}_{n}_{stem}{ext}" --pad 3
  photo.jpg   ->  20240315_001_photo.jpg

# 父目录名_序号（整理旅行照片）
xun brn . --template "{parent}_{n}{ext}" --pad 4
  vacation/DSC001.jpg  ->  vacation_0001.jpg

# 纯序号
xun brn . --template "{n}{ext}" --pad 4
  DSC001.jpg  ->  0001.jpg

# 步进序号
xun brn . --template "{n:step=2}_{stem}{ext}" --start 2
  a.txt  ->  002_a.txt
  b.txt  ->  004_b.txt
```

---

## 八、过滤与范围类

### 8.1 按名称过滤 `--filter` P2

只处理**完整文件名**（含扩展名）匹配指定 glob 模式的文件，而非仅词干匹配。

**接口设计：**

```
xun brn <dir> --filter "<glob>"
```

**示例：**

```
# 匹配完整文件名（含扩展名）
xun brn . --filter "*IMG*.jpg" --case lower
xun brn . --filter "[0-9]*.mp3" --slice "4:"

# 仅匹配词干（使用通配扩展名）
xun brn . --filter "*IMG*.*" --case lower
```

---

### 8.2 排除匹配文件 `--exclude` P2

排除**完整文件名**（含扩展名）匹配指定 glob 的文件，与 `--filter` 作用域一致。

**接口设计：**

```
xun brn <dir> --exclude "<glob>"
```

**示例：**

```
# 排除隐藏文件（以点开头的完整文件名）
xun brn . --case kebab --exclude ".*"

# 排除所有 .bak 文件
xun brn . --case kebab --exclude "*.bak"
```

---

### 8.3 递归深度控制 `--depth` P2

```
xun brn <dir> -r --depth <n>
```

```
# 只处理当前目录和直接子目录
xun brn . -r --depth 2 --case kebab
```

---

### 8.4 包含目录 `--include-dirs` P3

同时对目录名进行重命名（当前只处理文件）。

**遍历顺序**：必须采用**深度优先、从叶到根**（deepest first）顺序处理目录，先重命名最深层目录，再向上处理父目录。若先重命名父目录，子目录的绝对路径立即失效，导致后续操作全部出错。

---

## 九、执行与安全类

### 9.1 冲突策略 `--on-conflict` P1

当目标文件已存在时的处理策略。

**接口设计：**

```
xun brn <dir> --on-conflict abort|skip|rename-seq
```

| 策略 | 说明 |
|------|------|
| `abort`（默认）| 全量预检后退出，不执行任何重命名 |
| `skip` | 跳过有冲突的文件，继续处理其余文件 |
| `rename-seq` | 自动追加序号解决冲突：`file.txt -> file_1.txt` |

---

### 9.2 环形重命名检测 P1

批量重命名的经典陷阱：

```
a.txt -> b.txt
b.txt -> a.txt   # 直接执行时 b.txt 已被第一步覆盖
```

**处理方案：** apply 前进行依赖拓扑排序，检测到环路后引入临时文件中转打破环。临时名使用时间戳随机后缀避免与已有文件冲突，并纳入预检冲突检测范围：

```
a.txt -> __xun_brn_tmp_<timestamp>__.txt
b.txt -> a.txt
__xun_brn_tmp_<timestamp>__.txt -> b.txt
```

**重要**：环形检测必须作为两阶段执行的一部分，在 apply 前完成，不得在执行过程中发现。

---

### 9.3 NTFS 大小写中转 P1（Windows 专属内部逻辑）

Windows NTFS 大小写不敏感，`photo.JPG → photo.jpg` 对文件系统是同一个文件，直接 rename 会静默成功但实际无变化。必须通过临时名中转实现真正的大小写重命名。临时名同样使用时间戳随机后缀：

```
photo.JPG  →  __xun_brn_tmp_<timestamp>__.jpg  →  photo.jpg
```

**影响范围**：`--ext-case`、`--case`（仅大小写变化的情形）、以及任何可能导致输出名与输入名仅大小写不同的操作。实现时应在 apply 前检测此类情形，自动插入临时名中转步骤。

---

### 9.4 输出名合法性检测 P1（Windows 专属内部逻辑）

**非法字符检测**：Windows 文件名禁止包含 `\ / : * ? " < > |`。`--from`、`--to`、`--template`、`--insert-str` 等参数均可能产生含非法字符的输出名，必须在两阶段预检中拦截，给出明确错误信息而非等到 apply 时收到系统错误。

**保留名检测**：`CON`、`PRN`、`AUX`、`NUL`、`COM1`–`COM9`、`LPT1`–`LPT9` 无论附加何种扩展名均为非法文件名（`NUL.txt` 也是）。`--from "file" --to "aux"` 会产生无法访问的文件，预检阶段必须拦截。

---

### 9.5 多步 undo 历史 P3

当前 undo 只保留最后一次操作，可扩展为追加记录支持多步回退。

**接口设计：**

```
xun brn <dir> --undo           # 回退最后一次
xun brn <dir> --undo --steps 3 # 回退最近3次
xun brn <dir> --undo-list      # 查看历史
```

---

## 十、输出与交互类

### 10.1 预览差异高亮 P2

预览时对变更部分进行颜色高亮，而非只显示完整文件名对比。参考 rnr、f2 的做法：

```
photo (2024).jpg  ->  photo.jpg
      ━━━━━━━           # 红色标出删除部分
```

对 `--case kebab` 等全量转换，高亮能让用户一眼看出结果是否符合预期。

---

### 10.2 交互式确认模式 `--interactive` / `-i` P2

交互式逐条标记选择，选定后统一执行两阶段预检再 apply，不破坏两阶段执行原则。适合 `--regex` 等不确定结果的操作。

**`-i` 是独立标志**，等价于 `--apply` + 逐条确认，单独使用 `-i` 即可触发交互式执行，无需额外加 `--apply`。

**执行流程（三阶段）：**
1. **标记阶段**：逐条显示变更，用户按 `y/n/q` 标记每条是否执行
2. **预检阶段**：对所有标记为 `y` 的条目统一进行冲突检测和环形依赖检测
3. **执行阶段**：预检通过后一次性执行所有选中的重命名

**接口设计：**

```
xun brn <dir> --regex "..." --replace "..." -i
```

**交互流程：**

```
[1/98] photo_001.jpg  ->  photo-001.jpg  [y/n/q]? y
[2/98] IMG 002.jpg    ->  img-002.jpg    [y/n/q]? n
[3/98] ...
--- 标记完成，对 96 条执行预检 ---
预检通过，开始重命名...
```

---

### 10.3 JSON/CSV 输出 `--output-format` P2

```
xun brn <dir> --case kebab --output-format json
```

```json
{
  "total": 100,
  "effective": 98,
  "skipped": 2,
  "ops": [
    { "from": "my file.txt", "to": "my-file.txt" }
  ]
}
```

---

### 10.4 仅显示统计 `--count` P3

```
xun brn <dir> --case kebab --count
```

输出：`98 file(s) would be renamed (2 skipped).`

---

### 10.5 Unicode 规范化 `--normalize-unicode` P3

从 macOS 或 Linux 复制来的文件可能携带 NFD 编码的文件名，在 Windows 上显示相同但字节序列不同，导致搜索或脚本匹配失败。本命令将文件名统一转换为指定 Unicode 规范化形式（Windows 原生使用 NFC）。

**接口设计：**

```
xun brn <dir> --normalize-unicode nfc|nfd|nfkc|nfkd
```

---

## 实现优先级汇总

### P1（优先实现）

| # | 功能 | 参数 | 理由 |
|---|------|------|------|
| 1 | 字面量替换 | `--from` / `--to` | 最高频需求，比 `--regex` 门槛低 |
| 2 | 模板命名 | `--template` | 最高价值功能，后续功能可成为语法糖 |
| 3 | 移除词干后缀 | `--strip-suffix` | 与 `--strip-prefix` 对称 |
| 4 | 批量改扩展名 | `--ext-from` / `--ext-to` | 极高频，`.jpeg→.jpg` |
| 5 | 扩展名大小写 | `--ext-case` | 高频，`.JPG→.jpg` |
| 6 | 序号位置扩展 | `--seq-pos` / `--seq-only` | 补全前置/纯序号 |
| 7 | 冲突策略 | `--on-conflict` | 当前遇冲突直接退出，体验差 |
| 8 | 环形重命名检测 | 内部逻辑 | 安全性，批量工具必须处理 |
| 9 | 指定位置插入 | `--insert-at` + `--insert-str` | 高频实用，含具名关键词 |
| 10 | NTFS 大小写中转 | 内部逻辑 | Windows 专属，仅大小写变化时静默失败 |
| 11 | 输出名合法性检测 | 内部逻辑 | Windows 非法字符 + 保留名预检，防止 apply 时系统错误 |

### P2（次优先）

| # | 功能 | 参数 |
|---|------|------|
| 12 | 删除括号内容 | `--strip-brackets round\|square\|curly` |
| 13 | 去除首尾字符 | `--trim` |
| 14 | 截取词干 | `--slice`（含原 drop-prefix/suffix-n 场景）|
| 15 | 词首字母大写 | `--case title` |
| 16 | 按名称过滤 | `--filter` |
| 17 | 排除匹配文件 | `--exclude` |
| 18 | 递归深度控制 | `--depth` |
| 19 | 插入文件日期 | `--insert-date` |
| 20 | 预览差异高亮 | 渲染优化 |
| 21 | 交互式确认 | `--interactive` / `-i` |
| 22 | 已有编号规范化 | `--normalize-seq` |
| 23 | 按时间排序编号 | `--seq-by` |
| 24 | JSON/CSV 输出 | `--output-format` |

### P3（按需实现）

| # | 功能 | 参数 |
|---|------|------|
| 25 | 删除指定字符 | `--remove-chars` |
| 26 | EXIF 日期重命名 | `--exif-date` |
| 27 | 添加扩展名 | `--add-ext` |
| 28 | 包含目录重命名 | `--include-dirs` |
| 29 | 重新编号 | `--renumber` |
| 30 | 多步 undo 历史 | `--undo --steps` |
| 31 | 仅显示统计 | `--count` |
| 32 | Unicode 规范化 | `--normalize-unicode` |

---

## 十一、Dashboard UI 设计方案

### 11.1 核心设计理念：预览就是产品

批量重命名的最大用户焦虑是「会不会改错」。UI 的第一职责不是参数配置，而是**让用户在执行前完全确认结果**。所有设计决策都应服务于这个目标。

**交互原型参考**：`000_Inbox/020_Browser/021_Edge/brn_ui_mockup.html`

---

### 11.2 整体布局：三段式

```
┌─ 顶栏：目录输入（等宽字体）+ [选择目录] [预览] [执行重命名(红)] ──────────────┐
├─ 操作链（pipeline bar）：字面量替换 → 大小写转换  [+ 添加操作] ──────────────┤
├──────────────────────────────────────────────────────────────────────────────┤
│  左栏（~280px）              │  右栏（剩余空间）                              │
│  操作标签页导航              │  预览结果                                      │
│  ─────────────────           │  ┌─ 统计：98 将重命名 / 2 跳过（大字）────┐   │
│  [字面量替换]  ←当前          │  │ [全选] [仅有效]                          │   │
│  [大小写转换]                 │  ├──────────────────────────────────────────┤   │
│  [批量编号]                   │  │ ☑ photo (2024).jpg  →  photo.jpg                    │   │
│  │       ┄┄┄┄┄┄┄┄ 红色删除高亮                        │   │
│  │ ☑ IMG_001.JPG    →  img-001.jpg                    │   │
│  │ ☐ readme.txt     →  readme.txt  无变化              │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
├─ 底栏：dry-run 模式  Ctrl+Z  [撤销上次操作（执行后常驻）] ──┤
```

**关键比例**：预览区 > 配置区，预览才是目的。

---

### 11.3 操作链（Pipeline Bar）

将多操作组合显示为可视化芯片链，是与主流同类工具最大的区别点：

```
操作链：[字面量替换 ×] → [大小写转换 ×] → [批量编号 ×]  [+ 添加操作]
```

- 芯片顺序 = 操作执行顺序，与文档设计原则「按声明顺序依次应用」一致
- 点 × 删除单个操作，拖拽调整顺序（中期实现）
- 点 `+ 添加操作` 弹出操作选择菜单
- 当前激活的操作芯片高亮，左栏显示对应参数表单
- 参数任何变化**实时更新**预览，不需要手动点「刷新」

---

### 11.4 预览列表设计

**差异高亮**（来自原型 `.diff-del` / `.diff-add`）：

- 红色底色标出被删除/替换的内容
- 绿色底色标出新增/替换后的内容
- 无变化的行灰显 + `无变化` 标签；`[仅有效]` 按钮一键隐藏无变化行

**执行门控**：`[执行重命名]`（红色）仅在点击 `[预览]` 后才显示，强制用户先确认结果再执行，不依赖弹窗警告。

**逐行勾选**：每行有 checkbox，可排除个别不想改的文件；`[全选]` / `[仅有效]` 快捷操作。

**统计数字显眼**：「98 个将重命名 / 2 个跳过」放大字号，最需要一眼看到的信息。

**撤销常驻**：执行成功后底栏立刻出现 `[撤销上次操作]`，不藏在菜单里。

---

### 11.5 操作卡片定义（5 张，拆自原单卡片）

> 全部使用 `guardedTask`——大小写/格式转换同样是批量破坏性操作，不降级为 `runTask`。
> Regex 和 Template 拆为**两张独立卡片**（消除隐藏字段反模式），字段全部常驻。

#### 卡片 1：字面量替换

```typescript
guardedTask({
  id: 'brn-replace', workspace: 'integration-automation',
  title: '字面量替换',
  description: '将词干中的字符串替换为另一个字符串，无需正则表达式。',
  action: 'brn:replace', feature: 'batch_rename',
  fields: [
    { key: 'path',      label: '扫描目录',   type: 'text',     defaultValue: '.' },
    { key: 'from',      label: '查找',       type: 'text',     required: true, placeholder: '原字符串' },
    { key: 'to',        label: '替换为',     type: 'text',     placeholder: '新字符串（留空则删除）' },
    { key: 'ext',       label: '扩展名过滤', type: 'text',     placeholder: 'jpg,png' },
    { key: 'recursive', label: '递归扫描',   type: 'checkbox', defaultValue: false },
  ],
})
```

> **UI 限制**：Card 1 仅支持单组 `--from`/`--to`。多组替换请直接使用 CLI：`xun brn . --from " " --to "_" --from "(" --to ""`。

#### 卡片 2：大小写 / 格式转换

```typescript
guardedTask({
  id: 'brn-case',
  title: '大小写 / 格式转换',
  description: '统一命名风格，支持 kebab、snake、pascal、title，以及扩展名大小写。',
  fields: [
    { key: 'path',      label: '扫描目录',     type: 'text',     defaultValue: '.' },
    { key: 'case',      label: '命名风格',     type: 'select',   defaultValue: '', options: brnCaseOptions },
    { key: 'ext_case',  label: '扩展名大小写', type: 'select',   defaultValue: '', options: brnExtCaseOptions },
    { key: 'ext',       label: '扩展名过滤',  type: 'text' },
    { key: 'recursive', label: '递归扫描',    type: 'checkbox', defaultValue: false },
  ],
})
```

#### 卡片 3：批量编号

```typescript
guardedTask({
  id: 'brn-seq',
  title: '批量编号',
  description: '为文件追加或前置序号，支持自然排序。',
  fields: [
    { key: 'path',      label: '扫描目录',   type: 'text',     defaultValue: '.' },
    { key: 'seq_pos',   label: '序号位置',   type: 'select',   defaultValue: 'suffix', options: brnSeqPosOptions },
    { key: 'start',     label: '起始值',     type: 'number',   defaultValue: '1' },
    { key: 'pad',       label: '补零位数',   type: 'number',   defaultValue: '3' },
    { key: 'seq_by',    label: '排序依据',   type: 'select',   defaultValue: 'name', options: brnSeqByOptions },
    { key: 'ext',       label: '扩展名过滤', type: 'text' },
    { key: 'recursive', label: '递归扫描',   type: 'checkbox', defaultValue: false },
  ],
})
```

#### 卡片 4：Regex 替换（`tone: 'danger'`）

```typescript
guardedTask({
  id: 'brn-regex', tone: 'danger',
  title: 'Regex 替换',
  description: '正则表达式匹配词干并替换，支持捕获组 $1 $2。',
  notices: [{ text: '正则替换为高级功能，建议先预览确认结果。', tone: 'warning' }],
  fields: [
    { key: 'path',      label: '扫描目录',   type: 'text', defaultValue: '.' },
    { key: 'regex',     label: 'Regex',      type: 'text', required: true, placeholder: '^IMG_(\\d+)' },
    { key: 'replace',   label: 'Replace',    type: 'text', placeholder: 'photo_$1' },
    { key: 'ext',       label: '扩展名过滤', type: 'text' },
    { key: 'recursive', label: '递归扫描',   type: 'checkbox', defaultValue: false },
  ],
})
```

#### 卡片 5：模板命名

```typescript
guardedTask({
  id: 'brn-template',
  title: '模板命名',
  description: '用模板字符串自由组合文件名。变量：{stem} {ext} {n} {date} {parent} {upper} {lower}',
  notices: [{ text: '模板命名覆盖词干，建议先预览确认结果。', tone: 'info' }],
  fields: [
    { key: 'path',      label: '扫描目录', type: 'text', defaultValue: '.' },
    { key: 'template',  label: '模板',     type: 'text', required: true, placeholder: '{date}_{n}_{stem}{ext}' },
    { key: 'start',     label: '起始值',   type: 'number', defaultValue: '1' },
    { key: 'pad',       label: '补零位数', type: 'number', defaultValue: '3' },
    { key: 'ext',       label: '扩展名过滤', type: 'text' },
    { key: 'recursive', label: '递归扫描',   type: 'checkbox', defaultValue: false },
  ],
})
```

---

### 11.6 options 扩展（`catalog.options.ts`）

```typescript
// 扩展 brnCaseOptions，增加 title
export const brnCaseOptions: TaskFieldOption[] = [
  { label: '不转换', value: '' },
  { label: 'kebab',  value: 'kebab'  },
  { label: 'snake',  value: 'snake'  },
  { label: 'pascal', value: 'pascal' },
  { label: 'title',  value: 'title'  },  // 新增
  { label: 'upper',  value: 'upper'  },
  { label: 'lower',  value: 'lower'  },
]

export const brnExtCaseOptions: TaskFieldOption[] = [
  { label: '不转换', value: '' },
  { label: 'lower',  value: 'lower' },
  { label: 'upper',  value: 'upper' },
]

export const brnSeqPosOptions: TaskFieldOption[] = [
  { label: '后缀（默认）', value: 'suffix' },
  { label: '前缀',        value: 'prefix' },
  { label: '纯序号',      value: 'only'   },
]

export const brnSeqByOptions: TaskFieldOption[] = [
  { label: '文件名自然排序（默认）', value: 'name'  },
  { label: '修改时间',              value: 'mtime' },
  { label: '创建时间',              value: 'ctime' },
]
```

---

### 11.7 实现阶段规划

Phase 1 与 Phase 3 **合并为同一里程碑发布**：拆分卡片后若没有预览可视化，五张卡片只能各自显示终端文本，体验比单卡片更分散。因此卡片拆分和 `BrnPreviewPanel.vue` 必须同时落地，不单独发布中间态。

| 阶段 | 工作内容 | 前置依赖（必须先完成）|
|------|---------|------|
| **Phase 2** | 随 P1 CLI 落地同步补全 UI 字段：`--from`/`--to`、`--ext-from`/`--ext-to`、`--seq-pos`、`--insert-at`/`--insert-str`、`--template` | CLI P1 各功能实现 |
| **Phase 1+3（合并）** | 将现有单卡片拆分为 5 张卡片 + 更新 options + 前端接入 `BrnPreviewPanel.vue`（差异高亮列表）| CLI P2 `--output-format json` |
