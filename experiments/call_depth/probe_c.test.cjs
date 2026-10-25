'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {classify} = require('./probe_c.cjs');
test('C probe requires exit status, exact output, reason, origin and all ordered frames', () => {
  const expected = {kind:'depth-limit',limit:1,output:'before\n'};
  const run = {status:1,signal:null,stdout:'before\n',stderr:'call-v1 reason=depth-limit limit=1 function=2 node=3 file=0 bytes=4..8\ncall-v1 frame=0 function=1 origin=host\n'};
  assert.equal(classify(run,expected).matched,true);
  for (const other of [
    {...run,status:0}, {...run,signal:'SIGSEGV'}, {...run,error:{code:'ETIMEDOUT'}},
    {...run,stderr:''}, {...run,stdout:'before\nafter\n'},
    {...run,stderr:run.stderr.replace('limit=1','limit=2')},
    {...run,stderr:run.stderr.replace('frame=0','frame=1')},
    {...run,stderr:run.stderr.split('\n')[0]},
    {...run,stderr:run.stderr+'extra\n'},
    {...run,stderr:run.stderr.replace('origin=host','untracked')},
  ]) assert.notEqual(classify(other,expected).matched,true);
  assert.equal(classify({...run,stderr:run.stderr.replace(/\n/g,'\r\n')},expected).matched,true);
});
test('C probe never accepts physical termination as successful completion', () => {
  const expected={kind:'completed',output:'before\n32\nafter\n'};
  const run={status:0,stdout:expected.output,stderr:''};
  assert.equal(classify(run,expected).matched,true);
  for(const other of [{...run,status:1},{...run,signal:'SIGSEGV'},{...run,stderr:'stack overflow'},{...run,stdout:'before\n'}]) assert.notEqual(classify(other,expected).matched,true);
});