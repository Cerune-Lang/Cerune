# Cerune documentation

[日本語](README.md)

See the [roadmap](design/language-roadmap.en.md) for current capabilities, issues, and priorities, and the [2026-10-11 audit](design/development-status.en.md) for earlier decisions and evidence. Design records decisions, reference records implemented behavior, and ideas are unadopted topics.

## Design

Each document links to its Japanese counterpart.

### Overview and development direction

- [Compiler architecture](design/architecture.en.md)
- [Language features and roadmap](design/language-roadmap.en.md)
- [Execution routes and unfinished work](design/development-status.en.md)
- [Naming and migration](design/naming.en.md)
- [Responsibilities across projects](design/ecosystem.en.md)
- [Arguments and integer parsing from real uses](design/dogfooding-inputs.en.md)

### Language features, types, and syntax

- [Functions](design/functions.en.md)
- [Generic functions](design/generic-functions.en.md)
- [Recursion design and invocation storage](design/recursion.en.md)
- [Omitting infer in bindings](design/infer-omission.en.md)
- [Optional let and source formatting](design/formatting.en.md)
- [Compile-time constants](design/constants.en.md)
- [Explicit integer parsing](design/integer-parsing.en.md)
- [Floating-point rounding and route comparisons](design/float-rounding.en.md)
- [Rounding and saturation](design/rounding-conversions.en.md)
- [Truncating conversions](design/truncating-conversions.en.md)
- [Unsigned 64-bit integers](design/u64.en.md)
- [Strings](design/strings.en.md)
- [Fixed arrays](design/fixed-arrays.en.md)
- [Constants in array lengths](design/constant-array-lengths.en.md)
- [Array iteration](design/array-iteration.en.md)
- [Owned arrays](design/owned-arrays.en.md)
- [Dynamic strings and data](design/dynamic-data.en.md)
- [Named product types](design/product-types.en.md)
- [Product updates](design/product-updates.en.md)
- [Sum types and match](design/sum-types.en.md)
- [Aggregate values and match expressions](design/aggregate-values.en.md)
- [Modules, namespaces, and visibility](design/modules.en.md)
- [Source files and origins](design/source-files.en.md)
- [Compiler input limits](design/frontend-limits.en.md)

### IR, executors, and verification

- [IR stages and pass observation](design/ir-stages.en.md)
- [IR Executor](design/ir-executor.en.md)
- [MIR execution](design/mir-executor.en.md)
- [MIR SSA construction](design/mir-ssa.en.md)
- [LIR observation, validation, and executor decision](design/lir-observation.en.md)
- [VM embedding](design/vm-embedding.en.md)
- [AST execution study](design/ast-executor-study.en.md)
- [Constant-folding design (implementation deferred)](design/ssa-constant-folding.en.md)
- [Observability contract](design/observability.en.md)
- [Observation bundle](design/observation-bundle.en.md)
- [Runtime diagnostics](design/runtime-diagnostics.en.md)
- [Lean verification](design/lean-verification.en.md)

### Code generation and external hosts

- [Output routes and targets](design/targets.en.md)
- [Native lowering from MIR](design/native-mir.en.md)
- [Native-code observation](design/native-code.en.md)
- [Native object encoder](design/native-encoder.en.md)
- [Typed Program construction for complete Native programs](design/native-typed-runtime.en.md)
- [QBE Windows support](design/qbe-windows.en.md)
- [WAT library function exports](design/wat-library.en.md)
- [WAT library JS/TS API](design/wat-js-bindings.en.md)
- [WAT program-argument host contract v1](design/wat-arguments-host.en.md)
- [WAT output host contract v1](design/wat-output-host.en.md)
- [Versioned WAT input hosts](design/wat-input-host.en.md)
- [Evaluating WAT input storage splitting and coalescing](design/wat-input-storage.en.md)
- [Versioned WAT file hosts](design/wat-file-host.en.md)
- [Typed function calls from JS/TS](design/typed-host-calls.en.md)

### Runtime, I/O, and subprocesses

- [Standard I/O: execution, control, and observation (design)](design/standard-io.en.md)
- [Actual stdin in CLI and generated C/LLVM/QBE](design/stdin-connections.en.md)
- [Bounded UTF-8 line input](design/line-input.en.md)
- [Virtual standard modules and origins](design/standard-modules.en.md)
- [Limited file I/O](design/file-io.en.md)
- [Line-input retained capacity and memory limits](design/input-memory.en.md)
- [Reducing spare capacity in completed input lines](design/input-compaction.en.md)
- [Subprocess execution (common runtime and four executors)](design/subprocesses.en.md)
- [Subprocess CLI configuration](design/subprocess-cli.en.md)
- [Subprocess common IR and four executors](design/subprocess-execution.en.md)
- [Generated-route subprocess connections (proposal)](design/subprocess-generated.en.md)
- [Public subprocess data types](design/subprocess-values.en.md)
- [Subprocess transfers in flight and cancellation](design/subprocess-transfers.en.md)
- [Reserving subprocess results before launch](design/subprocess-storage.en.md)
- [Subprocess OS connections](design/subprocess-os.en.md)
- [Linux subprocess Backend](design/subprocess-linux.en.md)
- [Windows subprocess Backend](design/subprocess-windows.en.md)
- [Subprocess connections in generated C](design/subprocess-c.en.md)

### Build and distribution

- [Owned routes and build / release](design/owned-routes.en.md)
- [Native build: executable and generation/link records](design/native-build.en.md)
- [Native release: distribution, runtime requirements and checksums](design/native-release.en.md)

## Reference

Each document links to its Japanese counterpart.

- [Language](reference/language.en.md)
- [CLI](reference/cli.en.md)

## Ideas

Each document links to its Japanese counterpart.

- [Idea catalog](idea/idea-catalog.en.md)

[Verification experiments](../experiments/README.en.md) are separate from ordinary Cerune execution examples.

## Documentation checks

Run these commands from the repository root. CI runs the same checks.

```sh
node scripts/check-docs.cjs
node --test scripts/check-docs.test.cjs
```

For tracked and untracked non-ignored Markdown, check local file/directory targets, obsolete Cerune URLs, and translation pairs. Public guides use `.ja.md`/`.en.md`; READMEs use `README.md`/`README.en.md`. The English GitHub policy `SECURITY.md` is exempt from pairing.

Checks cover prose inline links/images and reference definitions, excluding code examples. External URL availability, heading anchors, translation equivalence, and implementation status require human review. This is not a full Markdown parser.
