import re
import sys
from pathlib import Path


CLI_ENTRY_PATH = Path("src/cli.rs")
CLI_MODULE_DIR = Path("src/cli")
CLI_EXTRA_PATHS = [Path("src/commands/diff.rs")]
README_PATH = Path("intro/cli/Commands-Generated.md")

START_MARKER = "<!-- XUN_COMMANDS_START -->"
END_MARKER = "<!-- XUN_COMMANDS_END -->"
CJK_RE = re.compile(r"[\u4e00-\u9fff]")

DOC_TRANSLATIONS = {
    "disable ANSI colors (or set NO_COLOR=1).": "绂佺敤 ANSI 棰滆壊锛堟垨璁剧疆 NO_COLOR=1锛夈€?,
    "show version and exit.": "杈撳嚭鐗堟湰鍙峰苟閫€鍑恒€?,
    "suppress UI output.": "鍑忓皯 UI 杈撳嚭銆?,
    "verbose output.": "杈撳嚭鏇村缁嗚妭銆?,
    "force non-interactive mode.": "寮哄埗闈炰氦浜掓ā寮忋€?,
    "Manage ~/.xun.config.json.": "绠＄悊 ~/.xun.config.json銆?,
    "Get a config value by dot path (e.g. proxy.defaultUrl).": "鎸夌偣璺緞璇诲彇閰嶇疆鍊硷紙濡?proxy.defaultUrl锛夈€?,
    "key path (dot separated).": "閰嶇疆閿矾寰勶紙鐐瑰垎闅旓級銆?,
    "Set a config value by dot path (e.g. tree.defaultDepth 3).": "鎸夌偣璺緞鍐欏叆閰嶇疆鍊硷紙濡?tree.defaultDepth 3锛夈€?,
    "value (JSON if possible, otherwise string).": "鍊硷紙鑳借В鏋愪负 JSON 鍒欑敤 JSON锛屽惁鍒欏瓧绗︿覆锛夈€?,
    "Open config file in an editor.": "鍦ㄧ紪杈戝櫒涓墦寮€閰嶇疆鏂囦欢銆?,
    "Context switch profiles.": "涓婁笅鏂囧垏鎹㈤厤缃€?,
    "Define or update a context profile.": "瀹氫箟鎴栨洿鏂颁笂涓嬫枃 profile銆?,
    "profile name.": "profile 鍚嶇О銆?,
    "working directory.": "宸ヤ綔鐩綍銆?,
    "proxy: <url> | off | keep.": "浠ｇ悊锛?url> | off | keep銆?,
    "NO_PROXY (when proxy is set).": "NO_PROXY锛坧roxy 涓?set 鏃剁敓鏁堬級銆?,
    "default tags (comma separated), or \"-\" to clear.": "榛樿鏍囩锛堥€楀彿鍒嗛殧锛沑"-\" 娓呯┖锛夈€?,
    "environment variable (KEY=VALUE), repeatable.": "鐜鍙橀噺锛圞EY=VALUE锛屽彲閲嶅锛夈€?,
    "import env from file (dotenv format).": "浠庢枃浠跺鍏?env锛坉otenv 鏍煎紡锛夈€?,
    "Activate a context profile.": "婵€娲讳笂涓嬫枃 profile銆?,
    "Deactivate current profile.": "鍋滅敤褰撳墠 profile銆?,
    "List profiles.": "鍒楀嚭 profile 鍒楄〃銆?,
    "Show profile details (default: active profile).": "鏄剧ず profile 璇︽儏锛堥粯璁ゅ綋鍓嶆縺娲伙級銆?,
    "profile name (optional, defaults to active).": "profile 鍚嶇О锛堝彲閫夛紝榛樿褰撳墠婵€娲伙級銆?,
    "Delete a profile.": "鍒犻櫎 profile銆?,
    "Rename a profile.": "閲嶅懡鍚?profile銆?,
    "Initialize shell integration (print wrapper function).": "杈撳嚭 Shell 闆嗘垚鑴氭湰锛坵rapper 鍑芥暟锛夈€?,
    "shell type: powershell | bash | zsh.": "Shell 绫诲瀷锛歱owershell | bash | zsh銆?,
    "Generate shell completion script.": "鐢熸垚 Shell 琛ュ叏鑴氭湰銆?,
    "shell type: powershell | bash | zsh | fish.": "Shell 绫诲瀷锛歱owershell | bash | zsh | fish銆?,
    "Internal completion entry (shell-pre-tokenized args).": "鍐呴儴琛ュ叏鍏ュ彛锛圫hell 宸插垎璇嶅弬鏁帮級銆?,
    "pre-tokenized args after command name.": "鍛戒护鍚嶅悗鐨勯鍒嗚瘝鍙傛暟銆?,
    "List all bookmarks.": "鍒楀嚭鎵€鏈変功绛俱€?,
    "filter by tag.": "鎸夋爣绛捐繃婊ゃ€?,
    "sort by: name | last | visits.": "鎺掑簭鏂瑰紡锛歯ame | last | visits銆?,
    "limit results.": "闄愬埗缁撴灉鏁伴噺銆?,
    "offset results.": "缁撴灉鍋忕Щ锛堝垎椤碉級銆?,
    "reverse sort order.": "鍙嶈浆鎺掑簭椤哄簭銆?,
    "output as TSV (Fast Path).": "杈撳嚭 TSV锛堝揩閫熻矾寰勶級銆?,
    "output format: auto|table|tsv|json.": "杈撳嚭鏍煎紡锛歛uto | table | tsv | json銆?,
    "Jump to a bookmark (fuzzy match).": "璺宠浆鍒颁功绛撅紙妯＄硦鍖归厤锛夈€?,
    "fuzzy pattern.": "妯＄硦鍖归厤鍏抽敭瀛椼€?,
    "Open in Explorer.": "鍦ㄨ祫婧愮鐞嗗櫒涓墦寮€銆?,
    "Workspace: open all paths under tag in WT tabs.": "Workspace锛氬湪 Windows Terminal 澶氭爣绛炬墦寮€鏍囩涓嬫墍鏈夎矾寰勩€?,
    "tag name.": "鏍囩鍚嶃€?,
    "Save current directory as bookmark (sv).": "淇濆瓨褰撳墠鐩綍涓轰功绛撅紙sv锛夈€?,
    "bookmark name (optional, defaults to current dir name).": "涔︾鍚嶏紙鍙€夛紝榛樿褰撳墠鐩綍鍚嶏級銆?,
    "tags (comma separated).": "鏍囩锛堥€楀彿鍒嗛殧锛夈€?,
    "Save current directory or specific path as bookmark.": "淇濆瓨褰撳墠鐩綍鎴栨寚瀹氳矾寰勪负涔︾銆?,
    "bookmark name.": "涔︾鍚嶃€?,
    "path (optional, defaults to current dir).": "璺緞锛堝彲閫夛紝榛樿褰撳墠鐩綍锛夈€?,
    "Delete a bookmark.": "鍒犻櫎涔︾銆?,
    "Delete a bookmark (alias).": "鍒犻櫎涔︾锛堝埆鍚嶏級銆?,
    "Clean up dead links.": "娓呯悊鏃犳晥璺緞銆?,
    "delete all dead links without confirmation.": "鏃犻渶纭鐩存帴鍒犻櫎鎵€鏈夋棤鏁堣矾寰勩€?,
    "Check bookmark health (missing paths, duplicates, stale).": "妫€鏌ヤ功绛惧仴搴凤紙缂哄け/閲嶅/杩囨湡锛夈€?,
    "stale threshold in days.": "杩囨湡闃堝€硷紙澶╋級銆?,
    "Update frecency (touch).": "鏇存柊璁块棶棰戞锛坱ouch锛夈€?,
    "Rename a bookmark.": "閲嶅懡鍚嶄功绛俱€?,
    "old name.": "鏃у悕绉般€?,
    "new name.": "鏂板悕绉般€?,
    "Tag management.": "鏍囩绠＄悊銆?,
    "Add tags to a bookmark.": "涓轰功绛炬坊鍔犳爣绛俱€?,
    "Remove tags from a bookmark.": "浠庝功绛剧Щ闄ゆ爣绛俱€?,
    "List all tags and counts.": "鍒楀嚭鎵€鏈夋爣绛惧強鏁伴噺銆?,
    "Rename a tag across all bookmarks.": "閲嶅懡鍚嶆爣绛撅紙鍏ㄥ眬锛夈€?,
    "old tag.": "鏃ф爣绛俱€?,
    "new tag.": "鏂版爣绛俱€?,
    "Show recent bookmarks.": "鏄剧ず鏈€杩戜功绛俱€?,
    "Show statistics.": "鏄剧ず缁熻淇℃伅銆?,
    "Deduplicate bookmarks.": "涔︾鍘婚噸銆?,
    "mode: path | name.": "鍘婚噸妯″紡锛歱ath | name銆?,
    "skip confirmation (interactive mode only).": "璺宠繃纭锛堜粎浜や簰妯″紡锛夈€?,
    "Export bookmarks.": "瀵煎嚭涔︾銆?,
    "format: json | tsv.": "鏍煎紡锛歫son | tsv銆?,
    "output file (optional).": "杈撳嚭鏂囦欢锛堝彲閫夛級銆?,
    "Import bookmarks.": "瀵煎叆涔︾銆?,
    "input file (optional, default stdin).": "杈撳叆鏂囦欢锛堝彲閫夛紝榛樿 stdin锛夈€?,
    "mode: merge | overwrite.": "瀵煎叆妯″紡锛歮erge | overwrite銆?,
    "skip confirmation.": "璺宠繃纭銆?,
    "List all keys (for tab completion).": "杈撳嚭鎵€鏈夐敭锛堢敤浜庤ˉ鍏級銆?,
    "Proxy management.": "浠ｇ悊绠＄悊銆?,
    "Proxy On (pon).": "寮€鍚唬鐞嗭紙pon锛夈€?,
    "proxy url (optional, auto-detect system proxy).": "浠ｇ悊鍦板潃锛堝彲閫夛紝鑷姩妫€娴嬬郴缁熶唬鐞嗭級銆?,
    "skip connectivity test after enabling proxy.": "鍚敤鍚庤烦杩囪繛閫氭€ф祴璇曘€?,
    "no_proxy list.": "no_proxy 鍒楄〃銆?,
    "msys2 root override.": "msys2 鏍圭洰褰曡鐩栥€?,
    "Proxy Off (poff).": "鍏抽棴浠ｇ悊锛坧off锛夈€?,
    "Proxy Status (pst).": "浠ｇ悊鐘舵€侊紙pst锛夈€?,
    "Proxy Exec (px).": "浠ｇ悊鎵ц锛坧x锛夈€?,
    "proxy url (optional).": "浠ｇ悊鍦板潃锛堝彲閫夛級銆?,
    "command and args.": "鍛戒护鍙婂弬鏁般€?,
    "All bookmarks (machine output).": "鎵€鏈変功绛撅紙鏈哄櫒杈撳嚭锛夈€?,
    "Fuzzy search (machine output).": "妯＄硦鎼滅储锛堟満鍣ㄨ緭鍑猴級銆?,
    "pattern.": "鍖归厤鍏抽敭瀛椼€?,
    "Set proxy.": "璁剧疆浠ｇ悊銆?,
    "proxy url (e.g. http://127.0.0.1:7890).": "浠ｇ悊鍦板潃锛堝 http://127.0.0.1:7890锛夈€?,
    "no_proxy list (default: localhost,127.0.0.1).": "no_proxy 鍒楄〃锛堥粯璁?localhost,127.0.0.1锛夈€?,
    "only set for: cargo,git,npm,msys2 (comma separated).": "浠呰缃寚瀹氱洰鏍囷細cargo,git,npm,msys2锛堥€楀彿鍒嗛殧锛夈€?,
    "Delete proxy.": "鍒犻櫎浠ｇ悊銆?,
    "only delete for: cargo,git,npm,msys2 (comma separated).": "浠呭垹闄ゆ寚瀹氱洰鏍囷細cargo,git,npm,msys2锛堥€楀彿鍒嗛殧锛夈€?,
    "Get current git proxy config.": "璇诲彇褰撳墠 git 浠ｇ悊閰嶇疆銆?,
    "Detect system proxy.": "妫€娴嬬郴缁熶唬鐞嗐€?,
    "Test proxy latency.": "娴嬭瘯浠ｇ悊寤惰繜銆?,
    "proxy url.": "浠ｇ悊鍦板潃銆?,
    "targets (comma separated), use \"proxy\" to test proxy itself.": "鐩爣鍒楄〃锛堥€楀彿鍒嗛殧锛涚敤 proxy 娴嬩唬鐞嗚嚜韬級銆?,
    "timeout seconds.": "瓒呮椂绉掓暟銆?,
    "max concurrent probes.": "鏈€澶у苟鍙戞帰娴嬫暟銆?,
    "List listening ports (TCP by default).": "鍒楀嚭鐩戝惉绔彛锛堥粯璁?TCP锛夈€?,
    "show all TCP listening ports.": "鏄剧ず鎵€鏈?TCP 鐩戝惉绔彛銆?,
    "show UDP bound ports.": "鏄剧ず UDP 缁戝畾绔彛銆?,
    "filter port range (e.g. 3000-3999).": "鎸夌鍙ｈ寖鍥磋繃婊わ紙濡?3000-3999锛夈€?,
    "filter by pid.": "鎸?PID 杩囨护銆?,
    "filter by process name (substring).": "鎸夎繘绋嬪悕杩囨护锛堝瓙涓诧級銆?,
    "Kill processes that occupy ports.": "缁堟鍗犵敤绔彛鐨勮繘绋嬨€?,
    "port list, e.g. 3000,8080,5173.": "绔彛鍒楄〃锛堝 3000,8080,5173锛夈€?,
    "tcp only.": "浠?TCP銆?,
    "udp only.": "浠?UDP銆?,
    "Incremental project backup.": "澧為噺椤圭洰澶囦唤銆?,
    "operation and args: `list` | `restore <name>` (default: create backup).": "鎿嶄綔涓庡弬鏁帮細`list` | `restore <name>`锛堥粯璁ゅ垱寤哄浠斤級銆?,
    "for restore: restore a single file (relative path).": "鐢ㄤ簬 restore锛氭仮澶嶅崟涓枃浠讹紙鐩稿璺緞锛夈€?,
    "backup description.": "澶囦唤鎻忚堪銆?,
    "working directory (default: cwd).": "宸ヤ綔鐩綍锛堥粯璁ゅ綋鍓嶇洰褰曪級銆?,
    "dry run (no copy/zip/cleanup).": "婕旂粌锛堜笉澶嶅埗/涓嶅帇缂?涓嶆竻鐞嗭級銆?,
    "skip compression for this run.": "鏈璺宠繃鍘嬬缉銆?,
    "override max backups.": "瑕嗙洊鏈€澶у浠芥暟銆?,
    "add include path (repeatable or comma separated).": "娣诲姞鍖呭惈璺緞锛堝彲閲嶅鎴栭€楀彿鍒嗛殧锛夈€?,
    "add exclude path (repeatable or comma separated).": "娣诲姞鎺掗櫎璺緞锛堝彲閲嶅鎴栭€楀彿鍒嗛殧锛夈€?,
    "skip prompts.": "璺宠繃鎻愮ず/纭銆?,
    "Generate directory tree.": "鐢熸垚鐩綍鏍戙€?,
    "target path (default: cwd).": "鐩爣璺緞锛堥粯璁ゅ綋鍓嶇洰褰曪級銆?,
    "max depth, 0=unlimited.": "鏈€澶ф繁搴︼紝0 涓轰笉闄愩€?,
    "output file.": "杈撳嚭鏂囦欢銆?,
    "include hidden files.": "鍖呭惈闅愯棌鏂囦欢銆?,
    "skip clipboard copy.": "涓嶅鍒跺埌鍓创鏉裤€?,
    "plain output (no box drawing).": "绾枃鏈緭鍑猴紙鏃犳绾匡級銆?,
    "stats only (no output lines).": "浠呯粺璁★紙涓嶈緭鍑烘爲锛夈€?,
    "fast mode (skip sorting and metadata).": "蹇€熸ā寮忥紙璺宠繃鎺掑簭鍜屽厓鏁版嵁锛夈€?,
    "sort by: name | mtime | size.": "鎺掑簭鏂瑰紡锛歯ame | mtime | size銆?,
    "show size for each item (directories show total size).": "鏄剧ず澶у皬锛堢洰褰曟樉绀烘€诲ぇ灏忥級銆?,
    "max output items.": "鏈€澶ц緭鍑洪」鏁般€?,
    "include pattern (repeatable or comma separated).": "鍖呭惈鍖归厤锛堝彲閲嶅鎴栭€楀彿鍒嗛殧锛夈€?,
    "exclude pattern (repeatable or comma separated).": "鎺掗櫎鍖归厤锛堝彲閲嶅鎴栭€楀彿鍒嗛殧锛夈€?,
    "File locking and unlocking.": "鏂囦欢鍗犵敤鏌ヨ/瑙ｉ攣銆?,
    "Show processes locking a file.": "鏄剧ず鍗犵敤鏂囦欢鐨勮繘绋嬨€?,
    "target path.": "鐩爣璺緞銆?,
    "Delete a file or directory.": "鍒犻櫎鏂囦欢鎴栫洰褰曘€?,
    "unlock file if locked.": "鑻ヨ鍗犵敤鍒欒В閿併€?,
    "force kill blocking processes.": "寮哄埗缁撴潫鍗犵敤杩涚▼銆?,
    "schedule deletion on reboot.": "閲嶅惎鍚庡垹闄ゃ€?,
    "dry run.": "婕旂粌/涓嶆墽琛屻€?,
    "force operation bypass protection.": "寮哄埗鎿嶄綔锛堢粫杩囦繚鎶わ級銆?,
    "reason for bypass protection.": "缁曡繃淇濇姢鐨勭悊鐢便€?,
    "Move a file or directory.": "绉诲姩鏂囦欢鎴栫洰褰曘€?,
    "source path.": "婧愯矾寰勩€?,
    "destination path.": "鐩爣璺緞銆?,
    "Rename a file or directory.": "閲嶅懡鍚嶆枃浠舵垨鐩綍銆?,
    "Manage protection rules.": "绠＄悊淇濇姢瑙勫垯銆?,
    "Set a protection rule.": "璁剧疆淇濇姢瑙勫垯銆?,
    "path to protect.": "瑕佷繚鎶ょ殑璺緞銆?,
    "actions to deny (e.g. delete,move,rename).": "绂佹鐨勬搷浣滐紙濡?delete,move,rename锛夈€?,
    "requirements to bypass (e.g. force,reason).": "缁曡繃瑕佹眰锛堝 force,reason锛夈€?,
    "apply NTFS ACL Deny Delete rule (deep Windows protection).": "搴旂敤 NTFS ACL 鍒犻櫎鎷掔粷瑙勫垯锛堟洿寮轰繚鎶わ級銆?,
    "Clear a protection rule.": "娓呴櫎淇濇姢瑙勫垯銆?,
    "path to clear protection.": "瑕佹竻闄や繚鎶ょ殑璺緞銆?,
    "remove NTFS ACL Deny Delete rule as well.": "鍚屾椂绉婚櫎 NTFS ACL 鍒犻櫎鎷掔粷瑙勫垯銆?,
    "Show protection status.": "鏄剧ず淇濇姢鐘舵€併€?,
    "filter by path prefix.": "鎸夎矾寰勫墠缂€杩囨护銆?,
    "Encrypt a file using Windows EFS (or other providers).": "浣跨敤 Windows EFS 鍔犲瘑鏂囦欢锛堟垨鍏朵粬鎻愪緵鑰咃級銆?,
    "use Windows EFS encryption (Encrypting File System).": "浣跨敤 Windows EFS 鍔犲瘑銆?,
    "public key to encrypt to (age format, can be repeated).": "鍔犲瘑鐩爣鍏挜锛坅ge 鏍煎紡锛屽彲閲嶅锛夈€?,
    "encrypt with a passphrase (interactive).": "浣跨敤鍙ｄ护鍔犲瘑锛堜氦浜掞級銆?,
    "output file path (default: <path>.age if not efs).": "杈撳嚭鏂囦欢璺緞锛堥潪 efs 榛樿 <path>.age锛夈€?,
    "Decrypt a file.": "瑙ｅ瘑鏂囦欢銆?,
    "use Windows EFS decryption.": "浣跨敤 Windows EFS 瑙ｅ瘑銆?,
    "identity file to decrypt with (age format, can be repeated).": "瑙ｅ瘑韬唤鏂囦欢锛坅ge 鏍煎紡锛屽彲閲嶅锛夈€?,
    "decrypt with a passphrase (interactive).": "浣跨敤鍙ｄ护瑙ｅ瘑锛堜氦浜掞級銆?,
    "output file path (default: remove .age extension if not efs).": "杈撳嚭鏂囦欢璺緞锛堥潪 efs 榛樿鍘绘帀 .age锛夈€?,
    "Start web dashboard server.": "鍚姩 Web Dashboard 鏈嶅姟銆?,
    "listen port (default: 9527).": "鐩戝惉绔彛锛堥粯璁?9527锛夈€?,
    "Redirect files in a directory into categorized subfolders.": "鎸夎鍒欏皢鐩綍鏂囦欢鍒嗙被鍒板瓙鐩綍銆?,
    "source directory (default: current directory).": "婧愮洰褰曪紙榛樿褰撳墠鐩綍锛夈€?,
    "profile name under config.redirect.profiles (default: \"default\").": "profile 鍚嶇О锛坈onfig.redirect.profiles锛岄粯璁?default锛夈€?,
    "explain why a file would match or not match rules (pure string mode).": "瑙ｉ噴鍖归厤鍘熷洜锛堢函瀛楃涓叉ā寮忥級銆?,
    "show rules coverage summary after a run (printed to stderr).": "杩愯鍚庤緭鍑鸿鍒欒鐩栫巼姹囨€伙紙stderr锛夈€?,
    "show preview summary and require confirmation before executing (interactive unless --yes).": "鎵ц鍓嶆樉绀洪瑙堢粺璁″苟纭锛堜氦浜掓ā寮忥紝鎴栭厤鍚?--yes锛夈€?,
    "review each planned file action interactively (y/n/a/q).": "閫愭潯浜や簰纭姣忎釜璁″垝鎿嶄綔锛坹/n/a/q锛夈€?,
    "query audit log (redirect tx history).": "鏌ヨ瀹¤鏃ュ織锛坮edirect tx 鍘嗗彶锛夈€?,
    "filter audit log by tx id (use with --log).": "鎸?tx 杩囨护瀹¤鏃ュ織锛堥厤鍚?--log锛夈€?,
    "show last N tx summaries (use with --log).": "鏄剧ず鏈€杩?N 鏉?tx锛堥厤鍚?--log锛夈€?,
    "validate config only (no scan/no watch).": "浠呮牎楠岄厤缃紙涓嶆壂鎻?涓?watch锛夈€?,
    "write a plan file instead of executing (json).": "鐢熸垚 plan 鏂囦欢锛坖son锛夛紝涓嶆墽琛屻€?,
    "apply a previously generated plan file (json).": "搴旂敤 plan 鏂囦欢锛坖son锛夈€?,
    "undo a previous redirect by tx id (read from audit.jsonl).": "鎸?tx 鎾ら攢 redirect锛堣 audit.jsonl锛夈€?,
    "watch mode (daemon: continuously apply redirect rules).": "watch 妯″紡锛堝畧鎶ゆ墽琛岋級銆?,
    "show watch status instead of starting watcher (use with --watch).": "鏄剧ず watch 鐘舵€侊紙閰嶅悎 --watch锛夈€?,
    "simulate matching for file names read from stdin (pure string mode).": "浠?stdin 妯℃嫙鍖归厤锛堢函瀛楃涓叉ā寮忥級銆?,
    "dry run (no changes).": "婕旂粌锛屼笉鎵ц銆?,
    "copy instead of move.": "澶嶅埗鏇夸唬绉诲姩銆?,
    "skip confirmations (required for overwrite in non-interactive mode).": "璺宠繃纭锛堥潪浜や簰 overwrite 闇€瑕侊級銆?,
}


