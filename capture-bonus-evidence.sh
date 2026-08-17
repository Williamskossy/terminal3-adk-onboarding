#!/usr/bin/env bash
# Capture evidence for the two BONUS contracts (walkthrough "beyond the first
# contract") and re-attempt registration against the live node.
#
# Writes:
#   evidence/logs/05-bonus-contracts.log
#   evidence/logs/06-registration-retry.log
#
# Timestamps: this host's clocks disagree with real time after a crash/reboot, so
# the caller passes authoritative epoch seconds (from an HTTP Date header) in
# TRUE_EPOCH and every stamp is derived from it. If TRUE_EPOCH is unset we fall
# back to the system clock and say so in the log.
#
# Usage: [TRUE_EPOCH=<epoch>] [SECTIONS=05|06|both] bash capture-bonus-evidence.sh
set -uo pipefail

SECTIONS="${SECTIONS:-both}"

export PATH="$HOME/.cargo/bin:$PATH"
export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

# patient network settings (see FINDINGS #3)
export CARGO_NET_RETRY=10
export CARGO_HTTP_TIMEOUT=300
export CARGO_HTTP_LOW_SPEED_LIMIT=1
export CARGO_HTTP_MULTIPLEXING=false
export CARGO_TERM_PROGRESS_WHEN=never

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
OUT="$PROJ/evidence/logs"
mkdir -p "$OUT"

if [ -n "${TRUE_EPOCH:-}" ]; then
  stamp() { date -u -d "@$((TRUE_EPOCH + SECONDS))" '+%Y-%m-%d %H:%M:%S UTC'; }
  STAMP_NOTE="# clock: system clock is unreliable on this host; stamps are anchored to an authoritative HTTP Date"
else
  stamp() { date -u '+%Y-%m-%d %H:%M:%S UTC'; }
  STAMP_NOTE="# clock: WARNING — system clock used, may be inaccurate on this host"
fi

