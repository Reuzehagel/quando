#!/usr/bin/env bash
# Every check CI runs, quiet unless something fails: tests (with the benchmark score), clippy, the wasm build.
set -uo pipefail
cd "$(dirname "$0")/.."
fail=0

out=$(cargo test --color never -- --nocapture 2>&1) || { echo "$out" | grep -vE '^\s+(Compiling|Running|Finished|Doc-tests)' ; fail=1; }
echo "$out" | grep -E 'agree \(' || true
[ $fail = 0 ] && echo "tests: ok"

out=$(cargo clippy --color never --all-targets -- -D warnings 2>&1) && echo "clippy: ok" || { echo "$out" | grep -vE '^\s+(Checking|Compiling|Finished)'; fail=1; }

out=$(cargo build --color never --release --target wasm32-unknown-unknown 2>&1) && echo "wasm: ok" || { echo "$out" | grep -vE '^\s+(Compiling|Finished)'; fail=1; }

exit $fail
