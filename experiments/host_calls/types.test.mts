import {create} from './host.mjs';
// Type-check only; deliberately invalid calls are not executed.
export async function checkTypes(bytes: Uint8Array) {
  const vm = await create(bytes, 'vm');
  const wat = await create(bytes, 'wat');
  const value: number = vm.api.step(10, 2, 0.5);
  const next: number = wat.api.increment(41);
  const wide: bigint = wat.api.echo_u64(18446744073709551615n);
  // @ts-expect-error string is not a scalar number
  vm.api.step('10', 2, 0.5);
  // @ts-expect-error the VM adapter does not support i32 yet
  vm.api.increment(41);
  // @ts-expect-error u64 must use bigint
  wat.api.echo_u64(42);
  // @ts-expect-error exact argument count
  wat.api.step(1);
  return [value, next, wide];
}
