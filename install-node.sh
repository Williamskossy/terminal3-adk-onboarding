#!/usr/bin/env bash
# Install nvm + Node 22 into $HOME (no sudo).
set -uo pipefail

if [ ! -s "$HOME/.nvm/nvm.sh" ]; then
  echo "=== installing nvm ==="
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh 2>/dev/null | bash 2>&1 | tail -5
else
  echo "nvm already present"
fi

export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

if ! command -v nvm >/dev/null 2>&1; then
  echo "FATAL: nvm not available after install"
  exit 1
fi

echo
echo "=== installing node 22 ==="
nvm install 22 2>&1 | tail -6
nvm alias default 22 2>&1 | tail -2

echo
echo "=== verify (fresh login shell, as later scripts will see it) ==="
bash -lc 'export NVM_DIR="$HOME/.nvm"; . "$NVM_DIR/nvm.sh"; echo "node: $(node --version)"; echo "npm:  $(npm --version)"; echo "which node: $(which node)"'
