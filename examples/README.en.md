# Cerune examples

[日本語](README.md)

Normal examples are the `.ceru` files in this directory.

| Location | Purpose |
| --- | --- |
| This directory | Standalone programs; included in the batch runner |
| [dynamic_arrays](dynamic_arrays/README.en.md) | Dynamic array copies, ranges, nesting; IR/MIR/VM/C/LLVM/QBE/WAT/ASM/native-object execution |
| [io_protocol](io_protocol/README.en.md) | Output/evaluation order, explicit flush, and scripted input; distinguish EOF, empty lines, and failure (no stdin) |
| [ir_stages](ir_stages/README.en.md) | Execute short circuits, loops, and calls; observe HIR→MIR→LIR→Native lowering and target-specific ABI |
| [modules](modules/README.en.md) | Imports, single-file comparison, and expected failures |
| [runtime_failures](runtime_failures/README.en.md) | Five expected stops: prior output and failing expressions |
| [source_files](source_files/README.en.md) | Rust API examples: parse type/function files separately and compare origins; no imports |

## Execute IR directly

Run `cargo run --quiet -- run-ir examples/ir_execution.ceru` to try functions, array copies, iteration, and string concatenation. `run` is the same execution route; use `run-vm` to compare with the VM. `emit-ir` shows the shared program.

| Example | Output |
| --- | --- |
| [ir_execution.ceru](ir_execution.ceru) | `Hello, IR` → `Hello, 世界` → `["Cerune", "世界"]` |

Other normal examples also work with `run-ir`. Batch scripts use direct IR execution through `run`; `cargo test --test examples --test ir_executor` compares both routes and expected values.

## Run all examples

Run from the repository root. Subdirectories are outside the batch runner.

| Environment | Run all | Select examples / reuse a build |
| --- | --- | --- |
| PowerShell | `.\scripts\run-examples.ps1` | `-Pattern "matrix*.ceru"` / `-SkipBuild` |
| WSL / Bash | `bash scripts/run-examples.sh` | `--pattern 'matrix*.ceru'` / `--skip-build` |

The runner shows names, output, exit status, and a summary. WSL needs its own Rust environment. Bash builds into `target/unix` by default, or `CARGO_TARGET_DIR` when set; reuse requires a build in the same location.

To run one example or inspect its representations, replace the filename below:

```sh
cargo run --quiet -- run examples/linear_regression.ceru
cargo run --quiet -- emit-ir examples/linear_regression.ceru
cargo run --quiet -- emit-bytecode examples/linear_regression.ceru
cargo run --quiet -- emit-c examples/linear_regression.ceru
```

## Find examples by type

The tables cover numeric ranges and representative uses. `infer` requests type inference rather than naming a value type; `void` means a function returns no value. See [floating_point.ceru](floating_point.ceru) and [functions.ceru](functions.ceru).

### Signed integers: negative and positive values

| Type | Range | Example | What to observe |
| --- | --- | --- | --- |
| `i8` | −128 through 127 | [sensor_calibration.ceru](sensor_calibration.ceru) | widening a small negative correction to `i16` before arithmetic |
| `i16` | −32768 through 32767 | [sensor_calibration.ceru](sensor_calibration.ceru) | signed readings, widened to `i32` for aggregation |
| `i32` | −2147483648 through 2147483647 | [maximum_subarray.ceru](maximum_subarray.ceru) | adding and comparing positive and negative changes |
| `i64` | −9223372036854775808 through 9223372036854775807 | [integer_limits.ceru](integer_limits.ceru) | minimum, maximum, and a check before overflow |

### Unsigned integers: nonnegative values and bits

| Type | Range | Example | What to observe |
| --- | --- | --- | --- |
| `u8` | 0 through 255 | [color_blending.ceru](color_blending.ceru), [bit_flags.ceru](bit_flags.ceru) | color channels and setting, clearing, or toggling eight bits |
| `u16` | 0 through 65535 | [color_blending.ceru](color_blending.ceru) | widening `u8` colors before addition, then converting the average back |
| `u32` | 0 through 4294967295 | [population_statistics.ceru](population_statistics.ceru) | values around three billion, widened to `i64` for aggregation |
| `u64` | 0 through 18446744073709551615 | [u64_values.ceru](u64_values.ceru), [packet_counter.ceru](packet_counter.ceru) | maximum, high bit, unsigned comparison/division, functions, arrays, copies, and exact conversions |

