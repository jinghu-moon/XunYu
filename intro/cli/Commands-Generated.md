# 命令说明（自动生成）

本节由脚本生成：`tools/gen_readme_commands.py`。修改 CLI 后请重新生成。

<!-- XUN_COMMANDS_START -->

### 全局组合（适用于所有命令）
- 示例：`xun --no-color list -f table`  
  说明：禁用彩色输出。
- 示例：`xun --version`  
  说明：输出版本号并退出。
- 示例：`xun --quiet ports -f json`  
  说明：尽量减少 UI 输出。
- 示例：`xun --verbose redirect D:\Downloads -f table`  
  说明：输出更多原因/细节。
- 示例：`xun --non-interactive redirect D:\Downloads --confirm --yes`  
  说明：强制非交互模式（危险操作需配合 --yes）。

### 初始化与帮助
#### `xun init`
- 示例：`xun init powershell`  
  说明：输出 Shell 集成脚本（wrapper 函数）。
- 示例：`xun init bash`  
  说明：输出 Shell 集成脚本（wrapper 函数）。
- 示例：`xun init zsh`  
  说明：输出 Shell 集成脚本（wrapper 函数）。

### 补全（Completion）
#### `xun __complete`
- 示例：`xun __complete`  
  说明：内部补全入口（Shell 已分词参数）。
- 示例：`xun __complete redirect --profile de ""`  
  说明：命令名后的预分词参数。
#### `xun completion`
- 示例：`xun completion powershell`  
  说明：生成 Shell 补全脚本。
- 示例：`xun completion bash`  
  说明：生成 Shell 补全脚本。
- 示例：`xun completion zsh`  
  说明：生成 Shell 补全脚本。
- 示例：`xun completion fish`  
  说明：生成 Shell 补全脚本。

### ACL（权限）
#### `xun acl add`
- 示例：`xun acl add`  
  说明：Add a permission entry (interactive wizard; use flags for scripted mode).
- 示例：`xun acl add --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl add --principal value`  
  说明：principal name, e.g. "BUILTIN\\Users".
- 示例：`xun acl add --rights FullControl`  
  说明：rights level: FullControl | Modify | ReadAndExecute | Read | Write.
- 示例：`xun acl add --rights Modify`  
  说明：rights level: FullControl | Modify | ReadAndExecute | Read | Write.
- 示例：`xun acl add --rights ReadAndExecute`  
  说明：rights level: FullControl | Modify | ReadAndExecute | Read | Write.
- 示例：`xun acl add --rights Read`  
  说明：rights level: FullControl | Modify | ReadAndExecute | Read | Write.
- 示例：`xun acl add --rights Write`  
  说明：rights level: FullControl | Modify | ReadAndExecute | Read | Write.
- 示例：`xun acl add --ace-type Allow`  
  说明：access type: Allow | Deny.
- 示例：`xun acl add --ace-type Deny`  
  说明：access type: Allow | Deny.
- 示例：`xun acl add --inherit BothInherit`  
  说明：inheritance: BothInherit | ContainerOnly | ObjectOnly | None.
- 示例：`xun acl add --inherit ContainerOnly`  
  说明：inheritance: BothInherit | ContainerOnly | ObjectOnly | None.
- 示例：`xun acl add --inherit ObjectOnly`  
  说明：inheritance: BothInherit | ContainerOnly | ObjectOnly | None.
- 示例：`xun acl add --inherit None`  
  说明：inheritance: BothInherit | ContainerOnly | ObjectOnly | None.
- 示例：`xun acl add --yes`  
  说明：跳过确认。
#### `xun acl audit`
- 示例：`xun acl audit`  
  说明：View or export the audit log.
- 示例：`xun acl audit --tail value`  
  说明：show last N entries.
- 示例：`xun acl audit --export value`  
  说明：export CSV.
#### `xun acl backup`
- 示例：`xun acl backup`  
  说明：Backup the ACL of a path to a JSON file.
- 示例：`xun acl backup --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl backup --output .\tree.txt`  
  说明：output JSON file (auto-named if omitted).
#### `xun acl batch`
- 示例：`xun acl batch`  
  说明：Process multiple paths from a file or comma-separated list.
- 示例：`xun acl batch --file src\main.rs`  
  说明：TXT file with one path per line.
- 示例：`xun acl batch --paths value`  
  说明：comma-separated path list.
- 示例：`xun acl batch --action repair`  
  说明：action: repair | backup | orphans | inherit-reset.
- 示例：`xun acl batch --action backup`  
  说明：action: repair | backup | orphans | inherit-reset.
- 示例：`xun acl batch --action orphans`  
  说明：action: repair | backup | orphans | inherit-reset.
- 示例：`xun acl batch --action inherit-reset`  
  说明：action: repair | backup | orphans | inherit-reset.
- 示例：`xun acl batch --output .\tree.txt`  
  说明：output directory for exports.
- 示例：`xun acl batch --yes`  
  说明：跳过确认。
#### `xun acl config`
- 示例：`xun acl config`  
  说明：View or edit ACL configuration.
- 示例：`xun acl config --set value`  
  说明：set a key-value pair: --set KEY VALUE.
#### `xun acl copy`
- 示例：`xun acl copy`  
  说明：Copy the entire ACL from a reference path onto the target.
- 示例：`xun acl copy --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl copy --reference value`  
  说明：reference path.
- 示例：`xun acl copy --yes`  
  说明：跳过确认。
#### `xun acl diff`
- 示例：`xun acl diff`  
  说明：Compare the ACLs of two paths.
- 示例：`xun acl diff --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl diff --reference value`  
  说明：reference path.
- 示例：`xun acl diff --output .\tree.txt`  
  说明：write diff result to CSV.
#### `xun acl effective`
- 示例：`xun acl effective`  
  说明：Show the effective access a user has on a path.
- 示例：`xun acl effective --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl effective --user value`  
  说明：user to check (default: current user).
