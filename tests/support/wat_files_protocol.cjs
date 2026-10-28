'use strict';
// Transport probe, not a second implementation of Cerune file semantics.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const {createFiles} = require('./wat_files.cjs');
const moduleBytes = fs.readFileSync(process.argv[2]);
const wasmModule = new WebAssembly.Module(moduleBytes);
assert(WebAssembly.Module.imports(wasmModule).every(i => i.module === 'cerune' && i.kind === 'function'));
assert(!WebAssembly.Module.exports(wasmModule).some(e => e.kind === 'memory'));
function connect(config) {
  const host = createFiles(config);
  const instance = new WebAssembly.Instance(wasmModule, {cerune: host.imports});
  return {host, api: instance.exports};
}
function pack(bytes) {
  let bits = 0n;
  bytes.forEach((b, i) => { bits |= BigInt(b) << BigInt(i * 8); });
  return BigInt.asIntN(64, bits);
}
function unpack(bits, count) {
  const bytes = Buffer.alloc(count); bits = BigInt.asUintN(64, bits);
  for (let i = 0; i < count; i++) { bytes[i] = Number(bits & 255n); bits >>= 8n; }
  return bytes;
}
function open(api, name, mode) {
  const bytes = Buffer.from(name);
  assert.equal(api.file_name_begin_v1(bytes.length), 0);
  for (let i = 0; i < bytes.length; i += 8) {
    const chunk = bytes.subarray(i, i + 8);
    assert.equal(api.file_name_chunk_v1(pack(chunk), chunk.length), 0);
  }
  return api.file_open_v1(mode);
}
function send(api, handle, bytes) {
  let offset = 0;
  while (offset < bytes.length) {
    const chunk = bytes.subarray(offset, offset + 8);
    const n = api.file_write_v1(handle, pack(chunk), chunk.length);
    if (n === -2) continue;
    if (n <= 0 || n > chunk.length) return {offset, response: n};
    offset += n;
  }
  return {offset, response: null};
}
function receive(api, handle) {
  const chunks = [];
  while (true) {
    const [bits, n] = api.file_read_v1(handle, 8);
    if (n === -2) continue;
    if (n <= 0 || n > 8) return {bytes: Buffer.concat(chunks), response: n};
    chunks.push(unpack(bits, n));
  }
}

