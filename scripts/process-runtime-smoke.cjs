'use strict';
// The archive must be produced locally by process-runtime-bundle.cjs, not supplied by an untrusted party.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {verify, hash, hostTarget, commandRunner} = require('./process-runtime-bundle.cjs');
const [target, clang, input, output, ...extra] = process.argv.slice(2);
if (!output || extra.length) throw Error('usage: node scripts/process-runtime-smoke.cjs <target> <clang> <local-archive> <new-directory>');
const info = hostTarget(target);
const archive = path.resolve(input), dir = path.resolve(output);
fs.mkdirSync(dir);
const run = commandRunner(path.join(dir, 'logs'), dir);
const checksum = fs.readFileSync(archive + '.sha256', 'utf8');
assert.equal(checksum, hash(fs.readFileSync(archive)) + '  ' + path.basename(archive) + '\n');
const unpacked = path.join(dir, 'runtime');
fs.mkdirSync(unpacked);
run('tar', ['-xzf', archive, '-C', unpacked]);
const m = verify(target, unpacked);
if (m.supervisor) {
    assert.equal(run(path.join(unpacked, m.supervisor.path), ['--version']),
        `cerune-process-supervisor ${m.version} protocol=cerune-process-supervisor-v1`);
}
const probe = path.join(dir, info.platform === 'win32' ? 'probe.exe' : 'probe');
fs.copyFileSync(path.join(__dirname, '../experiments/subprocess/c_abi/probe.c'), path.join(dir, 'probe.c'));
run(clang, ['--target=' + target, '-I' + path.join(unpacked, 'include'), '-std=c11', '-Wall', '-Wextra', '-Werror', '-O0',
    path.join(dir, 'probe.c'), path.join(unpacked, m.archive), ...m.systemLinkArgs, '-o', probe]);
const config = {schema: 'cerune-process-v1', platform: info.platform === 'win32' ? 'windows' : 'linux',
    linux_supervisor: m.supervisor ? path.join(unpacked, m.supervisor.path) : null,
    limits: {arguments: 32, argument_bytes: 16384, stdin_bytes: 1024, output_bytes: 1024, timeout_ms: 10000, cleanup_timeout_ms: 5000},
    transfer_bytes: 32768,
    targets: [{name: 'fixture', executable: probe, directory: dir, environment: []},
        {name: 'missing', executable: path.join(dir, 'absent-child'), directory: dir, environment: []}]};
const configPath = path.join(dir, 'config.json');
fs.writeFileSync(configPath, JSON.stringify(config));
const stdout = run(probe, [configPath], {env: {...process.env, PATH: ''}});
assert.equal(stdout, 'C ABI: ownership, states, origins, raw bytes, failures, cleanup: ok');
console.log(stdout);
console.log(`relocated runtime verified: ${unpacked}`);
