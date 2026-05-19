#!/usr/bin/env bash
# 下载 rustup 安装脚本或 clash-tui release 二进制到 bin/。

set -euo pipefail

# #----下载配置----
GITHUB_RELEASE_BASE="https://github.com/owner/clash/releases/tag"
GITEA_RELEASE_BASE="https://gitea.example.com/owner/clash/releases/tag"
RUSTUP_URL="https://sh.rustup.rs"
ASSET_PREFIX="clash-tui"
INSTALL_BIN_NAME="clash-tui"
RUSTUP_BIN_NAME="rustup-init.sh"

# #----路径定位----
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="$PROJECT_DIR/bin"
INSTALL_BIN="$INSTALL_DIR/$INSTALL_BIN_NAME"

# #----工具函数----
die() {
    echo "download: $*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage:
  download.sh rustup
  download.sh VERSION [--github|--gitea]
EOF
}

asset_suffix() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    [ "$os" = "Linux" ] || die "暂只支持 Linux：$os"
    case "$arch" in
        x86_64|amd64) printf 'linux-amd64-musl' ;;
        aarch64|arm64) printf 'linux-arm64-musl' ;;
        *) die "暂不支持架构：$arch" ;;
    esac
}

release_page_url() {
    local version="$1" source="$2"
    case "$source" in
        github) printf '%s/%s' "$GITHUB_RELEASE_BASE" "$version" ;;
        gitea) printf '%s/%s' "$GITEA_RELEASE_BASE" "$version" ;;
        *) die "未知下载源：$source" ;;
    esac
}

download_base_url() {
    local url="$1"
    case "$url" in
        */releases/tag/*) printf '%s' "${url/\/releases\/tag\//\/releases\/download\/}" ;;
        */releases/download/*) printf '%s' "$url" ;;
        *) die "RELEASE_PAGE_URL 需要是 releases/tag 或 releases/download 页面：$url" ;;
    esac
}

fetch() {
    local url="$1" output="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$output" "$url"
    else
        die "需要 curl 或 wget 下载 release"
    fi
}

download_rustup() {
    local tmp_file
    tmp_file="$(mktemp)"
    trap 'rm -f "$tmp_file"' RETURN

    echo "Downloading: $RUSTUP_URL"
    fetch "$RUSTUP_URL" "$tmp_file"
    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$tmp_file" "$INSTALL_DIR/$RUSTUP_BIN_NAME"
    echo "Installed: $INSTALL_DIR/$RUSTUP_BIN_NAME"
}

download_tui() {
    local version="$1" source="$2" suffix asset_name download_url tmp_file
    suffix="$(asset_suffix)"
    asset_name="$ASSET_PREFIX-$suffix"
    download_url="$(download_base_url "$(release_page_url "$version" "$source")")/$asset_name"
    tmp_file="$(mktemp)"
    trap 'rm -f "$tmp_file"' RETURN

    echo "Downloading: $download_url"
    fetch "$download_url" "$tmp_file"
    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$tmp_file" "$INSTALL_BIN"
    echo "Installed: $INSTALL_BIN"
}

# #----主入口----
case "${1:-}" in
    rustup)
        shift
        [ "$#" -eq 0 ] || die "rustup 不接受额外参数"
        download_rustup
        ;;
    -h|--help|help|"")
        usage
        ;;
    *)
        version="$1"
        source="github"
        shift
        while [ "$#" -gt 0 ]; do
            case "$1" in
                --github) source="github"; shift ;;
                --gitea) source="gitea"; shift ;;
                *) usage >&2; exit 1 ;;
            esac
        done
        download_tui "$version" "$source"
        ;;
esac
