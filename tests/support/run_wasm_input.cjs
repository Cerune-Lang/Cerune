// 行入力契約の試験host。同期file/pipeを1バイトずつ読み、memoryにはアクセスしません。
const fs = require('node:fs');
const floatFormat = require('./wat_float_format.cjs');
const config = JSON.parse(process.env.CERUNE_TEST_WAT_INPUT || '{}');
const events = [], diagnostics = [];
if (process.env.CERUNE_TEST_WAT_ARGUMENTS !== undefined && process.env.CERUNE_TEST_WAT_ARGUMENTS_FILE !== undefined) throw new Error('duplicate argument input');
const argumentConfig = process.env.CERUNE_TEST_WAT_ARGUMENTS_FILE === undefined ? process.env.CERUNE_TEST_WAT_ARGUMENTS : fs.readFileSync(process.env.CERUNE_TEST_WAT_ARGUMENTS_FILE,'utf8');
const argumentHost = argumentConfig === undefined ? null : require('./wat_arguments.cjs').createArguments(JSON.parse(argumentConfig));
const fileConfig = process.env.CERUNE_TEST_WAT_FILES_FILE === undefined ? null : JSON.parse(fs.readFileSync(process.env.CERUNE_TEST_WAT_FILES_FILE, 'utf8'));
const fileHost = fileConfig === null ? null : require('./wat_files.cjs').createFiles(fileConfig);
const connection = process.platform === 'win32' && require('node:tty').isatty(0) ? 2 : 0;
let consumed = 0, reads = 0, interrupted = false, interruptedAfter = false, failed = false;
const imports = { cerune: {
  stdin_status_v1() { events.push({operation:'connection'}); return config.connection ?? connection; },
  read_stdin_byte_v1() {
    if (failed) throw new Error('read after input failure');
    reads++; events.push({operation:'read'});
    if (config.interrupt && !interrupted) { interrupted = true; return [0,2]; }
    if (config.interrupt && consumed && !interruptedAfter) { interruptedAfter = true; return [0,2]; }
    if (config.fault && consumed === (config.fault_at ?? 2)) {
      failed = true;
      if (config.fault === 'byte') return [256,0];
      if (config.fault === 'negative-byte') return [-1,0];
      if (config.fault === 'status') return [0,99];
      if (config.fault === 'nonzero-eof') return [1,1];
      if (config.fault === 'nonzero-interrupt') return [1,2];
      return [0,3];
    }
    const byte = Buffer.alloc(1);
    try {
      const n = fs.readSync(0,byte,0,1,null);
      if (!n) return [0,1];
      consumed++; return [byte[0],0];
    } catch (error) {
      if (error.code === 'EINTR') return [0,2];
      failed = true; return [0,3];
    }
  },
  write_stdout_v1(bits,count) {
    argumentHost?.execution();
    events.push({operation:'write',requested:count});
    const bytes=Buffer.alloc(8); bytes.writeBigInt64LE(bits);
    try { return fs.writeSync(1,bytes,0,count); }
    catch (error) { return error.code==='EINTR' ? -2 : -1; }
  },
  flush_stdout_v1() { events.push({operation:'flush'}); return config.flush_failure ? -1 : 0; },
  format_f32_v1: (value,offset)=>floatFormat.chunk(value,9,offset),
  format_f64_v1: (value,offset)=>floatFormat.chunk(value,17,offset),
  write_error_byte(value) {
    if (!Number.isInteger(value) || value<0 || value>127) throw new Error('invalid diagnostic byte');
    diagnostics.push(value);
  },
}};