#### `xun acl inherit`
- 示例：`xun acl inherit`  
  说明：Enable or disable DACL inheritance on a path.
- 示例：`xun acl inherit --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl inherit --disable`  
  说明：break inheritance.
- 示例：`xun acl inherit --enable`  
  说明：restore inheritance.
- 示例：`xun acl inherit --preserve value`  
  说明：when breaking: keep inherited ACEs as explicit copies (default: true).
#### `xun acl orphans`
- 示例：`xun acl orphans`  
  说明：Scan for (and optionally clean up) orphaned SIDs in ACLs.
- 示例：`xun acl orphans --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl orphans --recursive value`  
  说明：scan recursively.
- 示例：`xun acl orphans --action none`  
  说明：action: none | export | delete | both.
- 示例：`xun acl orphans --action export`  
  说明：action: none | export | delete | both.
- 示例：`xun acl orphans --action delete`  
  说明：action: none | export | delete | both.
- 示例：`xun acl orphans --action both`  
  说明：action: none | export | delete | both.
- 示例：`xun acl orphans --output .\tree.txt`  
  说明：output CSV path.
- 示例：`xun acl orphans --yes`  
  说明：跳过确认。
#### `xun acl owner`
- 示例：`xun acl owner`  
  说明：Change the owner of a path.
- 示例：`xun acl owner --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl owner --set value`  
  说明：new owner principal.
- 示例：`xun acl owner --yes`  
  说明：跳过确认。
#### `xun acl purge`
- 示例：`xun acl purge`  
  说明：Remove ALL explicit rules for a specific principal.
- 示例：`xun acl purge --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl purge --principal value`  
  说明：principal to purge (interactive if omitted).
- 示例：`xun acl purge --yes`  
  说明：跳过确认。
#### `xun acl remove`
- 示例：`xun acl remove`  
  说明：Remove explicit ACE entries (interactive multi-select).
- 示例：`xun acl remove --path D:\Repo\MyProj`  
  说明：目标路径。
#### `xun acl repair`
- 示例：`xun acl repair`  
  说明：Forced ACL repair: take ownership + grant FullControl (parallel).
- 示例：`xun acl repair --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl repair --export-errors`  
  说明：export error CSV when failures occur.
- 示例：`xun acl repair --yes`  
  说明：跳过确认。
#### `xun acl restore`
- 示例：`xun acl restore`  
  说明：Restore an ACL from a previously created JSON backup.
- 示例：`xun acl restore --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl restore --from value`  
  说明：backup JSON file to read from.
- 示例：`xun acl restore --yes`  
  说明：跳过确认。
#### `xun acl view`
- 示例：`xun acl view`  
  说明：View ACL summary or detailed entries for a path.
- 示例：`xun acl view --path D:\Repo\MyProj`  
  说明：目标路径。
- 示例：`xun acl view --detail`  
  说明：show full detail for each ACE.
- 示例：`xun acl view --export value`  
  说明：export ACL entries to CSV.

### 配置管理
#### `xun config edit`
- 示例：`xun config edit`  
  说明：在编辑器中打开配置文件。
#### `xun config get`
- 示例：`xun config get proxy.defaultUrl`  
  说明：按点路径读取配置值（如 proxy.defaultUrl）。
#### `xun config set`
- 示例：`xun config set proxy.defaultUrl http://127.0.0.1:7890`  
  说明：按点路径写入配置值（如 tree.defaultDepth 3）。
- 示例：`xun config set proxy.defaultUrl 3`  
  说明：按点路径写入配置值（如 tree.defaultDepth 3）。

### 上下文切换（ctx）
#### `xun ctx del`
- 示例：`xun ctx del work`  
  说明：删除 profile。
#### `xun ctx list`
- 示例：`xun ctx list`  
  说明：列出 profile 列表。
- 示例：`xun ctx list --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ctx list --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ctx list --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ctx list --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun ctx off`
- 示例：`xun ctx off`  
  说明：停用当前 profile。
#### `xun ctx rename`
- 示例：`xun ctx rename old new`  
  说明：重命名 profile。
#### `xun ctx set`
- 示例：`xun ctx set work`  
  说明：定义或更新上下文 profile。
- 示例：`xun ctx set work --path D:\Repo\MyProj`  
  说明：工作目录。
- 示例：`xun ctx set work --proxy off`  
  说明：代理：<url> | off | keep。
- 示例：`xun ctx set work --proxy keep`  
  说明：代理：<url> | off | keep。
- 示例：`xun ctx set work --noproxy "localhost,127.0.0.1"`  
  说明：NO_PROXY（proxy 为 set 时生效）。
- 示例：`xun ctx set work --tag work`  
  说明：默认标签（逗号分隔；"-" 清空）。
- 示例：`xun ctx set work --env RUST_LOG=info`  
  说明：环境变量（KEY=VALUE，可重复）。
- 示例：`xun ctx set work --env-file .\.env`  
  说明：从文件导入 env（dotenv 格式）。
- 示例：`xun ctx set work --proxy http://127.0.0.1:7890`  
  说明：设置代理 URL。
#### `xun ctx show`
- 示例：`xun ctx show`  
  说明：显示 profile 详情（默认当前激活）。
- 示例：`xun ctx show work`  
  说明：profile 名称（可选，默认当前激活）。
- 示例：`xun ctx show --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ctx show --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ctx show --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ctx show --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun ctx use`
- 示例：`xun ctx use work`  
  说明：激活上下文 profile。

### 书签（Bookmarks）
#### `xun all`
- 示例：`xun all`  
  说明：所有书签（机器输出）。
- 示例：`xun all work`  
  说明：按标签过滤。
#### `xun check`
- 示例：`xun check`  
  说明：检查书签健康（缺失/重复/过期）。
- 示例：`xun check --days 30`  
  说明：过期阈值（天）。
