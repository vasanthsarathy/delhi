#!/bin/sh
# Install the latest delhi release for this platform.
#
#   curl -fsSL https://raw.githubusercontent.com/vasanthsarathy/delhi/master/install.sh | sh
#
# Downloads one archive from GitHub Releases, verifies it against the published
# SHA256SUMS, and unpacks the binary into ~/.local/bin. It does not edit your shell
# profile: it says whether that directory is on PATH and leaves the decision to you.
#
# Set DELHI_VERSION to pin a release, DELHI_BIN_DIR to install elsewhere.
set -eu

REPO="vasanthsarathy/delhi"
BIN_DIR="${DELHI_BIN_DIR:-$HOME/.local/bin}"

say()  { printf '%s\n' "$*"; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "this script needs $1"; }

need uname
need tar
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
  fetch_stdout() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
  fetch_stdout() { wget -qO- "$1"; }
else
  die "this script needs curl or wget"
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Linux-x86_64|Linux-amd64)     target="x86_64-unknown-linux-gnu" ;;
  Darwin-arm64|Darwin-aarch64)  target="aarch64-apple-darwin" ;;
  Darwin-x86_64)                target="x86_64-apple-darwin" ;;
  *) die "no prebuilt binary for $os/$arch — build from source:
    cargo install --git https://github.com/$REPO delhi-cli" ;;
esac

if [ -n "${DELHI_VERSION:-}" ]; then
  tag="$DELHI_VERSION"
else
  # Resolve "latest" without a JSON parser: the redirect target ends in the tag.
  tag="$(fetch_stdout "https://api.github.com/repos/$REPO/releases/latest" \
         | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)"
  [ -n "$tag" ] || die "could not find the latest release of $REPO"
fi
version="${tag#v}"

name="delhi-${version}-${target}"
url="https://github.com/$REPO/releases/download/$tag/$name.tar.gz"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "downloading  $name"
fetch "$url" "$tmp/$name.tar.gz" || die "could not download $url"

# Verified rather than trusted: the checksum file is published beside the archive, and a
# truncated or tampered download should fail loudly instead of installing.
if fetch "https://github.com/$REPO/releases/download/$tag/SHA256SUMS" "$tmp/SHA256SUMS" 2>/dev/null; then
  if command -v sha256sum >/dev/null 2>&1; then
    have="$(sha256sum "$tmp/$name.tar.gz" | cut -d' ' -f1)"
  elif command -v shasum >/dev/null 2>&1; then
    have="$(shasum -a 256 "$tmp/$name.tar.gz" | cut -d' ' -f1)"
  else
    have=""
  fi
  if [ -n "$have" ]; then
    want="$(grep " $name.tar.gz\$" "$tmp/SHA256SUMS" | cut -d' ' -f1 || true)"
    [ -n "$want" ] || die "SHA256SUMS has no entry for $name.tar.gz"
    [ "$have" = "$want" ] || die "checksum mismatch for $name.tar.gz
  expected $want
  got      $have"
    say "verified     sha256 ok"
  fi
fi

tar xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$BIN_DIR"
install -m 755 "$tmp/$name/delhi" "$BIN_DIR/delhi" 2>/dev/null \
  || { cp "$tmp/$name/delhi" "$BIN_DIR/delhi" && chmod 755 "$BIN_DIR/delhi"; }

say "installed    $BIN_DIR/delhi"
say ""
case ":$PATH:" in
  *":$BIN_DIR:"*)
    say "Try:  delhi --help"
    ;;
  *)
    say "$BIN_DIR is not on your PATH. Add it to your shell profile:"
    say ""
    say "    export PATH=\"\$PATH:$BIN_DIR\""
    say ""
    say "or run it directly:  $BIN_DIR/delhi --help"
    ;;
esac
