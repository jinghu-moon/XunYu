#!/bin/bash
# =============================================================================
# xun.sh — Bookmark Shell Wrapper (Bash/Zsh/MSYS2)
# Relies on `xun bookmark`
# =============================================================================

XUN_EXE="${XUN_EXE:-xun}"

# Core Wrapper: Handles magic lines via Output Capture
_xun_apply_magic() {
    local line
    local out_lines=()
    while IFS= read -r line; do
        if [[ "$line" == __BM_CD__* ]]; then
            local target="${line#__BM_CD__ }"
            if command -v cygpath &>/dev/null; then
                target=$(cygpath -u "$target")
            fi
            cd "$target" || return 1
        elif [[ "$line" == __CD__:* ]]; then
            local target="${line#__CD__:}"
            if command -v cygpath &>/dev/null; then
                target=$(cygpath -u "$target")
            fi
            cd "$target" || return 1
        elif [[ "$line" == __ENV_SET__:* ]]; then
            local kv="${line#__ENV_SET__:}"
            local k="${kv%%=*}"
            local v="${kv#*=}"
            export "$k=$v"
        elif [[ "$line" == __ENV_UNSET__:* ]]; then
            local k="${line#__ENV_UNSET__:}"
            unset "$k"
        else
            out_lines+=("$line")
        fi
    done
    if [ ${#out_lines[@]} -gt 0 ]; then
        printf '%s\n' "${out_lines[@]}"
    fi
}

_xun_wrapper() {
    local out
    out=$("$XUN_EXE" "$@")
    local ret=$?
    [ -z "$out" ] && return $ret
    printf '%s\n' "$out" | _xun_apply_magic
    return $ret
}

alias xun="$XUN_EXE"
bm() {
    "$XUN_EXE" bookmark "$@"
}

ctx() {
    if [ -z "$XUN_CTX_STATE" ]; then
        local tmp="${TEMP:-${TMPDIR:-/tmp}}"
        export XUN_CTX_STATE="$tmp/xun-ctx-$$.json"
    fi
    _xun_wrapper ctx "$@"
}

z() {
    _xun_wrapper bookmark z "$@"
}

zi() {
    _xun_wrapper bookmark zi "$@"
}

o() {
    "$XUN_EXE" bookmark o "$@"
}

oi() {
    "$XUN_EXE" bookmark oi "$@"
}
