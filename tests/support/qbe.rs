// 生成物を比較するテストではホストOSからABIを選ばず、Linuxを明示します。
// 実行テストは実行環境に対応したターゲットを個別に指定します。
pub fn compile(source: &str) -> Result<String, cerune_lang::diagnostic::Diagnostic> {
    cerune_lang::compile_to_qbe_with_target(
        source,
        Some(cerune_lang::codegen::qbe::Target::X86_64UnknownLinuxGnu),
    )
}
