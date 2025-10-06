#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: install.sh [--prefix DIR] [--destdir DIR]

Builds the ast-find and web-get binaries in release mode and installs them
into DIR/bin (default: /usr/local/bin). Use --destdir for packaging workflows.
USAGE
}

PREFIX="/usr/local"
DESTDIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      shift
      [[ $# -gt 0 ]] || { echo "--prefix requires a value" >&2; exit 1; }
      PREFIX="$1"
      ;;
    --destdir)
      shift
      [[ $# -gt 0 ]] || { echo "--destdir requires a value" >&2; exit 1; }
      DESTDIR="$1"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

CARGO_CMD=(cargo build --locked --release -p ast-find -p web-get --manifest-path "$PROJECT_ROOT/Cargo.toml")

echo "==> Building release binaries"
"${CARGO_CMD[@]}"

BIN_DIR="$DESTDIR$PREFIX/bin"
mkdir -p "$BIN_DIR"

for bin in ast-find web-get; do
  SRC="$PROJECT_ROOT/target/release/$bin"
  if [[ ! -x "$SRC" ]]; then
    echo "Expected binary not found: $SRC" >&2
    exit 1
  fi
  echo "==> Installing $bin to $BIN_DIR"
  install -Dm755 "$SRC" "$BIN_DIR/$bin"
done

echo "Installation complete. Binaries installed to $BIN_DIR"
