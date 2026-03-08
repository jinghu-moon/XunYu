pub(crate) fn completion_bash() -> &'static str {
    r#"
_xun_complete_static() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"
    local sub="${COMP_WORDS[1]}"
    local subcommands="init completion config ctx list z open workspace save set del delete check gc touch rename tag recent stats dedup export import proxy pon poff pst px ports kill keys all fuzzy bak tree env video lock rm mv renfile protect encrypt decrypt serve redirect"
    local formats="auto table tsv json"
    local proxy_sub="set del get detect test"
    local ctx_sub="set use off list show del rename"
    local tree_sort="name mtime size"

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
        return
    fi
    if [[ "$prev" == "-f" || "$prev" == "--format" ]]; then
        COMPREPLY=( $(compgen -W "$formats" -- "$cur") )
        return
    fi
    if [[ "$sub" == "redirect" && "$prev" == "--profile" ]]; then
        COMPREPLY=( $(compgen -W "$(_xun_redirect_profiles)" -- "$cur") )
        return
    fi
    if [[ "$sub" == "redirect" && ( "$prev" == "--undo" || "$prev" == "--tx" ) ]]; then
        COMPREPLY=( $(compgen -W "$(_xun_redirect_txs)" -- "$cur") )
        return
    fi
    if [[ "$sub" == "ctx" && $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "$ctx_sub" -- "$cur") )
        return
    fi
    if [[ "$sub" == "tree" && "$prev" == "--sort" ]]; then
        COMPREPLY=( $(compgen -W "$tree_sort" -- "$cur") )
        return
    fi
    if [[ "$sub" == "proxy" && $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "$proxy_sub" -- "$cur") )
        return
    fi
}

_xun_config_path() {
    if [ -n "$XUN_CONFIG" ]; then echo "$XUN_CONFIG"; else echo "$HOME/.xun.config.json"; fi
}

_xun_db_path() {
    if [ -n "$XUN_DB" ]; then echo "$XUN_DB"; else echo "$HOME/.xun.json"; fi
}

_xun_audit_path() {
    local db=$(_xun_db_path)
    local dir
    dir=$(dirname "$db")
    echo "$dir/audit.jsonl"
}

_xun_redirect_profiles() {
    local cfg=$(_xun_config_path)
    [ -f "$cfg" ] || return
    if command -v python &>/dev/null; then
        python - "$cfg" <<'PY'
import json, sys
path = sys.argv[1]
try:
    with open(path, 'r', encoding='utf-8') as f:
        cfg = json.load(f)
    profiles = cfg.get('redirect', {}).get('profiles', {})
    if isinstance(profiles, dict):
        for k in profiles.keys():
            print(k)
except Exception:
    pass
PY
        return
    fi
    if command -v jq &>/dev/null; then
        jq -r '.redirect.profiles | keys[]?' "$cfg" 2>/dev/null
        return
    fi
    grep -o '"profiles"[[:space:]]*:[[:space:]]*{[^}]*}' "$cfg" 2>/dev/null | \
        grep -o '"[^"]*"[[:space:]]*:' | sed 's/[": ]//g'
}

_xun_redirect_txs() {
    local audit=$(_xun_audit_path)
    [ -f "$audit" ] || return
    tail -n 200 "$audit" | \
        sed -n 's/.*"tx"[[:space:]]*:[[:space:]]*"\([^"]\+\)".*/\1/p; s/.*tx=\([^"[:space:]]\+\).*/\1/p' | \
        awk '!seen[$0]++'
}

_xun_complete_dynamic() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local cmd="${COMP_WORDS[0]}"
    if [[ -n "$XUN_DISABLE_DYNAMIC_COMPLETE" ]]; then
        return 1
    fi
    local args=("${COMP_WORDS[@]:1}")
    if [[ "$cmd" != "xun" && "$cmd" != "x" && "$cmd" != "xyu" && "$cmd" != "xy" ]]; then
        args=("$cmd" "${args[@]}")
    fi
    local out
    local code=0
    if [[ -n "$XUN_COMPLETE_TIMEOUT_MS" && "$XUN_COMPLETE_TIMEOUT_MS" =~ ^[0-9]+$ ]] && command -v timeout &>/dev/null; then
        local timeout_sec
        timeout_sec=$(awk -v ms="$XUN_COMPLETE_TIMEOUT_MS" 'BEGIN { printf "%.3f", ms/1000 }')
        out=$(timeout "${timeout_sec}s" xun __complete "${args[@]}" 2>/dev/null)
        code=$?
        if [[ $code -eq 124 || $code -eq 137 ]]; then
            return 1
        fi
    else
        out=$(xun __complete "${args[@]}" 2>/dev/null)
        code=$?
    fi
    if [[ $code -ne 0 ]]; then
        return 1
    fi
    local sentinel=""
    local directive=0
    local exts=""
    local candidates=()
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        if [[ "$line" == __XUN_COMPLETE__=* ]]; then
            sentinel="$line"
            continue
        fi
        local val="${line%%$'\t'*}"
        candidates+=("$val")
    done <<< "$out"

    if [[ "$sentinel" == "__XUN_COMPLETE__=fallback" ]]; then
        return 1
    fi
    if [[ -z "$sentinel" ]]; then
        return 1
    fi
    local version=""
    if [[ "$sentinel" =~ v=([0-9]+) ]]; then
        version=${BASH_REMATCH[1]}
    fi
    if [[ "$version" != "1" ]]; then
        return 1
    fi
    if [[ "$sentinel" =~ directive=([0-9]+) ]]; then
        directive=${BASH_REMATCH[1]}
    fi
    if [[ "$sentinel" =~ ext=([A-Za-z0-9_|.-]+) ]]; then
        exts=${BASH_REMATCH[1]}
    fi

    COMPREPLY=()
    for c in "${candidates[@]}"; do
        COMPREPLY+=("$c")
    done

    if (( directive & 2 )); then
        compopt -o nospace 2>/dev/null
    fi
    if (( directive & 4 )) && [[ ${#COMPREPLY[@]} -eq 0 ]]; then
        COMPREPLY=( $(compgen -d -- "$cur") )
    fi
    if (( directive & 8 )) && [[ ${#COMPREPLY[@]} -eq 0 ]] && [[ -n "$exts" ]]; then
        IFS='|' read -r -a ext_arr <<< "$exts"
        local files
        files=$(compgen -f -- "$cur")
        for f in $files; do
            for e in "${ext_arr[@]}"; do
                if [[ "$f" == *.$e ]]; then
                    COMPREPLY+=("$f")
                    break
                fi
            done
        done
    fi
    return 0
}

_xun_complete() {
    if ! _xun_complete_dynamic; then
        _xun_complete_static
    fi
}

complete -F _xun_complete xun x xyu xy z o delete rename
"#
}
