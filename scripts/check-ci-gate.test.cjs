'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const {checkResults} = require('./check-ci-gate.cjs');

test('only completed successful jobs pass the gate', () => {
    const passed = {result: 'success'};
    checkResults({rust: passed, linux: passed});
    for (const result of ['failure', 'cancelled', 'skipped', 'unknown', undefined]) {
        assert.throws(() => checkResults({rust: passed, linux: {result}}), /linux:/);
    }
    assert.throws(() => checkResults({}), /no prerequisite jobs/);
});

test('ci-gate depends directly on every other CI job', () => {
    const workflow = fs.readFileSync(path.join(__dirname, '../.github/workflows/ci.yml'), 'utf8');
    const jobs = workflow.split(/^jobs:\s*$/m)[1];
    const names = [...jobs.matchAll(/^  ([\w-]+):\s*$/gm)].map(match => match[1]);
    const gate = jobs.split(/^  ci-gate:\s*$/m)[1].split(/^  [\w-]+:\s*$/m)[0];
    const list = gate.match(/^    needs:\r?\n((?:      - [\w-]+\r?\n)+)/m)[1];
    const needs = [...list.matchAll(/- ([\w-]+)/g)].map(match => match[1]);
    assert.deepEqual(needs.sort(), names.filter(name => name !== 'ci-gate').sort());
    assert.match(gate, /^    if: always\(\)\s*$/m);
    assert.match(gate, /CI_JOB_RESULTS: \$\{\{ toJSON\(needs\) \}\}/);
    assert.match(gate, /run: node scripts\/check-ci-gate\.cjs/);
});
