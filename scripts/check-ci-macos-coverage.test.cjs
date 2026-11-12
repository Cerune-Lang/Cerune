'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const {check} = require('./check-ci-macos-coverage.cjs');
const root = path.resolve(__dirname, '..');
const workflow = fs.readFileSync(path.join(root, '.github/workflows/ci.yml'), 'utf8');
const names = fs.readdirSync(path.join(root, 'tests'))
  .filter(name => name.endsWith('.rs')).map(name => name.slice(0, -3));
test('current macOS matrix has a complete explicit inventory', () => {
  const result = check(workflow, names);
  assert.ok(result.covered > 0);
  assert.ok(result.excluded > 0);
});
test('new integration targets require an explicit macOS decision', () => {
  assert.throws(() => check(workflow, [...names, 'new_route_regression']),
    /macOS test inventory changed/);
});
test('removing a macOS integration target fails coverage validation', () => {
  assert.throws(() => check(workflow.replace('--test aggregate_values', ''), names),
    /macOS test inventory changed/);
});
test('stale exclusions cannot silently persist', () => {
  assert.throws(() => check(workflow, names.filter(name => name !== 'qbe_windows')),
    /stale macOS exclusion/);
});
test('one-architecture macOS matrix is rejected', () => {
  assert.throws(() => check(workflow.replace('macos-15-intel, macos-15', 'macos-15'), names),
    /expected both macOS architectures/);
});
