#!/usr/bin/env bash
echo "--- distro ---"
. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME"
echo "--- tools ---"
for t in curl gcc cc git node npm cargo rustup pkg-config; do
  if command -v "$t" >/dev/null 2>&1; then
    printf '%-11s OK   %s\n' "$t" "$($t --version 2>/dev/null | head -1)"
  else
    printf '%-11s MISSING\n' "$t"
  fi
done
echo "--- identity + disk ---"
echo "USER=$(whoami)  HOME=$HOME"
df -h / | tail -1
echo "--- windows mount ---"
if [ -d /mnt/c/Users/USER ]; then echo "/mnt/c/Users/USER reachable"; else echo "C: NOT mounted"; fi
echo "--- can we sudo without a password prompt? ---"
if sudo -n true 2>/dev/null; then echo "sudo: passwordless OK"; else echo "sudo: WILL PROMPT for password (interactive)"; fi
