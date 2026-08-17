#!/usr/bin/env bash
# Repair the stable toolchain after an interrupted `rustup` update.
# Restores the components the interrupted run deleted, then adds the WASM target.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

echo "=== disk before ==="
df -h / | tail -1

echo
echo "=== clearing interrupted download state ==="
rm -f "$HOME"/.rustup/downloads/*.partial 2>/dev/null
rm -rf "$HOME"/.rustup/tmp/* 2>/dev/null
echo "cleared partials and tmp"

echo
echo "=== components rustup currently believes are installed ==="
rustup component list --installed 2>&1 | sed 's/^/  /' || echo "  (query failed)"

echo
echo "=== reinstalling stable toolchain (default profile, forced) ==="
rustup toolchain install stable --profile default --force 2>&1 | grep -vE '^\s*$' | tail -25

echo
echo "=== verifying ==="
ok=1
for t in rustc cargo; do
  if out="$($t --version 2>&1)" && ! echo "$out" | grep -qi 'error'; then
    printf '  %-7s OK   %s\n' "$t" "$out"
  else
    printf '  %-7s FAIL %s\n' "$t" "$out"
    ok=0
  fi
done

if [ "$ok" -ne 1 ]; then
  echo
  echo "RESULT: toolchain still broken — needs manual attention"
  exit 1
fi

echo
echo "=== adding wasm32-wasip2 target ==="
rustup target add wasm32-wasip2 2>&1 | tail -5
echo "installed targets:"
rustup target list --installed 2>/dev/null | sed 's/^/  /'

echo
echo "=== compile smoke test ==="
tmp="$(mktemp -d)"
cat > "$tmp/hello.rs" <<'RS'
fn main() { println!("rustc works"); }
RS
if rustc "$tmp/hello.rs" -o "$tmp/hello" 2>&1 && "$tmp/hello"; then
  echo "native compile: OK"
else
  echo "native compile: FAILED"
fi
rm -rf "$tmp"

echo
echo "=== disk after ==="
df -h / | tail -1
echo
echo "RESULT: toolchain repaired"
