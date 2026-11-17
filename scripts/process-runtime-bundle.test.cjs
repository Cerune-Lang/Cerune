'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {test} = require('node:test');
const {verify, recordFiles, targets} = require('./process-runtime-bundle.cjs');
function fixture(t, target = 'x86_64-pc-windows-msvc') {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cerune-runtime-manifest-'));
    t.after(() => fs.rmSync(dir, {recursive: true, force: true}));
    const info = targets[target];
    for (const file of ['lib/' + info.archive, 'include/cerune_process_v1.h', 'Cargo.lock', 'README.md', 'README.en.md',
        'licenses/index.json', 'licenses/cerune/LICENSE-MIT', 'licenses/cerune/LICENSE-APACHE',
        'licenses/rust/COPYRIGHT-library.html', ...(info.platform === 'linux' ? ['bin/cerune-process-supervisor'] : [])]) {
        fs.mkdirSync(path.dirname(path.join(dir, file)), {recursive: true});
        fs.writeFileSync(path.join(dir, file), 'fixture bytes\n');
    }
    if (info.platform === 'linux') fs.chmodSync(path.join(dir, 'bin/cerune-process-supervisor'), 0o755);
    const m = {schema: 'cerune-process-runtime-bundle-v1', version: '0.1.0', target,
        source: {commit: 'a'.repeat(40), dirty: false}, abi: {name: 'cerune-process-v1', version: 1, reportWords: 32},
        archive: 'lib/' + info.archive, header: 'include/cerune_process_v1.h', systemLinkArgs: info.systemLinkArgs,
        supervisor: info.platform === 'linux' ? {path: 'bin/cerune-process-supervisor', protocol: 'cerune-process-supervisor-v1', binding: 'execution-time-json'} : null,
        configuration: {schema: 'cerune-process-v1', embedded: false},
        build: {profile: 'release', host: info.platform, architecture: 'x64', rustc: 'rustc fixture', cargo: 'cargo fixture', systemRuntime: 'fixture'},
        files: recordFiles(dir)};
    const save = () => fs.writeFileSync(path.join(dir, 'manifest.json'), JSON.stringify(m));
    save();
    return {dir, m, save, target};
}
for (const target of Object.keys(targets)) test(`verify ${target} without executing files`, t => {
    const f = fixture(t, target);
    assert.equal(verify(target, f.dir).target, target);
    assert.throws(() => verify('unsupported', f.dir), /unsupported/);
    assert.throws(() => verify(Object.keys(targets).find(t => t !== target), f.dir));
});
for (const change of ['schema', 'target', 'version', 'abi', 'commit', 'config', 'supervisor', 'libraries', 'duplicate', 'traversal']) {
    test(`reject inconsistent ${change}`, t => {
        const f = fixture(t);
        switch (change) {
        case 'schema': f.m.schema += '-future'; break;
        case 'target': f.m.target = 'x86_64-unknown-linux-gnu'; break;
        case 'version': f.m.version = ''; break;
        case 'abi': f.m.abi.reportWords = 31; break;
        case 'commit': f.m.source.commit = 'unknown'; break;
        case 'config': f.m.configuration.embedded = true; break;
        case 'supervisor': f.m.supervisor = {path: '../host'}; break;
        case 'libraries': f.m.systemLinkArgs = []; break;
        case 'duplicate': f.m.files.push(f.m.files[0]); break;
        case 'traversal': f.m.files[0].path = '../outside'; break;
        }
        f.save(); assert.throws(() => verify(f.target, f.dir));
    });
}
for (const change of ['modified', 'missing', 'extra', 'extra-recorded', 'missing-recorded']) {
    test(`reject ${change} payload`, t => {
        const f = fixture(t);
        const file = path.join(f.dir, f.m.archive);
        if (change === 'modified') fs.appendFileSync(file, 'changed');
        if (change.startsWith('missing')) fs.unlinkSync(file);
        if (change.startsWith('extra')) fs.writeFileSync(path.join(f.dir, 'config.json'), '{}');
        if (change.endsWith('recorded')) { f.m.files = recordFiles(f.dir); f.save(); }
        assert.throws(() => verify(f.target, f.dir));
    });
}
test('reject directory links, including a linked bundle root', t => {
    const f = fixture(t);
    const link = path.join(f.dir, 'linked');
    fs.symlinkSync(path.join(f.dir, 'licenses'), link, process.platform === 'win32' ? 'junction' : 'dir');
    assert.throws(() => verify(f.target, f.dir), /regular file/);
    assert.throws(() => verify(f.target, link), /regular directory/);
});
test('require executable permission for the Linux supervisor', {skip: process.platform === 'win32'}, t => {
    const f = fixture(t, 'x86_64-unknown-linux-gnu');
    fs.chmodSync(path.join(f.dir, f.m.supervisor.path), 0o644);
    assert.throws(() => verify(f.target, f.dir), /not executable/);
});

test('Windows tar selection ignores Git Bash PATH and rejects missing SystemRoot', () => {
    const {archiveTool} = require('./archive-tool.cjs');
    assert.equal(archiveTool('win32', {SystemRoot: 'D:\\Windows'}), 'D:\\Windows\\System32\\tar.exe');
    assert.throws(() => archiveTool('win32', {}), /SystemRoot/);
    assert.throws(() => archiveTool('win32', {SystemRoot: 'Windows'}), /SystemRoot/);
    assert.equal(archiveTool('linux', {}), 'tar');
});
