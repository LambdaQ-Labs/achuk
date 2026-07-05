#!/bin/sh
# Achuk installer — https://achuk.dev
#
#   curl -fsSL https://achuk.dev/install.sh | sh
#
# Downloads the latest Achuk release for your platform, unpacks it into
# ~/.achuk, and puts `achuk` on your PATH. No sudo, no system toolchain.
set -eu

REPO="LambdaQ-Labs/achuk"
PREFIX="${ACHUK_HOME:-$HOME/.achuk}"
BIN_DIR="$PREFIX/bin"

say()  { printf '\033[1;36machuk\033[0m %s\n' "$1"; }
err()  { printf '\033[1;31merror\033[0m %s\n' "$1" >&2; exit 1; }

# --- detect platform -------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin) os="macos" ;;
  Linux)  os="linux" ;;
  *) err "unsupported OS: $os (macOS and Linux only for now)" ;;
esac
case "$arch" in
  arm64|aarch64) arch="arm64" ;;
  x86_64|amd64)  arch="x64" ;;
  *) err "unsupported architecture: $arch" ;;
esac
TARGET="$os-$arch"

# Only ship the targets our release CI actually builds.
case "$TARGET" in
  macos-arm64|linux-x64) : ;;
  *) err "no prebuilt binary for $TARGET yet — build from source: https://github.com/$REPO" ;;
esac

# --- resolve the version ---------------------------------------------------
VERSION="${ACHUK_VERSION:-latest}"
if [ "$VERSION" = "latest" ]; then
  # ask the GitHub API for the latest tag (no jq dependency)
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$VERSION" ] || err "could not determine the latest release (set ACHUK_VERSION=vX.Y.Z)"
fi

ASSET="achuk-$VERSION-$TARGET.tar.gz"
URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET"

# --- download + unpack -----------------------------------------------------
say "installing $VERSION for $TARGET"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$URL" -o "$tmp/$ASSET" || err "download failed: $URL"

rm -rf "$PREFIX"
mkdir -p "$PREFIX"
tar -xzf "$tmp/$ASSET" -C "$PREFIX" || err "unpack failed"
chmod +x "$BIN_DIR"/* 2>/dev/null || true

# --- PATH ------------------------------------------------------------------
"$BIN_DIR/achuk" --version >/dev/null 2>&1 || err "installed binary failed to run"

if command -v achuk >/dev/null 2>&1 && [ "$(command -v achuk)" = "$BIN_DIR/achuk" ]; then
  : # already on PATH
else
  # append to the user's shell profile once
  case "${SHELL:-}" in
    */zsh) profile="$HOME/.zshrc" ;;
    */bash) profile="$HOME/.bashrc" ;;
    *) profile="$HOME/.profile" ;;
  esac
  line="export PATH=\"$BIN_DIR:\$PATH\""
  if [ ! -f "$profile" ] || ! grep -qF "$BIN_DIR" "$profile"; then
    printf '\n# Achuk\n%s\n' "$line" >> "$profile"
    say "added $BIN_DIR to PATH in $profile"
  fi
  export PATH="$BIN_DIR:$PATH"
fi

say "installed $("$BIN_DIR/achuk" --version)"
cat <<EOF

  Get started:
    achuk new hello
    cd hello
    achuk run

  (Restart your shell or run: export PATH="$BIN_DIR:\$PATH")
EOF
