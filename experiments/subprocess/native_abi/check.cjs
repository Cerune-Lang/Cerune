'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const {spawnSync} = require('child_process');
const [target, clang, output, runtime, supervisor] = process.argv.slice(2);
const windows = target === 'x86_64-pc-windows-msvc';
if (!['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu'].includes(target) ||
    windows !== (process.platform === 'win32') || !clang || !output || !runtime ||
    (!windows && !path.isAbsolute(supervisor || ''))) {
    throw Error('usage: node check.cjs <host-target> <clang> <new-output-dir> <runtime-archive> [absolute-linux-supervisor]');
}
const root = path.resolve(__dirname, '../../..'), dir = path.resolve(output);
const archive = path.resolve(runtime);
for (const file of [archive, ...(!windows ? [supervisor] : [])])
    if (!fs.statSync(file).isFile()) throw Error('not a file: ' + file);
fs.mkdirSync(dir);
const records = [];
function run(exe, args) {
    const result = spawnSync(exe, args, {cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 8 * 1024 * 1024});
    records.push({exe, args, status: result.status, signal: result.signal,
        stdout: result.stdout, stderr: result.stderr, error: result.error && result.error.message});
    fs.writeFileSync(path.join(dir, 'commands.json'), JSON.stringify(records, null, 2));
    if (result.error || result.status !== 0) throw Error(JSON.stringify(records[records.length - 1]));
    return result;
}
const hash = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
fs.writeFileSync(path.join(dir, 'inputs.json'), JSON.stringify({target, archive,
    archive_sha256: hash(archive), emitter_sha256: hash(path.join(__dirname, 'emit.rs')),
    cargo_lock_sha256: hash(path.join(root, 'Cargo.lock'))}, null, 2));
run(clang, ['--version']);
run('cargo', ['run', '--locked', '--example', 'subprocess_native_abi', '--', dir]);
// Keep both targets' ASM and directly encoded Objects. Assemble both ASM files,
// but execute only the explicitly selected host target.
const include = '-I' + path.join(root, 'src/runtime/process/abi');
for (const [name, triple] of [['windows', 'x86_64-pc-windows-msvc'], ['linux', 'x86_64-unknown-linux-gnu']]) {
    run(clang, ['--target=' + triple, '-Werror', '-c', path.join(dir, name, 'probe.s'),
        '-o', path.join(dir, name, 'assembled.o')]);
    // Independent reference; never linked into the probe.
    run(clang, ['--target=' + triple, include, '-std=c11', '-Werror', '-O0', '-S',
        path.join(__dirname, '../llvm_abi/reference.c'), '-o', path.join(dir, name, 'reference.s')]);
}
const hostDir = path.join(dir, windows ? 'windows' : 'linux');
const libs = windows ? ['-lkernel32', '-lntdll', '-luserenv', '-lws2_32', '-ldbghelp', '-ladvapi32', '-lbcrypt'] :
    ['-lgcc_s', '-lutil', '-lrt', '-lpthread', '-lm', '-ldl', '-lc'];
for (const route of ['probe', 'assembled']) {
    for (const optimization of ['O0', 'O2']) {
        const executable = path.join(hostDir, route + '-' + optimization + (windows ? '.exe' : ''));
        run(clang, ['--target=' + target, include, '-std=c11', '-Wall', '-Wextra', '-Werror', '-' + optimization,
            path.join(__dirname, 'driver.c'), path.join(hostDir, route + '.o'), archive, ...libs, '-o', executable]);
        const config = {
            schema: 'cerune-process-v1', platform: windows ? 'windows' : 'linux',
            linux_supervisor: windows ? null : supervisor,
            limits: {arguments: 32, argument_bytes: 16384, stdin_bytes: 1024, output_bytes: 1024,
                timeout_ms: 10000, cleanup_timeout_ms: 5000}, transfer_bytes: 32768,
            targets: [{name: 'fixture', executable, directory: hostDir, environment: []},
                {name: 'missing', executable: path.join(hostDir, 'absent-child'), directory: hostDir, environment: []}]
        };
        const configPath = path.join(hostDir, 'config-' + route + '-' + optimization + '.json');
        fs.writeFileSync(configPath, JSON.stringify(config));
        process.stdout.write(route + '/' + optimization + ': ' + run(executable, [configPath]).stdout);
    }
}
