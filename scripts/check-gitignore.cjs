'use strict';
// Regression checks for scoped generated output ignores.
const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = path.resolve(__dirname, '..');
function ignored(file) {
  const result = spawnSync('git', ['check-ignore', '--no-index', '-q', '--', file],
    { cwd: root, encoding: 'utf8' });
  assert.ok(result.status === 0 || result.status === 1,
    'git check-ignore failed for ' + file + ': ' + result.stderr);
  return result.status === 0;
}
for (const name of ['c', 'exe', 'out', 'll', 'ceir', 'ssa', 'wat', 's', 'cebc']) {
  assert.equal(ignored('output.' + name), true, 'root generated output should be ignored');
  assert.equal(ignored('tests/fixtures/new-example.' + name), false,
    'handwritten fixture must remain trackable');
}
for (const file of [
  'target/observations/sources.json',
  'observations/private/sources.json',
  'experiments/wat_host/target/generated.wat',
  'experiments/host_calls/vm/target/generated.c',
]) assert.equal(ignored(file), true, file + ' must be ignored');
for (const file of [
  'tests/support/input_memory.c',
  'tests/support/native_runtime_reference/new-runtime.s',
  'experiments/subprocess/c_abi/probe.c',
]) assert.equal(ignored(file), false, file + ' must remain trackable');
console.log('Generated artifact ignore rules: scoped output and hand-written fixtures verified');
