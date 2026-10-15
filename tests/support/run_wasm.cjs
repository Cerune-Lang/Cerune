// テスト専用ホストです。文字列はメモリを受け取らず、出力バイトだけを収集します。
const fs = require('node:fs');
const { parseRuntimeFailure } = require('../../scripts/observe-native.cjs');
const output = [];
const diagnostics = [];
const writeText = value => output.push(Buffer.from(`${value}\n`, 'utf8'));
const floatFormat = require('../../tests/support/wat_float_format.cjs');
const config = process.env.CERUNE_TEST_WAT_OUTPUT ? JSON.parse(process.env.CERUNE_TEST_WAT_OUTPUT) : null;
let accepted = 0, interrupted = false, interruptedAfter = false, failed = false;
function writePacked(bits, count) {
  if (failed) throw new Error('output after write failure');
  if (!Number.isInteger(count) || count < 1 || count > 8) throw new Error('invalid output count');
  if (config) {
    if (!interrupted) { interrupted = true; return -2; }
    if (!interruptedAfter && accepted) { interruptedAfter = true; return -2; }
    if (accepted === config.limit) {
      failed = true;
      return config.mode === 'zero' ? 0 : config.mode === 'invalid' ? count + 1 : config.mode === 'negative' ? -3 : -1;
    }
    count = Math.min(count, config.chunk, config.limit - accepted);
  }
  const bytes = Buffer.alloc(8);
  bytes.writeBigInt64LE(bits);
  output.push(bytes.subarray(0, count));
  accepted += count;
  return count;
}
const imports = { cerune: {
  write_stdout_v1: writePacked,
  format_f32_v1: (value, offset) => floatFormat.chunk(value, 9, offset),
  format_f64_v1: (value, offset) => floatFormat.chunk(value, 17, offset),
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
  write_f32: value => output.push(Buffer.from(floatFormat.format(value, 9))),
  write_f64: value => output.push(Buffer.from(floatFormat.format(value, 17))),
  print_bool: value => writeText(value ? 'true' : 'false'),
  print_i64: value => writeText(value.toString()),
  print_u64: value => writeText(BigInt.asUintN(64, value).toString()),
  // 旧生成物と診断拒否fixture向けの旧importも保持します。
  print_f32: value => writeText(floatFormat.format(value, 9)),
  print_f64: value => writeText(floatFormat.format(value, 17)),
} };
// 失敗後の追加flushを検出します。explicit_flushテストの差替えも保持します。
const flush = imports.cerune.flush_stdout_v1;
imports.cerune.flush_stdout_v1 = () => {
  if (failed) throw new Error('flush after write failure');
  return flush();
};
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
    /^cerune: output formatting failed node=\d+(?: file=\d+)? bytes=\d+\.\.\d+\n$/.test(record.toString('utf8')) ||
    /^cerune: io-v1 operation=(?:write|flush) requested=\d+ accepted=\d+ node=\d+(?: file=\d+)? bytes=\d+\.\.\d+\n$/.test(record.toString('utf8'));
  if (!(error instanceof WebAssembly.RuntimeError) || error.message !== 'unreachable' || !valid) console.error(error.message);
  process.exitCode = 1;
});
