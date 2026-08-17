#!/usr/bin/env bash
# Set up the Node/TS app and run the quickstart connect+authenticate step.
# Reads T3N_API_KEY from ../.env — the key is never echoed.
set -uo pipefail

export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

# tolerate a slow link
export npm_config_fetch_timeout=300000
export npm_config_fetch_retries=5
export npm_config_fetch_retry_maxtimeout=120000

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
APP="$PROJ/my-t3n-app"
cd "$APP" || exit 1

echo "=== node ==="
echo "  node $(node --version)  npm $(npm --version)"

# --- load the key without printing it -------------------------------------
if [ ! -f "$PROJ/.env" ]; then
  echo "FATAL: $PROJ/.env not found"
  exit 1
fi
set -a
# shellcheck disable=SC1091
. "$PROJ/.env"
set +a

if [ -z "${T3N_API_KEY:-}" ] || [ "$T3N_API_KEY" = "PASTE_YOUR_KEY_HERE" ]; then
  echo "FATAL: T3N_API_KEY is missing or still the placeholder in .env"
  exit 1
fi
echo "  api key loaded: ${#T3N_API_KEY} chars (value never printed)"

# --- project scaffold -----------------------------------------------------
if [ ! -f package.json ]; then
  echo
  echo "=== npm init ==="
  npm init -y >/dev/null 2>&1
  npm pkg set type=module          # required — quickstart.ts uses top-level await
  npm pkg set name=t3n-adk-walkthrough
  echo "  package.json created (type=module)"
else
  echo "  package.json already present (type=$(node -p "require('./package.json').type||'commonjs'"))"
fi

# --- deps ------------------------------------------------------------------
if [ ! -d node_modules/@terminal3 ] || [ ! -d node_modules/tsx ]; then
  echo
  echo "=== installing @terminal3/t3n-sdk + tsx (slow link tolerated) ==="
  if ! npm install @terminal3/t3n-sdk tsx 2>&1 | tail -12; then
    echo "FATAL: npm install failed"
    exit 1
  fi
else
  echo "  deps already installed"
fi

echo
echo "=== installed SDK version ==="
node -p "require('./node_modules/@terminal3/t3n-sdk/package.json').version" 2>/dev/null \
  | sed 's/^/  @terminal3\/t3n-sdk /' || echo "  (could not read version)"

# --- run -------------------------------------------------------------------
echo
echo "=== running quickstart.ts ==="
npx tsx quickstart.ts
rc=$?
echo
echo "=== exit code: $rc ==="
if [ -f did.json ]; then
  echo "--- did.json ---"
  cat did.json
fi
exit $rc
