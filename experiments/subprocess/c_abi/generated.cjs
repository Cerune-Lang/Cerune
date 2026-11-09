'use strict';
const fs=require('fs'),path=require('path'),cp=require('child_process'),assert=require('assert');
const [target,clang,directory,archive,supervisor]=process.argv.slice(2);
const windows=target==='x86_64-pc-windows-msvc',root=path.resolve(__dirname,'../../..');
if(!['x86_64-pc-windows-msvc','x86_64-unknown-linux-gnu'].includes(target)||windows!==(process.platform==='win32'))throw Error('explicit matching target required');
const dir=path.resolve(directory);fs.mkdirSync(dir);
const logs=[];
function run(exe,args,ok=true){
 const r=cp.spawnSync(exe,args,{cwd:root,encoding:'utf8',timeout:120000,maxBuffer:4*1024*1024,
  env:Object.assign({},process.env,{CERUNE_PARENT_SENTINEL:'not inherited'})});
 logs.push({exe,args,status:r.status,signal:r.signal,stdout:r.stdout,stderr:r.stderr,error:r.error&&r.error.message});
 fs.writeFileSync(path.join(dir,'commands.json'),JSON.stringify(logs,null,2));
 if(r.error||(ok&&r.status!==0))throw Error(JSON.stringify(logs[logs.length-1]));return r;
}
run('cargo',['build','--locked','--bin','cerune']);
const cli=path.resolve(process.env.CARGO_TARGET_DIR||path.join(root,'target'),'debug',windows?'cerune.exe':'cerune');
const child=path.join(dir,windows?'child.exe':'child');
run('rustc',['--edition=2024',path.join(root,'tests/support/process_child.rs'),'-o',child]);
fs.writeFileSync(path.join(dir,'child-directory-marker'),'registered cwd');
const config={schema:'cerune-process-v1',platform:windows?'windows':'linux',linux_supervisor:windows?null:supervisor,
 limits:{arguments:32,argument_bytes:16384,stdin_bytes:1024,output_bytes:1024,timeout_ms:10000,cleanup_timeout_ms:5000},
 transfer_bytes:32768,targets:[{name:'tool',executable:child,directory:dir,environment:[['CERUNE_CHILD_VALUE','日本語\r\n']]}]};
const libs=windows?['-lkernel32','-lntdll','-luserenv','-lws2_32','-ldbghelp','-ladvapi32','-lbcrypt']:['-lgcc_s','-lutil','-lrt','-lpthread','-lm','-ldl','-lc'];
const example=fs.readFileSync(path.join(root,'examples/process_execution/registered.ceru'),'utf8');
const routes=[['run'],['run-mir'],['run-mir','--ssa'],['run-vm']];
function build(label,source,options=[],hook=''){
 const entry=path.join(dir,label+'.ceru'),c=path.join(dir,label+'.c'),exe=path.join(dir,label+(windows?'.exe':''));
 fs.writeFileSync(entry,source);
 run(cli,['emit-c',entry,'--target',target,'-o',c,...options]);
 let text=fs.readFileSync(c,'utf8');
 assert(text.includes('cerune_process_v1_prepare'));assert(text.includes('cerune_process_reserve('));
 // Check all common-IR array owners are released, including short/empty result capacity.
 text=text.replace('    int result = cerune_program();','    int result = cerune_program();\n    if (cerune_array_live_bytes) exit(95);');
 fs.writeFileSync(c,hook+text);
 run(clang,['--target='+target,'-std=c11','-D_CRT_SECURE_NO_WARNINGS','-Werror','-O0',c,path.resolve(archive),...libs,'-o',exe]);
 return {entry,exe};
}
let comparisons=0;
function compare(label,source,cfg,options=[],expected){
 const x=build(label,source,options),file=path.join(dir,label+'.json');
 if(cfg)fs.writeFileSync(file,JSON.stringify(cfg));
 const ca=cfg?['--cerune-runtime-v1','--process-config',file,'--']:[];
 const actual=run(x.exe,ca);
 assert.strictEqual(actual.stderr,'');
 if(expected)assert(actual.stdout.includes(expected),label+': '+actual.stdout);
 for(const route of routes){
  const r=run(cli,[route[0],x.entry,...route.slice(1),...options,...(cfg?['--process-config',file]:[])]);
  assert.strictEqual(actual.stdout,r.stdout,label+' / '+route.join(' '));assert.strictEqual(r.stderr,'');comparisons++;
 }
 return {x,file};
}
const baseline=compare('registered',example,config,[],fs.readFileSync(path.join(root,'examples/process_execution/registered.stdout'),'utf8'));
compare('unconfigured',example,null,[],'Unconfigured{}');
for(const [label,mutate,src,expected]of [
 ['transfer-budget',c=>c.transfer_bytes=0,example,'OutputBudget{}'],
 ['request-limit',c=>c.limits.stdin_bytes=0,example,'RequestLimit{}'],
 ['missing-child',c=>c.targets[0].executable=path.join(dir,'missing'),example,'NotFound{}'],
 ['unregistered',c=>c.targets=[],example,'Unregistered{}'],
 ['invalid-request',()=>{},example.replace('timeout_ms: 10000','timeout_ms: 0'),'InvalidRequest{}'],
 ['output-limit',()=>{},example.replace('stdout_limit: 32','stdout_limit: 2').replace('print(result.report.exit);','').replace('print(result.stderr);',''),'OutputLimit{}'],
 ['exact-output',()=>{},example.replace('stdout_limit: 32','stdout_limit: 4').replace('stderr_limit: 32','stderr_limit: 6'),'None{}'],
 ['zero-output',()=>{},example.replace('stdout_limit: 32','stdout_limit: 0').replace('print(result.report.exit);','').replace('print(result.stderr);',''),'OutputLimit{}'],
 ['u64-limit',()=>{},example.replace('stdout_limit: 32','stdout_limit: 18446744073709551615'),'RequestLimit{}'],
 ['language-reservation',()=>{},example.replace('stdout_limit: 32','stdout_limit: 512'),'OutputBudget{}']
]){
 const c=JSON.parse(JSON.stringify(config));mutate(c);
 compare(label,src,c,label==='language-reservation'?['--array-heap-limit','4096']:[],expected);
}
// A timeout can differ in which partial transfer completed; compare selected semantic fields only.
const sleep=example.replace(/arguments: array_copy\(\[.*?\]\)/,'arguments: array_copy(["sleep"])')
 .replace('timeout_ms: 10000','timeout_ms: 200');
