#!/usr/bin/env bash
# Build the additional contracts in contracts/*, verify each is a real WASM
# component, and run their native unit tests.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

# patient network settings (see FINDINGS #3)
export CARGO_NET_RETRY=10
export CARGO_HTTP_TIMEOUT=300
export CARGO_HTTP_LOW_SPEED_LIMIT=1
export CARGO_HTTP_MULTIPLEXING=false
export CARGO_TERM_PROGRESS_WHEN=never

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
REF="$PROJ/z-tenant-flight"
mkdir -p "$PROJ/artifacts"

fail=0

for dir in "$PROJ"/contracts/*/; do
  name="$(basename "$dir")"
  echo "==================== $name ===================="

  # vendor the host WIT packages from the reference repo if not already present
  if [ ! -d "$dir/wit/deps" ]; then
    echo "-- vendoring wit/deps from the reference repo"
    mkdir -p "$dir/wit/deps"
    cp -r "$REF/wit/deps/host-interfaces-2.1.0" "$dir/wit/deps/" 2>/dev/null
    cp -r "$REF/wit/deps/host-tenant-1.0.0"     "$dir/wit/deps/" 2>/dev/null
    ls -1 "$dir/wit/deps" | sed 's/^/     /'
  fi

  export CARGO_TARGET_DIR="$HOME/.cache/t3n-target/$name"
  cd "$dir" || { fail=1; continue; }

  echo "-- native unit tests (pure logic, no host)"
  if cargo test --target x86_64-unknown-linux-gnu --quiet 2>&1 | tail -15; then
    echo "   tests OK"
  else
    echo "   TESTS FAILED"
    fail=1
  fi

  echo "-- release build for wasm32-wasip2"
  if ! cargo build --target wasm32-wasip2 --release 2>&1 | tail -20; then
    echo "   BUILD FAILED"
    fail=1
    continue
  fi

  art="$(ls -1 "$CARGO_TARGET_DIR/wasm32-wasip2/release/"*.wasm 2>/dev/null | head -1)"
  if [ -z "$art" ]; then
    echo "   NO ARTIFACT PRODUCED"
    fail=1
    continue
  fi

  hdr="$(xxd -l 8 -p "$art")"
  size="$(du -h "$art" | cut -f1)"
  printf -- "-- artifact: %s (%s)\n   header: %s" "$(basename "$art")" "$size" "$hdr"
  case "$hdr" in
    0061736d0d000100) echo "  -> WASM COMPONENT (layer=1) OK" ;;
    0061736d01000000) echo "  -> bare core module — NOT a component"; fail=1 ;;
    *)                echo "  -> unrecognized header"; fail=1 ;;
  esac
  cp "$art" "$PROJ/artifacts/"
  echo "   copied -> artifacts/$(basename "$art")"

  if command -v wasm-tools >/dev/null 2>&1; then
    echo "-- exported interface:"
    wasm-tools component wit "$art" 2>&1 | grep -E '^\s*(export|import|world)' | head -12 | sed 's/^/     /'
  fi
  echo
done

echo "==================== summary ===================="
ls -lh "$PROJ/artifacts"/*.wasm | sed 's/^/  /'
[ "$fail" -eq 0 ] && echo "ALL OK" || echo "ONE OR MORE STEPS FAILED"
exit "$fail"