Integer overflow stops execution; values never wrap. Convert explicitly with `i64(value)` or `convert<i64>(value)`; [integer_conversions.ceru](integer_conversions.ceru) compares both spellings. Untyped integers and array indices use `i64`. Widths describe numeric ranges; generated targets currently use 64-bit storage even for small integers. See [u64 representations](../docs/design/u64.en.md).

### Floating point: fractions and precision

| Type | Representation | Example | What to observe |
| --- | --- | --- | --- |
| `f32` | 32-bit floating point | [floating_point.ceru](floating_point.ceru), [logistic_map.ceru](logistic_map.ceru) | rounding and computation differences from `f64` |
| `f64` | 64-bit floating point | [floating_point.ceru](floating_point.ceru), [small_values.ceru](small_values.ceru) | small-value display and arithmetic rounding; floats without type information default to `f64` |

[measurement_statistics.ceru](measurement_statistics.ceru) and [normalized_histogram.ceru](normalized_histogram.ceru) convert between integers and floats. `convert<T>` succeeds only when it preserves the value. `trunc<T>` discards the fractional part toward zero. These are separate from ordinary floating-point arithmetic rounding.

| Conversion example | What to observe |
| --- | --- |
| [truncating_conversions.ceru](truncating_conversions.ceru) | `3.7 → 3`, `-3.7 → -3`, `u8` boundaries, `u64`, constants and type arguments |
| [truncating_evaluation_order.ceru](truncating_evaluation_order.ceru) | Arguments evaluated once from left to right, skipped conversions through short-circuiting, independent array copies |
| [rounding_conversions.ceru](rounding_conversions.ceru) | Compare all five rounding modes at positive/negative midpoints |
| [rounding_quantities.ceru](rounding_quantities.ceru) | Floor for cells, ceil for required boxes; observe constant expressions and results |
| [saturating_conversions.ceru](saturating_conversions.ceru) | Clamp colors to u8; NaN, infinities, and type endpoints |
| [rounding_evaluation_order.ceru](rounding_evaluation_order.ceru) | Preserve argument order, short-circuiting, and array copies |

