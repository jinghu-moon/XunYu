#!/bin/bash
# =============================================================================
# xun.sh — Bookmark Manager (Bash/Zsh/MSYS2 Wrapper)
# Relies on xun.exe (Pure Rust)
# =============================================================================

XUN_EXE="${XUN_EXE:-xun}"

# Core Wrapper: Handles magic lines via Output Capture
_xun_apply_magic() {
    local line
    local out_lines=()
    while IFS= read -r line; do
        if [[ "$line" == __CD__:* ]]; then
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
    # Capture stdout (machine data), stderr passes through (UI)
    out=$("$XUN_EXE" "$@")
    local ret=$?
    [ -z "$out" ] && return $ret
    printf '%s\n' "$out" | _xun_apply_magic
    return $ret
}

# Aliases / Functions matching original UX
alias xun="$XUN_EXE"
alias sv="$XUN_EXE set"
alias list="$XUN_EXE list"
alias delete="$XUN_EXE del"
alias gc="$XUN_EXE gc"

# Context switch
ctx() {
    if [ -z "$XUN_CTX_STATE" ]; then
        local tmp="${TEMP:-${TMPDIR:-/tmp}}"
        export XUN_CTX_STATE="$tmp/xun-ctx-$$.json"
    fi
    _xun_wrapper ctx "$@"
}

# Jump (z)
z() {
    _xun_wrapper z "$@"
}

# Open in Explorer (o) - Reuses 'z' logic but opens instead of cd
o() {
    local out
    out=$("$XUN_EXE" z "$@")
    if [[ "$out" == __CD__:* ]]; then
        local target="${out#__CD__:}"
        explorer.exe "$target"
    fi
}
