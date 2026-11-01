'use strict';
// In-memory test connection for the versioned ABI. No OS filesystem or Wasm memory.
function createFiles(config = {}) {
  config = {...config, openResponse: config.openResponse ? [...config.openResponse] : undefined};
  const files = new Map((config.files || []).map(([name, bytes]) =>
    [Buffer.from(name, 'utf8').toString('hex'), Buffer.from(bytes)]));
  const reads = [...(config.reads || [])], writes = [...(config.writes || [])];
  const handles = new Map(), events = [];
  const traceLimit = config.traceLimit ?? 1000;
  const configured = config.configured !== false, closeError = !!config.closeError;
  let pending = null, nextHandle = 1;
  function record(operation, requested, accepted) {
    if (events.length < traceLimit) events.push({operation, requested, accepted});
  }
  function count(n) {
    if (!Number.isInteger(n) || n < 1 || n > 8) throw new Error('invalid transport count');
  }
  function unpack(bits, n) {
    count(n);
    let value = BigInt.asUintN(64, bits);
    const bytes = Buffer.alloc(n);
    for (let i = 0; i < n; i++) { bytes[i] = Number(value & 255n); value >>= 8n; }
    return bytes;
  }
  function pack(bytes) {
    let bits = 0n;
    for (let i = 0; i < bytes.length; i++) bits |= BigInt(bytes[i]) << BigInt(8 * i);
    return BigInt.asIntN(64, bits);
  }
  function handle(id, mode) {
    const h = handles.get(id);
    if (!h || h.mode !== mode) throw new Error('invalid file handle');
    return h;
  }
  const imports = {
    files_status_v1() { return config.status ?? (configured ? 0 : 1); },
    file_name_begin_v1(length) {
      pending = null;
      if (!configured || !Number.isInteger(length) || length < 1 || length > 255)
        throw new Error('invalid name preparation');
      if (config.beginStatus) return config.beginStatus;
      pending = {bytes: Buffer.alloc(length), offset: 0};
      return 0;
    },
    file_name_chunk_v1(bits, length) {
      const bytes = unpack(bits, length);
      if (!pending || pending.offset + length > pending.bytes.length)
        throw new Error('invalid name chunk');
      if (config.chunkStatus) { pending = null; return config.chunkStatus; }
      bytes.copy(pending.bytes, pending.offset); pending.offset += length;
      return 0;
    },
    file_open_v1(mode) {
      const name = pending; pending = null;
      if (!configured || !name || name.offset !== name.bytes.length || (mode !== 0 && mode !== 1))
        throw new Error('invalid open protocol');
      if (config.openResponse) { record('open', mode, config.openResponse[1]); return [...config.openResponse]; }
      const key = name.bytes.toString('hex');
      if (mode === 0 && !files.has(key)) { record('open', mode, 1); return [0, 1]; }
      if (mode === 1 && files.has(key)) { record('open', mode, 2); return [0, 2]; }
      if (nextHandle > 2147483647) { record('open', mode, 4); return [0, 4]; }
      const id = nextHandle++;
      if (mode === 1) files.set(key, Buffer.alloc(0));
      handles.set(id, {key, mode, offset: 0});
      record('open', mode, 0); return [id, 0];
    },
    file_read_v1(id, requested) {
      count(requested); const h = handle(id, 0), bytes = files.get(h.key);
      const response = reads.length ? reads.shift() : null;
      const answer = response === 'invalid' ? requested + 1 : response === null ? Math.min(requested, bytes.length - h.offset) : response;
      record('read', requested, answer);
      if (answer <= 0 || answer > requested) return [0n, answer];
      if (answer > bytes.length - h.offset) throw new Error('test response exceeds stored data');
      const value = pack(bytes.subarray(h.offset, h.offset + answer)); h.offset += answer;
      return [value, answer];
    },
    file_write_v1(id, bits, requested) {
      const bytes = unpack(bits, requested), h = handle(id, 1);
      const response = writes.length ? writes.shift() : null;
      const answer = response === 'invalid' ? requested + 1 : response === null ? requested : response;
      record('write', requested, answer);
      if (answer > 0 && answer <= requested) {
        files.set(h.key, Buffer.concat([files.get(h.key), bytes.subarray(0, answer)]));
        h.offset += answer;
      }
      return answer;
    },
    file_close_v1(id) {
      if (!handles.delete(id)) throw new Error('handle already consumed');
      record('close', 0, closeError ? 1 : 0);
      return closeError ? 1 : 0;
    },
  };
  return {
    imports,
    events: () => events.map(e => ({...e})),
    // Test inspection only; returns copies and cannot alter connection state.
    contents: name => {
      const bytes = files.get(Buffer.from(name, 'utf8').toString('hex'));
      return bytes === undefined ? undefined : Buffer.from(bytes);
    },
    openHandles: () => handles.size,
  };
}
module.exports = {createFiles};
