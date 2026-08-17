#!/usr/bin/env bash
# Install the Terminal3 ADK toolchain into WSL. No sudo required — everything
# lands under $HOME (rustup -> ~/.cargo, nvm -> ~/.nvm).
set -uo pipefail

log() { printf '\n=== %s ===\n' "$1"; }

log "starting $(date -u '+%Y-%m-%d %H:%M:%S UTC')"

# ---------------------------------------------------------------- rust
if command -v rustup >/dev/null 2>&1; then
  echo "rustup already present: $(rustup --version 2>/dev/null | head -1)"
else
  log "installing rustup (no sudo, default profile)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
    || { echo "FATAL: rustup install failed"; exit 1; }
fi

export PATH="$HOME/.cargo/bin:$PATH"
command -v rustc >/dev/null 2>&1 || { echo "FATAL: rustc not on PATH after install"; exit 1; }
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"

log "adding wasm32-wasip2 target"
rustup target add wasm32-wasip2 || { echo "FATAL: could not add wasm32-wasip2"; exit 1; }
rustup target list --installed

# ---------------------------------------------------------------- node
if [ -s "$HOME/.nvm/nvm.sh" ]; then
  echo "nvm already installed"
else
  log "installing nvm"
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash \
    || { echo "WARN: nvm install failed"; }
fi

export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

if command -v nvm >/dev/null 2>&1; then
  log "installing node 22 LTS"
  nvm install 22 && nvm alias default 22
  echo "node: $(node --version 2>/dev/null)"
  echo "npm:  $(npm --version 2>/dev/null)"
else
  echo "WARN: nvm unavailable; node must be installed another way"
fi

# ---------------------------------------------------------------- wasm-tools
log "installing wasm-tools (compiles ~100 crates, ~2 min, silent)"
if command -v wasm-tools >/dev/null 2>&1; then
  echo "wasm-tools already present: $(wasm-tools --version)"
else
  cargo install wasm-tools --locked 2>&1 | tail -5 || echo "WARN: wasm-tools install failed (optional — only used to verify the component)"
fi

# ---------------------------------------------------------------- summary
log "SUMMARY"
for t in rustc cargo rustup node npm wasm-tools git gcc; do
  if command -v "$t" >/dev/null 2>&1; then
    printf '%-12s OK   %s\n' "$t" "$($t --version 2>/dev/null | head -1)"
  else
    printf '%-12s MISSING\n' "$t"
  fi
done
echo
echo "wasm targets:"
rustup target list --installed 2>/dev/null | sed 's/^/  /'
echo
echo "disk (WSL root):"
df -h / | tail -1
echo
log "done $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