- 示例：`xun check --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun check --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun check --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun check --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun dedup`
- 示例：`xun dedup`  
  说明：书签去重。
- 示例：`xun dedup --mode path`  
  说明：去重模式：path | name。
- 示例：`xun dedup --mode name`  
  说明：去重模式：path | name。
- 示例：`xun dedup --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun dedup --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun dedup --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun dedup --format json`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun dedup --yes`  
  说明：跳过确认（仅交互模式）。
#### `xun delete`
- 示例：`xun delete`  
  说明：Force delete files or delete bookmarks with --bookmark (-bm).
- 示例：`xun delete value`  
  说明：target paths (files or directories).
- 示例：`xun delete --bookmark`  
  说明：delete bookmark instead of files.
- 示例：`xun delete --reserved`  
  说明：only delete Windows reserved names (default).
- 示例：`xun delete --any`  
  说明：allow deleting non-reserved names (dangerous).
- 示例：`xun delete --name work`  
  说明：match file names (comma separated, repeatable).
- 示例：`xun delete --exclude target,.git`  
  说明：exclude directory names (comma separated, repeatable).
- 示例：`xun delete --pattern proj`  
  说明：exclude path glob pattern (repeatable).
- 示例：`xun delete --no-default-excludes`  
  说明：skip built-in default excludes.
- 示例：`xun delete --no-tui`  
  说明：skip TUI and run CLI pipeline directly.
- 示例：`xun delete --dry-run`  
  说明：simulate run without deleting.
- 示例：`xun delete --what-if`  
  说明：alias for --dry-run.
- 示例：`xun delete --collect-info`  
  说明：collect file info (sha256 + kind) before delete.
- 示例：`xun delete --log value`  
  说明：write results to CSV log file.
- 示例：`xun delete --level value`  
  说明：max delete level (1-6), default 2.
- 示例：`xun delete --on-reboot`  
  说明：schedule delete on reboot (requires admin).
- 示例：`xun delete --yes`  
  说明：skip confirmations.
- 示例：`xun delete --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun delete --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun delete --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun delete --format json`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun delete --force`  
  说明：强制操作（绕过保护）。
- 示例：`xun delete --reason "cleanup"`  
  说明：绕过保护的理由。
#### `xun export`
- 示例：`xun export`  
  说明：导出书签。
- 示例：`xun export --format json`  
  说明：格式：json | tsv。
- 示例：`xun export --format tsv`  
  说明：格式：json | tsv。
- 示例：`xun export --out .\bookmarks.tsv`  
  说明：输出文件（可选）。
#### `xun fuzzy`
- 示例：`xun fuzzy proj`  
  说明：模糊搜索（机器输出）。
- 示例：`xun fuzzy proj work`  
  说明：按标签过滤。
#### `xun gc`
- 示例：`xun gc`  
  说明：清理无效路径。
- 示例：`xun gc --purge`  
  说明：无需确认直接删除所有无效路径。
- 示例：`xun gc --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun gc --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun gc --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun gc --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun import`
- 示例：`xun import`  
  说明：导入书签。
- 示例：`xun import --format json`  
  说明：格式：json | tsv。
- 示例：`xun import --format tsv`  
  说明：格式：json | tsv。
- 示例：`xun import --input .\bookmarks.json`  
  说明：输入文件（可选，默认 stdin）。
- 示例：`xun import --mode merge`  
  说明：导入模式：merge | overwrite。
- 示例：`xun import --mode overwrite`  
  说明：导入模式：merge | overwrite。
- 示例：`xun import --yes`  
  说明：跳过确认。
#### `xun keys`
- 示例：`xun keys`  
  说明：输出所有键（用于补全）。
#### `xun list`
- 示例：`xun list`  
  说明：列出所有书签。
- 示例：`xun list --tag work`  
  说明：按标签过滤。
- 示例：`xun list --sort name`  
  说明：排序方式：name | last | visits。
- 示例：`xun list --sort last`  
  说明：排序方式：name | last | visits。
- 示例：`xun list --sort visits`  
  说明：排序方式：name | last | visits。
- 示例：`xun list --limit 10`  
  说明：限制结果数量。
- 示例：`xun list --offset 20`  
  说明：结果偏移（分页）。
- 示例：`xun list --reverse`  
  说明：反转排序顺序。
- 示例：`xun list --tsv`  
  说明：输出 TSV（快速路径）。
- 示例：`xun list --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun list --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun list --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun list --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun o`
- 示例：`xun o`  
  说明：在资源管理器中打开。
- 示例：`xun o proj`  
  说明：模糊匹配关键字。
- 示例：`xun o --tag work`  
  说明：按标签过滤。
#### `xun recent`
- 示例：`xun recent`  
  说明：显示最近书签。
- 示例：`xun recent --limit 10`  
  说明：限制结果数量。
- 示例：`xun recent --tag work`  
  说明：按标签过滤。
- 示例：`xun recent --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun recent --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun recent --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun recent --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun rename`
- 示例：`xun rename old new`  
  说明：重命名书签。
#### `xun set`
- 示例：`xun set work`  
  说明：保存当前目录或指定路径为书签。
- 示例：`xun set work D:\Repo\MyProj`  
  说明：路径（可选，默认当前目录）。
- 示例：`xun set work --tag work`  
  说明：标签（逗号分隔）。
#### `xun stats`
- 示例：`xun stats`  
  说明：显示统计信息。
- 示例：`xun stats --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun stats --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun stats --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun stats --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun sv`
- 示例：`xun sv`  
  说明：保存当前目录为书签（sv）。
- 示例：`xun sv work`  
  说明：书签名（可选，默认当前目录名）。
- 示例：`xun sv --tag work`  
  说明：标签（逗号分隔）。
#### `xun tag add`
- 示例：`xun tag add work dev,cli`  
  说明：为书签添加标签。
#### `xun tag list`
- 示例：`xun tag list`  
  说明：列出所有标签及数量。
