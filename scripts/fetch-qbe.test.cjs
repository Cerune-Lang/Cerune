'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const root = path.resolve(__dirname, '..');
const digest = 'd587905d620dc5e1d2bfa7c2cc642b9b837aa89a3188c6e37b53d756cf66e320';
const shellPath = value => process.platform === 'win32' ? value.replaceAll('\\', '/').replace(/^([A-Za-z]):/, (_, drive) => '/' + drive.toLowerCase()) : value;
const archive = fs.readFileSync(path.join(root, '.github/vendor/qbe/qbe-1.3.tar.xz'));
test('vendored archive and all four CI consumers retain the pinned source', () => {
  assert.equal(createHash('sha256').update(archive).digest('hex'), digest);
  const ci = fs.readFileSync(path.join(root, '.github/workflows/ci.yml'), 'utf8');
  assert.equal(ci.split('bash scripts/fetch-qbe.sh target/qbe-1.3.tar.xz').length - 1, 4);
  assert.ok(!ci.includes('https://c9x.me/compile/release/qbe-1.3.tar.xz'));
});
for (const mode of ['success', 'timeout', 'http-error', 'partial', 'wrong-hash', 'bad-backup', 'offline', 'existing', 'bad-existing']) {
  test('QBE download: ' + mode, { skip: process.platform === 'win32' && !process.env.CERUNE_TEST_BASH ? 'set CERUNE_TEST_BASH to MSYS2 bash on Windows' : false }, () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cerune-qbe-fetch-'));
    try {
      fs.mkdirSync(path.join(dir, 'scripts'));
      fs.mkdirSync(path.join(dir, '.github/vendor/qbe'), { recursive: true });
      fs.mkdirSync(path.join(dir, 'bin'));
      fs.copyFileSync(path.join(root, 'scripts/fetch-qbe.sh'), path.join(dir, 'scripts/fetch-qbe.sh'));
      fs.writeFileSync(path.join(dir, '.github/vendor/qbe/qbe-1.3.tar.xz'), mode === 'bad-backup' ? 'corrupt' : archive);
      fs.writeFileSync(path.join(dir, 'upstream.tar.xz'), archive);
      fs.writeFileSync(path.join(dir, 'bin/curl'), [
        '#!/usr/bin/env bash', 'set -eu',
        'printf called > curl-called',
        'while [[ $# -gt 0 ]]; do if [[ "$1" == -o ]]; then destination=$2; shift 2; else shift; fi; done',
        'case "$QBE_TEST_MODE" in',
        'success) cp upstream.tar.xz "$destination" ;;',
        'timeout|bad-backup) exit 28 ;;',
        'http-error) exit 22 ;;',
        'partial) printf partial > "$destination"; exit 18 ;;',
        'wrong-hash) printf corrupt > "$destination" ;;',
        '*) exit 99 ;;', 'esac', ''
      ].join('\n'), { mode: 0o755 });
      const output = path.join(dir, 'output with spaces.tar.xz');
      if (mode === 'existing') fs.writeFileSync(output, archive);
      if (mode === 'bad-existing') fs.writeFileSync(output, 'do not overwrite');
      const args = ['scripts/fetch-qbe.sh', ...(mode === 'offline' ? ['--offline'] : []), shellPath(output)];
      const result = spawnSync(process.env.CERUNE_TEST_BASH || 'bash', ['-c', 'export PATH="$1:/usr/bin:/bin"; shift; exec bash "$@"', 'fetch-qbe-test', shellPath(path.join(dir, 'bin')), ...args], {
        cwd: dir, env: { ...process.env, QBE_TEST_MODE: mode },
        encoding: 'utf8', timeout: 10000,
      });
      assert.ifError(result.error);
      const fails = ['wrong-hash', 'bad-backup', 'bad-existing'].includes(mode);
      assert.equal(result.status, fails ? 1 : 0, result.stdout + result.stderr);
      if (!fails) assert.deepEqual(fs.readFileSync(output), archive);
      else if (mode === 'bad-existing') assert.equal(fs.readFileSync(output, 'utf8'), 'do not overwrite');
      else assert.ok(!fs.existsSync(output));
      assert.ok(!fs.readdirSync(dir).some(name => name.includes('.tmp.')));
      if (['offline', 'existing', 'bad-existing'].includes(mode)) assert.ok(!fs.existsSync(path.join(dir, 'curl-called')));
      if (['timeout', 'partial', 'http-error', 'offline'].includes(mode)) assert.match(result.stdout, /verified repository backup/);
      if (mode === 'wrong-hash') {
        assert.match(result.stderr, /upstream SHA-256 mismatch/);
        assert.ok(!result.stdout.includes('verified repository backup'));
      }
    } finally { fs.rmSync(dir, { recursive: true, force: true }); }
  });
}