# ------------------------------------------------------ 05 the bonus contracts
if [ "$SECTIONS" = "both" ] || [ "$SECTIONS" = "05" ]; then
{
  echo "# Bonus criterion: beyond the first contract — two additional TEE contracts"
  echo "# captured $(stamp)"
  echo "$STAMP_NOTE"
  echo

  for name in z-remit-guard z-credit-band; do
    dir="$PROJ/contracts/$name"
    export CARGO_TARGET_DIR="$HOME/.cache/t3n-target/$name"
    echo "==================== $name ===================="
    cd "$dir" || { echo "  MISSING DIRECTORY"; continue; }

    echo
    echo "\$ cargo test --target x86_64-unknown-linux-gnu"
    # keep the per-test lines and the summary; drop cargo's compile chatter
    cargo test --target x86_64-unknown-linux-gnu 2>&1 \
      | grep -E '^(test |running |test result:|error|warning: unused)' \
      | sed 's/^/  /'

    echo
    echo "\$ cargo build --target wasm32-wasip2 --release"
    if cargo build --target wasm32-wasip2 --release 2>&1 | tail -3 | sed 's/^/  /'; then :; fi

    art="$(ls -1 "$CARGO_TARGET_DIR/wasm32-wasip2/release/"*.wasm 2>/dev/null | head -1)"
    if [ -z "$art" ]; then echo "  NO ARTIFACT"; echo; continue; fi
    base="$(basename "$art")"

    echo
    echo "\$ ls -l artifacts/$base   # exact byte count"
    ls -l "$art" | awk '{printf "  %s bytes  (%s)\n", $5, $9}' | sed "s|$CARGO_TARGET_DIR/wasm32-wasip2/release/||"
    echo "  $(du -h "$art" | cut -f1)  as du -h rounds it"

    echo
    echo "\$ xxd -l 8 -p artifacts/$base"
    hdr="$(xxd -l 8 -p "$art")"
    echo "  $hdr"
    case "$hdr" in
      0061736d0d000100) echo "  -> WASM COMPONENT (layer=1) — valid for registration" ;;
      0061736d01000000) echo "  -> bare core module (layer=0) — NOT a component" ;;
      *)                echo "  -> unrecognized header" ;;
    esac

    echo
    echo "\$ sha256sum artifacts/$base"
    sha256sum "$art" | awk '{printf "  %s  %s\n", $1, "'"$base"'"}'

    # exported interface, via jco (already vendored in my-t3n-app) if available.
    # The exports are the point here — §4 of the write-up tabulates them — so
    # show every export line and only summarise the imports.
    jco="$PROJ/my-t3n-app/node_modules/.bin/jco"
    if [ -x "$jco" ]; then
      echo
      echo "\$ jco wit artifacts/$base   # exported interface"
      wit="$("$jco" wit "$art" 2>/dev/null)"
      if [ -n "$wit" ]; then
        echo "$wit" | grep -E '^\s*world ' | head -1 | sed 's/^/  /'
        n_imp="$(echo "$wit" | grep -cE '^\s*import ')"
        echo "    ($n_imp imports: host:tenant, host:interfaces/*, wasi:*)"
        echo "$wit" | grep -E '^\s*export ' | sed 's/^/  /'
      else
        echo "  (jco could not read the component)"
      fi
    fi
    echo
  done

  echo "==================== both artifacts ===================="
  ls -l "$PROJ/artifacts"/*.wasm | awk '{printf "  %10s bytes  %s\n", $5, $9}' | sed "s|$PROJ/||"
} > "$OUT/05-bonus-contracts.log" 2>&1
echo "wrote 05-bonus-contracts.log"
fi

# ------------------------------------------- 06 registration retry (live node)
if [ "$SECTIONS" = "both" ] || [ "$SECTIONS" = "06" ]; then
{
  echo "# Walkthrough step 3 (register) — RE-ATTEMPT against the live testnet node"
  echo "# captured $(stamp)"
  echo "$STAMP_NOTE"
  echo "#"
  echo "# The sponsor confirmed on 2026-08-17 that the server-side fault behind"
  echo "# finding 7 is being fixed. This run re-tests it, so the log below is"
  echo "# either a third independent reproduction or the fix landing."
  echo "#"
  echo "# NOTE: the SDK ships obfuscated; when it throws, its stack dumps the whole"
  echo "#       minified bundle as one line. Lines >600 chars are filtered below."
  echo
  cd "$PROJ" || exit 1
  bash ./run-quickstart.sh 2>&1 | awk 'length($0)<600'
} > "$OUT/06-registration-retry.log" 2>&1
echo "wrote 06-registration-retry.log"
fi

# --------------------------------------------------------------- key safety
echo
echo "=== key safety check: does any log contain the API key? ==="
set -a; . "$PROJ/.env" 2>/dev/null; set +a
if [ -n "${T3N_API_KEY:-}" ] && grep -rqF "$T3N_API_KEY" "$OUT" 2>/dev/null; then
  echo "  !! FAIL — API KEY FOUND IN LOGS. Do not publish."
  grep -rlF "$T3N_API_KEY" "$OUT"
  exit 1
else
  echo "  OK — api key does not appear in any captured log"
fi

echo
echo "=== new logs ==="
ls -lh "$OUT" | tail -n +2 | sed 's/^/  /'
echo
echo "=== did the retry succeed? ==="
if [ ! -f "$OUT/06-registration-retry.log" ]; then
  echo "  (section 06 not run)"
elif grep -q "script_name" "$OUT/06-registration-retry.log"; then
  echo "  STILL BLOCKED — script_name error reproduced again"
  grep -oE "requestId: '?[0-9a-f-]{36}'?" "$OUT/06-registration-retry.log" | sed 's/^/    /'
elif grep -qiE "registered|contract_id" "$OUT/06-registration-retry.log"; then
  echo "  *** REGISTRATION SUCCEEDED — the fix has landed. Update the write-up. ***"
else
  echo "  inconclusive — read the log"
fi
