#!/usr/bin/env node
'use strict';

// 明示した外部ツールでASMから機械語までを観測します。Ceruneのemit APIは起動処理を持ちません。
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');

function parse(args) {
  const options = {};
  for (let index = 0; index < args.length; index++) {
    const key = args[index];
    if (key === '--run' || key === '--expect-trap') {
      if (options[key]) throw new Error('duplicate option: ' + key);
      options[key] = true;
    } else if (['--source', '--target', '--cerune', '--cc', '--objdump', '--output-dir', '--encoder'].includes(key)) {
      if (options[key] || !args[index + 1] || args[index + 1].startsWith('--')) throw new Error('missing or duplicate value: ' + key);
      options[key] = args[++index];
    } else {
      throw new Error('unknown option: ' + key);
    }
  }
  for (const key of ['--source', '--target', '--cerune', '--cc', '--objdump', '--output-dir']) {
    if (!options[key]) throw new Error('required option: ' + key);
  }
  if (!['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc'].includes(options['--target'])) throw new Error('unsupported target');
  if (options['--expect-trap'] && !options['--run']) throw new Error('--expect-trap requires --run');
  options['--encoder'] ||= 'external';
  if (!['external', 'cerune'].includes(options['--encoder'])) throw new Error('unsupported encoder');
  return options;
}

function observe(options) {
  const target = options['--target'];
  const windows = target === 'x86_64-pc-windows-msvc';
  if (options['--run'] && (process.arch !== 'x64' || process.platform !== (windows ? 'win32' : 'linux'))) {
    throw new Error('requested execution target does not match the host; omit --run to generate only');
  }
  const source = path.resolve(options['--source']);
  const directory = path.resolve(options['--output-dir']);
  if (fs.existsSync(directory)) throw new Error('output directory already exists: ' + directory);
  const sourceBytes = fs.readFileSync(source);
  const resolveTool = value => /[\\/]/.test(value) ? path.resolve(value) : value;
  const cerune = resolveTool(options['--cerune']);
  const cc = resolveTool(options['--cc']);
  const objdump = resolveTool(options['--objdump']);
  const report = { schema: 'cerune-native-observation-v1', target, encoder: options['--encoder'] || 'external', sourceSha256: hash(sourceBytes), tools: {}, steps: [], artifacts: {}, status: 'building' };
  let created = false;
  const save = () => { if (created) fs.writeFileSync(path.join(directory, 'manifest.json'), JSON.stringify(report, null, 2) + '\n'); };
  const run = (stage, tool, args, allowFailure = false) => {
    const result = spawnSync(tool, args, { cwd: created ? directory : undefined, timeout: 30000, maxBuffer: 32 * 1024 * 1024, windowsHide: true });
    report.steps.push({ stage, tool, args, exitCode: result.status, signal: result.signal, status: result.error ? 'tool-error' : result.status === 0 ? 'passed' : 'failed' });
    if (created && (result.error || result.status !== 0)) {
      write(stage + '.stdout', result.stdout || Buffer.alloc(0));
      write(stage + '.stderr', result.stderr || Buffer.alloc(0));
    }
    if (result.error || (!allowFailure && result.status !== 0)) {
      throw new Error(stage + ': ' + (result.error ? result.error.message : result.stderr.toString('utf8')));
    }
    return result;
  };
  const write = (name, bytes) => {
    fs.writeFileSync(path.join(directory, name), bytes);
    report.artifacts[name] = { sha256: hash(bytes), bytes: bytes.length };
  };
  try {
    for (const [name, tool] of [['cerune', cerune], ['cc', cc], ['objdump', objdump]]) {
      const version = run('version:' + name, tool, ['--version']);
      report.tools[name] = { command: tool, version: version.stdout.toString('utf8').trim() };
    }
    // 新規ディレクトリのみを使い、既存の成果物や失敗記録を上書きしません。
    fs.mkdirSync(directory);
    created = true;
    write('source.ceru', sourceBytes);
    // 元の入口から依存を読み、別ディレクトリへのコピーでimportの意味を変えません。
    const sourceManifest = run('sources', cerune, ['emit-sources', source]).stdout;
    if (!Buffer.from(JSON.parse(sourceManifest).files[0].text, 'utf8').equals(sourceBytes)) {
      throw new Error('entry source changed before observation');
    }
    write('sources.json', sourceManifest);
    write('program.ceir', run('cerune-ir', cerune, ['emit-ir', source]).stdout);
    write('program.s', run('assembly', cerune, ['emit-asm', source, '--target', target, '--annotate-origins']).stdout);
    const object = windows ? 'program.obj' : 'program.o';
    const executable = windows ? 'program.exe' : 'program';
    const flags = windows ? ['--target=' + target] : ['-m64'];
    if (report.encoder === 'cerune') {
      run('encode-object', cerune, ['emit-obj', source, '--target', target, '--annotate-origins', '-o', object]);
    } else {
      run('assemble', cc, [...flags, '-c', 'program.s', '-o', object]);
    }
    const header = run('object-format', objdump, ['-f', object]).stdout;
    const expectedFormat = windows ? /file format (coff-x86-64|pe-x86-64)/ : /file format elf64-x86-64/;
    if (!expectedFormat.test(header.toString('utf8'))) throw new Error('assembler produced an object for a different target');
    write('object.txt', Buffer.concat([header, run('object-observation', objdump, ['-h', '-t', '-r', '-d', object]).stdout]));
    write('text.txt', run('instruction-bytes', objdump, ['-s', '-j', '.text', object]).stdout);
    run('link', cc, [...flags, object, ...(windows ? [] : ['-Wl,--build-id=none']), '-o', executable]);
    write('executable.txt', run('executable-observation', objdump, ['-h', '-t', '-d', executable]).stdout);
    for (const name of [object, executable]) {
      const bytes = fs.readFileSync(path.join(directory, name));
      report.artifacts[name] = { sha256: hash(bytes), bytes: bytes.length };
    }
    if (options['--run']) {
      const vm = run('vm', cerune, ['run-vm', source, '--diagnostic-format', 'runtime-v1'], true);
      const native = run('native', path.join(directory, executable), [], true);
      write('vm.stdout', vm.stdout);
      write('vm.stderr', vm.stderr);
      write('native.stdout', native.stdout);
      write('native.stderr', native.stderr);
      const strings = fs.readFileSync(path.join(directory, 'program.s'), 'utf8').includes('callq _setmode');
      report.outputComparison = windows && !strings ? 'numeric-windows-crlf-to-lf' : 'exact-bytes';
      const actual = windows && !strings ? Buffer.from(native.stdout.toString('utf8').replace(/\r\n/g, '\n')) : native.stdout;
      if (options['--expect-trap']) {
        const trapped = windows ? native.status !== null && (native.status >>> 0) === 0xc0000409 : native.signal === 'SIGILL';
        const expectedFailure = parseRuntimeFailure(vm.stderr);
        const nativeFailure = parseRuntimeFailure(native.stderr);
        if (vm.status !== 1 || !trapped || !expectedFailure || !nativeFailure
            || JSON.stringify(expectedFailure) !== JSON.stringify(nativeFailure) || !actual.equals(vm.stdout)) {
          throw new Error('expected matching VM/native runtime-v1 failures, prior output, and a native illegal-instruction trap');
        }
        report.runtimeFailure = nativeFailure;
        report.steps.at(-1).status = 'expected-trap';
        report.steps.at(-2).status = 'expected-diagnostic';
        report.status = 'expected-failure-confirmed';
      } else {
        if (vm.status !== 0 || native.status !== 0 || vm.stderr.length || native.stderr.length) throw new Error('unexpected execution failure');
        // 既存Windows数値出力のCRT改行規則を比較条件として記録します。
        if (!actual.equals(vm.stdout)) throw new Error('native output differs from VM output');
        report.status = 'output-matched';
      }
    } else {
      report.status = 'generated-not-executed';
    }
    // 生成と実行の間に依存本文が変わった観測は、同じコンパイルとして合格にしません。
    if (!sourceManifest.equals(run('verify-sources', cerune, ['emit-sources', source]).stdout)) {
      throw new Error('source files changed during observation');
    }
    save();
    return report;
  } catch (error) {
    report.status = 'failed';
    report.error = error.message;
    save();
    throw error;
  }
}