#### `xun tag remove`
- 示例：`xun tag remove work dev,cli`  
  说明：从书签移除标签。
#### `xun tag rename`
- 示例：`xun tag rename old new`  
  说明：重命名标签（全局）。
#### `xun touch`
- 示例：`xun touch work`  
  说明：更新访问频次（touch）。
#### `xun ws`
- 示例：`xun ws work`  
  说明：Workspace：在 Windows Terminal 多标签打开标签下所有路径。
#### `xun z`
- 示例：`xun z`  
  说明：跳转到书签（模糊匹配）。
- 示例：`xun z proj`  
  说明：模糊匹配关键字。
- 示例：`xun z --tag work`  
  说明：按标签过滤。

### 代理（Proxy）
#### `xun poff`
- 示例：`xun poff`  
  说明：关闭代理（poff）。
- 示例：`xun poff --msys2 C:\msys64`  
  说明：msys2 根目录覆盖。
#### `xun pon`
- 示例：`xun pon`  
  说明：开启代理（pon）。
- 示例：`xun pon http://127.0.0.1:7890`  
  说明：代理地址（可选，自动检测系统代理）。
- 示例：`xun pon --no-test`  
  说明：启用后跳过连通性测试。
- 示例：`xun pon --noproxy "localhost,127.0.0.1"`  
  说明：no_proxy 列表。
- 示例：`xun pon --msys2 C:\msys64`  
  说明：msys2 根目录覆盖。
#### `xun proxy del`
- 示例：`xun proxy del`  
  说明：删除代理。
- 示例：`xun proxy del --msys2 C:\msys64`  
  说明：msys2 根目录覆盖。
- 示例：`xun proxy del --only cargo,git`  
  说明：仅删除指定目标：cargo,git,npm,msys2（逗号分隔）。
#### `xun proxy detect`
- 示例：`xun proxy detect`  
  说明：检测系统代理。
- 示例：`xun proxy detect --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun proxy detect --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun proxy detect --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun proxy detect --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun proxy get`
- 示例：`xun proxy get`  
  说明：读取当前 git 代理配置。
#### `xun proxy set`
- 示例：`xun proxy set http://127.0.0.1:7890`  
  说明：设置代理。
- 示例：`xun proxy set http://127.0.0.1:7890 --noproxy "localhost,127.0.0.1"`  
  说明：no_proxy 列表（默认 localhost,127.0.0.1）。
- 示例：`xun proxy set http://127.0.0.1:7890 --msys2 C:\msys64`  
  说明：msys2 根目录覆盖。
- 示例：`xun proxy set http://127.0.0.1:7890 --only cargo,git`  
  说明：仅设置指定目标：cargo,git,npm,msys2（逗号分隔）。
#### `xun proxy test`
- 示例：`xun proxy test http://127.0.0.1:7890`  
  说明：测试代理延迟。
- 示例：`xun proxy test http://127.0.0.1:7890 --targets proxy,github.com,crates.io`  
  说明：目标列表（逗号分隔；用 proxy 测代理自身）。
- 示例：`xun proxy test http://127.0.0.1:7890 --timeout 5`  
  说明：超时秒数。
- 示例：`xun proxy test http://127.0.0.1:7890 --jobs 3`  
  说明：最大并发探测数。
#### `xun pst`
- 示例：`xun pst`  
  说明：代理状态（pst）。
- 示例：`xun pst --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun pst --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun pst --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun pst --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun px`
- 示例：`xun px`  
  说明：代理执行（px）。
- 示例：`xun px -- cargo build`  
  说明：命令及参数。
- 示例：`xun px --url http://127.0.0.1:7890`  
  说明：代理地址（可选）。
- 示例：`xun px --noproxy "localhost,127.0.0.1"`  
  说明：no_proxy 列表。

### 端口（Ports）
#### `xun kill`
- 示例：`xun kill 5173,8080`  
  说明：终止占用端口的进程。
- 示例：`xun kill 5173,8080 --force`  
  说明：跳过确认。
- 示例：`xun kill 5173,8080 --tcp`  
  说明：仅 TCP。
- 示例：`xun kill 5173,8080 --udp`  
  说明：仅 UDP。
#### `xun ports`
- 示例：`xun ports`  
  说明：列出监听端口（默认 TCP）。
- 示例：`xun ports --all`  
  说明：显示所有 TCP 监听端口。
- 示例：`xun ports --udp`  
  说明：显示 UDP 绑定端口。
- 示例：`xun ports --range 3000-3999`  
  说明：按端口范围过滤（如 3000-3999）。
- 示例：`xun ports --pid 12345`  
  说明：按 PID 过滤。
- 示例：`xun ports --name work`  
  说明：按进程名过滤（子串）。
- 示例：`xun ports --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ports --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ports --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun ports --format json`  
  说明：输出格式：auto | table | tsv | json。

### 进程（Process）
#### `xun pkill`
- 示例：`xun pkill value`  
  说明：Kill processes by name, PID, or window title.
- 示例：`xun pkill value --window`  
  说明：treat target as window title.
- 示例：`xun pkill value --force`  
  说明：skip interactive confirmation.
#### `xun ps`
- 示例：`xun ps`  
  说明：List running processes by name, PID, or window title.
- 示例：`xun ps proj`  
  说明：fuzzy match by process name.
- 示例：`xun ps --pid 12345`  
  说明：exact PID lookup.
- 示例：`xun ps --win value`  
  说明：fuzzy match by window title.

### 备份（bak）
#### `xun bak`
- 示例：`xun bak`  
  说明：增量项目备份。
- 示例：`xun bak list`  
  说明：操作与参数：`list` | `verify <name>` | `find [tag]`（默认创建备份）。
- 示例：`xun bak verify v12-2026-02-23_1030`  
  说明：操作与参数：`list` | `verify <name>` | `find [tag]`（默认创建备份）。
