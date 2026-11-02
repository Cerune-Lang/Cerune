#!/usr/bin/env bash
set -euo pipefail

if (($# > 0)); then
    if (($# == 1)) && [[ "$1" == --help || "$1" == -h ]]; then
        printf '%s\n' 'Usage: bash scripts/test.sh' 'Runs formatting checks, Clippy, and all test targets.'
        exit 0
    fi
    printf '%s\n' '[ERROR] This script does not accept test filters. Use cargo test directly.' >&2
    exit 1
fi

repository_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$repository_root"

# Windows側のtarget/debugとは分離します。指定済みの出力先は尊重します。
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$repository_root/target/unix"}

# 任意のLLVM compilerが存在する場合は、テスト前に生成IRへの対応を確認します。
# 明示指定が見つからない場合も失敗にし、暗黙の別compilerへ切り替えません。
if [[ -n "${CERUNE_TEST_LLVM_CLANG+x}" ]] || command -v clang >/dev/null 2>&1; then
    bash scripts/check-llvm-toolchain.sh
fi

printf '%s\n' '=== Format ==='
cargo fmt --check
printf '\n%s\n' '=== Clippy ==='
cargo clippy --all-targets -- -D warnings
printf '\n%s\n' '=== Tests ==='
cargo test --all-targets
