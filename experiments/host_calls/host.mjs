// Fixed-function comparison adapter, not a stable Cerune JS API.
export async function create(bytes, route) {
  if (!['vm', 'wat'].includes(route)) throw new TypeError('route');
  const now = () => performance.now();
  let begin = now();
  const module = await WebAssembly.compile(bytes);
  const compileMs = now() - begin;
  let diagnostic = '';
  begin = now();
  const instance = await WebAssembly.instantiate(module, {
    cerune: {write_error_byte(byte) { diagnostic += String.fromCharCode(byte); }}
  });
  const instantiateMs = now() - begin;
  const raw = instance.exports;
  begin = now();
  if (route === 'vm') raw.vm_prepare();
  const prepareMs = now() - begin;
  let closed = false;
  const fault = (phase, message, record) => Object.assign(new Error(message), {phase, record});
  function call(name, args, types) {
    if (closed) throw fault('closed', 'instance was stopped');
    if (args.length !== types.length) throw fault('arguments', 'incorrect argument count');
    for (let i=0; i<types.length; i++) {
      const x=args[i], t=types[i];
      if ((t==='f64' && typeof x!=='number') ||
          (t==='i32' && (typeof x!=='number' || !Number.isInteger(x) || x < -2147483648 || x > 2147483647 || Object.is(x,-0))) ||
          (t==='u64' && (typeof x!=='bigint' || x<0n || x>18446744073709551615n))) {
        throw fault('arguments', 'incorrect argument type or range');
      }
    }
    diagnostic = '';
    try {
      let value=raw[name](...args);
      if (route==='vm') {
        const n=raw.vm_error_len();
        for(let i=0;i<n;i++) diagnostic+=String.fromCharCode(raw.vm_error_byte(i));
      }
      if (diagnostic) throw fault('language', 'Cerune stopped', diagnostic.trim().replace(/^cerune: /,''));
      if (name==='echo_u64') value=BigInt.asUintN(64,value);
      return value;
    } catch(error) {
      closed=true;
      if (diagnostic) throw fault('language', 'Cerune stopped', diagnostic.trim().replace(/^cerune: /,''));
      throw fault(error instanceof WebAssembly.RuntimeError ? 'trap' : 'host', error.message);
    }
  }
  const api = {
    step: (...args)=>call('step',args,['f64','f64','f64']),
    checked: (...args)=>call('checked',args,['f64']),
  };
  if(route==='wat') {
    api.increment=(...args)=>call('increment',args,['i32']);
    api.echo_u64=(...args)=>call('echo_u64',args,['u64']);
  }
  return Object.freeze({
    api: Object.freeze(api),
    measurements: Object.freeze({bytes:bytes.byteLength,compileMs,instantiateMs,prepareMs}),
  });
}
