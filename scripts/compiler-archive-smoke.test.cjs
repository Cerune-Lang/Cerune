'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {selectArchive, validatePlan, smoke, hosts} = require('./compiler-archive-smoke.cjs');
function plan() {
    const p = {releases: [{app_name: 'cerune-lang', app_version: '0.1.0', artifacts: []}], artifacts: {},
        ci: {github: {pr_run_mode: 'upload', artifacts_matrix: {include: []}}}};
    for (const [target, [platform, , runner]] of Object.entries(hosts)) {
        const name = `cerune-${target}.${platform === 'win32' ? 'zip' : 'tar.xz'}`;
        p.releases[0].artifacts.push(name);
        p.artifacts[name] = {name, kind: 'executable-zip', target_triples: [target], checksum: name + '.sha256',
            assets: [{name: 'cerune', kind: 'executable', path: platform === 'win32' ? 'cerune.exe' : 'cerune'}]};
        p.artifacts[name + '.sha256'] = {name: name + '.sha256', kind: 'checksum', target_triples: [target]};
        p.ci.github.artifacts_matrix.include.push({host: target, targets: [target], runner});
    }
    return p;
}
test('select the compiler archive separately for all five native hosts', () => {
    const p = plan(); validatePlan(p);
    for (const target of Object.keys(hosts)) assert(selectArchive(p, target).archive.includes(target));
});
for (const kind of ['absent', 'duplicate', 'missing-binary', 'missing-checksum', 'unsafe-name', 'unsafe-binary']) {
    test(`reject ${kind} archive plan`, () => {
        const p = plan(), target = 'x86_64-pc-windows-msvc', name = selectArchive(p, target).archive, a = p.artifacts[name];
        if (kind === 'absent') delete p.artifacts[name];
        if (kind === 'duplicate') { p.artifacts['another.zip'] = {...a, name: 'another.zip'}; p.releases[0].artifacts.push('another.zip'); }
        if (kind === 'missing-binary') a.assets = [];
        if (kind === 'missing-checksum') delete p.artifacts[a.checksum];
        if (kind === 'unsafe-name') a.name = '../outside.zip';
        if (kind === 'unsafe-binary') a.assets[0].path = '../cerune.exe';
        assert.throws(() => selectArchive(p, target));
    });
}
for (const kind of ['no-build', 'empty', 'missing-arm', 'wrong-runner', 'cross', 'container']) {
    test(`reject ${kind} build matrix instead of skipping execution`, () => {
        const p = plan(), gh = p.ci.github, matrix = gh.artifacts_matrix.include;
        if (kind === 'no-build') gh.pr_run_mode = 'plan';
        if (kind === 'empty') gh.artifacts_matrix.include = [];
        if (kind === 'missing-arm') matrix.pop();
        if (kind === 'wrong-runner') matrix[0].runner = 'ubuntu-22.04';
        if (kind === 'cross') matrix[0].targets = ['aarch64-unknown-linux-gnu'];
        if (kind === 'container') matrix[0].container = {image: 'other'};
        assert.throws(() => validatePlan(p));
    });
}
for (const kind of ['missing', 'checksum']) test(`record ${kind} failure without executing a fallback compiler`, t => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cerune-archive-negative-'));
    t.after(() => fs.rmSync(dir, {recursive: true, force: true}));
    const target = Object.keys(hosts).find(key => hosts[key][0] === process.platform && hosts[key][1] === process.arch);
    assert(target, 'test needs a supported native host');
    const p = plan(), selected = selectArchive(p, target), out = path.join(dir, 'result');
    if (kind === 'checksum') {
        fs.writeFileSync(path.join(dir, selected.archive), 'damaged');
        fs.writeFileSync(path.join(dir, selected.checksum), '0'.repeat(64) + ' *' + selected.archive);
    }
    assert.throws(() => smoke(p, target, dir, out));
    const result = JSON.parse(fs.readFileSync(path.join(out, 'observations.json')));
    assert.equal(result.success, false);
    assert.equal(result.commands.length, 0);
    assert(result.error);
});
test('refuse another CPU or OS before creating output', () => {
    const target = Object.keys(hosts).find(key => hosts[key][0] !== process.platform);
    assert.throws(() => smoke(plan(), target, '.', 'must-not-create'), /OS and CPU/);
});
