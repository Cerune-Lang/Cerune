# Cerune documentation

[日本語](README.md)

See the [roadmap](design/language-roadmap.en.md) for current capabilities, issues, and priorities, and the [2026-10-11 audit](design/development-status.en.md) for earlier decisions and evidence. Design records decisions, reference records implemented behavior, and ideas are unadopted topics.

## Design

Each document links to its Japanese counterpart.

- [Execution routes and unfinished work](design/development-status.en.md)
- [Naming and migration](design/naming.en.md)
- [Compiler architecture](design/architecture.en.md)
- [IR stages and pass observation](design/ir-stages.en.md)
- [LIR observation, validation, and executor decision](design/lir-observation.en.md)
- [MIR SSA construction](design/mir-ssa.en.md)
- [Observation bundle](design/observation-bundle.en.md)
- [Observability contract](design/observability.en.md)
- [Runtime diagnostics](design/runtime-diagnostics.en.md)
- [AST execution study](design/ast-executor-study.en.md)
- [MIR execution](design/mir-executor.en.md)
- [IR Executor](design/ir-executor.en.md)
- [VM embedding](design/vm-embedding.en.md)
- [Standard I/O: execution, control, and observation (design)](design/standard-io.en.md)
- [Actual stdin in CLI and generated C/LLVM](design/stdin-connections.en.md)
- [Virtual standard modules and origins](design/standard-modules.en.md)
- [Bounded UTF-8 line input](design/line-input.en.md)
- [Owned routes and build / release](design/owned-routes.en.md)
- [Lean verification](design/lean-verification.en.md)
- [Output routes and targets](design/targets.en.md)
- [QBE Windows support](design/qbe-windows.en.md)
- [Owned arrays](design/owned-arrays.en.md)
- [Dynamic strings and data](design/dynamic-data.en.md)
- [Language capabilities and roadmap](design/language-roadmap.en.md)
- [Rounding and saturation](design/rounding-conversions.en.md)
- [Truncating conversions](design/truncating-conversions.en.md)
- [Generic functions](design/generic-functions.en.md)
- [Functions](design/functions.en.md)
- [Named product types](design/product-types.en.md)
- [Aggregate values and match expressions](design/aggregate-values.en.md)
- [Sum types and match](design/sum-types.en.md)
- [Compile-time constants](design/constants.en.md)
- [Product updates](design/product-updates.en.md)
- [Array iteration](design/array-iteration.en.md)
- [Constants in array lengths](design/constant-array-lengths.en.md)
- [Fixed arrays](design/fixed-arrays.en.md)
- [Strings](design/strings.en.md)
- [Unsigned 64-bit integers](design/u64.en.md)
- [Native lowering from MIR](design/native-mir.en.md)
- [Native-code observation](design/native-code.en.md)
- [Native object encoder](design/native-encoder.en.md)
- [Constant-folding design (implementation deferred)](design/ssa-constant-folding.en.md)
- [Source files and origins](design/source-files.en.md)

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
