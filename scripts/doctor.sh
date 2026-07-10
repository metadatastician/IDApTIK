#!/usr/bin/env bash
# Report the presence and version of every toolchain IDApTIK depends on.
set -uo pipefail
cd "$(dirname "$0")/.."

check() {
  local name="$1" bin="$2"; shift 2
  if command -v "$bin" >/dev/null 2>&1; then
    printf '  \033[1;32m✓\033[0m %-10s %s\n' "$name" "$("$@" 2>&1 | head -n1)"
  else
    printf '  \033[1;31m✗\033[0m %-10s not installed\n' "$name"
  fi
}

echo "IDApTIK toolchain status:"
echo "Rust (gameplay truth):"
check rustc  rustc  rustc --version
check cargo  cargo  cargo --version
check clippy cargo-clippy cargo-clippy --version
echo "Systems / FFI / config / tasks:"
check zig    zig    zig version
check just   just   just --version
check nickel nickel nickel --version
check idris2 idris2 idris2 --version
echo "BEAM (multiplayer / session):"
check erlang erl    erl -eval 'io:format("~s",[erlang:system_info(otp_release)]), halt().' -noshell
check elixir elixir elixir --version
check mix    mix    mix --version
echo "Manager:"
check mise   mise   mise --version
