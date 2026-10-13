// テスト専用ホストです。文字列はメモリを受け取らず、出力バイトだけを収集します。
const fs = require('node:fs');
const { parseRuntimeFailure } = require('../../scripts/observe-native.cjs');
const output = [];
const diagnostics = [];
const writeText = value => output.push(Buffer.from(`${value}\n`, 'utf8'));
// fixtureで使う、二進数でも十進数でも正確な値だけを整形します。
const exactFloat = value => {
  if (![0, 1, 1.5, 2, 2.5, 3, 4, 10, 11, 13, 15, 20, 35, 99].includes(value)) throw new Error('unsupported test float');
  return value.toString();
};
const imports = { cerune: {
  // 収集先は同期的に受理済みです。実OSへ接続するhostでは実際にflushします。
  flush_stdout_v1: () => 0,
  write_error_byte(value) {
    if (!Number.isInteger(value) || value < 0 || value > 127) throw new Error('invalid diagnostic byte');
    diagnostics.push(Buffer.from([value]));
  },
  write_byte(value) {
    if (!Number.isInteger(value) || value < 0 || value > 255) throw new Error('invalid output byte');
    output.push(Buffer.from([value]));
  },
  write_i64: value => output.push(Buffer.from(value.toString())),
  write_u64: value => output.push(Buffer.from(BigInt.asUintN(64, value).toString())),
  write_f32: value => output.push(Buffer.from(exactFloat(value))),
  write_f64: value => output.push(Buffer.from(exactFloat(value))),
  print_bool: value => writeText(value ? 'true' : 'false'),
  print_i64: value => writeText(value.toString()),
  print_u64: value => writeText(BigInt.asUintN(64, value).toString()),
  // 汎用の数値整形ホストではありません。
  print_f32: value => writeText(exactFloat(value)),
  print_f64: value => writeText(exactFloat(value)),
} };
WebAssembly.instantiate(fs.readFileSync(process.argv[2]), imports).then(({ instance }) => {
  if (Object.keys(instance.exports).join(',') !== 'main') throw new Error('unexpected public export');
  const repeats = Number(process.argv[3] || 1);
  if (!Number.isInteger(repeats) || repeats < 1 || repeats > 1000) throw new Error('invalid repeat count');
  for (let i = 0; i < repeats; ++i) instance.exports.main();
  if (diagnostics.length) throw new Error('runtime diagnostic without a trap');
  process.stdout.write(Buffer.concat(output));
}).catch(error => {
  // trapより前のprintを保持します。診断だけを出す正常終了は合格にしません。
  process.stdout.write(Buffer.concat(output));
  const record = Buffer.concat(diagnostics);
  process.stderr.write(record);
  const valid = parseRuntimeFailure(record) !== null ||
    /^cerune: io-v1 operation=flush requested=0 accepted=0 node=\d+(?: file=\d+)? bytes=\d+\.\.\d+\n$/.test(record.toString('utf8'));
  if (!(error instanceof WebAssembly.RuntimeError) || error.message !== 'unreachable' || !valid) console.error(error.message);
  process.exitCode = 1;
});
