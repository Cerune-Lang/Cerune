// Check that Linux CI partitions all Cargo integration targets, not just
// the Windows regression subset. Lean suites have dedicated proof jobs.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const workflow = fs.readFileSync(path.join(root, '.github/workflows/ci.yml'), 'utf8');
const body = workflow.split(/^  linux-backends:\s*$/m)[1]?.split(/^  lean-verification:\s*$/m)[0];
assert.ok(body, 'linux-backends job is missing');
const entries = [...body.matchAll(
  /^          - suite: ([\w-]+)\r?\n            timeout: (\d+)\r?\n            tests: >-\r?\n((?:              [^\r\n]+\r?\n)+)/gm
)];
const suites = ['core','c-llvm','native','wat','qbe','cross-io','cross-runtime','sanitizer'];
assert.deepEqual(entries.map(e => e[1]).sort(), suites.sort(), 'Linux suites changed');
const all = fs.readdirSync(path.join(root, 'tests'))
  .filter(name => name.endsWith('.rs')).map(name => name.slice(0,-3));
const lean = ['lean_ssa','lean_verification'];
for (const name of lean) {
  assert.ok(all.includes(name), `missing Lean test target: ${name}`);
  assert.match(workflow, /suite: \[lean_verification, lean_ssa\]/,
    'Lean must keep separate verification jobs');
}
const expected = all.filter(name => !lean.includes(name)).sort();
const assigned = [];
let lib = 0, bins = 0, examples = 0, sanitizer = [];
for (const [,suite,timeout,body] of entries) {
  assert.ok(Number(timeout) > 0, `invalid timeout in ${suite}`);
  const args = body.trim().split(/\s+/);
  for (let i=0; i<args.length; i++) {
    if (args[i] === '--lib' || args[i] === '--bins' || args[i] === '--examples') {
      if (args[i] === '--lib') lib++;
      if (args[i] === '--bins') bins++;
      if (args[i] === '--examples') examples++;
      assert.equal(suite, 'core', `Rust package targets must be tested by core: ${suite}`);
      continue;
    }
    assert.equal(args[i], '--test', `unrecognized Cargo flag in ${suite}: ${args[i]}`);
    const name = args[++i];
    assert.ok(name, `missing target in ${suite}`);
    assert.ok(all.includes(name), `unknown target in ${suite}: ${name}`);
    if (suite === 'sanitizer') sanitizer.push(name);
    else assigned.push(name);
  }
}
assert.deepEqual([lib,bins,examples], [1,1,1], 'lib/bins/examples must be verified once');
assert.deepEqual(sanitizer.sort(), ['c_strings','dynamic_arrays','standard_input'], 'sanitizer coverage changed');
assert.equal(assigned.length, new Set(assigned).size, 'Linux route target assigned more than once');
assert.deepEqual(assigned.sort(), expected, 'Linux integration targets are missing or duplicated');
assert.match(body, /matrix\.suite == 'sanitizer'/, 'sanitizer setup missing');
assert.match(body, /CERUNE_TEST_QBE=\$GITHUB_WORKSPACE/, 'QBE must be explicitly configured');
assert.match(body, /CERUNE_TEST_WAT2WASM_JS=\$wabt/, 'WAT must be explicitly configured');
console.log(`Linux CI coverage: ${assigned.length} integration targets, package lib/bins/examples, two Lean suites and sanitizer`);
