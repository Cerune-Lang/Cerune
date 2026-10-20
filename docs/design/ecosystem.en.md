# Responsibilities across Cerune projects

[日本語](ecosystem.ja.md)

Inventory of the [Organization](https://github.com/Cerune-Lang) on 2026-10-20. This repository owns language specifications and implementation status. Other repositories own their documents/code; this inventory does not modify them.

| Responsibility | Project | Current scope |
| --- | --- | --- |
| Language, semantics, execution | [Cerune](https://github.com/Cerune-Lang/Cerune) | Common IR, executors, generated routes, ownership/failure/observation contracts |
| Observation UI | [Tint-St](https://github.com/Cerune-Lang/Tint-St) | CLI-backed display/execution and browser Wasm experiment; no duplicated language semantics |
| Editing | [Cerune-vscode](https://github.com/Cerune-Lang/Cerune-vscode) | Syntax highlighting/snippets, not a completed compiler or LSP |
| Differential tests | [Parallax](https://github.com/Cerune-Lang/Parallax) | IR/MIR/VM, fixed seeds, bounded expression/control-flow/loop/array generation, expectations and stops |
| Safety research | [Fracta](https://github.com/Cerune-Lang/Fracta) | Expected overflow/division/index/conversion stops, not assumed vulnerabilities |
| General numerics | [Quantelia](https://github.com/Cerune-Lang/Quantelia) | Initial f32 Tensor representation; no shape-consistency guarantee yet |
| Machine learning | [Nerune](https://github.com/Cerune-Lang/Nerune) | Initial Tensor code; learning features are future work |
| Real uses | [Cisa](https://github.com/Cerune-Lang/Cisa) / [stelle](https://github.com/Cerune-Lang/stelle) | Virtual CPU / CLI RPG expose concrete missing capabilities |
| Explanation and entry points | [website](https://github.com/Cerune-Lang/website) / [.github](https://github.com/Cerune-Lang/.github) | Reader-oriented explanations / Organization profile; Cerune docs remain authoritative |

Whitebase is outside this Organization inventory. Treat it as an external measurement/comparison environment, not a required dependency of Cerune or its numerical libraries.

## Verified implementation scope

- [Parallax 356f357](https://github.com/Cerune-Lang/Parallax/tree/356f357cb2bf347ef8ec7462e6bdd77410a6e7d7) compares status, stdout and runtime-v1 codes for success and failure, including prior output and generated tests. General NodeId/Span equivalence, SSA and compiled-route coverage remain incomplete.
- [Fracta 4124812](https://github.com/Cerune-Lang/Fracta/tree/41248121ca0cdcaf759e61b8d0972bbe4130f7da) classifies four expected safe stops. Follow each SECURITY.md for unpublished security reports.
- Both build Cerune master in their own CI. This is distinct from integration into Cerune PR CI or compatibility verification against a pinned release. Record the actual Cerune commit for reproduction.
- Retain Parallax's independent Python oracle. Moving generation into Cerune must not leave only checks sharing the same implementation defects.

Core [#131](https://github.com/Cerune-Lang/Cerune/issues/131) tracks generation/route comparison, [#153](https://github.com/Cerune-Lang/Cerune/issues/153) hardening, and [#173](https://github.com/Cerune-Lang/Cerune/issues/173) concrete missing language capabilities. Keep core regression tests alongside external exploratory tests.

## Documentation discrepancies

| Location | Follow-up |
| --- | --- |
| Parallax README | Update initial success-only/no-generation claims |
| Fracta README | Describe four cases rather than only the first |
| .github/profile | Add website, Parallax and Fracta |
| stelle README | Replace the no-stdin claim; distinguish supported line input from unsupported Windows console input |
| Quantelia / Nerune | Resolve duplicated Tensor code. Intended dependency is Nerune→Quantelia→Cerune; integration is not yet complete |
| Links / bilingual documents | Check old-owner URLs and mixed-language documents in their respective repositories |
| website | Track the source Cerune version; successful deployment CI is not proof of documentation accuracy |

## Next core work

#171 merged WAT i32/f64 exports, JS/TS generation and HTML examples, completing the initial scope. Next is [argument/integer parsing design](dogfooding-inputs.en.md), followed by limited file I/O and subprocesses. Native build can proceed independently and is not a prerequisite.

Compiler release v0.1.0 differs from master. #163 rounding fixes, #164 input limits and #170/#171 library exports arrived after v0.1.0. Examples, sites and surrounding CI must distinguish their tested versions rather than describe master-only features as released.
