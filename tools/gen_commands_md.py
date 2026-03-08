import sys
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS_DIR))

import gen_readme_commands as grc

CLI_PATH = Path("src/cli.rs")
OUT_PATH = Path("intro/cli/Commands.md")


def option_placeholder(field: grc.Field) -> str:
    enum_vals = grc.extract_enum_values(field.doc)
    if enum_vals:
        if field.name == "proxy" and "off" in enum_vals and "keep" in enum_vals and "url" not in enum_vals:
            return "<url|off|keep>"
        return f"<{'|'.join(enum_vals)}>"
    name = grc.to_kebab(field.name)
    if name in ("path", "src", "dst", "dir", "file", "out", "output", "input"):
        return "<path>"
    if name in ("url", "proxy", "noproxy"):
        return "<url>"
    if name in ("port", "ports"):
        return "<port>"
    if name in ("pid",):
        return "<pid>"
    if name in ("range",):
        return "<start-end>"
    if name in ("format",):
        return "<format>"
    if name in ("mode", "sort"):
        return f"<{name}>"
    return f"<{name}>"


def format_command(path_parts, info: grc.StructInfo) -> str:
    pieces = ["xun"] + path_parts
    positionals = [f for f in info.fields if f.kind == "positional"]
    for f in positionals:
        if grc.is_vec_type(f.typ):
            pieces.append(f"<{grc.to_kebab(f.name)}...>")
        elif grc.is_optional_type(f.typ):
            pieces.append(f"[{grc.to_kebab(f.name)}]")
        else:
            pieces.append(f"<{grc.to_kebab(f.name)}>")
    return " ".join(pieces)


def format_options(info: grc.StructInfo) -> str:
    options = []
    for f in info.fields:
        if f.kind not in ("option", "switch"):
            continue
        flag = f"--{grc.to_kebab(f.name)}"
        if f.kind == "switch":
            options.append(f"`{flag}`")
        else:
            placeholder = option_placeholder(f)
            options.append(f"`{flag} {placeholder}`")
    return "锛?.join(options) if options else "-"


def format_desc(info: grc.StructInfo) -> str:
    if info.doc:
        return grc.localize_desc(info.doc)
    return "鎵ц璇ュ懡浠ゃ€?


def category_for(path_parts):
    top = path_parts[0]
    if top == "init":
        return None
    if top in ("completion", "__complete"):
        return "琛ュ叏锛圕ompletion锛?
    if top == "config":
        return "閰嶇疆绠＄悊"
    if top == "ctx":
        return "Context Switch锛坈tx锛?
    if top in (
        "list",
        "z",
        "o",
        "ws",
        "sv",
        "set",
        "del",
        "delete",
        "check",
        "gc",
        "touch",
        "rename",
        "tag",
        "recent",
        "stats",
        "dedup",
        "export",
        "import",
        "keys",
        "all",
        "fuzzy",
    ):
        return "涔︾鍛戒护"
    if top in ("proxy", "pon", "poff", "pst", "px"):
        return "浠ｇ悊鍛戒护"
    if top in ("ports", "kill"):
        return "绔彛鍛戒护"
    if top == "bak":
        return "澶囦唤鍛戒护"
    if top == "tree":
        return "鐩綍鏍戝懡浠?
    if top == "redirect":
        return "Redirect 鍒嗙被寮曟搸锛坒eature: `redirect`锛?
    if top in ("lock", "rm", "mv", "ren"):
        return "鏂囦欢瑙ｉ攣涓庢搷浣滐紙feature: `lock`锛?
    if top == "protect":
        return "闃茶鎿嶄綔淇濇姢锛坒eature: `protect`锛?
    if top in ("encrypt", "decrypt"):
        return "鏂囦欢鍔犲瘑锛坒eature: `crypt`锛?
    if top == "serve":
        return "Dashboard锛坒eature: `dashboard`锛?
    return None


