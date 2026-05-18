#!/usr/bin/env bash
# 注册 clash 单入口函数，让 on/off 能影响当前 shell 的代理环境变量。

set -euo pipefail

# #----路径定位----
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLASH_ENTRY="$SCRIPT_DIR/clash"

# #----注入配置----
BLOCK_START="#====Clash===="
BLOCK_END="#====Clash End===="
FUNCTION_BLOCK="$BLOCK_START
clash() {
    source \"$CLASH_ENTRY\" \"\$@\"
}
CLASH_QUIET=1 clash env
unset CLASH_QUIET
$BLOCK_END"

inject() {
    local file="$1"
    [ -f "$file" ] || return 0
    sed -i "/$BLOCK_START/,/$BLOCK_END/d" "$file"
    printf '\n%s\n' "$FUNCTION_BLOCK" >> "$file"
    echo "Updated $file"
}

remove_block() {
    local file="$1"
    [ -f "$file" ] || return 0
    sed -i "/$BLOCK_START/,/$BLOCK_END/d" "$file"
    echo "Updated $file"
}

# #----主入口----
case "${1:-install}" in
    install)
        chmod +x "$CLASH_ENTRY"
        inject "$HOME/.zshrc"
        inject "$HOME/.bashrc"
        if ! command -v yq >/dev/null 2>&1; then
            echo "Warning: yq not found. Install it before using clash on/tun/config:"
            echo "  sudo apt install yq"
        fi
        echo "Done. Reload your shell or run: source ~/.zshrc"
        ;;
    uninstall)
        remove_block "$HOME/.zshrc"
        remove_block "$HOME/.bashrc"
        echo "Removed clash shell function"
        ;;
    *)
        echo "Usage: $0 [install|uninstall]" >&2
        exit 1
        ;;
esac
