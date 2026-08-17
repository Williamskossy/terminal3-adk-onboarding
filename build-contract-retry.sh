#!/usr/bin/env bash
# Retry the contract build with a network config that tolerates a slow link.
#
# cargo's defaults give up if a transfer moves <10 bytes in 30s. On a slow or
# jittery connection that aborts a download that would have completed.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

# be patient instead of fast
export CARGO_NET_RETRY=10
export CARGO_HTTP_TIMEOUT=300           # per-transfer ceiling (default 30)
export CARGO_HTTP_LOW_SPEED_LIMIT=1     # bytes/sec floor before abort (default 10)
export CARGO_HTTP_MULTIPLEXING=false    # HTTP/1 is steadier on flaky links
export CARGO_TERM_PROGRESS_WHEN=never

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
REPO="$PROJ/z-tenant-flight"
export CARGO_TARGET_DIR="$HOME/.cache/t3n-target/z-tenant-flight"

cd "$REPO" || exit 1

echo "=== net settings ==="
echo "  retry=$CARGO_NET_RETRY timeout=${CARGO_HTTP_TIMEOUT}s low_speed_limit=${CARGO_HTTP_LOW_SPEED_LIMIT}B/s multiplexing=$CARGO_HTTP_MULTIPLEXING"

# Fetch dependencies separately from compiling, so a network stall is retried
# without also discarding compile progress.
echo
echo "=== fetching dependencies (attempt 1) ==="
attempt=1
until cargo fetch --target wasm32-wasip2 2>&1 | tail -8; do
  attempt=$((attempt + 1))
  if [ "$attempt" -gt 4 ]; then
    echo "FATAL: dependency fetch failed after $((attempt - 1)) attempts"
    exit 1
  fi
  echo
  echo "=== fetch stalled — retrying (attempt $attempt) ==="
  sleep 5
done
echo "dependencies fetched"

echo
echo "=== building (offline, deps already local) ==="
start=$SECONDS
if cargo build --target wasm32-wasip2 --release --offline 2>&1 | tail -30; then
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
  exit 1
fi
ls -lh "$art" | sed 's/^/  /'
mkdir -p "$PROJ/artifacts"
cp "$art" "$PROJ/artifacts/"
echo "  copied -> artifacts/$(basename "$art")"

echo
echo "=== component check ==="
hdr="$(xxd -l 8 -p "$art")"
echo "  header: $hdr"
case "$hdr" in
  0061736d0d000100) echo "  -> WASM COMPONENT (layer=1) OK" ;;
  0061736d01000000) echo "  -> bare core module — NOT a component" ;;
  *)                echo "  -> unrecognized header" ;;
esac
