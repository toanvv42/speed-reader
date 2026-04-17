#!/usr/bin/env sh
# Install speed-reader from GitHub releases.
#
# Quick install:
#   curl -fsSL https://raw.githubusercontent.com/toanvv42/speed-reader/master/install.sh | sh
#
# With options:
#   curl -fsSL .../install.sh | sh -s -- --version v0.1.0
#   curl -fsSL .../install.sh | PREFIX=/usr/local sh

set -eu

REPO="toanvv42/speed-reader"
BIN="speed-reader"
PREFIX="${PREFIX:-$HOME/.local}"
VERSION=""

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --prefix)  PREFIX="$2";  shift 2 ;;
    -h|--help)
      cat <<EOF
Install speed-reader.

Options:
  --version VERSION   install a specific release (e.g. v0.1.0). default: latest
  --prefix  PATH      install prefix (binary goes to \$PREFIX/bin). default: \$HOME/.local
  -h, --help          show this help

Env vars:
  PREFIX              same as --prefix
EOF
      exit 0
      ;;
    *) printf 'unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

need() { command -v "$1" >/dev/null 2>&1 || { printf 'error: %s is required\n' "$1" >&2; exit 1; }; }
need curl
need tar
need uname

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os_tag="apple-darwin" ;;
  *)
    printf 'error: unsupported OS: %s\n' "$os" >&2
    printf 'speed-reader currently ships prebuilt macOS binaries only.\n' >&2
    printf 'install from source instead:\n  cargo install --git https://github.com/%s\n' "$REPO" >&2
    exit 1
    ;;
esac

case "$arch" in
  arm64|aarch64) arch_tag="aarch64" ;;
  x86_64|amd64)  arch_tag="x86_64" ;;
  *) printf 'error: unsupported arch: %s\n' "$arch" >&2; exit 1 ;;
esac

target="${arch_tag}-${os_tag}"

if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | awk -F'"' '/"tag_name"/ {print $4; exit}')"
  if [ -z "$VERSION" ]; then
    printf 'error: could not resolve latest release tag\n' >&2
    exit 1
  fi
fi

asset="${BIN}-${target}.tar.gz"
url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"

printf '» target:  %s\n' "$target"
printf '» version: %s\n' "$VERSION"
printf '» prefix:  %s\n' "$PREFIX"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf '» fetching %s\n' "$url"
if ! curl -fsSL -o "$tmp/$asset" "$url"; then
  printf 'error: download failed (no asset for %s in %s?)\n' "$target" "$VERSION" >&2
  exit 1
fi

tar -xzf "$tmp/$asset" -C "$tmp"

bin_dir="$PREFIX/bin"
mkdir -p "$bin_dir"
install -m 0755 "$tmp/$BIN" "$bin_dir/$BIN"

printf '» installed %s\n' "$bin_dir/$BIN"

case ":${PATH}:" in
  *":${bin_dir}:"*) ;;
  *)
    printf '\nnote: %s is not on $PATH. add it to your shell rc:\n' "$bin_dir"
    printf '  export PATH="%s:$PATH"\n' "$bin_dir"
    ;;
esac
