#!/usr/bin/env bash
set -euo pipefail

# パスは一つの実行ファイルとして扱い、引数やshell文字列として解釈しません。
llvm_compiler=${CERUNE_TEST_LLVM_CLANG-clang}
if ! command -v -- "$llvm_compiler" >/dev/null 2>&1; then
    printf '[ERROR] LLVM compiler not found: %s\n' "$llvm_compiler" >&2
    exit 1
fi
"$llvm_compiler" --version
# バージョン番号の推測だけでなく、生成IRで使うptrを実際にコンパイルします。
if ! "$llvm_compiler" -Werror -Wno-override-module -x ir -c -o /dev/null - <<'IR'
define i8 @cerune_toolchain_probe(ptr %p) {
entry:
  %value = load i8, ptr %p
  ret i8 %value
}
IR
then
    printf '%s\n' '[ERROR] This compiler cannot compile the LLVM IR used by Cerune (opaque ptr).' \
        'Set CERUNE_TEST_LLVM_CLANG to a compatible compiler, for example /usr/bin/clang-18.' >&2
    exit 1
fi
printf '%s\n' 'LLVM IR capability check passed.'
