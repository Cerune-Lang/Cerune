'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {fixture, classify} = require('./probe.cjs');
const expected = {kind: 'completed', output: 'before\n8\nafter\n'};
const stdout = result => ['preparing','prepared','executing','finished'].map(stage => JSON.stringify({stage, ...(stage === 'finished' ? {result} : {})})).join('\n') + '\n';
const run = (result, extra = {}) => ({status: 0, signal: null, stdout: stdout(result), stderr: '', ...extra});
test('known completion and depth record require matching output and counts', () => {
  assert.equal(classify(run(expected), expected).matched, true);
  const limit = {kind: 'depth-limit', output: 'before\n', limit: 7};
  const result = {...limit, active: 7, record: 'call-v1 reason=depth-limit limit=7 function=1'};
  assert.equal(classify(run(result), limit).matched, true);
  for (const mutation of [{output: ''}, {active: 6}, {limit: 8}, {record: 'allocation-failed'}]) {
    assert.equal(classify(run({...result, ...mutation}), limit).kind, 'mismatch');
  }
});
test('a crash after a success-looking record is never success', () => {
  assert.equal(classify(run(expected, {status: 3}), expected).kind, 'process-terminated');
  assert.equal(classify(run(expected, {status: null, signal: 'SIGSEGV'}), expected).kind, 'process-terminated');
});
test('stack exhaustion needs stderr evidence and a started execution', () => {
  const stderr = "thread 'cerune-depth-probe' has overflowed its stack";
  assert.equal(classify(run(null, {status: 3, stderr, stdout: '{"stage":"executing"}\n'}), expected).kind, 'host-stack-exhausted');
  assert.equal(classify(run(null, {status: 3, stderr, stdout: '{"stage":"preparing"}\n'}), expected).kind, 'process-terminated');
  assert.equal(classify(run(null, {status: 3, stderr: 'aborted'}), expected).kind, 'process-terminated');
  assert.equal(classify(run(expected, {status: 3, stderr}), expected).kind, 'process-terminated');
});
test('timeout and launch failure are not semantic failures', () => {
  assert.equal(classify(run(null, {stdout: 'partial JSON', error: {code: 'ETIMEDOUT'}}), expected).kind, 'timeout');
  assert.equal(classify(run(null, {error: {code: 'ENOENT', message: 'missing'}}), expected).kind, 'process-error');
});
test('missing, reordered, duplicate and malformed events fail closed', () => {
  for (const out of ['', '{}\n', 'not JSON', stdout(expected) + '{"stage":"finished"}', '{"stage":"finished"}']) {
    assert.equal(classify(run(expected, {stdout: out}), expected).kind, 'invalid-protocol');
  }
  assert.equal(classify(run({kind: 'runtime-error', output: 'before\n'}), expected).kind, 'mismatch');
  assert.equal(classify(run(expected), {...expected, sourceFunctions:8}).kind, 'mismatch');
});
test('fixtures use ordinary calls without numeric-type name collisions', () => {
  for (const shape of ['scalar','aggregate']) {
    const source = fixture(64, shape);
    assert.equal((source.match(/fn step_/g) || []).length, 64);
    assert.ok(source.includes('step_32()'));
    assert.ok(!source.includes('f32()'));
    assert.ok(source.includes('print("before")'));
  }
  for (const depth of [0,1,257,NaN,2.5]) assert.throws(() => fixture(depth,'scalar'));
  assert.throws(() => fixture(8,'unknown'));
});