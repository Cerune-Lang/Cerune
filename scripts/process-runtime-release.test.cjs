'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const assert = require('node:assert/strict');
const {test} = require('node:test');
const {names, identity, checksum, catalog, stage, baseline, validateBaseline} = require('./process-runtime-release.cjs');
const {hash, recordFiles, targets} = require('./process-runtime-bundle.cjs');
const version = '0.1.0', commit = 'a'.repeat(40), target = 'x86_64-unknown-linux-gnu';
function temp(t) {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cerune-runtime-release-'));
    t.after(() => fs.rmSync(dir, {recursive: true, force: true}));
    return dir;
}
function manifest() { return {version, target, source: {commit, dirty: false}, build: {systemRuntime: 'glibc 2.35'}}; }
test('record matching clean source and baseline', () => {
    identity(manifest(), version, commit, target);
    validateBaseline(manifest());
    validateBaseline({...manifest(), target: 'x86_64-pc-windows-msvc', build: {systemRuntime: 'Windows 10.0.20348'}});
});
for (const change of ['version', 'commit', 'dirty', 'target']) test(`reject mismatched ${change}`, () => {
    const m = manifest();
    if (change === 'version') m.version = '0.2.0';
    if (change === 'commit') m.source.commit = 'b'.repeat(40);
    if (change === 'dirty') m.source.dirty = true;
    if (change === 'target') m.target = 'x86_64-pc-windows-msvc';
    assert.throws(() => identity(m, version, commit, target));
});
test('reject builds from another OS version instead of recording the declared baseline', () => {
    assert.throws(() => validateBaseline({...manifest(), build: {systemRuntime: 'glibc 2.39'}}));
    assert.throws(() => validateBaseline({...manifest(), target: 'x86_64-pc-windows-msvc', build: {systemRuntime: 'Windows 10.0.26100'}}));
});
test('asset names reject unknown targets and path-like versions', () => {
    assert.throws(() => names('../0.1.0', target));
    assert.throws(() => names(version, 'other-target'));
});
test('missing and changed archives fail checksum validation', t => {
    const dir = temp(t), n = names(version, target), bytes = Buffer.from('fixture archive');
    assert.throws(() => checksum(dir, n));
    fs.writeFileSync(path.join(dir, n.archive), bytes);
    fs.writeFileSync(path.join(dir, n.checksum), hash(bytes) + '  ' + n.archive + '\n');
    assert.equal(checksum(dir, n), hash(bytes));
    fs.appendFileSync(path.join(dir, n.archive), 'changed');
    assert.throws(() => checksum(dir, n), /hash mismatch/);
});
test('an incomplete target set cannot produce a publication catalog', t => {
    const dir = temp(t), input = path.join(dir, 'input'), out = path.join(dir, 'output');
    fs.mkdirSync(input);
    assert.throws(() => catalog({releases: [{app_name: 'cerune-lang', app_version: version}]}, commit, input, out));
    assert(!fs.existsSync(path.join(out, 'cerune-process-runtime.json')));
});

// Payloads below are inert fixtures; actual ABI execution belongs to the CI smoke test.
function releaseFixtures(t) {
    const root = temp(t), input = path.join(root, 'assets');
    fs.mkdirSync(input);
    for (const target of Object.keys(baseline)) {
        const info = targets[target], build = path.join(root, target), bundle = path.join(build, 'bundle');
        fs.mkdirSync(bundle, {recursive: true});
        for (const file of ['lib/' + info.archive, 'include/cerune_process_v1.h', 'Cargo.lock', 'README.md', 'README.en.md',
            'licenses/index.json', 'licenses/cerune/LICENSE-MIT', 'licenses/cerune/LICENSE-APACHE',
            'licenses/rust/COPYRIGHT-library.html', ...(info.platform === 'linux' ? ['bin/cerune-process-supervisor'] : [])]) {
            fs.mkdirSync(path.dirname(path.join(bundle, file)), {recursive: true});
            fs.writeFileSync(path.join(bundle, file), 'inert fixture bytes\n');
        }
        if (info.platform === 'linux') fs.chmodSync(path.join(bundle, 'bin/cerune-process-supervisor'), 0o755);
        const m = {schema: 'cerune-process-runtime-bundle-v1', version, target,
            source: {commit, dirty: false}, abi: {name: 'cerune-process-v1', version: 1, reportWords: 32},
            archive: 'lib/' + info.archive, header: 'include/cerune_process_v1.h', systemLinkArgs: info.systemLinkArgs,
            supervisor: info.platform === 'linux' ? {path: 'bin/cerune-process-supervisor', protocol: 'cerune-process-supervisor-v1', binding: 'execution-time-json'} : null,
            configuration: {schema: 'cerune-process-v1', embedded: false},
            build: {profile: 'release', host: info.platform, architecture: 'x64', rustc: 'rustc fixture', cargo: 'cargo fixture',
                systemRuntime: info.platform === 'linux' ? 'glibc 2.35' : 'Windows 10.0.20348'}, files: recordFiles(bundle)};
        fs.writeFileSync(path.join(bundle, 'manifest.json'), JSON.stringify(m));
        const n = names(version, target), archive = path.join(build, n.archive);
        require('child_process').execFileSync(require('./archive-tool.cjs').archiveTool(), ['-czf', archive, '-C', bundle, '.']);
        fs.writeFileSync(path.join(build, n.checksum), hash(fs.readFileSync(archive)) + '  ' + n.archive + '\n');
        const staged = path.join(build, 'staged');
        stage(target, build, version, commit, staged);
        for (const file of fs.readdirSync(staged)) fs.copyFileSync(path.join(staged, file), path.join(input, file));
    }
    return {root, input, out: path.join(root, 'catalog'), plan: {releases: [{app_name: 'cerune-lang', app_version: version}]}};
}
test('stage and catalog both target archives with exact version, commit and hashes', t => {
    const f = releaseFixtures(t), result = catalog(f.plan, commit, f.input, f.out);
    assert.equal(result.version, version);
    assert.equal(result.commit, commit);
    assert.deepEqual(result.assets.map(a => a.target), Object.keys(baseline));
    for (const a of result.assets) assert.equal(a.sha256, hash(fs.readFileSync(path.join(f.input, a.archive))));
    assert.deepEqual(JSON.parse(fs.readFileSync(path.join(f.out, 'cerune-process-runtime.json'))), result);
});
for (const change of ['metadata', 'archive', 'extra-log']) test('catalog rejects ' + change + ' after staging', t => {
    const f = releaseFixtures(t), n = names(version, target);
    if (change === 'metadata') {
        const file = path.join(f.input, n.metadata), m = JSON.parse(fs.readFileSync(file));
        m.manifest.build.rustc = 'different compiler';
        fs.writeFileSync(file, JSON.stringify(m));
    } else if (change === 'archive') fs.appendFileSync(path.join(f.input, n.archive), 'changed');
    else fs.writeFileSync(path.join(f.input, 'commands.log'), 'build log');
    assert.throws(() => catalog(f.plan, commit, f.input, f.out), change === 'metadata' ? /archive and release metadata differ/ : change === 'archive' ? /hash mismatch/ : /exactly both target/);
    assert(!fs.existsSync(path.join(f.out, 'cerune-process-runtime.json')));
});