- 示例：`xun bak find demo`  
  说明：操作与参数：`list` | `verify <name>` | `find [tag]`（默认创建备份）。
- 示例：`xun bak --msg "baseline"`  
  说明：备份描述。
- 示例：`xun bak --dir D:\Repo\MyProj`  
  说明：工作目录（默认当前目录）。
- 示例：`xun bak --dry-run`  
  说明：演练（不复制/不压缩/不清理）。
- 示例：`xun bak --no-compress`  
  说明：本次跳过压缩。
- 示例：`xun bak --retain 10`  
  说明：覆盖最大备份数。
- 示例：`xun bak --include src,docs`  
  说明：添加包含路径（可重复或逗号分隔）。
- 示例：`xun bak --exclude target,.git`  
  说明：添加排除路径（可重复或逗号分隔）。
- 示例：`xun bak --incremental`  
  说明：增量备份：仅复制新增或修改的文件。

### 恢复（restore）
#### `xun restore`
- 示例：`xun restore v12-2026-02-23_1030`  
  说明：从 `xun bak` 创建的备份中恢复文件。
- 示例：`xun restore v12-2026-02-23_1030 --file src\main.rs`  
  说明：恢复单个文件（相对路径，例如 `src/main.rs`）。
- 示例：`xun restore v12-2026-02-23_1030 --glob "**/*.ts"`  
  说明：恢复匹配 glob 的文件（例如 `**/*.ts`）。
- 示例：`xun restore v12-2026-02-23_1030 --to D:\Restore-Preview`  
  说明：恢复到指定目录，而不是项目根目录。
- 示例：`xun restore v12-2026-02-23_1030 --snapshot`  
  说明：恢复前先为当前状态创建快照（生成 `pre_restore` 备份）。
- 示例：`xun restore v12-2026-02-23_1030 -C D:\Repo\MyProj`  
  说明：项目根目录（默认当前目录）。
- 示例：`xun restore v12-2026-02-23_1030 --dry-run`  
  说明：演练模式：仅显示将恢复的文件，不写入磁盘。
- 示例：`xun restore v12-2026-02-23_1030 -y`  
  说明：跳过确认提示。

### 目录树（tree）
#### `xun tree`
- 示例：`xun tree`  
  说明：生成目录树。
- 示例：`xun tree D:\Repo\MyProj`  
  说明：目标路径（默认当前目录）。
- 示例：`xun tree --depth 2`  
  说明：最大深度，0 为不限。
- 示例：`xun tree --output .\tree.txt`  
  说明：输出文件。
- 示例：`xun tree --hidden`  
  说明：包含隐藏文件。
- 示例：`xun tree --no-clip`  
  说明：不复制到剪贴板。
- 示例：`xun tree --plain`  
  说明：纯文本输出（无框线）。
- 示例：`xun tree --stats-only`  
  说明：仅统计（不输出树）。
- 示例：`xun tree --fast`  
  说明：快速模式（跳过排序和元数据）。
- 示例：`xun tree --sort name`  
  说明：排序方式：name | mtime | size。
- 示例：`xun tree --sort mtime`  
  说明：排序方式：name | mtime | size。
- 示例：`xun tree --sort size`  
  说明：排序方式：name | mtime | size。
- 示例：`xun tree --size`  
  说明：显示大小（目录显示总大小）。
- 示例：`xun tree --max-items 200`  
  说明：最大输出项数。
- 示例：`xun tree --include src,docs`  
  说明：包含匹配（可重复或逗号分隔）。
- 示例：`xun tree --exclude target,.git`  
  说明：排除匹配（可重复或逗号分隔）。

### 查找（find）
#### `xun find`
- 示例：`xun find`  
  说明：Find files and directories by pattern and metadata.
- 示例：`xun find value`  
  说明：base directories (default: cwd).
- 示例：`xun find --include src,docs`  
  说明：include glob pattern (repeatable or comma separated).
- 示例：`xun find --exclude target,.git`  
  说明：exclude glob pattern (repeatable or comma separated).
- 示例：`xun find --regex-include value`  
  说明：include regex pattern (repeatable).
- 示例：`xun find --regex-exclude value`  
  说明：exclude regex pattern (repeatable).
- 示例：`xun find --extension value`  
  说明：include extensions (comma separated, repeatable).
- 示例：`xun find --not-extension value`  
  说明：exclude extensions (comma separated, repeatable).
- 示例：`xun find --name work`  
  说明：include names (comma separated, repeatable).
- 示例：`xun find --filter-file value`  
  说明：load rules from file (glob, default exclude).
- 示例：`xun find --size value`  
  说明：size filter (repeatable).
- 示例：`xun find --fuzzy-size value`  
  说明：fuzzy size filter.
- 示例：`xun find --mtime value`  
  说明：mtime filter (repeatable).
- 示例：`xun find --ctime value`  
  说明：ctime filter (repeatable).
- 示例：`xun find --atime value`  
  说明：atime filter (repeatable).
- 示例：`xun find --depth 2`  
  说明：depth filter.
- 示例：`xun find --attribute value`  
  说明：attribute filter (e.g. +h,-r).
- 示例：`xun find --empty-files`  
  说明：only empty files.
- 示例：`xun find --not-empty-files`  
  说明：exclude empty files.
- 示例：`xun find --empty-dirs`  
  说明：only empty directories.
- 示例：`xun find --not-empty-dirs`  
  说明：exclude empty directories.
- 示例：`xun find --case`  
  说明：case sensitive matching.
- 示例：`xun find --count`  
  说明：count only.
- 示例：`xun find --dry-run`  
  说明：dry run (no filesystem scan).
- 示例：`xun find --test-path value`  
  说明：test path for dry run.
- 示例：`xun find --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun find --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun find --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun find --format json`  
  说明：输出格式：auto | table | tsv | json。

### 别名（alias）
#### `xun alias add`
- 示例：`xun alias add work value`  
  说明：Add shell alias.
