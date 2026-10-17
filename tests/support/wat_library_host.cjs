const assert = require('node:assert/strict');
const fs = require('node:fs');
(async () => {
  const [wasm, manifest, failure] = process.argv.slice(2);
  const descriptor = JSON.parse(fs.readFileSync(manifest, 'utf8'));
  assert.equal(descriptor.schema, 'cerune-wat-library-v1');
  assert.deepEqual(descriptor.exports.map(e => e.name), ['step', 'sum_to', 'checked']);
  assert.deepEqual(descriptor.exports[1].parameters, [{cerune:'i32', wasm:'i32', internal:'i64'}]);
  for (const e of descriptor.exports) {
    assert.equal(e.origin.file, 1);
    assert.ok(e.origin.start < e.origin.end);
  }
  const module = await WebAssembly.compile(fs.readFileSync(wasm));
  assert.deepEqual(WebAssembly.Module.exports(module).map(e => e.name).sort(), ['checked','step','sum_to']);
  assert.deepEqual(WebAssembly.Module.imports(module), [{module:'cerune',name:'write_error_byte',kind:'function'}]);
  let diagnostic = '';
  const instantiate = () => WebAssembly.instantiate(module, {
    cerune: {write_error_byte(byte) {diagnostic += String.fromCharCode(byte);}}
  });
  const instance = await instantiate();
  const api = instance.exports;
  for (let i = 0; i < 100; i++) {
    assert.equal(api.step(i, 2, 0.5), i + 1);
    assert.equal(api.sum_to(i), i * (i + 1) / 2);
  }
  assert.equal(api.sum_to(-2147483648), 0);
  assert.ok(Number.isNaN(api.step(NaN, 1, 1)));
  assert.equal(api.step(Infinity, 1, 1), Infinity);
  assert.ok(Object.is(api.step(-0, -0, 1), -0));
  assert.equal(api.checked(42), 42);
  assert.throws(() => api.checked(1.5), WebAssembly.RuntimeError);
  assert.equal(diagnostic.trim().replace(/^cerune: /,''), fs.readFileSync(failure,'utf8'));
  // A stopped instance is discarded; raw exports do not enforce handle closure.
  assert.equal((await instantiate()).exports.step(1,2,3), 7);
})().catch(error => { console.error(error); process.exitCode = 1; });
