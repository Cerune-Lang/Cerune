// Types for the fixed experimental adapter, not generated public bindings.
export interface Calls {
  step(position: number, velocity: number, delta: number): number;
  checked(value: number): number;
}
export interface WatCalls extends Calls {
  increment(value: number): number;
  echo_u64(value: bigint): bigint;
}
export interface Probe<T> {
  readonly api: Readonly<T>;
  readonly measurements: Readonly<{bytes: number; compileMs: number; instantiateMs: number; prepareMs: number}>;
}
export function create(bytes: Uint8Array, route: 'vm'): Promise<Probe<Calls>>;
export function create(bytes: Uint8Array, route: 'wat'): Promise<Probe<WatCalls>>;
