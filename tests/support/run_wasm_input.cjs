// 行入力契約の試験host。同期file/pipeを1バイトずつ読み、memoryにはアクセスしません。
const fs = require('node:fs');
const floatFormat = require('./wat_float_format.cjs');
const config = JSON.parse(process.env.CERUNE_TEST_WAT_INPUT || '{}');
const events = [], diagnostics = [];
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
for (const name of config.omit || []) delete imports.cerune[name];
WebAssembly.instantiate(fs.readFileSync(process.argv[2]),imports).then(({instance})=>{
  if (Object.keys(instance.exports).join(',')!=='main') throw new Error('unexpected public export');
  try { instance.exports.main(); }
  catch(error) {
    if (config.retry_after_failure) {
      const previous=reads;
      try { instance.exports.main(); throw new Error('failed instance resumed'); }
      catch(second) { if (!(second instanceof WebAssembly.RuntimeError)) throw second; }
      if (reads!==previous) throw new Error('read after failed instance restart');
    }
    throw error;
  }
  if (diagnostics.length) throw new Error('diagnostic without a trap');
}).catch(error=>{
  fs.writeSync(2,Buffer.from(diagnostics));
  if (!(error instanceof WebAssembly.RuntimeError) || error.message!=='unreachable' || !diagnostics.length) console.error(error.message);
  process.exitCode=1;
}).finally(()=>{
  if (process.env.CERUNE_TEST_WAT_INPUT_TRACE) fs.writeFileSync(process.env.CERUNE_TEST_WAT_INPUT_TRACE,JSON.stringify({consumed,reads,events}));
});