[Observe generation](../docs/design/rounding-conversions.en.md#try-it) from the same source through IR, bytecode, and annotated LLVM/ASM.

### Booleans and strings

| Type | Values | Example | What to observe |
| --- | --- | --- | --- |
| `bool` | `true` / `false` | [boolean_comparisons.ceru](boolean_comparisons.ceru), [short_circuit.ceru](short_circuit.ceru) | comparison, negation, and skipped evaluation through short-circuiting |
| `string` | immutable UTF-8 content | [string_values.ceru](string_values.ceru), [string_byte_length.ceru](string_byte_length.ceru), [string_concat.ceru](string_concat.ceru) | Japanese text, equality, byte length, concatenation, and independent copies after reassignment |

[string_origins.ceru](string_origins.ceru) traces string operations through Cerune IR and annotated LLVM. String content is immutable; mutable bindings can be reassigned.

### Arrays and products: combining types

| Type form | Example | What to observe |
| --- | --- | --- |
| Dynamic array `[T]` | [copy.ceru](dynamic_arrays/copy.ceru), [nested.ceru](dynamic_arrays/nested.ceru), [repeat.ceru](dynamic_arrays/repeat.ceru) | Empty ranges, runtime repetition, independent copies, nesting, functions, enums, iteration |
| Fixed array `[T; N]` | [fixed_arrays.ceru](fixed_arrays.ceru), [bubble_sort.ceru](bubble_sort.ceru) | indexing, element updates, and independent copies |
| Product `type Point { ... }` | [product-point.ceru](product-point.ceru), [product_arrays.ceru](product_arrays.ceru) | fields, defaults, and arrays of products |
| Nested arrays and products | [function_values.ceru](function_values.ceru), [u64_values.ceru](u64_values.ceru), [string_lookup.ceru](string_lookup.ceru) | combining numbers or strings and passing values to functions |
| Fixed capacity and used length | [bounded_sequence.ceru](bounded_sequence.ceru) | Empty/full/invalid lengths; a capacity-four value, not a dynamic array |
| Ownership across calls | [owned_arguments.ceru](owned_arguments.ceru) | Independent fixed arrays, argument order, unused arguments, and returned-value lifetime; loops within a 22-byte string budget |
| Reads and value copies | [borrowed_reads.ceru](borrowed_reads.ceru) | Array display/comparison, temporary projections, independent updates; runs with a 30-byte string budget |
| Nested copies and lifetime | [array_copy_lifetimes.ceru](array_copy_lifetimes.ceru) | Dynamic strings inside fixed arrays: survival after return, range copies, independent updates |
| Range copy into fixed capacity | [array_window.ceru](array_window.ceru) | Empty/invalid ranges, evaluation order, short circuiting, independent source updates; not a borrowed slice |
| Aggregate equality | [aggregate_comparison.ceru](aggregate_comparison.ceru) | Arrays/products/sums, operand order, constants and generic functions |
| Aggregate display | [aggregate_display.ceru](aggregate_display.ceru) | Nested values, maximum u64, quoted strings, preserved NUL/CR/LF |

### Sum types: values for each alternative

| Example | Type and behavior |
| --- | --- |
| [sum_lookup.ceru](sum_lookup.ceru) | `enum Lookup`: found string/u64 payload or a missing entry |
| [sum_divide.ceru](sum_divide.ceru) | `enum Division`: success/zero-divisor/overflow values allow processing to continue |
| [sum_values.ceru](sum_values.ceru) | Array/product payloads, copies, construction order, reassignment inside an arm |
| [modules/sum_lookup.ceru](modules/sum_lookup.ceru) | Imported public enum, construction, exhaustive branching |
| [match_values.ceru](match_values.ceru) | Match results, guards, type arguments, constant array lengths |
| [match_guards.ceru](match_guards.ceru) | One subject evaluation, ordered/skipped guards, independent copies, continue/break |
| [modules/aggregate_match.ceru](modules/aggregate_match.ceru) | Compare/display public enums/constants; match expression in a separate file |

[Comparison, display, branching, and generation rules](../docs/design/aggregate-values.en.md).

### Compile-time constants

| Example | What it demonstrates |
| --- | --- |
| [constants.ceru](constants.ceru) | u64 limits, strings, f32/struct/array constants, independent copies, and short circuiting |
| [modules/constants.ceru](modules/constants.ceru) | Import public settings computed using a private calibration constant |

### Product update expressions

| Example | Checks |
| --- | --- |
| [product_update.ceru](product_update.ceru) | Update u64, strings, arrays, and nested structs while preserving originals |
| [product_update_order.ceru](product_update_order.ceru) | Evaluate the base once, preserve field order, skip inherited defaults, short circuiting and loops |
| [modules/product_update.ceru](modules/product_update.ceru) | Update using imported types and functions |

[Update semantics](../docs/design/product-updates.en.md).

### Combining many arguments

[function_arguments.ceru](function_arguments.ceru) compares a nested seven-argument call with an eleven-argument value transfer.

| Type | What to observe |
| --- | --- |
| `i64` | `observed(1)` through `observed(7)` execute once in order; the nested call preserves the sum `28` |
| `u64` | The maximum value and high bit survive in arguments beyond the fourth |
| `f32`, `f64`, `bool` | Mixing with integers preserves `1.5`, `2.5`, and `true`; comparisons print `true` |
| `string` | Japanese text, NUL, CR, and LF pass through unchanged |
| Arrays and products | Updating a callee's array copy leaves the original unchanged; product results remain independent |

Argument counts and types must match the declaration; there is no fixed count limit. See [function placement and validation](../docs/design/functions.en.md).

## Basics and control flow

| Example | Demonstrates |
| --- | --- |
| [hello.ceru](hello.ceru) | a first example: name two integers, add them, and show the result with `print` |
| [greeting.ceru](greeting.ceru) | Pass a `string` name to a function and build a greeting with `concat` |
| [shopping_total.ceru](shopping_total.ceru) | Sum a `u64` price array with `for … in` and print the total and change |
| [countdown.ceru](countdown.ceru) | Use a mutable `i64` and `while` to print 3, 2, 1 and finish |
| [short_circuit.ceru](short_circuit.ceru) | combining conditions with `&&`/`\|\|` to skip unnecessary division, indexing, and function calls |
| [conditional.ceru](conditional.ceru) | `if` / `else` and scope |
| [loop_control.ceru](loop_control.ceru) | `while`, `break`, and `continue` |
| [for_sum.ceru](for_sum.ceru) | `for` and assignment as its start statement |
| [generic_functions.ceru](generic_functions.ceru) | Reuse aggregation/copying with explicit types and lengths: strings, u64, products, sums |
| [generic_evaluation_order.ceru](generic_evaluation_order.ceru) | Generic forwarding, argument evaluation order, short-circuiting |
| [functions.ceru](functions.ceru) | typed functions, parameters, results, and `void` functions |

## Data structures

| Example | Demonstrates |
| --- | --- |
| [constant_array_lengths.ceru](constant_array_lengths.ceru) | Shared fixed sizes, forward references, type identity, functions, strings/u64/enums, and copies |
| [modules/constant_array_lengths.ceru](modules/constant_array_lengths.ceru) | Imported dimensions and private constants used inside public types |
| [array_iteration.ceru](array_iteration.ceru) | `for … in` aggregation/search, indices, early exit, and captured copies |
| [array_iteration_values.ceru](array_iteration_values.ceru) | Iterate strings/u64/nested arrays/products/sums; mutate per-iteration copies |
| [array_length.ceru](array_length.ceru) | Aggregate using element counts; constants, nesting, arrays of strings/u64/products/sums, and copies |
| [array_length_order.ceru](array_length_order.ceru) | Evaluate calls/elements once despite a known length; short circuiting and loop conditions |
| [ring_buffer.ceru](ring_buffer.ceru) | cycling a storage position with `%` to keep the latest four values and their average |
| [string_lookup.ceru](string_lookup.ceru) | linear search by a string key in an array of structs, returning display text or a default |
| [product-point.ceru](product-point.ceru) | grouping point coordinates in a struct, with field defaults and access |
| [fixed_arrays.ceru](fixed_arrays.ceru) | indexing, summation, and linear search in fixed arrays, and independent values after copying |
| [product_arrays.ceru](product_arrays.ceru) | arrays of structs, nearest-point search, and array value copies |
| [function_values.ceru](function_values.ceru) | passing and returning structs and nested fixed arrays as values |
| [packet_counter.ceru](packet_counter.ceru) | Pass a high-bit u64 sequence, u8 flags, and immutable text in a product; run with the Cerune encoder |
| [native_values.ceru](native_values.ceru) | Pass u64, integers, floats, strings, and structures through four mixed arguments; follow execution and machine code on Windows/Linux |

## Numerical computation

| Example | Demonstrates |
| --- | --- |
| [measurement_statistics.ceru](measurement_statistics.ceru) | computing a fractional mean and variance from integer samples, then storing them exactly as `f32` |
| [normalized_histogram.ceru](normalized_histogram.ceru) | converting integer counts to probabilities and recovering the original counts |
| [square_root.ceru](square_root.ceru) | unrolled square-root approximation steps |
| [while_square_root.ceru](while_square_root.ceru) | repeated square-root approximation with `while` |
| [logistic_map.ceru](logistic_map.ceru) | result differences between `f32` and `f64` computation |
| [matrix_vector_product.ceru](matrix_vector_product.ceru) | multiplying a 3-by-3 matrix by a three-element vector using nested fixed arrays |
| [matrix_composition.ceru](matrix_composition.ceru) | passing structs and nested arrays through functions to compose 2-by-2 matrices and transform a vector |
| [population_statistics.ceru](population_statistics.ceru) | widening large `u32` values to `i64` for aggregation, returning average and maximum in a struct |
| [heat_diffusion.ceru](heat_diffusion.ceru) | four steps of heat diffusion along a rod, computing new temperatures from the previous array |
| [linear_regression.ceru](linear_regression.ceru) | learning a line from five points while observing slope, intercept, and loss |

## Algorithms

| Example | Demonstrates |
| --- | --- |
| [color_blending.ceru](color_blending.ceru) | widening `u8` color channels to `u16` before adding and averaging them |
| [sensor_calibration.ceru](sensor_calibration.ceru) | correcting `i16` readings with an `i8` offset, then aggregating in `i32` |
| [maximum_subarray.ceru](maximum_subarray.ceru) | maximum sum of a contiguous `i32` subarray, converting a `u32` position to an index |
| [subset_sum_bits.ceru](subset_sum_bits.ceru) | finding reachable sums together using shifts and bitwise OR |
| [euclidean_gcd.ceru](euclidean_gcd.ceru) | greatest common divisor with the Euclidean algorithm |
| [fibonacci.ceru](fibonacci.ceru) | the Fibonacci sequence and update order for multiple values |
| [factorial.ceru](factorial.ceru) | factorial with `for` |
| [collatz.ceru](collatz.ceru) | Collatz steps and conditional state transitions |
| [prime_check.ceru](prime_check.ceru) | trial-division primality testing and early exit |
| [integer_square_root.ceru](integer_square_root.ceru) | integer square root with binary search |
| [exponentiation_by_squaring.ceru](exponentiation_by_squaring.ceru) | exponentiation by squaring |
| [pythagorean_triples.ceru](pythagorean_triples.ceru) | Pythagorean triples with nested `for` loops |
| [bubble_sort.ceru](bubble_sort.ceru) | in-place bubble sort by swapping elements in a `mut` fixed array |
| [xor_neural_network.ceru](xor_neural_network.ceru) | XOR inference with a tiny neural network and fixed-array weights |
| [coin_change.ceru](coin_change.ceru) | dynamic programming for minimum coin counts, followed by reconstruction of the chosen coins |
| [shortest_paths.ceru](shortest_paths.ceru) | all-pairs shortest paths by gradually allowing more intermediate towns |

## Reading output and intermediate results

`\0`, `\r`, and `\n` below denote NUL, CR, and LF. Arrows and commas separate output lines. Japanese comments in each source describe their order.

| Example | Output and interpretation |
| --- | --- |
| `u64_values.ceru` | Starts with `u64 → 18446744073709551615 → 9223372036854775808 → 0`. Reassignment preserves copies; the high bit stays positive |
| `sum_lookup.ceru` | `空\0\r\n\n6\n未登録\n`. IR shows variant tags, the subject copy, and branches |
| `constants.ceru` | Starts with `128 → 18446744073709551615 → 9` (string byte length). Updating an array copy to `99` leaves both struct constants at `10`. IR shows initializers and evaluated values |
| `product_update.ceru` | Original/updated sequence: `9223372036854775808 / 9223372036854775809`; array element: `10 / 99` |
| `product_update_order.ceru` | Starts with `base → default → replacement → 2`. Defaults run only when creating the base, never on updates or copies |
| `function_arguments.ceru` | Starts with `引数の評価順`, `1` through `7`, then `28` |
| `string_origins.ceru` | `日本語\0\ntrue\nfalse\n`; no `skipped`. Compare IR and annotated LLVM: equality `#7` and short circuit `#14` lead to calls/branches ([walkthrough](../docs/reference/cli.en.md#following-llvm-origins)) |
| `string_byte_length.ceru` | `0, 9, 3, 2, 3, 4, 7, 3, 9, left, right, 9, false, false, 6, 10`, each with LF. `left`/`right` run once; no `skipped`. `byte_len` covers UTF-8 lengths, copies, calls, arrays, and defaults |
| `generic_functions.ceru` | Totals `6 → 9223372036854775809 → 4`, original/replaced strings, product and sum values |
| `generic_evaluation_order.ceru` | `評価順 → 配列 → 添字 → 2 → 末尾 → false → true`; does not print `呼ばれない` |
| `constant_array_lengths.ceru` | Dimensions `2 → 3`, row totals `6 → 15`; after an update the original remains `6` and the copy is `99`. Also prints string lengths/bytes and u64 boundaries |
| `array_iteration.ceru` | Positive total `15`; search index `2` and `未登録`. Visits `1 → 2 → 3` despite original-array updates; original second element becomes `99` |
| `array_iteration_values.ceru` | String indices/byte lengths/contents and u64 boundaries. After copy updates, original row starts remain `1 → 3`, original product count `10`; sums print `空 → 海` |
| `array_length.ceru` | Aggregation: `4 → 20 → 4 → 118 → 20`; updating a copy preserves the original. Type examples: `2 → 3 → 2 → 9 → 2 → 2 → 2` (`9` is the string byte count) |
| `array_length_order.ceru` | `make → 10 → 20 → 2`; the second call precedes `later → 5`. No `skipped`; loop-condition `2` appears three times, including the exit check |
| `coin_change.ceru` | Minimum coin counts for amounts 1–6, then selected coins `3 → 3` |
| `shortest_paths.ceru` | Distance from town 0 to 3, then a 4×4 distance table in row order; `-1` means unreachable |
| `heat_diffusion.ceru` | Five temperatures per step for four steps, then the saved initial center temperature |
| `linear_regression.ceru` | Initial loss; epoch, slope, intercept, and loss every ten epochs; finally the prediction for input 3 |
| `integer_limits.ceru` | Normally succeeds. Uncomment an expression at the end to inspect overflow and its diagnostic location |

Change `rate` (the learning step size) or the iteration count in `linear_regression.ceru` to compare learning progress. It learns slope/intercept using gradient descent; `xor_neural_network.ceru` uses fixed weights for inference and does not train XOR.

## Generated output and checks

The VM, C, LLVM, QBE, WAT, Windows/Linux ASM, and native objects support the types and features above including dynamic arrays. Mutable arrays support in-place sorting and dynamic programming; recursion remains unavailable.

| Route | Requirements / observation |
| --- | --- |
| C | External C compiler; compile and run as below |
| LLVM | Explicit Windows/Linux target ([commands](../docs/reference/cli.en.md#llvm-target-selection)) |
| QBE | Explicit Linux x86-64 target |
| WAT | WebAssembly environment with output host functions |
| ASM / native objects | Windows x64 or Linux x86-64; explicit target and tools. `native_values.ceru` covers mixed arguments and value passing ([external tools](../docs/design/native-code.en.md), [internal encoder](../docs/design/native-encoder.en.md)) |

```sh
cargo run --quiet -- emit-c examples/string_lookup.ceru -o target/string_lookup.c
clang -std=c11 target/string_lookup.c -o target/string_lookup
```

Run with `./target/string_lookup` in Bash or `.\target\string_lookup.exe` on Windows. For annotated LLVM, for example:

```sh
cargo run -- emit-llvm examples/string_byte_length.ceru --target x86_64-unknown-linux-gnu --annotate-origins -o target/string-byte-length.ll
```

| Check | Command / scope |
| --- | --- |
| Batch runner | Commands at the top; checks exit status |
| Array iteration | `cargo test --test array_iteration`; example output, copies, control, diagnostic locations |
| Expected output | `cargo test --test examples` |
| C strings | `cargo test --test c_strings`; compare with the VM, with and without optimization |
| LLVM strings | `cargo test --test llvm_strings`; compare VM/C/LLVM bytes |
| QBE / WAT / ASM strings | `cargo test --test string_routes` ([tools and scope](../docs/design/strings.en.md#validation-scope)) |
| Full checks | `bash scripts/test.sh`; fmt, Clippy, all test targets |

`u64_values.ceru` is checked against known output across routes. String byte lengths are also checked against expected bytes; the [observation fixture](../tests/fixtures/observation/string-byte-length/) contains small inputs and each representation.

Missing default compilers may skip execution comparisons. Setting `CERUNE_TEST_CC` / `CERUNE_TEST_LLVM_CLANG` makes the selected compilers mandatory. CI requires Clang and also runs AddressSanitizer / UndefinedBehaviorSanitizer checks.
