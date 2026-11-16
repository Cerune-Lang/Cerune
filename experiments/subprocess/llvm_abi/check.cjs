'use strict';
const fs = require('fs'), path = require('path'), {spawnSync} = require('child_process');
const [target, clang, output, runtime, supervisor] = process.argv.slice(2);
const windows = target === 'x86_64-pc-windows-msvc';
if (!['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu'].includes(target) ||
    windows !== (process.platform === 'win32') || !clang || !output || !runtime ||
    (!windows && !path.isAbsolute(supervisor || ''))) {
    throw Error('usage: node check.cjs <host-target> <clang> <new-output-dir> <runtime-archive> [absolute-linux-supervisor]');
}
const root = path.resolve(__dirname, '../../..'), dir = path.resolve(output);
const archive = path.resolve(runtime);
if (!fs.statSync(archive).isFile()) throw Error('runtime archive is not a file');
if (!windows && !fs.statSync(supervisor).isFile()) throw Error('supervisor is not a file');
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
run(clang, ['--version']);
const include = '-I' + path.join(root, 'src/runtime/process/abi');
const platform = windows ? 'windows' : 'linux';
const reference = path.join(__dirname, 'reference.c');
run(clang, ['--target=' + target, include, '-std=c11', '-Werror', '-O0', '-S', '-emit-llvm',
    reference, '-o', path.join(dir, 'reference.ll')]);
run(clang, ['--target=' + target, include, '-std=c11', '-Werror', '-O0', '-S',
    reference, '-o', path.join(dir, 'reference.s')]);
const referenceIR = fs.readFileSync(path.join(dir, 'reference.ll'), 'utf8');
const triple = referenceIR.match(/^target triple = "([^"]+)"$/m);
if (!triple || !(triple[1] === target ||
    (windows && /^x86_64-pc-windows-msvc[0-9]+(?:\.[0-9]+)*$/.test(triple[1]))))
    throw Error('Clang target differs from the explicit probe target');
const declaration = referenceIR.match(/^declare[^\n]*@cerune_process_v1_create\(([^)]*)\)/m);
const types = declaration && declaration[1].split(',').map(arg => arg.trim().split(/\s+/)[0]);
const expected = windows ? ['i64', 'i64', 'ptr', 'ptr'] : ['i64', 'i64', 'ptr', 'i64', 'ptr'];
if (JSON.stringify(types) !== JSON.stringify(expected))
    throw Error('Clang create signature differs from the handwritten LLVM contract');
let ir = 'target triple = "' + triple[1] + '"\n' +
    fs.readFileSync(path.join(__dirname, 'common.ll'), 'utf8') +
    fs.readFileSync(path.join(__dirname, platform + '.ll'), 'utf8');
// Compute offsets using LLVM layout; compare with independent C offsetof values.
const expressions = [
    'ptrtoint (ptr getelementptr (%cerune.process.bytes, ptr null, i64 1) to i64)',
    'ptrtoint (ptr getelementptr (%cerune.process.request, ptr null, i64 1) to i64)'
];
for (let index = 0; index < 13; index++)
    expressions.push('ptrtoint (ptr getelementptr (%cerune.process.request, ptr null, i32 0, i32 ' + index + ') to i64)');
ir += '\ndefine void @cerune_llvm_layout(ptr %out) {\nentry:\n';
expressions.forEach((value, index) => {
    ir += '  %p' + index + ' = getelementptr i64, ptr %out, i64 ' + index + '\n';
    ir += '  store i64 ' + value + ', ptr %p' + index + '\n';
});
ir += '  ret void\n}\n';
const ll = path.join(dir, 'probe.ll');
fs.writeFileSync(ll, ir);
const libs = windows ? ['-lkernel32', '-lntdll', '-luserenv', '-lws2_32', '-ldbghelp', '-ladvapi32', '-lbcrypt'] :
    ['-lgcc_s', '-lutil', '-lrt', '-lpthread', '-lm', '-ldl', '-lc'];
for (const optimization of ['O0', 'O2']) {
    const object = path.join(dir, 'probe-' + optimization + (windows ? '.obj' : '.o'));
    const executable = path.join(dir, 'probe-' + optimization + (windows ? '.exe' : ''));
    run(clang, ['--target=' + target, '-' + optimization, '-Werror', '-c', ll, '-o', object]);
    run(clang, ['--target=' + target, '-' + optimization, '-Werror', '-S', ll,
        '-o', path.join(dir, 'probe-' + optimization + '.s')]);
    run(clang, ['--target=' + target, include, '-std=c11', '-Wall', '-Wextra', '-Werror', '-' + optimization,
        path.join(__dirname, 'driver.c'), object, archive, ...libs, '-o', executable]);
    const config = {
        schema: 'cerune-process-v1', platform,
        linux_supervisor: windows ? null : supervisor,
        limits: {arguments: 32, argument_bytes: 16384, stdin_bytes: 1024, output_bytes: 1024,
            timeout_ms: 10000, cleanup_timeout_ms: 5000}, transfer_bytes: 32768,
        targets: [{name: 'fixture', executable, directory: dir, environment: []},
            {name: 'missing', executable: path.join(dir, 'absent-child'), directory: dir, environment: []}]
    };
    const configPath = path.join(dir, 'config-' + optimization + '.json');
    fs.writeFileSync(configPath, JSON.stringify(config));
    process.stdout.write(optimization + ': ' + run(executable, [configPath]).stdout);
}
