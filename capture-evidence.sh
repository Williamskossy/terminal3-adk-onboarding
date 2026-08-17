#!/usr/bin/env bash
# Capture verifiable evidence logs for the bounty submission.
# Every log here is real command output, written to evidence/logs/*.log.
# The API key is never printed.
set -uo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
OUT="$PROJ/evidence/logs"
mkdir -p "$OUT"

stamp() { date -u '+%Y-%m-%d %H:%M:%S UTC'; }

# ---------------------------------------------------------------- 01 toolchain
{
  echo "# Toolchain — captured $(stamp)"
  echo "# host: Windows 11 + WSL2"
  echo
  . /etc/os-release 2>/dev/null && echo "distro:  $PRETTY_NAME"
  echo "rustc:   $(rustc --version)"
  echo "cargo:   $(cargo --version)"
  echo "rustup:  $(rustup --version 2>/dev/null | head -1)"
  echo "node:    $(node --version)"
  echo "npm:     $(npm --version)"
  echo
  echo "\$ rustup target list --installed"
  rustup target list --installed | sed 's/^/  /'
} > "$OUT/01-toolchain.log" 2>&1
echo "wrote 01-toolchain.log"

# ------------------------------------------------------------------- 02 build
{
  echo "# Walkthrough step 1-2: write + build the TEE contract — captured $(stamp)"
  echo
  echo "\$ git -C z-tenant-flight log --oneline -1"
  git -C "$PROJ/z-tenant-flight" log --oneline -1 2>&1 | sed 's/^/  /'
  echo
  echo "\$ cargo build --target wasm32-wasip2 --release"
  echo "  (already built; showing the artifact and verifying it is a component)"
  echo
  art="$PROJ/artifacts/z_tenant_flight.wasm"
  echo "\$ ls -lh artifacts/z_tenant_flight.wasm"
  ls -lh "$art" | sed 's/^/  /'
  echo
  echo "\$ xxd -l 8 -p artifacts/z_tenant_flight.wasm"
  hdr="$(xxd -l 8 -p "$art")"
  echo "  $hdr"
  case "$hdr" in
    0061736d0d000100) echo "  -> WASM COMPONENT (layer=1) — valid for registration" ;;
    0061736d01000000) echo "  -> bare core module (layer=0) — NOT a component" ;;
  esac
  echo
  echo "\$ sha256sum artifacts/z_tenant_flight.wasm"
  sha256sum "$art" | sed "s|$PROJ/||" | sed 's/^/  /'
} > "$OUT/02-build-contract.log" 2>&1
echo "wrote 02-build-contract.log"

# ------------------------------------------------- 03 quickstart + register run
{
  echo "# Quickstart (connect/authenticate) + Walkthrough step 3 (register)"
  echo "# captured $(stamp)"
  echo "# NOTE: the SDK ships obfuscated; when it throws, its stack dumps the whole"
  echo "#       minified bundle as one line. Lines >600 chars are filtered below."
  echo
  cd "$PROJ" || exit 1
  bash ./run-quickstart.sh 2>&1 | awk 'length($0)<600'
} > "$OUT/03-quickstart-and-register.log" 2>&1
echo "wrote 03-quickstart-and-register.log"

# ---------------------------------------------------------------- 04 node facts
{
  echo "# Node / cluster identity — captured $(stamp)"
  echo
  echo "\$ curl -s https://cn-api.sg.testnet.t3n.terminal3.io/api/trust-manifest"
  curl -s -m 25 "https://cn-api.sg.testnet.t3n.terminal3.io/api/trust-manifest" \
    | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{console.log(JSON.stringify(JSON.parse(s),null,2))}catch(e){console.log(s)}})' \
    | sed 's/^/  /'
  echo
  echo "\$ npm view @terminal3/t3n-sdk version"
  cd "$PROJ/my-t3n-app" && npm view @terminal3/t3n-sdk version 2>/dev/null | sed 's/^/  /'
  echo
  echo "\$ node -p \"require('./node_modules/@terminal3/t3n-sdk/package.json').version\"  # installed"
  node -p "require('./node_modules/@terminal3/t3n-sdk/package.json').version" 2>/dev/null | sed 's/^/  /'
  echo
  echo "# Evidence that the SDK never sends script_name:"
  echo "\$ grep -c 'script_name' node_modules/@terminal3/t3n-sdk/dist/index.esm.js"
  echo "  $(grep -o 'script_name' node_modules/@terminal3/t3n-sdk/dist/index.esm.js | wc -l)"
  echo "\$ grep -n 'script_name' node_modules/@terminal3/t3n-sdk/dist/index.d.ts"
  grep -n 'script_name' node_modules/@terminal3/t3n-sdk/dist/index.d.ts | sed 's/^/  /'
  echo "  (both occurrences are inside ChargeReason — a token.get-usage RESPONSE type)"
} > "$OUT/04-node-and-sdk-versions.log" 2>&1
echo "wrote 04-node-and-sdk-versions.log"

echo
echo "=== captured logs ==="
ls -lh "$OUT" | tail -n +2 | sed 's/^/  /'
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