if (argumentHost) Object.assign(imports.cerune, argumentHost.imports);
if (fileHost) Object.assign(imports.cerune, fileHost.imports);
const memoryEvents = [];
if (process.env.CERUNE_TEST_WAT_MEMORY_TRACE) {
  imports.probe = { memory(phase, logical, capacity, free, owners, pages) {
    memoryEvents.push({phase,logical:Number(logical),capacity:Number(capacity),free:Number(free),owners,pages});
  }};
}
const storageEvents = [];
if (process.env.CERUNE_TEST_WAT_STORAGE_TRACE) {
  imports.probe = {...imports.probe, storage(...values) {
    storageEvents.push(values.map(Number).join(','));
  }};
}
for (const name of config.omit || []) delete imports.cerune[name];
WebAssembly.instantiate(fs.readFileSync(process.argv[2]),imports).then(({instance})=>{
  if (Object.keys(instance.exports).join(',')!=='main') throw new Error('unexpected public export');
  try { instance.exports.main(); }
  catch(error) {
    if (config.retry_after_failure) {
      const previous=reads;
      const previousArguments=argumentHost?.calls();
      try { instance.exports.main(); throw new Error('failed instance resumed'); }
      catch(second) { if (!(second instanceof WebAssembly.RuntimeError)) throw second; }
      if (reads!==previous) throw new Error('read after failed instance restart');
      if (argumentHost?.calls()!==previousArguments) throw new Error('argument reread after failed restart');
    }
    throw error;
  }
  if (config.retry_after_success) {
    const previous=argumentHost?.calls(), eventCount=events.length;
    let trapped=false;
    try { instance.exports.main(); } catch(error) { if (!(error instanceof WebAssembly.RuntimeError)) throw error; trapped=true; }
    if (!trapped || argumentHost?.calls()!==previous || events.length!==eventCount) throw new Error('completed argument instance resumed');
  }
  if (diagnostics.length) throw new Error('diagnostic without a trap');
  if (process.env.CERUNE_TEST_WAT_MEMORY_TRACE) {
    const assert = require('node:assert/strict');
    const lines = memoryEvents.filter(e => e.phase === 1);
    const releases = memoryEvents.filter(e => e.phase === 2 && e.owners === 0);
    assert.equal(lines.length, 24);
    assert.equal(memoryEvents.filter(e => e.phase === 2).length, 24);
    assert.equal(releases.length, 3);
    const block = lines[0].capacity;
    assert.ok(block === 8 || block === 1048576);
    for (let i=0; i<lines.length; ++i) {
      const e=lines[i], owners=i%8+1;
      assert.equal(e.logical, owners*2);
      assert.equal(e.owners, owners);
      assert.equal(e.capacity, owners*block);
      if (i>=8) {
        assert.equal(e.free, (8-owners)*block);
        assert.equal(e.pages, lines[7].pages);
      }
    }
    for (const e of releases) {
      assert.equal(e.logical,0);
      assert.equal(e.capacity,0);
      assert.equal(e.free,8*block);
      assert.equal(e.pages,lines[7].pages);
    }
    fs.writeFileSync(process.env.CERUNE_TEST_WAT_MEMORY_TRACE,JSON.stringify(memoryEvents,null,2));
  }

}).catch(error=>{
  fs.writeSync(2,Buffer.from(diagnostics));
  if (!(error instanceof WebAssembly.RuntimeError) || error.message!=='unreachable' || !diagnostics.length) console.error(error.message);
  process.exitCode=1;
}).finally(()=>{
  if (fileHost && process.env.CERUNE_TEST_WAT_FILES_TRACE) fs.writeFileSync(process.env.CERUNE_TEST_WAT_FILES_TRACE, JSON.stringify({
    events: fileHost.events(), handles: fileHost.openHandles(),
    contents: Object.fromEntries((fileConfig.inspect || []).map(name => [name, fileHost.contents(name) === undefined ? null : [...fileHost.contents(name)]])),
  }));
  if (process.env.CERUNE_TEST_WAT_STORAGE_TRACE) fs.writeFileSync(process.env.CERUNE_TEST_WAT_STORAGE_TRACE,storageEvents.join('\n')+'\n');
  if (process.env.CERUNE_TEST_WAT_INPUT_TRACE) fs.writeFileSync(process.env.CERUNE_TEST_WAT_INPUT_TRACE,JSON.stringify({consumed,reads,events}));
});
