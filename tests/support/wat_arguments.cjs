'use strict';
// Explicit immutable input for the test host; no process arguments or memory access.
function createArguments(config) {
  if (!config || typeof config !== 'object') throw new Error('invalid argument host configuration');
  config = {...config};
  const values = (config.values || []).map((value, index) => {
    if (typeof value === 'string') {
      // Buffer.from would silently replace unpaired surrogates.
      for (let i=0; i<value.length; i++) {
        const c=value.charCodeAt(i);
        if (c>=0xd800 && c<=0xdbff) {
          const d=value.charCodeAt(++i);
          if (!(d>=0xdc00 && d<=0xdfff)) throw new Error(`arguments-v1 phase=capture code=invalid-unicode index=${index}`);
        } else if (c>=0xdc00 && c<=0xdfff) throw new Error(`arguments-v1 phase=capture code=invalid-unicode index=${index}`);
      }
      return Buffer.from(value,'utf8');
    }
    if (!Array.isArray(value) || value.some(b=>!Number.isInteger(b)||b<0||b>255)) throw new Error('invalid argument byte list');
    return Buffer.from(value);
  });
  let calls=0, execution=false;
  function access() { calls++; if (execution) throw new Error('argument host reread during execution'); }
  const imports = {
    arguments_count_v1() {
      access();
      if (config.configured===false) return [0n,1];
      return [BigInt(config.count ?? values.length), config.count_status ?? 0];
    },
    argument_length_v1(index) {
      access();
      if (index<0n || index>=BigInt(values.length)) return [0n,2];
      return [BigInt(config.length ?? values[Number(index)].length), config.length_status ?? 0];
    },
    argument_byte_v1(index,offset) {
      access();
      if (index<0n || index>=BigInt(values.length) || offset<0n || offset>=BigInt(values[Number(index)].length)) return [0,2];
      return [config.byte ?? values[Number(index)][Number(offset)], config.byte_status ?? 0];
    },
  };
  return {imports, calls:()=>calls, execution:()=>{execution=true;}};
}
module.exports={createArguments};
