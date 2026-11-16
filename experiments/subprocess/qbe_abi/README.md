# 子プロセスQBE ABI試験

[English](README.en.md)

**2026-11-16：QBE 1.3から共通Rust runtimeを呼ぶ試験はWindows/Linuxで成功しました。** [本体のemit-qbe](../../../docs/design/subprocess-qbe.ja.md)にも接続済みです。この独立試験は外部関数の呼出し方を引き続き検証します。

## 確認した呼出し方

[probe.ssa](probe.ssa)を変更せず、amd64_winとamd64_sysvへ渡します。C frontendは検査用の呼出し元だけをコンパイルします。runtimeの7関数は手書きQBEから呼び、IR ExecutorやVMを内部利用しません。

| 操作 | QBE ILの表現 | 確認すること |
| --- | --- | --- |
| create | 16バイトの`:bytes = { l, l }`を値渡し | Windowsではコピーのアドレス、Linuxではポインタと長さの2整数として渡る。targetと版は明示 |
| prepare | 120バイトをalloc8＋blitで一時領域へコピーしてポインタ渡し | runtimeはprepareの終了までに引数・stdinを所有。元データの書換え後も実行内容を保持 |
| execute | contextと予約結果をlで渡し、wの状態番号を受領 | 予約成功時だけ起動。容量不足・確保失敗を別の結果として返す |
| report | context、u64配列、word数を渡す | 32 word、出自、最上位bit付き上限、UINT64_MAXを保持 |
| copy | ポインタとlの計6引数 | レジスタとstackの引数位置、反復読取り、任意バイト、書込み数 |
| release / destroy | context／context格納先のポインタ | 二重操作を拒否。結果解放で停止した接続を再開せず、破棄後のhandleはNULL |

生成ASMのcreateでは、Windowsはrcx／rdx／r8／r9へ版・target・Bytesコピーのアドレス・出力先を渡し、呼出し用に32バイトを確保します。Linuxはrdi／rsi／rdx／rcx／r8へ版・target・data・len・出力先を渡します。Clangが正規Cヘッダから生成した参考ASMも保存します。

この7関数ではASMの書換えや追加の回避処理なしでassemble・link・実行できました。この結果をQBE全機能の検証や、既存のWindows関連調査の解決とは扱いません。研究ブランチや既存emitterの回避処理は変更しません。

## 実行

Rust 1.98で構築した同一targetのarchive、QBE 1.3、Clang、Node、対象OSの開発ライブラリを使います。archiveとLinux監督は先に[C ABI試験](../c_abi/README.md)の手順で構築してください。出力先は新規ディレクトリです。

~~~powershell
node experiments/subprocess/qbe_abi/check.cjs x86_64-pc-windows-msvc target/qbe-1.3/qbe.exe clang target/process-qbe-abi target/process-c-abi/build/x86_64-pc-windows-msvc/debug/cerune_lang.lib
~~~

~~~sh
node experiments/subprocess/qbe_abi/check.cjs x86_64-unknown-linux-gnu target/qbe-1.3/qbe clang target/process-qbe-abi target/process-c-abi/build/x86_64-unknown-linux-gnu/debug/libcerune_lang.a /absolute/path/to/subprocess_linux_supervisor
~~~

指定targetがhostと違う場合は実行しません。両targetのASMを生成・assembleし、指定したtargetだけをリンクして実行します。QBEには-O0/-O2の切替えはなく、ILを通常のQBEの処理で変換します。試験のO0/O2はC検査コードにだけ適用し、QBEの最適化有無の比較とは呼びません。各コマンドは120秒、記録出力は8 MiBを上限とし、失敗・時間超過・出力超過は試験失敗にします。

CIはQBE 1.3配布archiveのSHA-256をd587905d620dc5e1d2bfa7c2cc642b9b837aa89a3188c6e37b53d756cf66e320に固定します。実行スクリプト自体は指定したQBE実行ファイルを使い、そのhashを記録します。バージョン番号を自己判定したことにはしません。

## 試験と保存物

[driver.c](driver.c)はCヘッダからBytes=16／Request=120を独立に検査します。QBE ILにはsizeof相当がないため、LLVM試験のように型からoffsetを計算したとは扱いません。[既存C試験](../c_abi/probe.c)の期待値で全Requestフィールドの転送、要求の所有、出自、NUL・CR/LF・非UTF-8、非0終了、不正版・不正状態、予約失敗、未登録・起動失敗、上限・時間切れを検証します。正常回収のtermination_requestedは実OSで変わり得るため固定せず、時間切れ後の進行不明は接続停止として検査します。

| 保存物 | 内容 |
| --- | --- |
| probe.ssa | 両targetへ渡した同一IL |
| amd64_win.s / amd64_sysv.s | QBEが生成したASM。加工せずassemble |
| reference.s | C frontendのcreate呼出し。試験の実行ファイルへはリンクしない |
| commands.json | ツールの情報、コマンド、終了状態、stdout/stderr |
| inputs.json | 明示target、QBEとarchiveのパス、QBE実行ファイルとILのSHA-256 |
| config-O0.json / config-O2.json | fixture登録。ローカル絶対パスを含むためCI artifactに含めない |

## 本体の実行比較

[生成QBE](../../../docs/design/subprocess-qbe.ja.md)にも接続し、要求変換・両出力予約・Report変換・起動設定・解放を実装しました。[比較試験](../c_abi/generated.cjs)に--qbeを指定すると、既存registered.ceruと異常系を4実行器と比較できます。CERUNE_TEST_QBEでQBEを明示します。ABI試験は言語example一覧へ追加しません。Native実行と明示runtimeによるbuild/releaseも実装済みです。WATとruntime・監督の公式配布は後続です。
