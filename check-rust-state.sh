#!/usr/bin/env bash
export PATH="$HOME/.cargo/bin:$PATH"
echo "--- dirs ---"
for d in "$HOME/.rustup" "$HOME/.cargo" "$HOME/.cargo/bin" "$HOME/.nvm"; do
  if [ -d "$d" ]; then echo "$d exists ($(du -sh "$d" 2>/dev/null | cut -f1))"; else echo "$d absent"; fi
done
echo
echo "--- ~/.cargo/bin contents ---"
ls -1 "$HOME/.cargo/bin" 2>/dev/null || echo "(none)"
echo
echo "--- toolchains on disk ---"
ls -1 "$HOME/.rustup/toolchains" 2>/dev/null || echo "(none)"
echo
echo "--- do the binaries actually run? ---"
for t in rustup rustc cargo; do
  if command -v "$t" >/dev/null 2>&1; then
    v="$($t --version 2>&1 | head -1)"
    printf '%-8s -> %s\n' "$t" "$v"
  else
    printf '%-8s -> NOT ON PATH\n' "$t"
  fi
done
echo
echo "--- settings.toml ---"
cat "$HOME/.rustup/settings.toml" 2>/dev/null || echo "(none)"
echo
echo "--- any stale download/tmp state ---"
ls -1 "$HOME/.rustup/downloads" 2>/dev/null | head -5 || echo "(no downloads dir)"
ls -1 "$HOME/.rustup/tmp" 2>/dev/null | head -5 || echo "(no tmp dir)"
