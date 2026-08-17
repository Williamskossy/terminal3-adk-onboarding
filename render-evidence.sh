#!/usr/bin/env bash
# Render every captured log in evidence/logs/ to a terminal-styled HTML page in
# evidence/html/, and print "<name>|<height>" for each so the caller can drive a
# headless browser screenshot at the right viewport height.
#
#   bash render-evidence.sh            # all logs
#   bash render-evidence.sh 05 06      # only these
set -uo pipefail

export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

PROJ="/mnt/c/Users/USER/terminal3-adk-bounty"
LOGS="$PROJ/evidence/logs"
HTML="$PROJ/evidence/html"
mkdir -p "$HTML"

# window title shown in the rendered chrome, per log
title_for() {
  case "$1" in
    01-toolchain)              echo "kossy@wsl:~ — toolchain versions" ;;
    02-build-contract)         echo "kossy@wsl:~/terminal3-adk-bounty — build TEE contract" ;;
    03-quickstart-and-register) echo "kossy@wsl:~/terminal3-adk-bounty — quickstart + register" ;;
    04-node-and-sdk-versions)  echo "kossy@wsl:~/terminal3-adk-bounty — node + SDK versions" ;;
    05-bonus-contracts)        echo "kossy@wsl:~/terminal3-adk-bounty — bonus contracts: tests + components" ;;
    06-registration-retry)     echo "kossy@wsl:~/terminal3-adk-bounty — register retry (still blocked)" ;;
    *)                         echo "kossy@wsl:~/terminal3-adk-bounty — $1" ;;
  esac
}

want=("$@")
matches() {
  [ "${#want[@]}" -eq 0 ] && return 0
  for w in "${want[@]}"; do case "$1" in "$w"*) return 0 ;; esac; done
  return 1
}

for log in "$LOGS"/*.log; do
  name="$(basename "$log" .log)"
  matches "$name" || continue
  h="$(node "$PROJ/tools/render-terminal.js" "$log" "$HTML/$name.html" "$(title_for "$name")")"
  echo "$name|$h"
done