def to_kebab(name: str) -> str:
    return name.replace("_", "-")


def normalize_doc(doc: str) -> str:
    s = " ".join(d.strip() for d in doc.splitlines()).strip()
    if not s:
        return ""
    if not s.endswith("."):
        s += "."
    return s


def has_cjk(text: str) -> bool:
    return bool(CJK_RE.search(text))


def ensure_punct(text: str) -> str:
    s = text.strip()
    if not s:
        return s
    if has_cjk(s):
        if s.endswith("."):
            s = s[:-1] + "銆?
        elif not s.endswith(("銆?, "锛?, "锛?)):
            s += "銆?
    else:
        if not s.endswith("."):
            s += "."
    return s


def localize_desc(desc: str) -> str:
    if not desc:
        return desc
    normalized = desc.strip()
    translated = DOC_TRANSLATIONS.get(normalized)
    if translated:
        return ensure_punct(translated)
    if has_cjk(normalized):
        return ensure_punct(normalized)
    return ensure_punct(normalized)


def extract_enum_values(doc: str):
    if not doc:
        return []
    match = re.search(r"([a-zA-Z0-9_-]+(?:\s*\|\s*[a-zA-Z0-9_-]+)+)", doc)
    if not match:
        return []
    raw = match.group(1)
    values = [v.strip() for v in raw.split("|") if v.strip()]
    return values


class Field:
    def __init__(self, name, kind, doc, typ):
        self.name = name
        self.kind = kind
        self.doc = doc
        self.typ = typ

    @property
    def flag(self):
        return f"--{to_kebab(self.name)}"


class StructInfo:
    def __init__(self, name, argh_name, doc, feature):
        self.name = name
        self.argh_name = argh_name
        self.doc = doc
        self.feature = feature
        self.fields = []
        self.subcommand_enum = None


class EnumInfo:
    def __init__(self, name, doc, feature):
        self.name = name
        self.doc = doc
        self.feature = feature
        self.variants = []


def parse_argh_name(attrs):
    for a in attrs:
        m = re.search(r'name\s*=\s*"([^"]+)"', a)
        if m:
            return m.group(1)
    return None


def parse_feature(attrs):
    for a in attrs:
        m = re.search(r'feature\s*=\s*"([^"]+)"', a)
        if m:
            return m.group(1)
    return None


def parse_cli(path: Path):
    lines = path.read_text(encoding="utf-8").splitlines()
    structs = {}
    enums = {}

    pending_doc = []
    pending_attrs = []
    pending_cfg = []
    pending_fromargs = False

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if stripped.startswith("///"):
            pending_doc.append(stripped[3:].strip())
            i += 1
            continue
        if stripped.startswith("#[cfg("):
            pending_cfg.append(stripped)
            i += 1
            continue
        if stripped.startswith("#[derive(FromArgs)]"):
            pending_fromargs = True
            i += 1
            continue
        if pending_fromargs and stripped.startswith("#[argh("):
            pending_attrs.append(stripped)
            i += 1
            continue

        m = re.match(r"pub\s+(struct|enum)\s+(\w+)", stripped)
        if pending_fromargs and m:
            kind = m.group(1)
            name = m.group(2)
            doc = normalize_doc("\n".join(pending_doc))
            feature = parse_feature(pending_cfg)
            argh_name = parse_argh_name(pending_attrs) or to_kebab(name.replace("Cmd", ""))

            if kind == "struct":
                info = StructInfo(name, argh_name, doc, feature)
                i = parse_struct(lines, i, info)
                structs[name] = info
            else:
                info = EnumInfo(name, doc, feature)
                i = parse_enum(lines, i, info)
                enums[name] = info

            pending_doc = []
            pending_attrs = []
            pending_cfg = []
            pending_fromargs = False
            continue

        pending_doc = []
        pending_attrs = []
        pending_cfg = []
        pending_fromargs = False
        i += 1

    return structs, enums


def parse_all_cli():
    paths = [CLI_ENTRY_PATH]
    if CLI_MODULE_DIR.exists():
        for p in sorted(CLI_MODULE_DIR.glob("*.rs")):
            paths.append(p)
    for p in CLI_EXTRA_PATHS:
        if p.exists():
            paths.append(p)

    merged_structs = {}
    merged_enums = {}
    for p in paths:
        if not p.exists():
            continue
        structs, enums = parse_cli(p)
        merged_structs.update(structs)
        merged_enums.update(enums)
    return merged_structs, merged_enums


def parse_struct(lines, start_idx, info: StructInfo):
    depth = lines[start_idx].count("{") - lines[start_idx].count("}")
    i = start_idx + 1
    field_doc = []
    field_attrs = []

    while i < len(lines) and depth > 0:
        line = lines[i]
        depth += line.count("{") - line.count("}")
        stripped = line.strip()
        if depth <= 0:
            break

        if stripped.startswith("///"):
            field_doc.append(stripped[3:].strip())
            i += 1
            continue
        if stripped.startswith("#[argh("):
            attr = stripped
            while ")]" not in stripped and i + 1 < len(lines):
                i += 1
                stripped = lines[i].strip()
                attr += " " + stripped
            field_attrs.append(attr)
            i += 1
            continue

        m = re.match(r"pub\s+(\w+)\s*:\s*([^,]+),", stripped)
        if m:
            name = m.group(1)
            typ = m.group(2).strip()
            attrs = " ".join(field_attrs)
            kind = None
            if "subcommand" in attrs:
                kind = "subcommand"
                info.subcommand_enum = typ
            elif "option" in attrs:
                kind = "option"
            elif "switch" in attrs:
                kind = "switch"
            elif "positional" in attrs:
                kind = "positional"

            if kind:
                doc = normalize_doc("\n".join(field_doc))
                info.fields.append(Field(name, kind, doc, typ))

            field_doc = []
            field_attrs = []

        i += 1

    return i


def parse_enum(lines, start_idx, info: EnumInfo):
    depth = lines[start_idx].count("{") - lines[start_idx].count("}")
    i = start_idx + 1
    while i < len(lines) and depth > 0:
        line = lines[i]
        depth += line.count("{") - line.count("}")
        stripped = line.strip()
        if depth <= 0:
            break
        m = re.match(r"(\w+)\s*\(\s*([\w:]+)\s*\)", stripped)
        if m:
            info.variants.append((m.group(1), m.group(2)))
        i += 1
    return i


def is_optional_type(typ: str) -> bool:
    return typ.startswith("Option<")


def is_vec_type(typ: str) -> bool:
    return typ.startswith("Vec<")


def example_value(field: Field, command_key: str):
    custom = VALUE_OVERRIDES.get((command_key, field.name))
    if custom:
        return custom
    return VALUE_OVERRIDES.get(field.name, "value")


VALUE_OVERRIDES = {
    "path": r"D:\Repo\MyProj",
    "source": r"D:\Downloads",
    "src": r"D:\Temp\a.txt",
    "dst": r"D:\Temp\b.txt",
    "file": r"src\main.rs",
    "name": "work",
    "old": "old",
    "new": "new",
    "tag": "work",
    "tags": "dev,cli",
    "pattern": "proj",
    "url": "http://127.0.0.1:7890",
    "port": "9527",
    "ports": "5173,8080",
    "pid": "12345",
    "range": "3000-3999",
    "profile": "default",
    "key": "proxy.defaultUrl",
    "value": "3",
    "msg": "\"baseline\"",
    "dir": r"D:\Repo\MyProj",
    "out": r".\bookmarks.tsv",
    "output": r".\tree.txt",
    "input": r".\bookmarks.json",
    "include": "src,docs",
    "exclude": "target,.git",
    "noproxy": "\"localhost,127.0.0.1\"",
    "targets": "proxy,github.com,crates.io",
    "timeout": "5",
    "jobs": "3",
    "depth": "2",
    "max_items": "200",
    "reason": "\"cleanup\"",
    "cmd": "-- cargo build",
    "days": "30",
    "limit": "10",
    "offset": "20",
    "env": "RUST_LOG=info",
    "env_file": r".\.env",
    "msys2": r"C:\msys64",
    "only": "cargo,git",
    "retain": "10",
    "deny": "delete,rename",
    "require": "force,reason",
    "identity": r"D:\Keys\me.agekey",
    "to": "age1exampleexampleexampleexampleexample",
    "last": "5",
}

VALUE_OVERRIDES.update(
    {
        ("redirect", "plan"): r".\xun.plan.json",
        ("redirect", "apply"): r".\xun.plan.json",
        ("redirect", "undo"): "redirect_1740000000_1234",
        ("redirect", "tx"): "redirect_1740000000_1234",
        ("redirect", "explain"): "2026-02_report.jpg",
        ("config set", "value"): "http://127.0.0.1:7890",
    }
)


OPTION_DEPENDENCIES = {
    ("redirect", "status"): ["--watch"],
    ("redirect", "tx"): ["--log"],
    ("redirect", "last"): ["--log"],
}


POSITIONAL_VARIANTS = {
    ("bak", "op_args"): [
        "list",
        "restore v12-2026-02-23_1030",
    ],
    ("config set", "value"): [
        "http://127.0.0.1:7890",
        "3",
    ],
    ("__complete", "args"): [
        'redirect --profile de ""',
    ],
}


CUSTOM_EXAMPLES = {
    "ctx set": [
        {
            "command": r"xun ctx set work --proxy http://127.0.0.1:7890",
            "desc": "璁剧疆浠ｇ悊 URL銆?,
            "covers": ["proxy"],
        },
    ],
    "redirect": [
        {
            "command": r'"a.jpg`nreport_2026.pdf`nrandom.xyz" | xun redirect --simulate -f tsv',
            "desc": "浠?stdin 鎵归噺妯℃嫙鍖归厤锛堢函瀛楃涓叉ā寮忥級銆?,
            "covers": ["simulate"],
        },
        {
            "command": r"xun redirect D:\Downloads --watch --status -f json",
            "desc": "璇诲彇 watch 鐘舵€佹枃浠讹紙涓嶅惎鍔?watcher锛夈€?,
            "covers": ["watch", "status"],
        },
        {
            "command": r"xun redirect D:\Downloads --review --dry-run -f table",
            "desc": "閫愭潯棰勮涓嶆墽琛屻€?,
            "covers": ["review", "dry_run"],
        },
    ]
}


CATEGORY_BY_TOP = {
    "__global__": "鍏ㄥ眬缁勫悎锛堥€傜敤浜庢墍鏈夊懡浠わ級",
    "init": "鍒濆鍖栦笌甯姪",
    "completion": "琛ュ叏锛圕ompletion锛?,
    "__complete": "琛ュ叏锛圕ompletion锛?,
    "acl": "ACL锛堟潈闄愶級",
    "config": "閰嶇疆绠＄悊",
    "ctx": "涓婁笅鏂囧垏鎹紙ctx锛?,
    "list": "涔︾锛圔ookmarks锛?,
    "z": "涔︾锛圔ookmarks锛?,
    "o": "涔︾锛圔ookmarks锛?,
    "ws": "涔︾锛圔ookmarks锛?,
    "sv": "涔︾锛圔ookmarks锛?,
    "set": "涔︾锛圔ookmarks锛?,
    "del": "涔︾锛圔ookmarks锛?,
    "delete": "涔︾锛圔ookmarks锛?,
    "check": "涔︾锛圔ookmarks锛?,
    "gc": "涔︾锛圔ookmarks锛?,
    "touch": "涔︾锛圔ookmarks锛?,
    "rename": "涔︾锛圔ookmarks锛?,
    "tag": "涔︾锛圔ookmarks锛?,
    "recent": "涔︾锛圔ookmarks锛?,
    "stats": "涔︾锛圔ookmarks锛?,
    "dedup": "涔︾锛圔ookmarks锛?,
    "export": "涔︾锛圔ookmarks锛?,
    "import": "涔︾锛圔ookmarks锛?,
    "keys": "涔︾锛圔ookmarks锛?,
    "all": "涔︾锛圔ookmarks锛?,
    "fuzzy": "涔︾锛圔ookmarks锛?,
    "proxy": "浠ｇ悊锛圥roxy锛?,
    "pon": "浠ｇ悊锛圥roxy锛?,
    "poff": "浠ｇ悊锛圥roxy锛?,
    "pst": "浠ｇ悊锛圥roxy锛?,
    "px": "浠ｇ悊锛圥roxy锛?,
    "ports": "绔彛锛圥orts锛?,
    "kill": "绔彛锛圥orts锛?,
    "ps": "杩涚▼锛圥rocess锛?,
    "pkill": "杩涚▼锛圥rocess锛?,
    "bak": "澶囦唤锛坆ak锛?,
    "tree": "鐩綍鏍戯紙tree锛?,
    "find": "鏌ユ壘锛坒ind锛?,
    "alias": "鍒悕锛坅lias锛?,
    "lock": "鏂囦欢瑙ｉ攣涓庢搷浣滐紙lock/fs锛?,
    "rm": "鏂囦欢瑙ｉ攣涓庢搷浣滐紙lock/fs锛?,
    "mv": "鏂囦欢瑙ｉ攣涓庢搷浣滐紙lock/fs锛?,
    "ren": "鏂囦欢瑙ｉ攣涓庢搷浣滐紙lock/fs锛?,
    "protect": "闃茶鎿嶄綔淇濇姢锛坧rotect锛?,
    "encrypt": "鍔犲瘑/瑙ｅ瘑锛坈rypt锛?,
    "decrypt": "鍔犲瘑/瑙ｅ瘑锛坈rypt锛?,
    "redirect": "Redirect 鏂囦欢鍒嗙被寮曟搸",
    "serve": "Web Dashboard锛坉ashboard锛?,
    "diff": "Diff锛坉iff锛?,
    "brn": "鎵归噺閲嶅懡鍚嶏紙brn锛?,
    "cstat": "浠ｇ爜浣撴锛坈stat锛?,
    "img": "鍥惧儚澶勭悊锛坕mg锛?,
}


CATEGORY_ORDER = [
    "鍏ㄥ眬缁勫悎锛堥€傜敤浜庢墍鏈夊懡浠わ級",
    "鍒濆鍖栦笌甯姪",
    "琛ュ叏锛圕ompletion锛?,
    "ACL锛堟潈闄愶級",
    "閰嶇疆绠＄悊",
    "涓婁笅鏂囧垏鎹紙ctx锛?,
    "涔︾锛圔ookmarks锛?,
    "浠ｇ悊锛圥roxy锛?,
    "绔彛锛圥orts锛?,
    "杩涚▼锛圥rocess锛?,
    "澶囦唤锛坆ak锛?,
    "鐩綍鏍戯紙tree锛?,
    "鏌ユ壘锛坒ind锛?,
    "鍒悕锛坅lias锛?,
    "鏂囦欢瑙ｉ攣涓庢搷浣滐紙lock/fs锛?,
    "闃茶鎿嶄綔淇濇姢锛坧rotect锛?,
    "鍔犲瘑/瑙ｅ瘑锛坈rypt锛?,
    "Redirect 鏂囦欢鍒嗙被寮曟搸",
    "Web Dashboard锛坉ashboard锛?,
    "Diff锛坉iff锛?,
    "鎵归噺閲嶅懡鍚嶏紙brn锛?,
    "浠ｇ爜浣撴锛坈stat锛?,
    "鍥惧儚澶勭悊锛坕mg锛?,
]


def build_commands(structs, enums):
    commands = []

    top_enum = enums.get("SubCommand")
    if not top_enum:
        return commands

    for _, struct_name in top_enum.variants:
        info = structs.get(struct_name)
        if not info:
            continue
        parent_path = [info.argh_name]
        if info.subcommand_enum:
            sub_enum = enums.get(info.subcommand_enum)
            if not sub_enum:
                continue
            for _, child_struct in sub_enum.variants:
                child_info = structs.get(child_struct)
                if not child_info:
                    continue
                commands.append((parent_path + [child_info.argh_name], child_info))
        else:
            commands.append((parent_path, info))

    return commands


def command_key(path_parts):
    return " ".join(path_parts)


def base_args_for_command(path_parts):
    return ["xun"] + path_parts


def collect_examples(path_parts, info: StructInfo):
    key = command_key(path_parts)
    examples = []
    covered = set()

    fields = [f for f in info.fields if f.kind in ("option", "switch", "positional")]
    positionals = [f for f in fields if f.kind == "positional"]
    required_positionals = [
        f for f in positionals if not is_optional_type(f.typ) and not is_vec_type(f.typ)
    ]
    optional_positionals = [f for f in positionals if f not in required_positionals]

    bases = [base_args_for_command(path_parts)]
    for f in required_positionals:
        positional_variants = POSITIONAL_VARIANTS.get((key, f.name))
        if positional_variants:
            values = positional_variants
        else:
            enum_vals = extract_enum_values(f.doc)
            values = enum_vals if enum_vals else [example_value(f, key)]
        new_bases = []
        for b in bases:
            for v in values:
                new_bases.append(b + [v])
        bases = new_bases
        covered.add(f.name)

    base_desc = localize_desc(info.doc) if info.doc else "鎵ц璇ュ懡浠ゃ€?
    for base in bases:
        examples.append((base, base_desc, set()))

    primary_base = bases[0]

    for f in optional_positionals:
        values = POSITIONAL_VARIANTS.get((key, f.name))
        if values:
            for v in values:
                args = primary_base + [v]
                desc = localize_desc(f.doc) if f.doc else "浣跨敤鍙€変綅缃弬鏁般€?
                examples.append((args, desc, {f.name}))
                covered.add(f.name)
        else:
            enum_vals = extract_enum_values(f.doc)
            if enum_vals:
                for v in enum_vals:
                    args = primary_base + [v]
                    desc = localize_desc(f.doc) if f.doc else "浣跨敤鍙€変綅缃弬鏁般€?
                    examples.append((args, desc, {f.name}))
                    covered.add(f.name)
            else:
                args = primary_base + [example_value(f, key)]
                desc = localize_desc(f.doc) if f.doc else "浣跨敤鍙€変綅缃弬鏁般€?
                examples.append((args, desc, {f.name}))
                covered.add(f.name)

    for f in fields:
        if f.kind == "positional":
            continue
        values = []
        if f.kind == "switch":
            values = [None]
        else:
            enum_vals = extract_enum_values(f.doc)
            if enum_vals:
                values = enum_vals
            else:
                values = [example_value(f, key)]

        for val in values:
            deps = OPTION_DEPENDENCIES.get((key, f.name), [])
            args = primary_base + deps + [f.flag]
            if val is not None:
                args.append(str(val))
            desc = localize_desc(f.doc) if f.doc else "閫夐」銆?
            examples.append((args, desc, {f.name}))
            covered.add(f.name)

    for ex in CUSTOM_EXAMPLES.get(key, []):
        examples.append(([ex["command"]], ex["desc"], set(ex.get("covers", []))))
        covered.update(ex.get("covers", []))

    return examples, covered, {f.name for f in fields}


def format_example(args, desc):
    if len(args) == 1 and args[0].startswith(("xun ", "\"", "'")):
        cmd = args[0]
    else:
        cmd = " ".join(args)
    return f"- 绀轰緥锛歚{cmd}`  \n  璇存槑锛歿desc}"


def generate_markdown(structs, enums):
    sections = {k: [] for k in CATEGORY_ORDER}
    missing = []

    # global examples
    global_examples = [
        (["xun", "--no-color", "list", "-f", "table"], "绂佺敤褰╄壊杈撳嚭銆?, {"no_color"}),
        (["xun", "--version"], "杈撳嚭鐗堟湰鍙峰苟閫€鍑恒€?, {"version"}),
        (["xun", "--quiet", "ports", "-f", "json"], "灏介噺鍑忓皯 UI 杈撳嚭銆?, {"quiet"}),
        (["xun", "--verbose", "redirect", r"D:\Downloads", "-f", "table"], "杈撳嚭鏇村鍘熷洜/缁嗚妭銆?, {"verbose"}),
        (
            ["xun", "--non-interactive", "redirect", r"D:\Downloads", "--confirm", "--yes"],
            "寮哄埗闈炰氦浜掓ā寮忥紙鍗遍櫓鎿嶄綔闇€閰嶅悎 --yes锛夈€?,
            {"non_interactive"},
        ),
    ]
    sections["鍏ㄥ眬缁勫悎锛堥€傜敤浜庢墍鏈夊懡浠わ級"] = []
    for args, desc, _ in global_examples:
        sections["鍏ㄥ眬缁勫悎锛堥€傜敤浜庢墍鏈夊懡浠わ級"].append(format_example(args, desc))

    global_struct = structs.get("Xun")
    if global_struct:
        all_global = {f.name for f in global_struct.fields if f.kind in ("option", "switch")}
        covered_global = set()
        for _, _, cov in global_examples:
            covered_global.update(cov)
        missing_global = all_global - covered_global
        if missing_global:
            missing.append(f"Global options missing examples: {sorted(missing_global)}")

    commands = build_commands(structs, enums)
    commands.sort(key=lambda x: command_key(x[0]))

    for path_parts, info in commands:
        key = command_key(path_parts)
        top = path_parts[0]
        category = CATEGORY_BY_TOP.get(top, "鍏朵粬")
        if category not in sections:
            sections[category] = []

        header = f"#### `{ ' '.join(['xun'] + path_parts) }`"
        if not sections[category] or sections[category][-1] != header:
            sections[category].append(header)

        examples, covered, field_names = collect_examples(path_parts, info)
        for args, desc, _ in examples:
            sections[category].append(format_example(args, desc))

        missing_fields = field_names - covered
        if missing_fields:
            missing.append(f"{key}: missing examples for {sorted(missing_fields)}")

    if missing:
        sys.stderr.write("\n".join(missing) + "\n")

    output = []
    for cat in CATEGORY_ORDER:
        block = sections.get(cat)
        if not block:
            continue
        output.append(f"### {cat}")
        output.extend(block)
        output.append("")

    return "\n".join(output).rstrip()


def replace_section(readme_text, new_section):
    if START_MARKER not in readme_text or END_MARKER not in readme_text:
        raise SystemExit("Commands-Generated.md markers not found.")

    before, rest = readme_text.split(START_MARKER, 1)
    _, after = rest.split(END_MARKER, 1)
    return f"{before}{START_MARKER}\n\n{new_section}\n\n{END_MARKER}{after}"


def main():
    if not CLI_ENTRY_PATH.exists():
        raise SystemExit("src/cli.rs not found")
    structs, enums = parse_all_cli()
    md = generate_markdown(structs, enums)
    readme = README_PATH.read_text(encoding="utf-8")
    updated = replace_section(readme, md)
    README_PATH.write_text(updated, encoding="utf-8")


if __name__ == "__main__":
    main()

