'use strict';
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');

function fixture(depth, shape) {
  if (!Number.isSafeInteger(depth) || depth < 2 || depth > 256) throw Error('depth must be 2..256');
  if (!['scalar', 'aggregate'].includes(shape)) throw Error('unknown shape');
  let text = shape === 'aggregate' ? 'type Packet { text: string, numbers: [i64; 4], }\n' : '';
  for (let i = depth - 1; i >= 0; --i) {
    const next = i + 1 === depth ? '1' : `step_${i + 1}() + 1`;
    text += shape === 'scalar' ? `fn step_${i}() -> i64 { return ${next}; }\n`
      : `fn step_${i}() -> Packet { ${i + 1 === depth
        ? 'return Packet { text: concat("a", "b"), numbers: [1, 2, 3, 4] };'
        : `value: Packet = step_${i + 1}(); return Packet { ..value, numbers: [value.numbers[0] + 1, 2, 3, 4] };`} }\n`;
  }
  return text + (shape === 'scalar'
    ? 'print("before"); print(step_0()); print("after");\n'
    : 'print("before"); value: Packet = step_0(); print(value.numbers[0]); print(value.text); print("after");\n');
}
function classify(run, expected) {
  const stdout = String(run.stdout || '');
  const stderr = String(run.stderr || '');
  if (run.error?.code === 'ETIMEDOUT') return {kind: 'timeout'};
  if (run.error) return {kind: 'process-error', message: run.error.message};
  let events;
  try { events = stdout.trim().split(/\r?\n/).filter(Boolean).map(JSON.parse); }
  catch { return { kind: 'invalid-protocol', evidence: 'non-JSON stdout' }; }
  const stage = events.at(-1)?.stage ?? 'not-started';
  if (run.status !== 0 || run.signal) {
    const overflow = /(?:has overflowed its stack|stack overflow)/i.test(stderr);
    return {kind: overflow && stage === 'executing' ? 'host-stack-exhausted' : 'process-terminated', stage};
  }
  if (events.map(e => e.stage).join(',') !== 'preparing,prepared,executing,finished') return {kind: 'invalid-protocol', stage};
  if (expected.sourceFunctions !== undefined && events[1].source_functions !== expected.sourceFunctions) return {kind:"mismatch", stage, evidence:"source function count"};
  const result = events.at(-1).result;
  if (!result || result.kind !== expected.kind || result.output !== expected.output) return {kind: 'mismatch', stage, actual: result};
  if (result.kind === 'depth-limit' && (result.limit !== expected.limit || result.active !== expected.limit || !result.record?.startsWith(`call-v1 reason=depth-limit limit=${expected.limit} `))) return {kind: 'mismatch', stage, actual: result};
  return {kind: result.kind, stage, matched: true};
}
function main(args) {
  const [binary, directory, depthsText = '8,32,64', stackText = '2097152'] = args;
  if (!binary || !directory || args.length > 4) throw Error('usage: node probe.cjs <built-probe> <new-output-directory> [depths] [stack-bytes]');
  const executable = path.resolve(binary);
  const depths = depthsText.split(',').map(Number);
  const stack = Number(stackText);
  if (!Number.isSafeInteger(stack) || stack < 65536 || stack > 268435456) throw Error('invalid stack request');
  for (const depth of depths) fixture(depth, 'scalar');
  const binarySha256 = createHash('sha256').update(fs.readFileSync(executable)).digest('hex');
  // 新規ディレクトリだけを使い、以前の測定結果を上書きしません。
  const out = path.resolve(directory);
  fs.mkdirSync(out);
  const rustc = spawnSync("rustc", ["-Vv"], {encoding:"utf8", windowsHide:true});
  const metadata = { rustc: rustc.status === 0 ? rustc.stdout.trim() : null, platform: process.platform, arch: process.arch, release: os.release(), node: process.version,
    binary: executable, binarySha256, requestedStackBytes: stack, depths, timestamp: new Date().toISOString() };
  fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(metadata, null, 2) + '\n');
  let failed = false;
  for (const depth of depths) for (const shape of ['scalar', 'aggregate']) {
    const file = path.join(out, `${shape}-${depth}.ceru`);
    fs.writeFileSync(file, fixture(depth, shape));
    for (const route of ['hir', 'mir', 'ssa', 'vm']) for (const limit of [depth - 1, depth]) {
      const command = [route, file, String(limit), String(stack)];
      const run = spawnSync(executable, command, {encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024, windowsHide: true});
      const expected = limit < depth ? {kind: 'depth-limit', limit, output: 'before\n'}
        : {kind: 'completed', output: `before\n${depth}\n${shape === 'aggregate' ? 'ab\n' : ''}after\n`};
      expected.sourceFunctions = depth;
      const observation = classify(run, expected);
      const name = `${shape}-${depth}-${route}-${limit}`;
      const record = {name, command: [executable, ...command], expected, observation, status: run.status,
        signal: run.signal, error: run.error ? {code: run.error.code, message: run.error.message} : null};
      fs.writeFileSync(path.join(out, name + '.stdout'), run.stdout || '');
      fs.writeFileSync(path.join(out, name + '.stderr'), run.stderr || '');
      fs.appendFileSync(path.join(out, 'results.jsonl'), JSON.stringify(record) + '\n');
      console.log(`${name}: ${observation.kind}`);
      if (!observation.matched) failed = true;
    }
  }
  // 資源不足を「テスト成功」に数えません。観測ファイルは失敗時にも残ります。
  if (failed) process.exitCode = 1;
}
module.exports = {fixture, classify};
if (require.main === module) {
  try {main(process.argv.slice(2));} catch (e) {console.error(e.message);process.exitCode = 1;}
}