'use strict';
const {archiveTool} = require('./archive-tool.cjs');
// Build inputs are explicit; this tool never reads subprocess registration/configuration.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {spawnSync} = require('node:child_process');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const schema = 'cerune-process-runtime-bundle-v1';
const targets = {
    'x86_64-pc-windows-msvc': {
        platform: 'win32', archive: 'process-runtime.lib', cargoArchive: 'cerune_lang.lib',
        systemLinkArgs: ['-lkernel32', '-lntdll', '-luserenv', '-lws2_32', '-ldbghelp', '-ladvapi32', '-lbcrypt'],
    },
    'x86_64-unknown-linux-gnu': {
        platform: 'linux', archive: 'process-runtime.a', cargoArchive: 'libcerune_lang.a',
        systemLinkArgs: ['-lgcc_s', '-lutil', '-lrt', '-lpthread', '-lm', '-ldl', '-lc'],
    },
};
function targetInfo(target) {
    if (!Object.hasOwn(targets, target)) throw Error(`unsupported explicit target: ${target}`);
    return targets[target];
}
function hostTarget(target) {
    const info = targetInfo(target);
    if (info.platform !== process.platform || process.arch !== 'x64') throw Error('build/smoke requires the matching x86_64 host');
    return info;
}
function hash(bytes) { return crypto.createHash('sha256').update(bytes).digest('hex'); }
function json(file, value) { fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n'); }
function relative(file) {
    if (typeof file !== 'string' || !/^[A-Za-z0-9_.+/-]+$/.test(file) ||
        file.split('/').some(p => !p || p === '.' || p === '..')) throw Error(`invalid bundle path: ${file}`);
    return file;
}
function walk(dir, prefix = '') {
    if (!fs.lstatSync(dir).isDirectory()) throw Error(`not a regular directory: ${dir}`);
    return fs.readdirSync(dir).sort().flatMap(name => {
        const file = path.join(dir, name), rel = prefix + name, stat = fs.lstatSync(file);
        if (stat.isDirectory()) return walk(file, rel + '/');
        if (!stat.isFile()) throw Error(`not a regular file: ${rel}`);
        relative(rel);
        if (stat.size > 128 * 1024 * 1024) throw Error(`bundle file exceeds 128 MiB: ${rel}`);
        return [rel];
    });
}
function recordFiles(dir) {
    return walk(dir).filter(p => p !== 'manifest.json').map(p => {
        const bytes = fs.readFileSync(path.join(dir, p));
        return {path: p, bytes: bytes.length, sha256: hash(bytes), executable: p === 'bin/cerune-process-supervisor'};
    });
}
function verify(target, dir) {
    const info = targetInfo(target);
    const actual = recordFiles(dir); // Reject links before opening the manifest or payloads.
    const stat = fs.lstatSync(path.join(dir, 'manifest.json'));
    if (!stat.isFile() || stat.size > 1024 * 1024) throw Error('invalid manifest file');
    const m = JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf8'));
    assert.equal(m.schema, schema, 'bundle schema');
    assert.equal(m.target, target, 'requested target');
    assert.deepEqual(m.abi, {name: 'cerune-process-v1', version: 1, reportWords: 32});
    assert.equal(m.archive, 'lib/' + info.archive);
    assert.equal(m.header, 'include/cerune_process_v1.h');
    assert.deepEqual(m.systemLinkArgs, info.systemLinkArgs);
    assert.deepEqual(m.supervisor, info.platform === 'linux' ? {
        path: 'bin/cerune-process-supervisor', protocol: 'cerune-process-supervisor-v1', binding: 'execution-time-json',
    } : null);
    assert.deepEqual(m.configuration, {schema: 'cerune-process-v1', embedded: false});
    assert.match(m.version, /^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/);
    assert.match(m.source.commit, /^[0-9a-f]{40}$/);
    assert.equal(typeof m.source.dirty, 'boolean');
    assert.equal(m.build.profile, 'release');
    assert.equal(m.build.host, info.platform);
    assert.equal(m.build.architecture, 'x64');
    assert.match(m.build.rustc, /^rustc /);
    assert.match(m.build.cargo, /^cargo /);
    assert.equal(typeof m.build.systemRuntime, 'string');
    assert(m.build.systemRuntime.length > 0);
    assert.deepEqual(m.files, actual, 'bundle contents, lengths and SHA-256');
    const names = new Set(actual.map(f => f.path));
    for (const required of [m.archive, m.header, 'Cargo.lock', 'README.md', 'README.en.md', 'licenses/index.json',
        'licenses/cerune/LICENSE-MIT', 'licenses/cerune/LICENSE-APACHE',
        'licenses/rust/COPYRIGHT-library.html', ...(m.supervisor ? [m.supervisor.path] : [])]) {
        assert(names.has(required), `missing ${required}`);
    }
    for (const file of actual) {
        assert(file.path === m.archive || file.path === m.header || file.path === 'Cargo.lock' || file.path === 'README.md' || file.path === 'README.en.md' ||
            file.path.startsWith('licenses/') || file.path === m.supervisor?.path, `unexpected payload: ${file.path}`);
        if (file.executable && process.platform !== 'win32') {
            assert(fs.statSync(path.join(dir, file.path)).mode & 0o111, `not executable: ${file.path}`);
        }
    }
    return m;
}
function commandRunner(logDir, cwd) {
    fs.mkdirSync(logDir, {recursive: true});
    const commands = [];
    return (exe, args, options = {}) => {
        const r = spawnSync(exe, args, {cwd, encoding: 'utf8', timeout: 600000, maxBuffer: 32 * 1024 * 1024, ...options});
        commands.push({exe, args, status: r.status, signal: r.signal, stdout: r.stdout, stderr: r.stderr, error: r.error?.message});
        json(path.join(logDir, 'commands.json'), commands);
        if (r.error || r.status !== 0) throw Error(`${exe} failed: ${r.error?.message || r.stderr || r.stdout}; see ${logDir}`);
        return r.stdout.trim();
    };
}
function copyFile(from, dir, rel) {
    relative(rel);
    if (!fs.lstatSync(from).isFile()) throw Error(`not a regular input file: ${from}`);
    const to = path.join(dir, rel);
    fs.mkdirSync(path.dirname(to), {recursive: true});
    fs.copyFileSync(from, to, fs.constants.COPYFILE_EXCL);
}
function collectLicenses(metadata, sysroot, dir) {
    for (const file of ['LICENSE-MIT', 'LICENSE-APACHE']) copyFile(path.join(root, file), dir, 'licenses/cerune/' + file);
    const entries = [];
    for (const pkg of metadata.packages.filter(p => p.name !== 'cerune-lang')) {
        const base = path.dirname(pkg.manifest_path);
        const files = fs.readdirSync(base).filter(f => /^(license|licence|copying|notice|copyright)([._-]|$)/i.test(f) && fs.lstatSync(path.join(base, f)).isFile());
        if (pkg.license_file && !files.includes(pkg.license_file)) files.push(pkg.license_file);
        if (!files.length) throw Error(`missing license files for ${pkg.name} ${pkg.version}`);
        const prefix = `licenses/dependencies/${pkg.name}-${pkg.version}/`;
        for (const file of files) copyFile(path.join(base, file), dir, prefix + relative(file));
        entries.push({name: pkg.name, version: pkg.version, license: pkg.license, files: files.map(f => prefix + f)});
    }
    const rustDocs = path.join(sysroot, 'share/doc/rust');
    copyFile(path.join(rustDocs, 'COPYRIGHT-library.html'), dir, 'licenses/rust/COPYRIGHT-library.html');
    for (const file of walk(path.join(rustDocs, 'licenses'))) copyFile(path.join(rustDocs, 'licenses', file), dir, 'licenses/rust/licenses/' + file);
    json(path.join(dir, 'licenses/index.json'), {scope: 'Cargo.lock resolved dependencies; not a claim that every package is linked', packages: entries});
}
function build(target, output) {
    const info = hostTarget(target);
    const out = path.resolve(output);
    fs.mkdirSync(out); // Never overwrite earlier artifacts or observations.
    const dir = path.join(out, 'bundle');
    fs.mkdirSync(dir);
    const run = commandRunner(path.join(out, 'logs'), root);
    const commit = run('git', ['rev-parse', 'HEAD']);
    const dirty = run('git', ['status', '--porcelain']).length !== 0;
    const rustc = run('rustc', ['-vV']);
    const cargo = run('cargo', ['--version']);
    const systemRuntime = info.platform === 'linux' ? run('getconf', ['GNU_LIBC_VERSION']) : 'Windows ' + require('node:os').release();
    const metadata = JSON.parse(run('cargo', ['metadata', '--locked', '--format-version', '1']));
    const version = metadata.packages.find(p => p.name === 'cerune-lang').version;
    const targetDir = path.join(out, 'build');
    const common = ['--locked', '--release', '--target', target, '--target-dir', targetDir];
    run('cargo', ['rustc', ...common, '--lib', '--features', 'process-c-abi', '--crate-type', 'staticlib', '--', '--print', 'native-static-libs']);
    copyFile(path.join(targetDir, target, 'release', info.cargoArchive), dir, 'lib/' + info.archive);
    copyFile(path.join(root, 'src/runtime/process/abi/cerune_process_v1.h'), dir, 'include/cerune_process_v1.h');
    if (info.platform === 'linux') {
        run('cargo', ['build', ...common, '--example', 'subprocess_linux_supervisor']);
        copyFile(path.join(targetDir, target, 'release/examples/subprocess_linux_supervisor'), dir, 'bin/cerune-process-supervisor');
        fs.chmodSync(path.join(dir, 'bin/cerune-process-supervisor'), 0o755);
    }
    copyFile(path.join(root, 'Cargo.lock'), dir, 'Cargo.lock');
    collectLicenses(metadata, run('rustc', ['--print', 'sysroot']), dir);
    for (const file of ['README.md', 'README.en.md']) copyFile(path.join(root, 'docs/distribution/process-runtime', file), dir, file);
    const manifest = {
        schema, version, target, source: {commit, dirty},
        abi: {name: 'cerune-process-v1', version: 1, reportWords: 32},
        archive: 'lib/' + info.archive, header: 'include/cerune_process_v1.h',
        systemLinkArgs: info.systemLinkArgs,
        supervisor: info.platform === 'linux' ? {path: 'bin/cerune-process-supervisor', protocol: 'cerune-process-supervisor-v1', binding: 'execution-time-json'} : null,
        configuration: {schema: 'cerune-process-v1', embedded: false},
        build: {profile: 'release', rustc, cargo, host: process.platform, architecture: process.arch, systemRuntime},
        files: recordFiles(dir),
    };
    json(path.join(dir, 'manifest.json'), manifest);
    verify(target, dir);
    const name = `cerune-process-runtime-${version}-${target}.tar.gz`;
    run(archiveTool(), ['-czf', path.join(out, name), '-C', dir, '.']);
    fs.writeFileSync(path.join(out, name + '.sha256'), hash(fs.readFileSync(path.join(out, name))) + '  ' + name + '\n');
    return path.join(out, name);
}
if (require.main === module) {
    try {
        const [operation, target, output, ...extra] = process.argv.slice(2);
        if (!output || extra.length || !['build', 'verify'].includes(operation)) throw Error('usage: node scripts/process-runtime-bundle.cjs <build|verify> <explicit-target> <directory>');
        if (operation === 'build') console.log(build(target, output));
        else { const m = verify(target, path.resolve(output)); console.log(`${m.schema}: ${m.target}: ${m.files.length} files verified`); }
    } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = {build, verify, recordFiles, hash, hostTarget, commandRunner, targets};