CATEGORY_ORDER = [
    "琛ュ叏锛圕ompletion锛?,
    "閰嶇疆绠＄悊",
    "Context Switch锛坈tx锛?,
    "涔︾鍛戒护",
    "浠ｇ悊鍛戒护",
    "绔彛鍛戒护",
    "澶囦唤鍛戒护",
    "鐩綍鏍戝懡浠?,
    "Redirect 鍒嗙被寮曟搸锛坒eature: `redirect`锛?,
    "鏂囦欢瑙ｉ攣涓庢搷浣滐紙feature: `lock`锛?,
    "闃茶鎿嶄綔淇濇姢锛坒eature: `protect`锛?,
    "鏂囦欢鍔犲瘑锛坒eature: `crypt`锛?,
    "Dashboard锛坒eature: `dashboard`锛?,
]

SECTION_PRE_NOTES = {
    "Redirect 鍒嗙被寮曟搸锛坒eature: `redirect`锛?: "> 闇€ `--features redirect` 缂栬瘧銆傝鍒欏瓨鍌ㄥ湪 `~/.xun.config.json` 鐨?`redirect.profiles.<profile>`銆?,
    "鏂囦欢瑙ｉ攣涓庢搷浣滐紙feature: `lock`锛?: "> 闇€ `--features lock` 缂栬瘧銆俙rm`/`mv`/`ren` 涓烘枃浠剁郴缁熸搷浣滐紝鍖哄埆浜庝功绛?`del`/`delete`銆?,
    "闃茶鎿嶄綔淇濇姢锛坒eature: `protect`锛?: "> 闇€ `--features protect` 缂栬瘧銆傝鍒欏瓨鍌ㄥ湪 `~/.xun.config.json` 鐨?`protect.rules` 瀛楁銆?,
    "鏂囦欢鍔犲瘑锛坒eature: `crypt`锛?: "> 闇€ `--features crypt` 缂栬瘧銆俙--efs` 璧?Windows EFS 绯荤粺鍔犲瘑锛屽惁鍒欒蛋 age 搴旂敤灞傚姞瀵嗐€?,
    "Dashboard锛坒eature: `dashboard`锛?: "> 闇€ `--features dashboard` 缂栬瘧銆?,
}

SECTION_POST_NOTES = {
    "鏂囦欢瑙ｉ攣涓庢搷浣滐紙feature: `lock`锛?: "閫€鍑虹爜锛歚0` 鎴愬姛 / `2` 鍙傛暟閿欒 / `3` 鏉冮檺涓嶈冻 / `10` 鍗犵敤鏈巿鏉?/ `11` 瑙ｉ攣澶辫触 / `20` 宸茬櫥璁伴噸鍚?,
    "Redirect 鍒嗙被寮曟搸锛坒eature: `redirect`锛?: "Watch 璋冧紭鐜鍙橀噺锛歚XUN_REDIRECT_WATCH_DEBOUNCE_MS`銆乣XUN_REDIRECT_WATCH_SETTLE_MS`銆乣XUN_REDIRECT_WATCH_RETRY_MS`銆乣XUN_REDIRECT_WATCH_SCAN_RECHECK_MS`銆乣XUN_REDIRECT_WATCH_MAX_BATCHES`銆乣XUN_REDIRECT_WATCH_MAX_PATHS`銆乣XUN_REDIRECT_WATCH_MAX_RETRY_PATHS`銆乣XUN_REDIRECT_WATCH_MAX_SWEEP_DIRS`銆乣XUN_REDIRECT_WATCH_SWEEP_MAX_DEPTH`锛涚綉缁滃叡浜紙UNC锛夌害鏉燂細`nBufferLength <= 64KB`銆?,
}


def build_sections(structs, enums):
    sections = {k: [] for k in CATEGORY_ORDER}
    commands = grc.build_commands(structs, enums)
    commands.sort(key=lambda x: grc.command_key(x[0]))

    for path_parts, info in commands:
        category = category_for(path_parts)
        if not category:
            continue
        cmd = format_command(path_parts, info)
        desc = format_desc(info)
        opts = format_options(info)
        sections[category].append((cmd, desc, opts))

    return sections


