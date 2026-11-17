'use strict';
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');
const {spawnSync} = require('node:child_process');
const {archiveTool} = require('./archive-tool.cjs');
const hosts = {
    'x86_64-pc-windows-msvc': ['win32', 'x64', 'windows-2022'],
    'x86_64-unknown-linux-gnu': ['linux', 'x64', 'ubuntu-22.04'],
    'aarch64-unknown-linux-gnu': ['linux', 'arm64', 'ubuntu-22.04-arm'],
    'x86_64-apple-darwin': ['darwin', 'x64', 'macos-15-intel'],
    'aarch64-apple-darwin': ['darwin', 'arm64', 'macos-15'],
};
function filename(value) {
    assert(typeof value === 'string' && /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(value), 'expected a plain artifact filename');
    return value;
}
function selectArchive(plan, target) {
    assert(Object.hasOwn(hosts, target), `unsupported compiler target: ${target}`);
    const releases = plan.releases.filter(r => r.app_name === 'cerune-lang');
    assert.equal(releases.length, 1, 'exactly one Cerune release required');
    const release = releases[0];
    assert.match(release.app_version, /^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/);
    const selected = release.artifacts.map(id => ({id, ...plan.artifacts[id]})).filter(a =>
        a.kind === 'executable-zip' && a.target_triples?.includes(target) &&
        a.assets?.some(asset => asset.kind === 'executable' && asset.name === 'cerune'));
    assert.equal(selected.length, 1, `exactly one compiler archive required for ${target}`);
    const archive = selected[0];
    assert.equal(archive.name, archive.id);
    assert.deepEqual(archive.target_triples, [target]);
    const binary = archive.assets.filter(a => a.kind === 'executable' && a.name === 'cerune');
    assert.equal(binary.length, 1);
    assert.equal(binary[0].path, hosts[target][0] === 'win32' ? 'cerune.exe' : 'cerune');
    filename(archive.name);
    filename(archive.checksum);
    assert.equal(plan.artifacts[archive.checksum]?.kind, 'checksum', 'checksum must be planned');
    return {archive: archive.name, checksum: archive.checksum, binary: binary[0].path, version: release.app_version};
}
function validatePlan(plan) {
    assert.equal(plan.ci.github.pr_run_mode, 'upload', 'PRs must build actual archives');
    const matrix = plan.ci.github.artifacts_matrix.include;
    assert.equal(matrix.length, Object.keys(hosts).length, 'all five native hosts must be scheduled');
    assert.deepEqual(matrix.map(m => m.host).sort(), Object.keys(hosts).sort());
    for (const entry of matrix) {
        assert.deepEqual(entry.targets, [entry.host], 'cross-compiled output needs its own native execution runner');
        assert.equal(entry.runner, hosts[entry.host][2], `runner for ${entry.host}`);
        assert(!entry.container, 'container execution requires a separately verified host mapping');
        selectArchive(plan, entry.host);
    }
}
function files(dir) {
    return fs.readdirSync(dir).flatMap(name => {
        const file = path.join(dir, name), stat = fs.lstatSync(file);
        if (stat.isDirectory()) return files(file);
        assert(stat.isFile(), `extracted non-regular file: ${file}`);
        return [file];
    });
}
function smoke(plan, target, inputDir, outputDir) {
    assert.deepEqual(hosts[target]?.slice(0, 2), [process.platform, process.arch], 'target must match the executing OS and CPU');
    const selected = selectArchive(plan, target);
    const out = path.resolve(outputDir), input = path.resolve(inputDir);
    fs.mkdirSync(out);
    const observation = {schema: 'cerune-compiler-archive-smoke-v1', target, version: selected.version, archive: selected.archive, commands: [], success: false};
    const save = () => fs.writeFileSync(path.join(out, 'observations.json'), JSON.stringify(observation, null, 2) + '\n');
    function run(exe, args, env) {
        const r = spawnSync(exe, args, {cwd: out, env, encoding: 'utf8', timeout: 60000, maxBuffer: 4 * 1024 * 1024});
        observation.commands.push({exe, args, status: r.status, signal: r.signal, stdout: r.stdout, stderr: r.stderr, error: r.error?.message});
        save();
        assert(!r.error && r.status === 0, `${exe} failed: ${r.error?.message || r.stderr}`);
        return r;
    }
    try {
        const archive = path.join(input, selected.archive), checksum = path.join(input, selected.checksum);
        assert(fs.lstatSync(archive).isFile(), 'missing or linked compiler archive');
        assert(fs.lstatSync(checksum).isFile(), 'missing or linked checksum');
        const sha256 = createHash('sha256').update(fs.readFileSync(archive)).digest('hex');
        const checksumText = fs.readFileSync(checksum, 'utf8').trimEnd();
        assert([`${sha256} *${selected.archive}`, `${sha256}  ${selected.archive}`].includes(checksumText), 'archive checksum mismatch');
        observation.sha256 = sha256;
        const unpacked = path.join(out, 'unpacked');
        fs.mkdirSync(unpacked);
        const entries = run(archiveTool(), ['-tf', archive]).stdout.trim().split(/\r?\n/);
        for (const entry of entries) {
            assert(entry && !entry.startsWith('/') && !entry.includes('\\') && !entry.includes(':') &&
                !entry.split('/').includes('..'), `unsafe archive entry: ${entry}`);
        }
        run(archiveTool(), ['-xf', archive, '-C', unpacked]);
        const binaries = files(unpacked).filter(f => path.basename(f) === selected.binary);
        assert.equal(binaries.length, 1, 'exactly one extracted compiler required');
        const exe = binaries[0], env = {...process.env, PATH: ''};
        const help = run(exe, ['--help'], env);
        assert(help.stdout.includes(`Cerune ${selected.version}`), 'packaged compiler version/help mismatch');
        fs.writeFileSync(path.join(out, 'minimal.ceru'), 'print(7);\n');
        const execution = run(exe, ['run', path.join(out, 'minimal.ceru')], env);
        assert.equal(execution.stdout, '7\n', 'minimal source output');
        assert.equal(execution.stderr, '', 'minimal source diagnostics');
        observation.success = true;
        save();
        return observation;
    } catch (error) {
        observation.error = error.message;
        save();
        throw error;
    }
}
if (require.main === module) {
    try {
        const [planFile, target, input, output, ...extra] = process.argv.slice(2);
        assert(planFile && target && !extra.length, 'usage: node compiler-archive-smoke.cjs <plan.json> <validate-plan|target> [artifacts new-output]');
        const plan = JSON.parse(fs.readFileSync(planFile, 'utf8'));
        if (target === 'validate-plan') { assert(!input && !output); validatePlan(plan); console.log('Five native compiler archive targets planned'); }
        else { assert(input && output); const r = smoke(plan, target, input, output); console.log(`${r.target}: packaged compiler help and print(7) passed`); }
    } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = {selectArchive, validatePlan, smoke, hosts};
