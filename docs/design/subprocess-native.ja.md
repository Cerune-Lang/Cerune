# Nativeの子プロセス実行

[English](subprocess-native.en.md)

**Windows/Linux x86-64のASM・自前Objectで実装済みです。** 基準MIRとSSA経由のMIRを同じLIRへ渡します。要求検査、転送、期限、回収、失敗の選択は[共通runtime](subprocess-generated.ja.md)を使います。Native build/releaseの追加runtime指定・記録・配布は未実装で、子プロセスを含むbuildは説明付きで拒否します。releaseで配布するための依存記録も後続です。

## 生成する処理

[実装](../../src/codegen/x86_64/process.rs)はSASHIMIの型付き命令を組み立てます。ASMの再解析、Cへの変換、IR ExecutorやVMの内部実行は行いません。生成過程は従来のemit-ir / emit-mir / emit-lir / emit-asmと出自記録で確認できます。

| 段階 | Nativeで行う処理 |
| --- | --- |
| 要求 | 評価済みのRequestから、引数のdata/len表と生バイトstdinを準備。NodeId/SourceId/Spanと上限を120バイトのABI要求へ保存 |
| prepare | 共通runtimeが要求を所有した後、一時表とstdinを解放。結果確定済みなら起動用の予約を行わない |
| 予約 | stdout/stderrの合計容量を検査し、両方を起動前に確保。容量不足はOutputBudget、確保失敗はAllocationFailed。片側の確保も取り消す |
| execute | 予約結果を共通runtimeへ渡す。成功時だけ時計取得・起動・転送・回収へ進む |
| 結果 | 32 wordの版・サイズ・予約wordとenum番号を検査。宣言内の名前からタグを選び、出力を1024バイトずつ既存配列へ移す |
| 解放 | ABI結果を明示解放。返却配列は通常の所有処理で解放。停止した接続を再開しない |

Nativeの配列は48バイトの所有者情報と要素領域を1回で確保し、要素を高いアドレスから低いアドレスへ8バイトずつ保存します。stdinはこれを生バイトへ詰め、出力は逆に広げます。短い結果・空結果でも予約容量の使用量は所有者の解放まで保持します。起動後の結果変換では追加のヒープ確保をしません。1024バイトの一時bufferにより変換補助関数のstack使用量を4096バイト未満に保ちます。

LIRの変更による不正な要求型・結果型を機械命令生成前に拒否します。Report宣言の検査は既存runtimeと共有します。ABIの不正応答は、先行出力をflushし、元の操作の位置付きcerune-process-abi-v1診断で停止します。要求の一時コピーを確保できない場合はruntime-v1 code=allocation-failedで停止します。これらは回復可能な子プロセスの失敗結果とは区別します。

## 生成・リンク・起動

対象OSと一致するprocess-c-abiのarchiveと、必要なシステムライブラリを明示してリンクします。archiveの作成方法は[C ABI試験](../../experiments/subprocess/c_abi/README.md)を参照してください。Linuxの例です。

~~~sh
cerune emit-obj examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.o
clang --target=x86_64-unknown-linux-gnu target/process.o target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

Windowsではtargetをx86_64-pc-windows-msvc、archiveをcerune_lang.libとし、kernel32・ntdll・userenv・ws2_32・dbghelp・advapi32・bcryptをリンクします。emit-asmの出力をClangで組み立てる場合も同じarchiveを使います。SSA経由はemit-obj / emit-asmに--ssaを明示します。通常の非最適化IRの観測は変えません。

process.jsonは[CLIと共通の設定](subprocess-cli.ja.md)です。Linux監督のパスも明示します。先頭の--cerune-runtime-v1がある場合だけ設定を解釈し、--以降はプログラム引数として保持します。ファイル操作も含む場合は--file-rootと併用できます。マーカーがなければ接続は未設定です。JSONは上限1 MiB、Windowsの設定パスはUTF-16で開きます。欠落・重複・未知設定、読取り失敗、不正JSONはプログラム開始前に停止します。

## 検証と残る作業

[共通比較スクリプト](../../experiments/subprocess/c_abi/generated.cjs)へ--nativeを渡すと基準MIR・ASM、--native --native-object --native-ssaを渡すとSSA・自前Objectを検証します。Windows/Linuxとも、既存の[実行example](../../examples/process_execution/README.md)と4実行器を各76件比較します。コピー、評価順、短絡、4097バイトの出力、非0終了、時間超過、出力上限、予約失敗、全配列解放、起動引数とファイル接続を確認します。確保失敗・ABI失敗・cleanup値の注入はASM版で実行し、コマンドと生の結果を保存します。

子プロセスを使わない生成物にarchive依存は追加しません。追加runtimeを明示するNative build/release、runtimeとLinux監督の配布、WAT hostは後続です。
