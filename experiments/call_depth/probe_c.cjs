'use strict';
const fs = require('node:fs');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const {createHash} = require('node:crypto');
const {fixture} = require('./probe.cjs');

function classify(run, expected) {
  if (run.error) return {kind: run.error.code === 'ETIMEDOUT' ? 'timeout' : 'process-error'};
  if (run.signal) return {kind: 'process-terminated', signal: run.signal};
  const output = String(run.stdout || '');
  const error = String(run.stderr || '').replace(/\r\n/g, '\n');
  if (expected.kind === 'completed') {
    return run.status === 0 && output === expected.output && error === ''
      ? {kind: 'completed', matched: true} : {kind: 'mismatch', status: run.status};
  }
  const lines = error.trimEnd().split('\n');
  const fields = ' function=\\d+(?: node=\\d+(?: file=\\d+)? bytes=\\d+\\.\\.\\d+| origin=host)';
  if (run.status !== 1 || output !== expected.output || lines.length !== expected.limit + 1
      || !new RegExp(`^call-v1 reason=depth-limit limit=${expected.limit}${fields}$`).test(lines[0])
      || !lines.slice(1).every((line, i) => new RegExp(`^call-v1 frame=${i}${fields}$`).test(line))) {
    return {kind: 'mismatch', status: run.status};
  }
  return {kind: 'depth-limit', matched: true};
}
function main(args) {
  const [emitter, compiler, target, directory, depthsText = '32,128', stackText = '2097152'] = args;
  if (!directory || args.length > 6) throw Error('usage: node probe_c.cjs <built-emitter> <clang> <windows-msvc|linux-gnu> <new-directory> [depths] [stack-bytes]');
  if (!['windows-msvc', 'linux-gnu'].includes(target)) throw Error('explicit target required');
  const windows = target === 'windows-msvc';
  if (process.platform !== (windows ? 'win32' : 'linux') || process.arch !== 'x64') throw Error('run on the explicitly selected x86_64 target');
  const stack = Number(stackText), depths = depthsText.split(',').map(Number);
  if (!Number.isSafeInteger(stack) || stack < 65536 || stack > 268435456) throw Error('stack request must be 64 KiB..256 MiB');
  for (const depth of depths) fixture(depth, 'scalar');
  const tool = path.resolve(emitter), out = path.resolve(directory);
  fs.mkdirSync(out);
  const hash = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
  const version = spawnSync(compiler, ['--version'], {encoding: 'utf8', timeout: 10000, windowsHide: true});
  if (version.status !== 0) throw Error('compiler unavailable');
  fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify({target, platform: process.platform, arch: process.arch, stack, depths, compiler, compilerVersion: version.stdout.trim(), emitter: tool, emitterSha256: hash(tool)}, null, 2) + '\n');
  let failed = false;
  function invoke(name, command, timeout) {
    const r = spawnSync(command[0], command.slice(1), {encoding: 'utf8', timeout, windowsHide: true, maxBuffer: 4 * 1024 * 1024});
    fs.writeFileSync(path.join(out, name + '.stdout'), r.stdout || '');
    fs.writeFileSync(path.join(out, name + '.stderr'), r.stderr || '');
    return r;
  }
  for (const depth of depths) for (const shape of ['scalar', 'aggregate']) {
    const source = path.join(out, `${shape}-${depth}.ceru`);
    fs.writeFileSync(source, fixture(depth, shape));
    for (const limit of [depth - 1, depth]) for (const opt of ['-O0', '-O2']) {
      const name = `${shape}-${depth}-${limit}-${opt.slice(1)}`;
      const c = path.join(out, name + '.c'), exe = path.join(out, name + (windows ? '.exe' : '.bin'));
      const emitCommand = [tool, source, String(limit), c];
      const emission = invoke(name + '-emit', emitCommand, 30000);
      if (emission.status !== 0 || emission.error || emission.signal) throw Error(`emission failed: ${name}; outputs retained in ${out}`);
      const compileCommand = [compiler, `--target=x86_64-${windows ? 'pc-windows-msvc' : 'unknown-linux-gnu'}`, '-std=c11', opt, c, '-o', exe];
      if (windows) compileCommand.push('-Xlinker', `/STACK:${stack}`);
      const compilation = invoke(name + '-compile', compileCommand, 120000);
      if (compilation.status !== 0 || compilation.error || compilation.signal) throw Error(`compilation failed: ${name}; outputs retained in ${out}`);
      const command = windows ? [exe] : ['prlimit', `--stack=${stack}:${stack}`, '--core=0:0', '--', exe];
      const run = invoke(name, command, 30000);
      const expected = limit < depth ? {kind: 'depth-limit', limit, output: 'before\n'}
        : {kind: 'completed', output: `before\n${depth}\n${shape === 'aggregate' ? 'ab\n' : ''}after\n`};
      const observation = classify(run, expected);
      const record = {name, emitCommand, compileCommand, command, cSha256: hash(c), executableSha256: hash(exe), expected, observation, status: run.status, signal: run.signal, error: run.error ? {code: run.error.code, message: run.error.message} : null};
      fs.appendFileSync(path.join(out, 'results.jsonl'), JSON.stringify(record) + '\n');
      console.log(`${name}: ${observation.kind}`);
      if (!observation.matched) failed = true;
    }
  }
  if (failed) process.exitCode = 1;
}
module.exports = {classify};
if (require.main === module) {
  try {main(process.argv.slice(2));} catch (error) {console.error(error.message); process.exitCode = 1;}
}