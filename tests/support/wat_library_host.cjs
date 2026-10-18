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

  const path = require('node:path');
  const {pathToFileURL} = require('node:url');
  const dir = path.dirname(wasm);
  const generated = await import(pathToFileURL(path.join(dir, 'library.mjs')));
  const {createLibrary, metadata, CeruneHostError} = generated;
  const bytes = fs.readFileSync(wasm);
  const phase = expected => e => e instanceof CeruneHostError && e.phase === expected;
  assert.deepEqual(metadata, descriptor);
  assert.ok(Object.isFrozen(metadata.exports[0].parameters[0]));
  const handle = await createLibrary(bytes);
  assert.deepEqual(Object.keys(handle), ['functions']);
  assert.equal(Object.getPrototypeOf(handle.functions), null);
  assert.ok(Object.isFrozen(handle.functions));
  assert.equal(Reflect.set(metadata.exports[0], 'name', 'changed'), false);
  assert.equal(Reflect.set(handle.functions, 'step', () => -1), false);
  const fn = handle.functions;
  let coerced = false;
  const object = {[Symbol.toPrimitive]() {coerced = true; throw new Error('coerced');}};
  for (const args of [[1], [1,2,3,4], ['1',2,3], [1n,2,3], [object,2,3], [new Number(1),2,3]]) {
    assert.throws(() => fn.step(...args), phase('arguments'));
  }
  assert.equal(coerced, false);
  for (const x of [2147483648, -2147483649, 0.5, -0, NaN, Infinity, '1', 1n, undefined]) {
    assert.throws(() => fn.sum_to(x), phase('arguments'));
  }
  assert.throws(() => fn.checked('1.5'), phase('arguments'));
  for (let i = 0; i < 100; i++) {
    assert.equal(fn.step(i,2,0.5), i+1);
    assert.equal(fn.sum_to(i), i*(i+1)/2);
  }
  assert.equal(fn.sum_to(-2147483648), 0);
  assert.ok(Number.isNaN(fn.step(NaN,1,1)));
  assert.equal(fn.step(Infinity,1,1), Infinity);
  assert.ok(Object.is(fn.step(-0,-0,1), -0));
  const other = await createLibrary(bytes);
  assert.throws(() => fn.checked(1.5), e => {
    assert.ok(phase('language')(e));
    assert.equal(e.functionName, 'checked');
    assert.equal(e.record, fs.readFileSync(failure,'utf8'));
    assert.equal(e.failure.code, 'conversion-inexact');
    assert.ok(Object.isFrozen(e.failure));
    const sources = JSON.parse(fs.readFileSync(path.join(dir, 'sources.json'),'utf8')).files;
    const file = sources.find(f => f.id === e.failure.file);
    assert.equal(Buffer.from(file.text).subarray(e.failure.start,e.failure.end).toString('utf8'), 'convert<u8>(value)');
    return true;
  });
  assert.throws(() => fn.step(1,2,3), phase('closed'));
  assert.throws(() => fn.checked('1.5'), phase('closed'));
  assert.equal(other.functions.step(1,2,3), 7);
  assert.equal((await createLibrary(bytes)).functions.step(1,2,3), 7);
  await assert.rejects(createLibrary(new Uint8Array([1,2,3])), phase('compile'));
  await assert.rejects(createLibrary('not bytes'), phase('compile'));
  await assert.rejects(createLibrary(new Uint8Array([0,97,115,109,1,0,0,0])), phase('link'));
  await assert.rejects(createLibrary(fs.readFileSync(path.join(dir,'extra.wasm'))), phase('link'));
  await assert.rejects(createLibrary(fs.readFileSync(path.join(dir,'instantiate.wasm'))), phase('instantiate'));
  for (const [name, expected] of [['trap','trap'], ['host','host'], ['result','host']]) {
    const broken = await createLibrary(fs.readFileSync(path.join(dir,name+'.wasm')));
    assert.throws(() => broken.functions.checked(1), phase(expected));
    assert.throws(() => broken.functions.step(1,2,3), phase('closed'));
  }
  const names = await import(pathToFileURL(path.join(dir,'names.mjs')));
  const named = await names.createLibrary(fs.readFileSync(path.join(dir,'names.wasm')));
  assert.equal(named.functions.__proto__(), 7);
  assert.equal(named.functions.constructor(9), 9);
  assert.equal(named.functions.then(), 8);
  for (const value of [-2147483648, 0, 2147483647]) assert.equal(named.functions.identity(value), value);

})().catch(error => { console.error(error); process.exitCode = 1; });
