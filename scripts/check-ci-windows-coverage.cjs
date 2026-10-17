// Guard the Windows integration-test inventory from issue #152.
// Update this baseline deliberately when changing Windows CI coverage.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const expected = [
  "aggregate_values",
  "array_iteration",
  "array_length",
  "c_output",
  "c_strings",
  "constant_array_lengths",
  "constants",
  "dynamic_arrays",
  "execution_output",
  "execution_output_cli",
  "explicit_flush",
  "generic_functions",
  "ir_executor",
  "line_input",
  "llvm_output",
  "llvm_strings",
  "mir_executor",
  "modules",
  "native_assembly",
  "native_mir",
  "native_output",
  "observation_bundle",
  "product_updates",
  "rounding_conversions",
  "runtime_routes",
  "source_files",
  "ssa_cli",
  "ssa_executor",
  "ssa_native",
  "standard_input",
  "stdin_routes",
  "string_concat",
  "string_heap_routes",
  "string_routes",
  "sums",
  "truncating_conversions",
  "wat_output",
];
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
assert.ok(!/^  windows-qbe:/m.test(workflow), 'QBE must be part of windows-backends');
assert.match(workflow, /matrix\.suite == 'qbe'/, 'QBE tool setup is missing');
assert.equal(new Set(assigned).size, assigned.length, 'a Windows test is assigned twice');
assert.deepEqual(assigned.sort(), expected.sort(), 'Windows CI test coverage changed');
console.log(`Windows CI coverage: ${assigned.length} original integration targets + --lib and ${qbeTests.length} QBE checks across ${entries.length} suites`);