const timed=build('timeout',sleep);
for(const [exe,args] of [[timed.exe,['--cerune-runtime-v1','--process-config',baseline.file,'--']],
 ...routes.map(route=>[cli,[route[0],timed.entry,...route.slice(1),'--process-config',baseline.file]])]){
 const r=run(exe,args);assert(r.stdout.includes('Timeout{}')&&r.stdout.endsWith('continued\n'));
}
compare('origins',example+'\nprint(result.report.origin);\n',config);
const copies=example.replace('result: types::Outcome','mut result: types::Outcome')+
 '\ncopy: types::Outcome = result;\nmut bytes: [u8] = result.stdout;\nbytes[0] = u8(1);\nprint(copy.stdout);\nprint(bytes);\nprint(result.stdout);\n';
compare('copies',copies,config);
const sequencing=example+
 '\nfn invoke(r: types::Request) -> u64 { print("called"); return process::run(r).report.stdin_accepted; }\n'+
 'print(false && invoke(request) == 4);\nprint(invoke(request) + invoke(request));\n';
compare('sequencing',sequencing,config);
for(let n=1;n<=4;n++){
 const hook='#define _GNU_SOURCE\n#include <stdlib.h>\nstatic unsigned attempts;\n'+
 'static void *fail_result(size_t n) { return ++attempts == '+n+' ? NULL : malloc(n); }\n#define CERUNE_PROCESS_RESULT_MALLOC fail_result\n';
 const x=build('allocation-'+n,example,[],hook);
 const r=run(x.exe,['--cerune-runtime-v1','--process-config',baseline.file,'--']);
 assert(r.stdout.includes('AllocationFailed{}')&&r.stdout.includes('Absent{}')&&r.stdout.endsWith('continued\n'));
}
for(const args of [
 ['--cerune-runtime-v1','--process-config'],
 ['--cerune-runtime-v1','--process-config',baseline.file],
 ['--cerune-runtime-v1','--process-config',baseline.file,'--process-config',baseline.file,'--'],
 ['--cerune-runtime-v1','--unknown','x','--']
]){
 const r=run(baseline.x.exe,args,false);assert.notStrictEqual(r.status,0);assert.strictEqual(r.stdout,'');
 assert(r.stderr.includes('cerune-process-startup-v1'));
}
const unmarked=run(baseline.x.exe,['--process-config',baseline.file]);assert(unmarked.stdout.includes('Unconfigured{}'));
const missingTarget=run(cli,['emit-c',baseline.x.entry],false);
assert.notStrictEqual(missingTarget.status,0);assert(missingTarget.stderr.includes('explicit --target'));


const mixed=build('mixed','import "std:env" as env;\nimport "std:fs" as fs;\n'+example+
 '\nprint(env::arg_count());\nprint(env::arg(0));\nprint(fs::read_text("child-directory-marker", 100));\n');
const argv=['--cerune-runtime-v1','--process-config',baseline.file,'--file-root',dir,'--','--cerune-runtime-v1'];
const mixedC=run(mixed.exe,argv);
for(const route of routes){
 const r=run(cli,[route[0],mixed.entry,...route.slice(1),'--process-config',baseline.file,'--file-root',dir,'--','--cerune-runtime-v1']);
 assert.strictEqual(mixedC.stdout,r.stdout); comparisons++;
}
console.log('Startup coexistence: file-root, process-config and literal marker argument: ok');

for(const [label,text]of [['empty',''],['invalid','{'],['platform',JSON.stringify(Object.assign({},config,{platform:'other'}))]]){
 const file=path.join(dir,label+'-config.json');fs.writeFileSync(file,text);
 const r=run(baseline.x.exe,['--cerune-runtime-v1','--process-config',file,'--'],false);
 assert.notStrictEqual(r.status,0);assert.strictEqual(r.stdout,'');assert(r.stderr.includes('cerune-process-'));
}
const other=windows?'x86_64-unknown-linux-gnu':'x86_64-pc-windows-msvc';
const wrong=path.join(dir,'wrong-target.c');
run(cli,['emit-c',baseline.x.entry,'--target',other,'-o',wrong]);
const mismatch=run(clang,['--target='+target,'-std=c11','-D_CRT_SECURE_NO_WARNINGS','-fsyntax-only',wrong],false);
assert.notStrictEqual(mismatch.status,0);assert(mismatch.stderr.includes('Cerune_process_runtime_target_mismatch'));
console.log('Generated C: '+comparisons+' executor comparisons, allocation failures, startup checks: ok');
