'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const {spawnSync} = require('child_process');
const [target, compiler, clang, output, runtime, supervisor] = process.argv.slice(2);
const windows = target === 'x86_64-pc-windows-msvc';
if (!['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu'].includes(target) ||
    windows !== (process.platform === 'win32') || !compiler || !clang || !output || !runtime ||
    (!windows && !path.isAbsolute(supervisor || ''))) {
    throw Error('usage: node check.cjs <host-target> <qbe-path> <clang> <new-output-dir> <runtime-archive> [absolute-linux-supervisor]');
}
const root = path.resolve(__dirname, '../../..'), dir = path.resolve(output);
const qbe = path.resolve(compiler), archive = path.resolve(runtime);
for (const file of [qbe, archive, ...(!windows ? [supervisor] : [])])
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
const input = path.join(__dirname, 'probe.ssa');
fs.writeFileSync(path.join(dir, 'inputs.json'), JSON.stringify({target, qbe, archive,
    qbe_sha256: crypto.createHash('sha256').update(fs.readFileSync(qbe)).digest('hex'),
    il_sha256: crypto.createHash('sha256').update(fs.readFileSync(input)).digest('hex')}, null, 2));
run(qbe, ['-h']);
run(clang, ['--version']);
fs.copyFileSync(input, path.join(dir, 'probe.ssa'));
// Keep both target assemblies from exactly the same IL, and assemble both.
// Only the explicitly selected host target is linked and executed.
for (const [name, triple] of [['amd64_win', 'x86_64-pc-windows-msvc'], ['amd64_sysv', 'x86_64-unknown-linux-gnu']]) {
    const asm = path.join(dir, name + '.s');
    run(qbe, ['-t', name, '-o', asm, input]);
    run(clang, ['--target=' + triple, '-Werror', '-c', asm, '-o', path.join(dir, name + '.o')]);
}
const include = '-I' + path.join(root, 'src/runtime/process/abi');
// Reference only: the C frontend's create call is not linked into the probe.
run(clang, ['--target=' + target, include, '-std=c11', '-Werror', '-O0', '-S',
    path.join(__dirname, '../llvm_abi/reference.c'), '-o', path.join(dir, 'reference.s')]);
const object = path.join(dir, (windows ? 'amd64_win' : 'amd64_sysv') + '.o');
const libs = windows ? ['-lkernel32', '-lntdll', '-luserenv', '-lws2_32', '-ldbghelp', '-ladvapi32', '-lbcrypt'] :
    ['-lgcc_s', '-lutil', '-lrt', '-lpthread', '-lm', '-ldl', '-lc'];
// QBE has no -O0/-O2 switch. These options apply only to the C assertion driver.
for (const optimization of ['O0', 'O2']) {
    const executable = path.join(dir, 'probe-' + optimization + (windows ? '.exe' : ''));
    run(clang, ['--target=' + target, include, '-std=c11', '-Wall', '-Wextra', '-Werror', '-' + optimization,
        path.join(__dirname, 'driver.c'), object, archive, ...libs, '-o', executable]);
    const config = {
        schema: 'cerune-process-v1', platform: windows ? 'windows' : 'linux',
        linux_supervisor: windows ? null : supervisor,
        limits: {arguments: 32, argument_bytes: 16384, stdin_bytes: 1024, output_bytes: 1024,
            timeout_ms: 10000, cleanup_timeout_ms: 5000}, transfer_bytes: 32768,
        targets: [{name: 'fixture', executable, directory: dir, environment: []},
            {name: 'missing', executable: path.join(dir, 'absent-child'), directory: dir, environment: []}]
    };
    const configPath = path.join(dir, 'config-' + optimization + '.json');
    fs.writeFileSync(configPath, JSON.stringify(config));
    process.stdout.write('C driver ' + optimization + ': ' + run(executable, [configPath]).stdout);
}