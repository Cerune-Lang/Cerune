// Guard the Windows integration-test inventory from issue #152.
// Update this baseline deliberately when changing Windows CI coverage.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

// Targets deliberately not selected by Windows jobs. New integration targets must
// be assigned to a suite or added here with an explicit coverage decision.
const excluded = [
  'diagnostics_cli',
  'examples',
  'fixed_width_integers',
  'float_output',
  'integer_bit_operations',
  'integer_conversions',
  'integer_types',
  'io_protocol',
  'lean_ssa',
  'lean_verification',
  'lir_observation',
  'llvm_origins',
  'main_entry',
  'mir',
  'mir_ssa',
  'numeric_conversions',
  'observation_cli',
  'short_circuit',
  'small_integers',
  'ssa_construction',
  'string_lexing',
  'string_values',
  'u64_values',
];
const qbeOnly = ['qbe_output', 'qbe_windows'];
const all = fs.readdirSync(path.join(__dirname, '..', 'tests'))
  .filter(name => name.endsWith('.rs')).map(name => name.slice(0, -3));
assert.equal(new Set(excluded).size, excluded.length, 'duplicate Windows exclusions');
assert.equal(new Set(qbeOnly).size, qbeOnly.length, 'duplicate QBE-only targets');
for (const name of [...excluded, ...qbeOnly]) {
  assert.ok(all.includes(name), 'stale Windows test classification: ' + name);
}
const expected = all.filter(name => !excluded.includes(name) && !qbeOnly.includes(name)).sort();
const repository = path.resolve(__dirname, '..');
const workflow = fs.readFileSync(path.join(repository, '.github/workflows/ci.yml'), 'utf8');
const windows = workflow.split(/^  windows-backends:\s*$/m)[1];
assert.ok(windows, 'windows-backends job is missing');
const entries = [...windows.matchAll(
  /^          - suite: ([\w-]+)\r?\n            timeout: (\d+)\r?\n            tests: >-\r?\n((?:              [^\r\n]+\r?\n)+)/gm
)];
const suites = ['core', 'c-llvm', 'native', 'wat', 'cross-io', 'cross-runtime', 'qbe'];
assert.deepEqual(entries.map((entry) => entry[1]).sort(), suites.sort(), 'unexpected Windows suites');

const assigned = [];
let libCount = 0;
const qbeExpected = ['qbe_windows', 'qbe_output', 'dynamic_arrays', 'explicit_flush', 'stdin_routes'];
let qbeTests = [];
for (const entry of entries) {
  const [_, suite, timeout, body] = entry;
  assert.ok(Number(timeout) > 0, `missing timeout: ${suite}`);
  const tokens = body.trim().split(/\s+/);
  for (let i = 0; i < tokens.length; i++) {
    if (tokens[i] === '--lib') {
      libCount++;
      continue;
    }
    assert.equal(tokens[i], '--test', `invalid test flag in ${suite}: ${tokens[i]}`);
    assert.ok(tokens[i + 1], `missing test name in ${suite}`);
    const target = tokens[++i];
    assert.ok(fs.existsSync(path.join(repository, 'tests', `${target}.rs`)),
      `test target ${target} no longer exists`);
    if (suite === 'qbe') qbeTests.push(target);
    else assigned.push(target);
  }
}
assert.equal(libCount, 1, '--lib must be assigned exactly once');
assert.deepEqual(qbeTests.sort(), qbeExpected.sort(), 'QBE suite coverage changed');
assert.ok(qbeOnly.every(name => qbeTests.includes(name)), 'QBE-only targets lost coverage');
assert.ok(!/^  windows-qbe:/m.test(workflow), 'QBE must be part of windows-backends');
assert.match(workflow, /matrix\.suite == 'qbe'/, 'QBE tool setup is missing');
assert.equal(new Set(assigned).size, assigned.length, 'a Windows test is assigned twice');
assert.deepEqual(assigned.sort(), expected.sort(), 'Windows CI test coverage changed');
console.log(`Windows CI coverage: ${assigned.length} general integration targets, ${qbeOnly.length} QBE-only targets, ${excluded.length} deliberate exclusions and ${qbeTests.length} QBE checks across ${entries.length} suites`);

assert.match(windows, /cargo test --test standard_input program_arguments::qbe/, 'QBE arguments must run with the required QBE toolchain');

assert.match(windows, /cargo test --test standard_input program_arguments::wat/, 'WAT arguments must run with the required host tools');

assert.match(windows, /cargo test --test standard_input program_arguments::native/, 'Native arguments must run with the required linker');