const text = Buffer.from('日本語\0\r\ne\u0301\n');
assert.equal(text.length, 16);
for (const traceLimit of [0, 1, 1000]) {
  const {host, api} = connect({writes: [-2, 1, 2], reads: [-2, 1, 2], traceLimit});
  assert.equal(api.files_status_v1(), 0);
  const [writer, status] = open(api, '日本語-sample.txt', 1);
  assert.equal(status, 0); assert(writer > 0);
  assert.deepEqual(send(api, writer, text), {offset: 16, response: null});
  assert.equal(api.file_close_v1(writer), 0);
  assert.deepEqual(host.contents('日本語-sample.txt'), text);
  const [reader, readStatus] = open(api, '日本語-sample.txt', 0);
  assert.equal(readStatus, 0);
  assert.deepEqual(receive(api, reader), {bytes: text, response: 0});
  assert.equal(api.file_close_v1(reader), 0);
  assert.deepEqual(open(api, '日本語-sample.txt', 1), [0, 2]);
  assert.deepEqual(open(api, 'missing', 0), [0, 1]);
  assert.equal(host.openHandles(), 0);
  assert(host.events().length <= traceLimit);
  const copy = host.contents('日本語-sample.txt'); copy.fill(0);
  const events = host.events(); if (events.length) events[0].accepted = 999;
  assert.deepEqual(host.contents('日本語-sample.txt'), text);
  assert(!host.events().some(e => e.accepted === 999));
}
// All bits, including signed i64 and invalid UTF-8, cross the ABI unchanged.
// UTF-8 validation belongs to future generated WAT, not the host.
{
  const bytes = Buffer.from(Array.from({length: 256}, (_, i) => i));
  const {host, api} = connect({});
  const [h] = open(api, 'raw', 1);
  assert.equal(send(api, h, bytes).offset, 256); api.file_close_v1(h);
  const [r] = open(api, 'raw', 0);
  assert.deepEqual(receive(api, r).bytes, bytes); api.file_close_v1(r);
  assert.deepEqual(host.contents('raw'), bytes);
}
for (const response of [0, -1, -3, 9]) {
  const {host, api} = connect({writes: [2, response]});
  const [h] = open(api, 'partial', 1);
  assert.deepEqual(send(api, h, Buffer.alloc(16, 97)), {offset: 2, response});
  assert.equal(api.file_close_v1(h), 0);
  assert.deepEqual(host.contents('partial'), Buffer.from('aa'));
  assert.equal(host.events().filter(e => e.operation === 'write').length, 2);
}
for (const response of [-1, -3, 9]) {
  const {host, api} = connect({files: [['read', text]], reads: [2, response], closeError: true});
  const [h] = open(api, 'read', 0);
  assert.deepEqual(receive(api, h), {bytes: text.subarray(0, 2), response});
  assert.equal(api.file_close_v1(h), 1);
  assert.equal(host.openHandles(), 0);
  assert.throws(() => api.file_close_v1(h), /consumed/);
}
for (const length of [0, 1, 8, 9]) {
  const bytes = Buffer.alloc(length, 120);
  const {api} = connect({files: [['exact', bytes]]});
  const [h] = open(api, 'exact', 0);
  let offset = 0;
  while (offset < length) {
    const requested = Math.min(8, length - offset);
    const [bits, n] = api.file_read_v1(h, requested);
    assert.equal(n, requested);
    assert.deepEqual(unpack(bits, n), bytes.subarray(offset, offset + n)); offset += n;
  }
  assert.deepEqual(api.file_read_v1(h, 1), [0n, 0]); api.file_close_v1(h);
}
{
  const original = Buffer.from('before'), writes = [-2];
  const {api, host} = connect({files: [['original', original]], writes});
  original.fill(0); writes[0] = -1;
  assert.deepEqual(host.contents('original'), Buffer.from('before'));
  // A new begin abandons an incomplete name and has no file effect.
  api.file_name_begin_v1(2); api.file_name_chunk_v1(97n, 1);
  assert.equal(host.contents('a'), undefined);
  const [h] = open(api, 'empty', 1);
  assert.equal(api.file_write_v1(h, 120n, 1), -2);
  assert.equal(api.file_close_v1(h), 0);
  assert.deepEqual(host.contents('empty'), Buffer.alloc(0));
  assert.deepEqual(open(api, 'empty', 1), [0, 2]);
}

{
  const {api, host} = connect({files: [['known-read', Buffer.from([1,2,3,4,5,6,7,8])]]});
  const [w] = open(api, 'known-write', 1);
  assert.equal(api.file_write_v1(w, BigInt.asIntN(64, 0x8070605040302010n), 8), 8);
  assert.equal(api.file_write_v1(w, 0x7777777744332211n, 4), 4);
  api.file_close_v1(w);
  assert.deepEqual(host.contents('known-write'), Buffer.from([16,32,48,64,80,96,112,128,17,34,51,68]));
  const [r] = open(api, 'known-read', 0);
  assert.deepEqual(api.file_read_v1(r, 8), [0x0807060504030201n, 8]);
  api.file_close_v1(r);
  const [empty] = open(api, 'x'.repeat(255), 1);
  api.file_close_v1(empty);
  assert.deepEqual(host.contents('x'.repeat(255)), Buffer.alloc(0));
  assert.throws(() => api.file_name_begin_v1(256), /invalid name/);
}
assert.equal(connect({configured: false}).api.files_status_v1(), 1);
for (const name of Object.keys(createFiles().imports)) {
  const imports = {...createFiles().imports}; delete imports[name];
  assert.throws(() => new WebAssembly.Instance(wasmModule, {cerune: imports}), WebAssembly.LinkError);
}
{
  const imports = {...createFiles().imports, files_status_v1() { throw new Error('host failure'); }};
  const instance = new WebAssembly.Instance(wasmModule, {cerune: imports});
  assert.throws(() => instance.exports.files_status_v1(), /host failure/);
}
// A module with no file operations needs no new imports.
new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from([0,97,115,109,1,0,0,0])), {});
console.log('file host ABI probe: roundtrip, partial transfers, failures, EOF, handles and compatibility passed');
