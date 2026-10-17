const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {performance}=require('node:perf_hooks');
(async()=>{
  const {create}=await import('./host.mjs');
  const dir=process.argv[2], vmFile=process.argv[3];
  if(!dir || !vmFile) throw new Error('usage: compare.cjs output-directory vm.wasm');
  const watBytes=fs.readFileSync(path.join(dir,'exports.wasm'));
  const vmBytes=fs.readFileSync(vmFile);
  const expected=fs.readFileSync(path.join(dir,'failure.txt'),'utf8');
  const report={node:process.version,platform:process.platform,architecture:process.arch,calls:10000,routes:{}};
  const phase=p=>error=>error.phase===p;
  for(const [route,bytes] of [['vm',vmBytes],['wat',watBytes]]) {
    const {api,measurements}=await create(bytes,route);
    assert.equal(api.step(10,2,0.5),11);
    assert.equal(api.step(-1,4,0.25),0);
    assert.equal(api.checked(42),42);
    assert.ok(Number.isNaN(api.step(NaN,1,1)));
    assert.equal(api.step(Infinity,1,1),Infinity);
    assert.ok(Object.is(api.step(-0,-0,1),-0));
    for(const args of [[1],[1,2,3,4],['1',2,3],[{},2,3],[1n,2,3]]) {
      assert.throws(()=>api.step(...args),phase('arguments'));
    }
    assert.equal(api.step(10,2,0.5),11);
    if(route==='wat') {
      assert.equal(api.increment(41),42);
      assert.equal(api.increment(-2147483648),-2147483647);
      for(const x of [2147483648,-2147483649,0.5,-0,NaN,Infinity,'1',1n]) {
        assert.throws(()=>api.increment(x),phase('arguments'));
      }
      assert.equal(api.echo_u64(18446744073709551615n),18446744073709551615n);
      for(const x of [-1n,18446744073709551616n,1,'1']) assert.throws(()=>api.echo_u64(x),phase('arguments'));
    }
    for(let i=0;i<1000;i++) api.step(i%17,0.5,0.25);
    let sum=0,known=0;
    const start=performance.now();
    for(let i=0;i<report.calls;i++) sum+=api.step(i%17,0.5,0.25);
    const callsMs=performance.now()-start;
    for(let i=0;i<report.calls;i++) known+=(i%17)+0.125;
    assert.equal(sum,known);
    report.routes[route]={...measurements,callsMs};
    assert.throws(()=>api.checked(1.5),error=>error.phase==='language' && error.record===expected);
    assert.throws(()=>api.step(1,2,3),phase('closed'));
    assert.equal((await create(bytes,route)).api.step(1,2,3),7);
  }
  // Raw WebAssembly entry points coerce JS inputs; the checked adapter must not.
  const module=await WebAssembly.compile(watBytes);
  assert.ok(!WebAssembly.Module.exports(module).some(e=>e.kind==='memory'));
  assert.deepEqual(WebAssembly.Module.exports(module).map(e=>e.name).sort(),['checked','echo_f32','echo_u64','increment','step']);
  assert.deepEqual(WebAssembly.Module.imports(module),[{module:'cerune',name:'write_error_byte',kind:'function'}]);
  const raw=await WebAssembly.instantiate(module,{cerune:{write_error_byte(){}}});
  assert.equal(raw.exports.increment(4294967296),1);
  assert.equal(raw.exports.increment(1.5),2);
  assert.equal(raw.exports.echo_u64(-1n),-1n);
  assert.equal(raw.exports.echo_u64(18446744073709551616n),0n);
  assert.equal(raw.exports.echo_f32(0.1),Math.fround(0.1));
  assert.ok(Object.is(raw.exports.echo_f32(-0),-0));
  assert.ok(Number.isNaN(raw.exports.step(1)));
  fs.writeFileSync(path.join(dir,'measurements.json'),JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report,null,2));
})().catch(error=>{console.error(error);process.exitCode=1;});