def render_table(rows):
    lines = ["| 鍛戒护 | 璇存槑 | 閫夐」/澶囨敞 |", "| --- | --- | --- |"]
    for cmd, desc, opts in rows:
        lines.append(f"| `{cmd}` | {desc} | {opts} |")
    return lines


def main():
    if not CLI_PATH.exists():
        raise SystemExit("src/cli.rs not found")

    structs, enums = grc.parse_cli(CLI_PATH)
    sections = build_sections(structs, enums)

    lines = []
    lines.append("# xun 鍛戒护鏂囨。")
    lines.append("")
    lines.append("**鍏ㄥ眬绾﹀畾**")
    lines.append("- `stdout` 杈撳嚭鏈哄櫒鍙鍐呭锛宍stderr` 杈撳嚭浜や簰 UI 涓庤〃鏍笺€?)
    lines.append("- `XUN_UI=1` 寮哄埗琛ㄦ牸杈撳嚭锛堝嵆渚胯绠￠亾閲嶅畾鍚戯級銆?)
    lines.append("- 鎵€鏈夊懡浠ゆ敮鎸?`--help` 鏌ョ湅鍙傛暟璇存槑銆?)
    lines.append("- 鍏ㄥ眬閫夐」锛歚--no-color`锛堟垨 `NO_COLOR=1`锛夈€乣--version`銆乣-q/--quiet`銆乣-v/--verbose`銆乣--non-interactive`銆?)
    lines.append("- 瀵瑰簲鐜鍙橀噺锛歚XUN_QUIET`銆乣XUN_VERBOSE`銆乣XUN_NON_INTERACTIVE`銆?)
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## Shell 闆嗘垚")
    lines.append("")
    lines.append("| 鍛戒护 | 璇存槑 | 澶囨敞 |")
    lines.append("| --- | --- | --- |")
    lines.append("| `xun init powershell` | 杈撳嚭 PowerShell 闆嗘垚鑴氭湰 | 閰嶅悎 `Invoke-Expression` 鎵ц |")
    lines.append("| `xun init bash` | 杈撳嚭 Bash 闆嗘垚鑴氭湰 | 閫傜敤浜?Git Bash/MSYS2 |")
    lines.append("| `xun init zsh` | 杈撳嚭 Zsh 闆嗘垚鑴氭湰 | 閫傜敤浜?Zsh |")
    lines.append("")
    lines.append("---")
    lines.append("")

    for category in CATEGORY_ORDER:
        rows = sections.get(category, [])
        if not rows:
            continue
        lines.append(f"## {category}")
        lines.append("")
        pre = SECTION_PRE_NOTES.get(category)
        if pre:
            lines.append(pre)
            lines.append("")
        lines.extend(render_table(rows))
        lines.append("")
        post = SECTION_POST_NOTES.get(category)
        if post:
            lines.append(post)
            lines.append("")
        lines.append("---")
        lines.append("")

    lines.append("## Shell wrapper 鍒悕")
    lines.append("")
    lines.append("`xun init` 杈撳嚭鐨勮剼鏈腑鍖呭惈锛?)
    lines.append("`x`銆乣sv`銆乣list`銆乣ctx`銆乣delete`銆乣gc`銆乣z`銆乣o`銆乣ws`銆乣pon`銆乣poff`銆乣pst`銆乣px`銆乣rename`銆乣tag`銆乣recent`銆乣stats`銆乣dedup`銆乣bak`銆乣xtree`銆乣xr`銆乣redir`銆?)
    lines.append("鍏朵腑 `xtree` 鐢ㄤ簬閬垮厤瑕嗙洊绯荤粺 `tree` 鍛戒护銆?)
    lines.append("")

    OUT_PATH.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()

