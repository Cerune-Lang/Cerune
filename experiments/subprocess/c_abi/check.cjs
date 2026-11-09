'use strict';
const fs=require('fs'), path=require('path'), {spawnSync}=require('child_process');
const [target,clang,output,supervisor]=process.argv.slice(2);
const windows=target==='x86_64-pc-windows-msvc';
if(!['x86_64-pc-windows-msvc','x86_64-unknown-linux-gnu'].includes(target) ||
    windows!==(process.platform==='win32') || !clang || !output || (!windows&&!path.isAbsolute(supervisor||''))) {
    throw Error('usage: node check.cjs <host-target> <clang> <new-output-directory> [absolute-linux-supervisor]');
}
const root=path.resolve(__dirname,'../../..');
const dir=path.resolve(output);
fs.mkdirSync(dir); // Existing observations are never overwritten.
const records=[];
function run(exe,args) {
    const r=spawnSync(exe,args,{cwd:root,encoding:'utf8',timeout:240000,maxBuffer:8*1024*1024});
    records.push({exe,args,status:r.status,signal:r.signal,stdout:r.stdout,stderr:r.stderr,error:r.error && r.error.message});
    fs.writeFileSync(path.join(dir,'commands.json'),JSON.stringify(records,null,2));
    if(r.error||r.status!==0) throw Error(JSON.stringify(records[records.length-1]));
    return r;
}
run('cargo',['rustc','--locked','--lib','--features','process-c-abi','--target',target,
    '--target-dir',path.join(dir,'build'),'--crate-type','staticlib','--','--print','native-static-libs']);
const archive=path.join(dir,'build',target,'debug',windows?'cerune_lang.lib':'libcerune_lang.a');
const probe=path.join(dir,windows?'probe.exe':'probe');
const libs=windows?['-lkernel32','-lntdll','-luserenv','-lws2_32','-ldbghelp','-ladvapi32','-lbcrypt']:
    ['-lgcc_s','-lutil','-lrt','-lpthread','-lm','-ldl','-lc'];
run(clang,['--target='+target,'-I'+path.join(root,'src/runtime/process/abi'),'-std=c11','-Wall','-Wextra','-Werror','-O0',
    path.join(__dirname,'probe.c'),archive,...libs,'-o',probe]);
const config={
    schema:'cerune-process-v1',platform:windows?'windows':'linux',
    linux_supervisor:windows?null:supervisor,
    limits:{arguments:32,argument_bytes:16384,stdin_bytes:1024,output_bytes:1024,
        timeout_ms:10000,cleanup_timeout_ms:5000},transfer_bytes:32768,
    targets:[{name:'fixture',executable:probe,directory:dir,environment:[]},
        {name:'missing',executable:path.join(dir,'absent-child'),directory:dir,environment:[]}]
};
const configPath=path.join(dir,'config.json');
fs.writeFileSync(configPath,JSON.stringify(config));
process.stdout.write(run(probe,[configPath]).stdout);
