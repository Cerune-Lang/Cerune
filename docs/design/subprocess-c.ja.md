# 生成Cの子プロセス接続

[English](subprocess-c.en.md)

**Windows x86-64 MSVC・Linux x86-64 GNUで実装・検証済みです。** 共通IRのprocess.runを[既存runtimeのC ABI](../../experiments/subprocess/c_abi/README.md)へ接続します。LLVM・QBE・Native・WATの実行操作と、runtime・Linux監督の配布統合は後続です。

## 生成・リンク・起動

子プロセス操作を含むemit-cはtarget指定が必須です。実行中OSから選ばず、生成Cにもtarget検査を残します。操作を含まない既存C生成には新しいリンク依存を追加しません。Rust APIはemit_c_with_target(program, target, call_limit)です。targetを持たない既存APIは、子プロセス操作に位置付き診断を返します。

Linuxでは次の手順で構築できます。process.jsonは[既存の明示設定](subprocess-cli.ja.md)です。linux_supervisorには構築した監督の絶対パスを記載します。

~~~sh
cargo build --example subprocess_linux_supervisor
cargo rustc --lib --features process-c-abi --target x86_64-unknown-linux-gnu --crate-type staticlib -- --print native-static-libs
cargo run --quiet -- emit-c examples/process_execution/registered.ceru --target x86_64-unknown-linux-gnu -o target/process.c
clang --target=x86_64-unknown-linux-gnu -std=c11 target/process.c target/x86_64-unknown-linux-gnu/debug/libcerune_lang.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -o target/process
target/process --cerune-runtime-v1 --process-config process.json --
~~~

Windowsでは同じソースを次の手順で構築します。MSVC/Windows SDKの開発環境が必要です。

~~~powershell
cargo rustc --lib --features process-c-abi --target x86_64-pc-windows-msvc --crate-type staticlib -- --print native-static-libs
cargo run --quiet -- emit-c examples/process_execution/registered.ceru --target x86_64-pc-windows-msvc -o target/process.c
clang --target=x86_64-pc-windows-msvc -std=c11 -D_CRT_SECURE_NO_WARNINGS target/process.c target/x86_64-pc-windows-msvc/debug/cerune_lang.lib -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -ladvapi32 -lbcrypt -o target/process.exe
target/process.exe --cerune-runtime-v1 --process-config process.json --
~~~

リンクは利用者が明示して行います。Rustの版やtargetを変える場合はnative-static-libsの出力も確認してください。生成CにABI宣言を含めるため、外部ヘッダの探索は不要です。archiveは現時点では処理系crateから構築する試作配布形態で、Native build/releaseの完成成果物ではありません。

先頭に専用マーカーがある場合だけ設定を解釈します。--以降はプログラム引数です。マーカーがなければ全引数を維持し、接続は未設定です。ファイルI/Oも使うプログラムでは--file-rootと併用できます。重複・欠落・未知設定、読めない設定、不正JSONはソース実行前に停止します。JSONは最大1 MiBで、CLIと同じ解析器を使います。空ファイルは不正設定であり、未接続の指定とは区別します。

## 生成物で見える処理

| 順序 | 処理と責務 |
| --- | --- |
| 1 | 共通IRからProcessRunをC IRへ保持。要求評価は一度だけ。短絡・左右の評価順は既存の順序変換に従う |
| 2 | 要求の文字列・配列をABI入力へ変換。Cのu8要素はint64_t格納から生バイトへ明示的にコピー |
| 3 | 固定Outcomeと空配列を準備し、prepareで共通runtimeの要求検査と転送領域確保を行う |
| 4 | Pendingなら両返却配列を予約。容量不足・確保失敗は追加領域を取り消し、executeへ失敗を渡す。子は起動しない |
| 5 | 予約成功後だけ時計・起動・転送・回収へ進む。Reportと保持済み出力を位置指定で読み取る |
| 6 | u64 wordを公開型へ変換。タグはIRのバリアント名から決める。非使用payloadは0で初期化する |
| 7 | 結果を解放。接続停止・未回収OS資源は保持。配列は共通IRの所有・解放処理へ渡す |

返却配列は両上限×8バイトを既存の配列上限へ計上します。短い結果や長さ0でも予約capacityは配列解放まで残します。意味上の上限、サイズの物理表現、malloc失敗を分けます。通常配列の確保失敗時停止の規則は変えません。両配列の保持長を確認してから、追加確保なしで有効長と初期化数を確定します。

ReportのNodeId/SourceId/Spanは共通IR操作の出自を保持します。要求コピーなどABI外の資源不足は既存runtime診断、ABI状態の不正はcerune-process-abi-v1と操作位置で停止します。起動・転送・回収の失敗は従来どおりOutcomeです。観測のためのreport/copyは実行や解放を進めません。

## 検証と残件

[比較スクリプト](../../experiments/subprocess/c_abi/generated.cjs)は[既存example](../../examples/process_execution/README.md)を生成CとHIR/MIR/SSA/VMで動かします。両OSで64比較に加え、確保の4箇所の強制失敗、起動設定の異常、誤targetのコンパイル拒否を確認します。全配列解放後の使用量0も検査します。

通常終了、出自、未接続、検査・予約失敗、コピー、評価順は出力を比較します。出力上限と時間切れでは、別OS実行の通知順で終了状態や他方向の部分出力が変わり得ます。実行記録は変更せず、出力上限では確定したstdoutと失敗、時間切れではTimeoutと後続処理を検査します。模擬接続での厳密な通知列比較は共通runtimeで継続します。

C ABIの互換性保証、追加runtimeのmanifest、Linux監督の配置・同梱は配布対応で仕上げます。生成Cを使う場合は現在も一致するarchiveの明示リンクが必要です。
