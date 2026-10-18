import {createLibrary, metadata, CeruneHostError, type HostErrorPhase} from './library.mjs';
const library = await createLibrary(new Uint8Array());
const result: number = library.functions.step(10, 2, 0.5);
library.functions.sum_to(4);
library.functions.checked(result);
const phase: HostErrorPhase = new CeruneHostError('language', 'stopped').phase;
void phase;
// @ts-expect-error exact argument count
library.functions.step(10);
// @ts-expect-error no implicit string conversion
library.functions.checked('1.5');
// @ts-expect-error public i32 takes number, not bigint
library.functions.sum_to(4n);
// @ts-expect-error no undeclared export
library.functions.missing();
// @ts-expect-error immutable callable catalog
library.functions.step = () => 1;
// @ts-expect-error immutable observation metadata
metadata.exports[0].origin.file = 5;
// @ts-expect-error private raw instance
library.instance;
// @ts-expect-error bytes must be explicit
await createLibrary('library.wasm');