function hash(bytes) { return crypto.createHash('sha256').update(bytes).digest('hex'); }

// 完全な1レコードだけを受け付け、別のクラッシュや追加のエラーを合格にしません。
function parseRuntimeFailure(bytes) {
  const text = bytes.toString('utf8').replace(/\r\n/g, '\n');
  const match = /^cerune: runtime-v1 code=([a-z-]+) node=(0|[1-9][0-9]*)(?: file=([1-9][0-9]*))? bytes=(0|[1-9][0-9]*)\.\.(0|[1-9][0-9]*)\n$/.exec(text);
  if (!match || match[0].length !== text.length) return null;
  const codes = ['integer-overflow', 'division-by-zero', 'division-overflow', 'remainder-by-zero',
    'invalid-shift-count', 'integer-conversion-out-of-range', 'conversion-out-of-range',
    'conversion-inexact', 'conversion-not-finite', 'conversion-nan', 'conversion-negative-zero', 'array-index-out-of-bounds', 'array-range-out-of-bounds', 'array-length-out-of-range', 'allocation-size-overflow', 'allocation-limit-exceeded', 'allocation-failed'];
  const node = Number(match[2]), file = Number(match[3] || 0), start = Number(match[4]), end = Number(match[5]);
  if (!codes.includes(match[1]) || ![node, file, start, end].every(Number.isSafeInteger) || start >= end) return null;
  return { schema: 'runtime-v1', code: match[1], node, ...(file ? { file } : {}), start, end };
}

if (require.main === module) {
  if (process.argv.length === 3 && process.argv[2] === '--help') {
    console.log('Usage: node scripts/observe-native.cjs --source <file> --target <triple> --cerune <tool> --cc <tool> --objdump <tool> --output-dir <new-directory> [--encoder external|cerune] [--run [--expect-trap]]');
    process.exit(0);
  }
  try {
    const options = parse(process.argv.slice(2));
    const report = observe(options);
    console.log(JSON.stringify({ status: report.status, target: report.target, manifest: path.resolve(options['--output-dir'], 'manifest.json') }));
  } catch (error) {
    console.error(JSON.stringify({ status: 'failed', error: error.message }));
    process.exitCode = 1;
  }
}

module.exports = { parse, observe, parseRuntimeFailure };
