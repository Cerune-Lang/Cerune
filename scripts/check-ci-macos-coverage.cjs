'use strict';
// Keep the macOS host test inventory explicit; new targets must be classified.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const excluded = [
  'dynamic_arrays', 'execution_output', 'execution_output_cli',
  'explicit_flush', 'io_protocol', 'lean_ssa', 'lean_verification',
  'lir_observation', 'llvm_origins', 'llvm_output', 'llvm_strings',
  'native_assembly', 'native_mir', 'native_output', 'native_program',
  'observation_bundle', 'observation_cli', 'qbe_output', 'qbe_windows',
  'runtime_routes', 'source_files', 'ssa_native', 'stdin_routes',
  'string_heap_routes', 'string_routes',
];
function check(workflow, integrationTargets) {
  const mac = workflow.split(/^  macos-backends:\s*$/m)[1]?.split(/^  host-call-probe:\s*$/m)[0];
  assert.ok(mac, 'macos-backends job is missing');
  assert.match(mac, /runner: \[macos-15-intel, macos-15\]/, 'expected both macOS architectures');
  assert.match(mac, /suite: \[core, c, wat\]/, 'macOS suites changed');
  const branches = [...mac.matchAll(/^ {14}(core|c|wat)\)\r?\n([\s\S]*?)^ {16};;/gm)];
  assert.deepEqual(branches.map(branch => branch[1]), ['core', 'c', 'wat'],
    'expected one core/c/wat shell branch in order');
  const assigned = [];
  for (const [, suite, body] of branches) {
    const joined = body.replace(/\\\r?\n\s*/g, ' ');
    const commands = [...joined.matchAll(/\bcargo test ([^\n]+)/g)];
    assert.ok(commands.length, 'missing cargo test in macOS ' + suite);
    for (const command of commands) {
      const flags = command[1].trim().split(/\s+/);
      const names = [];
      for (let i = 0; i < flags.length; i++) {
        if (flags[i] === '--test') {
          assert.ok(flags[i + 1], 'missing --test argument in ' + suite);
          names.push(flags[++i]);
        }
      }
      assert.ok(names.length, 'no integration target in macOS ' + suite);
      for (const name of names) assigned.push({suite, name});
    }
  }
  const targets = new Set(integrationTargets);
  assert.equal(targets.size, integrationTargets.length, 'duplicate integration target in inventory');
  assert.equal(new Set(excluded).size, excluded.length, 'duplicate macOS exclusions');
  for (const name of excluded) assert.ok(targets.has(name), 'stale macOS exclusion: ' + name);
  for (const {name, suite} of assigned) {
    assert.ok(targets.has(name), 'unknown macOS target in ' + suite + ': ' + name);
    assert.ok(!excluded.includes(name), 'macOS target both assigned and excluded: ' + name);
  }
  const counts = new Map();
  for (const {name} of assigned) counts.set(name, (counts.get(name) || 0) + 1);
  for (const [name, count] of counts) {
    assert.equal(count, name === 'standard_input' ? 3 : 1,
      'unexpected macOS assignment count for ' + name + ': ' + count);
  }
  const expected = integrationTargets.filter(name => !excluded.includes(name)).sort();
  assert.deepEqual([...counts.keys()].sort(), expected,
    'macOS test inventory changed: assign new targets or review exclusions');
  assert.match(mac, /program_arguments::wat/, 'macOS WAT argument check missing');
  assert.match(mac, /file_io::wat/, 'macOS WAT file check missing');
  return {covered: expected.length, excluded: excluded.length};
}
if (require.main === module) {
  const root = path.resolve(__dirname, '..');
  const workflow = fs.readFileSync(path.join(root, '.github/workflows/ci.yml'), 'utf8');
  const names = fs.readdirSync(path.join(root, 'tests'))
    .filter(name => name.endsWith('.rs')).map(name => name.slice(0, -3));
  const result = check(workflow, names);
  console.log('macOS CI coverage: ' + result.covered + ' integration targets, ' +
    result.excluded + ' deliberate exclusions, x64/arm64');
}
module.exports = {check};
