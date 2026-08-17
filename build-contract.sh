#!/usr/bin/env bash
# Clone + build the reference TEE contract.
#
# Sources live in the project folder on /mnt/c so everything stays together,
# but CARGO_TARGET_DIR points at ext4 — cargo build I/O across the 9p mount to
# the Windows filesystem is roughly an order of magnitude slower.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
REPO="$PROJ/z-tenant-flight"
export CARGO_TARGET_DIR="$HOME/.cache/t3n-target/z-tenant-flight"
mkdir -p "$CARGO_TARGET_DIR"

cd "$PROJ" || exit 1

if [ -d "$REPO/.git" ]; then
  echo "=== repo already cloned ==="
  git -C "$REPO" log --oneline -1
else
  echo "=== cloning z-tenant-flight ==="
  git clone --depth 1 https://github.com/Terminal-3/z-tenant-flight.git "$REPO" 2>&1 | tail -3 \
    || { echo "FATAL: clone failed"; exit 1; }
fi

cd "$REPO" || exit 1

echo
echo "=== repo structure ==="
find . -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.wit' -o -name '*.md' \) \
  -not -path './.git/*' | sort | sed 's/^/  /'

echo
echo "=== package identity ==="
grep -E '^(name|version|edition)' Cargo.toml | sed 's/^/  /'
echo "  wit package: $(grep -m1 '^package' wit/world.wit 2>/dev/null)"

echo
echo "=== building (target: wasm32-wasip2, release) ==="
echo "target dir: $CARGO_TARGET_DIR"
start=$SECONDS
if cargo build --target wasm32-wasip2 --release 2>&1 | tail -25; then
  echo "build wall time: $((SECONDS - start))s"
else
  echo "BUILD FAILED after $((SECONDS - start))s"
  exit 1
fi

echo
echo "=== artifact ==="
art="$(ls -1 "$CARGO_TARGET_DIR/wasm32-wasip2/release/"*.wasm 2>/dev/null | head -1)"
if [ -z "$art" ]; then
  echo "FATAL: no .wasm produced"
  ls -la "$CARGO_TARGET_DIR/wasm32-wasip2/release/" 2>/dev/null | head -20
  exit 1
fi
ls -lh "$art" | sed 's/^/  /'

# copy the artifact back into the project folder so it sits with the sources
mkdir -p "$PROJ/artifacts"
cp "$art" "$PROJ/artifacts/"
echo "  copied -> artifacts/$(basename "$art")"

echo
echo "=== is it a component (not a bare module)? ==="
if command -v wasm-tools >/dev/null 2>&1; then
  wasm-tools component wit "$art" 2>&1 | head -40
else
  # component preamble: magic \0asm, version 0x0d 0x00, layer 0x01 0x00
  hdr="$(xxd -l 8 -p "$art")"
  echo "  header bytes: $hdr"
  case "$hdr" in
    0061736d0d000100) echo "  -> WASM COMPONENT (layer=1) OK" ;;
    0061736d01000000) echo "  -> bare core module (layer=0) — NOT a component" ;;
    *)                echo "  -> unrecognized header" ;;
  esac
  echo "  (install wasm-tools for the full WIT interface dump)"
fi
