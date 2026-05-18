#!/usr/bin/env bash
# 下载 clash-tui release 二进制到 bin/，供 clash tui 直接运行。

set -euo pipefail

# #----Release 配置----
# 支持 Gitea/GitHub 常见页面格式：
#   https://gitea.example.com/owner/clash/releases/tag/v0.1.0
#   https://github.com/owner/clash/releases/tag/v0.1.0
RELEASE_PAGE_URL="https://gitea.example.com/owner/clash/releases/tag/v0.1.0"
ASSET_PREFIX="clash-tui"
INSTALL_BIN_NAME="clash-tui"

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

# #----主流程----
SUFFIX="$(asset_suffix)"
ASSET_NAME="$ASSET_PREFIX-$SUFFIX"
DOWNLOAD_URL="$(download_base_url "$RELEASE_PAGE_URL")/$ASSET_NAME"
TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_FILE"' EXIT

echo "Downloading: $DOWNLOAD_URL"
fetch "$DOWNLOAD_URL" "$TMP_FILE"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$TMP_FILE" "$INSTALL_BIN"
echo "Installed: $INSTALL_BIN"