- 示例：`xun alias add work value --mode auto`  
  说明：alias mode: auto|exe|cmd.
- 示例：`xun alias add work value --mode exe`  
  说明：alias mode: auto|exe|cmd.
- 示例：`xun alias add work value --mode cmd`  
  说明：alias mode: auto|exe|cmd.
- 示例：`xun alias add work value --desc value`  
  说明：description.
- 示例：`xun alias add work value --tag work`  
  说明：tags (comma-separated, repeatable).
- 示例：`xun alias add work value --shell cmd`  
  说明：limit to shells: cmd|ps|bash|nu (comma-separated, repeatable).
- 示例：`xun alias add work value --shell ps`  
  说明：limit to shells: cmd|ps|bash|nu (comma-separated, repeatable).
- 示例：`xun alias add work value --shell bash`  
  说明：limit to shells: cmd|ps|bash|nu (comma-separated, repeatable).
- 示例：`xun alias add work value --shell nu`  
  说明：limit to shells: cmd|ps|bash|nu (comma-separated, repeatable).
- 示例：`xun alias add work value --force`  
  说明：overwrite existing alias.
#### `xun alias app`
- 示例：`xun alias app`  
  说明：App alias operations.
#### `xun alias export`
- 示例：`xun alias export`  
  说明：Export aliases config.
- 示例：`xun alias export --output .\tree.txt`  
  说明：output file path (stdout when omitted).
#### `xun alias find`
- 示例：`xun alias find value`  
  说明：Find aliases with fuzzy match.
#### `xun alias import`
- 示例：`xun alias import src\main.rs`  
  说明：Import aliases config.
- 示例：`xun alias import src\main.rs --force`  
  说明：overwrite conflicts.
#### `xun alias ls`
- 示例：`xun alias ls`  
  说明：List aliases.
- 示例：`xun alias ls --tag cmd`  
  说明：filter: cmd|app filter by tag.
- 示例：`xun alias ls --tag app`  
  说明：filter: cmd|app filter by tag.
- 示例：`xun alias ls --json`  
  说明：json output.
#### `xun alias rm`
- 示例：`xun alias rm`  
  说明：Remove aliases.
- 示例：`xun alias rm value`  
  说明：alias names.
#### `xun alias setup`
- 示例：`xun alias setup`  
  说明：Setup alias runtime (shim template + shells).
- 示例：`xun alias setup --no-cmd`  
  说明：skip cmd backend.
- 示例：`xun alias setup --no-ps`  
  说明：skip powershell backend.
- 示例：`xun alias setup --no-bash`  
  说明：skip bash backend.
- 示例：`xun alias setup --no-nu`  
  说明：skip nushell backend.
- 示例：`xun alias setup --core-only`  
  说明：only setup core shells (cmd + powershell).
#### `xun alias sync`
- 示例：`xun alias sync`  
  说明：Sync shim + app paths + shells.
#### `xun alias which`
- 示例：`xun alias which work`  
  说明：Show alias target and shim info.

### 文件解锁与操作（lock/fs）
#### `xun lock who`
- 示例：`xun lock who D:\Repo\MyProj`  
  说明：显示占用文件的进程。
- 示例：`xun lock who D:\Repo\MyProj --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun lock who D:\Repo\MyProj --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun lock who D:\Repo\MyProj --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun lock who D:\Repo\MyProj --format json`  
  说明：输出格式：auto | table | tsv | json。
#### `xun mv`
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt`  
  说明：移动文件或目录。
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt --unlock`  
  说明：若被占用则解锁。
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt --force-kill`  
  说明：强制结束占用进程。
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt --dry-run`  
  说明：演练/不执行。
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt --yes`  
  说明：跳过确认。
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt --force`  
  说明：强制操作（绕过保护）。
- 示例：`xun mv D:\Temp\a.txt D:\Temp\b.txt --reason "cleanup"`  
  说明：绕过保护的理由。
#### `xun ren`
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt`  
  说明：重命名文件或目录。
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt --unlock`  
  说明：若被占用则解锁。
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt --force-kill`  
  说明：强制结束占用进程。
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt --dry-run`  
  说明：演练/不执行。
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt --yes`  
  说明：跳过确认。
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt --force`  
  说明：强制操作（绕过保护）。
- 示例：`xun ren D:\Temp\a.txt D:\Temp\b.txt --reason "cleanup"`  
  说明：绕过保护的理由。
#### `xun rm`
- 示例：`xun rm D:\Repo\MyProj`  
  说明：删除文件或目录。
- 示例：`xun rm D:\Repo\MyProj --unlock`  
  说明：若被占用则解锁。
- 示例：`xun rm D:\Repo\MyProj --force-kill`  
  说明：强制结束占用进程。
- 示例：`xun rm D:\Repo\MyProj --on-reboot`  
  说明：重启后删除。
- 示例：`xun rm D:\Repo\MyProj --dry-run`  
  说明：演练/不执行。
- 示例：`xun rm D:\Repo\MyProj --yes`  
  说明：跳过确认。
