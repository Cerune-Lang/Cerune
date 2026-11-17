'use strict';
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {verify, hash, commandRunner} = require('./process-runtime-bundle.cjs');
const {archiveTool} = require('./archive-tool.cjs');
const baseline = {
    'x86_64-unknown-linux-gnu': 'ubuntu-22.04',
    'x86_64-pc-windows-msvc': 'windows-2022',
};
function names(version, target) {
    assert.match(version, /^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/);
    assert(Object.hasOwn(baseline, target), 'unsupported runtime release target');
    const stem = `cerune-process-runtime-${version}-${target}`;
    return {archive: stem + '.tar.gz', checksum: stem + '.tar.gz.sha256', metadata: stem + '.json'};
}
function identity(manifest, version, commit, target) {
    assert.match(commit, /^[0-9a-f]{40}$/);
    assert.equal(manifest.version, version, 'runtime/compiler version mismatch');
    assert.equal(manifest.target, target);
    assert.equal(manifest.source.commit, commit, 'runtime source commit mismatch');
    assert.equal(manifest.source.dirty, false, 'release assets require clean source');
}
function validateBaseline(manifest) {
    const system = manifest.build.systemRuntime;
    if (manifest.target === 'x86_64-unknown-linux-gnu') assert.equal(system, 'glibc 2.35', 'release runtime must be built on the glibc 2.35 baseline');
    else assert.match(system, /^Windows 10\.0\.20348(?:\.|$)/, 'release runtime must be built on Windows Server 2022');
}
function checksum(dir, names) {
    for (const file of [names.archive, names.checksum]) assert(fs.lstatSync(path.join(dir, file)).isFile(), 'regular runtime asset required');
    const sha256 = hash(fs.readFileSync(path.join(dir, names.archive)));
    assert.equal(fs.readFileSync(path.join(dir, names.checksum), 'utf8'), `${sha256}  ${names.archive}\n`, 'runtime archive hash mismatch');
    return sha256;
}
function stage(target, buildDir, version, commit, output) {
    const m = verify(target, path.join(buildDir, 'bundle'));
    identity(m, version, commit, target);
    validateBaseline(m);
    const n = names(version, target), sha256 = checksum(buildDir, n);
    fs.mkdirSync(output);
    for (const file of [n.archive, n.checksum]) fs.copyFileSync(path.join(buildDir, file), path.join(output, file), fs.constants.COPYFILE_EXCL);
    fs.writeFileSync(path.join(output, n.metadata), JSON.stringify({schema: 'cerune-process-runtime-asset-v1',
        archive: n.archive, sha256, validationRunner: baseline[target], manifest: m}, null, 2) + '\n');
}
function catalog(plan, commit, input, output) {
    const releases = plan.releases.filter(r => r.app_name === 'cerune-lang');
    assert.equal(releases.length, 1, 'one Cerune compiler release required');
    const version = releases[0].app_version;
    const out = path.resolve(output);
    fs.mkdirSync(out);
    const run = commandRunner(path.join(out, 'logs'), out);
    const expected = Object.keys(baseline).flatMap(target => Object.values(names(version, target))).sort();
    assert.deepEqual(fs.readdirSync(input).sort(), expected, 'runtime Release files must contain exactly both target archives, checksums and metadata');
    const assets = [];
    for (const target of Object.keys(baseline)) {
        const n = names(version, target), sha256 = checksum(input, n);
        const metadataFile = path.join(input, n.metadata);
        assert(fs.lstatSync(metadataFile).isFile(), 'regular runtime metadata required');
        const metadata = JSON.parse(fs.readFileSync(metadataFile, 'utf8'));
        assert.equal(metadata.schema, 'cerune-process-runtime-asset-v1');
        assert.equal(metadata.archive, n.archive);
        assert.equal(metadata.sha256, sha256);
        assert.equal(metadata.validationRunner, baseline[target]);
        identity(metadata.manifest, version, commit, target);
        validateBaseline(metadata.manifest);
        const extracted = path.join(out, target);
        fs.mkdirSync(extracted);
        // Input artifacts come from this workflow's tested local builds.
        run(archiveTool(), ['-xzf', path.resolve(input, n.archive), '-C', extracted]);
        const m = verify(target, extracted);
        assert.deepEqual(m, metadata.manifest, 'archive and release metadata differ');
        assets.push({target, ...n, sha256, validationRunner: baseline[target], abi: m.abi,
            systemLinkArgs: m.systemLinkArgs, buildSystemRuntime: m.build.systemRuntime});
    }
    const result = {schema: 'cerune-process-runtime-release-v1', version, commit, assets};
    fs.writeFileSync(path.join(out, 'cerune-process-runtime.json'), JSON.stringify(result, null, 2) + '\n');
    return result;
}
if (require.main === module) {
    try {
        const [op, ...args] = process.argv.slice(2);
        if (op === 'stage' && args.length === 5) stage(...args);
        else if (op === 'catalog' && args.length === 4) {
            const [plan, commit, input, output] = args;
            console.log(JSON.stringify(catalog(JSON.parse(fs.readFileSync(plan, 'utf8')), commit, input, output)));
        } else throw Error('usage: process-runtime-release.cjs stage <target> <build-dir> <version> <commit> <new-dir> | catalog <plan.json> <commit> <assets-dir> <new-dir>');
    } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = {names, identity, checksum, stage, catalog, baseline, validateBaseline};
