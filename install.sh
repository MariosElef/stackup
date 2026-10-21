#!/bin/sh
# Install stackup on Linux or macOS:
#   curl -fsSL https://raw.githubusercontent.com/MariosElef/stackup/main/install.sh | sh
#   wget -qO- https://raw.githubusercontent.com/MariosElef/stackup/main/install.sh | sh
# Installs the latest release into ~/.local/bin (STACKUP_INSTALL_DIR changes it). STACKUP_VERSION
# picks a version, like v0.1.0.
set -eu

REPO="MariosElef/stackup"
BASE="${STACKUP_RELEASE_BASE:-https://github.com/$REPO/releases}"
DEST="${STACKUP_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

# curl or wget, whichever there is: fetch URL to FILE (or to stdout with -).
fetch() {
    if command -v curl >/dev/null 2>&1; then
        if [ "$2" = "-" ]; then curl -fsSL "$1"; else curl -fsSL -o "$2" "$1"; fi
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        fail "need curl or wget"
    fi
}

target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$os" in
        Linux) os_part="unknown-linux-gnu" ;;
        Darwin) os_part="apple-darwin" ;;
        *) fail "this is for Linux and macOS: on Windows use install.ps1 (see the README)" ;;
    esac
    case "$arch" in
        x86_64 | amd64) arch_part="x86_64" ;;
        arm64 | aarch64) arch_part="aarch64" ;;
        *) fail "no build for the $arch processor" ;;
    esac
    printf '%s-%s' "$arch_part" "$os_part"
}

latest() {
    # The release page redirects to the newest tag; the tag is the last part of its address.
    if command -v curl >/dev/null 2>&1; then
        curl -fsSLI -o /dev/null -w '%{url_effective}' "$BASE/latest" | sed 's#.*/##'
    else
        wget -qS --spider "$BASE/latest" 2>&1 | sed -n 's#^ *Location: .*/tag/##p' | tail -n 1 | tr -d '\r'
    fi
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | cut -d ' ' -f 1
    else fail "need sha256sum or shasum to check the download"; fi
}

main() {
    triple=$(target)
    version="${STACKUP_VERSION:-}"
    [ -n "$version" ] || version=$(latest)
    [ -n "$version" ] || fail "could not find the latest version: set STACKUP_VERSION, like v0.1.0"
    name="stackup-$version-$triple"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT

    say "Downloading stackup $version for $triple..."
    fetch "$BASE/download/$version/$name.tar.gz" "$tmp/$name.tar.gz"
    fetch "$BASE/download/$version/$name.tar.gz.sha256" "$tmp/$name.tar.gz.sha256"

    expected=$(cut -d ' ' -f 1 "$tmp/$name.tar.gz.sha256")
    actual=$(sha256_of "$tmp/$name.tar.gz")
    [ "$expected" = "$actual" ] || fail "the download does not match its checksum: not installing it"

    tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
    mkdir -p "$DEST"
    cp "$tmp/$name/stackup" "$DEST/stackup"
    chmod 755 "$DEST/stackup"
    say "Installed $DEST/stackup"
    case ":$PATH:" in
        *":$DEST:"*) ;;
        *) say "Add it to your PATH: export PATH=\"$DEST:\$PATH\"" ;;
    esac
}

main