- 示例：`xun rm D:\Repo\MyProj --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun rm D:\Repo\MyProj --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun rm D:\Repo\MyProj --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun rm D:\Repo\MyProj --format json`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun rm D:\Repo\MyProj --force`  
  说明：强制操作（绕过保护）。
- 示例：`xun rm D:\Repo\MyProj --reason "cleanup"`  
  说明：绕过保护的理由。

### 防误操作保护（protect）
#### `xun protect clear`
- 示例：`xun protect clear D:\Repo\MyProj`  
  说明：清除保护规则。
- 示例：`xun protect clear D:\Repo\MyProj --system-acl`  
  说明：同时移除 NTFS ACL 删除拒绝规则。
#### `xun protect set`
- 示例：`xun protect set D:\Repo\MyProj`  
  说明：设置保护规则。
- 示例：`xun protect set D:\Repo\MyProj --deny delete,rename`  
  说明：禁止的操作（如 delete,move,rename）。
- 示例：`xun protect set D:\Repo\MyProj --require force,reason`  
  说明：绕过要求（如 force,reason）。
- 示例：`xun protect set D:\Repo\MyProj --system-acl`  
  说明：应用 NTFS ACL 删除拒绝规则（更强保护）。
#### `xun protect status`
- 示例：`xun protect status`  
  说明：显示保护状态。
- 示例：`xun protect status D:\Repo\MyProj`  
  说明：按路径前缀过滤。
- 示例：`xun protect status --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun protect status --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun protect status --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun protect status --format json`  
  说明：输出格式：auto | table | tsv | json。

### 加密/解密（crypt）
#### `xun decrypt`
- 示例：`xun decrypt D:\Repo\MyProj`  
  说明：解密文件。
- 示例：`xun decrypt D:\Repo\MyProj --efs`  
  说明：使用 Windows EFS 解密。
- 示例：`xun decrypt D:\Repo\MyProj --identity D:\Keys\me.agekey`  
  说明：解密身份文件（age 格式，可重复）。
- 示例：`xun decrypt D:\Repo\MyProj --passphrase`  
  说明：使用口令解密（交互）。
- 示例：`xun decrypt D:\Repo\MyProj --out .\bookmarks.tsv`  
  说明：输出文件路径（非 efs 默认去掉 .age）。
#### `xun encrypt`
- 示例：`xun encrypt D:\Repo\MyProj`  
  说明：使用 Windows EFS 加密文件（或其他提供者）。
- 示例：`xun encrypt D:\Repo\MyProj --efs`  
  说明：使用 Windows EFS 加密。
- 示例：`xun encrypt D:\Repo\MyProj --to age1exampleexampleexampleexampleexample`  
  说明：加密目标公钥（age 格式，可重复）。
- 示例：`xun encrypt D:\Repo\MyProj --passphrase`  
  说明：使用口令加密（交互）。
- 示例：`xun encrypt D:\Repo\MyProj --out .\bookmarks.tsv`  
  说明：输出文件路径（非 efs 默认 <path>.age）。

### Redirect 文件分类引擎
#### `xun redirect`
- 示例：`xun redirect`  
  说明：按规则将目录文件分类到子目录。
- 示例：`xun redirect D:\Downloads`  
  说明：源目录（默认当前目录）。
- 示例：`xun redirect --profile default`  
  说明：profile 名称（config.redirect.profiles，默认 default）。
- 示例：`xun redirect --explain 2026-02_report.jpg`  
  说明：解释匹配原因（纯字符串模式）。
- 示例：`xun redirect --stats`  
  说明：运行后输出规则覆盖率汇总（stderr）。
- 示例：`xun redirect --confirm`  
  说明：执行前显示预览统计并确认（交互模式，或配合 --yes）。
- 示例：`xun redirect --review`  
  说明：逐条交互确认每个计划操作（y/n/a/q）。
- 示例：`xun redirect --log`  
  说明：查询审计日志（redirect tx 历史）。
- 示例：`xun redirect --log --tx redirect_1740000000_1234`  
  说明：按 tx 过滤审计日志（配合 --log）。
- 示例：`xun redirect --log --last 5`  
  说明：显示最近 N 条 tx（配合 --log）。
- 示例：`xun redirect --validate`  
  说明：仅校验配置（不扫描/不 watch）。
- 示例：`xun redirect --plan .\xun.plan.json`  
  说明：生成 plan 文件（json），不执行。
- 示例：`xun redirect --apply .\xun.plan.json`  
  说明：应用 plan 文件（json）。
- 示例：`xun redirect --undo redirect_1740000000_1234`  
  说明：按 tx 撤销 redirect（读 audit.jsonl）。
- 示例：`xun redirect --watch`  
  说明：watch 模式（守护执行）。
- 示例：`xun redirect --watch --status`  
  说明：显示 watch 状态（配合 --watch）。
- 示例：`xun redirect --simulate`  
  说明：从 stdin 模拟匹配（纯字符串模式）。
- 示例：`xun redirect --dry-run`  
  说明：演练，不执行。
- 示例：`xun redirect --copy`  
  说明：复制替代移动。
- 示例：`xun redirect --yes`  
  说明：跳过确认（非交互 overwrite 需要）。
- 示例：`xun redirect --format auto`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun redirect --format table`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun redirect --format tsv`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`xun redirect --format json`  
  说明：输出格式：auto | table | tsv | json。
- 示例：`"a.jpg`nreport_2026.pdf`nrandom.xyz" | xun redirect --simulate -f tsv`  
  说明：从 stdin 批量模拟匹配（纯字符串模式）。
- 示例：`xun redirect D:\Downloads --watch --status -f json`  
  说明：读取 watch 状态文件（不启动 watcher）。
- 示例：`xun redirect D:\Downloads --review --dry-run -f table`  
  说明：逐条预览不执行。

### Web Dashboard（dashboard）
#### `xun serve`
- 示例：`xun serve`  
  说明：启动 Web Dashboard 服务。
- 示例：`xun serve --port 9527`  
  说明：监听端口（默认 9527）。

### Diff（diff）
#### `xun diff`
- 示例：`xun diff old new`  
  说明：比较两个文件的差异。
- 示例：`xun diff old new --mode line`  
  说明：diff 模式：auto（默认）| line | ast。
- 示例：`xun diff old new --mode ast`  
  说明：diff 模式：auto（默认）| line | ast。
- 示例：`xun diff old new --diff-algorithm myers`  
  说明：diff 算法：histogram（默认）| myers | minimal | patience。
- 示例：`xun diff old new --diff-algorithm minimal`  
  说明：diff 算法：histogram（默认）| myers | minimal | patience。
- 示例：`xun diff old new --diff-algorithm patience`  
  说明：diff 算法：histogram（默认）| myers | minimal | patience。
- 示例：`xun diff old new --format value`  
  说明：输出格式：text（默认）| json。
- 示例：`xun diff old new --context value`  
  说明：上下文行数（默认 3，对齐 GNU diff -U）。
- 示例：`xun diff old new --max-size value`  
  说明：文件大小上限，如 512K、1M（默认 512K）。
- 示例：`xun diff old new --ignore-space-change`  
  说明：忽略行内空白量变化（类似 -b）。
- 示例：`xun diff old new --ignore-all-space`  
  说明：忽略所有空白（类似 -w）。
- 示例：`xun diff old new --ignore-blank-lines`  
  说明：忽略空行差异（类似 -B）。
- 示例：`xun diff old new --strip-trailing-cr`  
  说明：剥离行尾 CR，消除 CRLF/LF 噪声。
- 示例：`xun diff old new --text`  
  说明：强制按文本处理二进制文件（类似 GNU diff --text / -a）。

### 批量重命名（brn）
#### `xun brn`
- 示例：`xun brn D:\Repo\MyProj`  
  说明：Batch file renamer — dry-run by default, --apply to execute.
- 示例：`xun brn D:\Repo\MyProj --regex value`  
  说明：regex pattern to match against file stems.
- 示例：`xun brn D:\Repo\MyProj --replace value`  
  说明：replacement string for --regex (supports $1, $2 capture groups).
- 示例：`xun brn D:\Repo\MyProj --case value`  
  说明：convert naming convention: kebab, snake, pascal, upper, lower.
- 示例：`xun brn D:\Repo\MyProj --prefix value`  
  说明：prepend a string to the file stem.
- 示例：`xun brn D:\Repo\MyProj --suffix value`  
  说明：append a string to the file stem (before extension).
- 示例：`xun brn D:\Repo\MyProj --strip-prefix value`  
  说明：remove a prefix from the file stem.
- 示例：`xun brn D:\Repo\MyProj --seq`  
  说明：append zero-padded sequence number to each stem.
- 示例：`xun brn D:\Repo\MyProj --start value`  
  说明：sequence start value (default: 1, requires --seq).
- 示例：`xun brn D:\Repo\MyProj --pad value`  
  说明：zero-padding width (default: 3, requires --seq).
- 示例：`xun brn D:\Repo\MyProj --ext value`  
  说明：only process files with these extensions (repeatable).
- 示例：`xun brn D:\Repo\MyProj --recursive`  
  说明：recurse into subdirectories.
- 示例：`xun brn D:\Repo\MyProj --apply`  
  说明：execute renames (default: dry-run preview).
- 示例：`xun brn D:\Repo\MyProj --yes`  
  说明：skip confirmation prompt (requires --apply).

### 代码体检（cstat）
#### `xun cstat`
- 示例：`xun cstat D:\Repo\MyProj`  
  说明：Code statistics and project cleanup scanner.
- 示例：`xun cstat D:\Repo\MyProj --empty`  
  说明：find empty files (0 bytes).
- 示例：`xun cstat D:\Repo\MyProj --large value`  
  说明：find files with more than N lines.
- 示例：`xun cstat D:\Repo\MyProj --dup`  
  说明：find duplicate files by content hash (BLAKE3).
- 示例：`xun cstat D:\Repo\MyProj --tmp`  
  说明：find temporary/leftover files.
- 示例：`xun cstat D:\Repo\MyProj --all`  
  说明：enable all issue detections.
- 示例：`xun cstat D:\Repo\MyProj --ext value`  
  说明：only scan files with these extensions (repeatable).
- 示例：`xun cstat D:\Repo\MyProj --depth 2`  
  说明：max directory recursion depth.
- 示例：`xun cstat D:\Repo\MyProj --format value`  
  说明：output format: auto, table, json.
- 示例：`xun cstat D:\Repo\MyProj --output .\tree.txt`  
  说明：export JSON report to file.

### 图像处理（img）
#### `xun img`
- 示例：`xun img`  
  说明：image compression and format conversion.
- 示例：`xun img --input .\bookmarks.json`  
  说明：input file or directory.
- 示例：`xun img --output .\tree.txt`  
  说明：output directory (created automatically when missing).
- 示例：`xun img --format jpeg`  
  说明：output format [jpeg|png|webp|avif], default webp.
- 示例：`xun img --format png`  
  说明：output format [jpeg|png|webp|avif], default webp.
- 示例：`xun img --format webp`  
  说明：output format [jpeg|png|webp|avif], default webp.
- 示例：`xun img --format avif`  
  说明：output format [jpeg|png|webp|avif], default webp.
- 示例：`xun img --jpeg-backend auto`  
  说明：jpeg backend [auto|moz|turbo], default auto.
- 示例：`xun img --jpeg-backend moz`  
  说明：jpeg backend [auto|moz|turbo], default auto.
- 示例：`xun img --jpeg-backend turbo`  
  说明：jpeg backend [auto|moz|turbo], default auto.
- 示例：`xun img --quality value`  
  说明：encode quality 1-100 (ignored by lossless modes), default 80.
- 示例：`xun img --png-lossy value`  
  说明：png lossy quantization (true=pngquant, false=oxipng).
- 示例：`xun img --png-dither-level value`  
  说明：png dithering level in lossy mode [0.0-1.0], default 0.0.
- 示例：`xun img --webp-lossy value`  
  说明：webp lossy encoding (true=lossy, false=lossless).
- 示例：`xun img --mw value`  
  说明：max width (keep aspect ratio, never upscale).
- 示例：`xun img --mh value`  
  说明：max height (keep aspect ratio, never upscale).
- 示例：`xun img --threads value`  
  说明：worker threads, default cpu core count.
- 示例：`xun img --avif-threads value`  
  说明：avif encoder internal threads (default auto).
- 示例：`xun img --overwrite`  
  说明：overwrite existing output files.

<!-- XUN_COMMANDS_END -->
