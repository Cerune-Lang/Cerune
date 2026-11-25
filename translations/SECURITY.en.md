# Security Policy

[日本語](../SECURITY.md) | English

Cerune is an experimental programming language developed as a personal project. It is under active development and is **not intended for security-critical or production use**. No security guarantees or fixed response times are provided.

## Versions and support

Reports affecting the current `master` branch or the latest published release are the most useful. Older versions may not receive individual fixes. Investigation, fixes, and publication of updated releases are handled on a best-effort basis.

## Usage assumptions and trust boundaries

Cerune's input checks and runtime limits are not substitutes for operating-system-level isolation. Do not treat its executors or generated code as a sandbox for safely running untrusted programs.

- When handling untrusted source code or generated artifacts, use a dedicated environment with appropriate isolation and restricted privileges, including during compilation. Avoid access to sensitive data and important files, and use external mechanisms to limit time, memory, process counts, disk usage, and other resources.
- File connection working directories, registered subprocess targets, working locations, and environment variables should be selected by a trusted user or host and limited to what is necessary. Consider the operations permitted by registered executables and their arguments.
- External compilers, linkers, SDKs, runtimes, Wasm hosts, and embedding backends are also part of the trust boundary. Choose trusted implementations and distribution sources, and use appropriate privileges.
- Check the source and version of distribution artifacts, and compare SHA-256 hashes where provided. A matching hash alone does not establish the trustworthiness of the distribution source or the safety of a program.

## Security-related design and limitations

The following summarizes existing measures and their scope. See the linked design documents and the [CLI reference](../docs/reference/cli.en.md) for specific conditions, supported execution routes, and targets.

| Area | Measures and limitations |
| --- | --- |
| [Compilation input](../docs/design/frontend-limits.en.md) | Syntax and expression depth, source loading size, and file counts are limited. These limits do not bound overall compiler resource usage or confine relative imports to an allowed root. |
| [Runtime resources](../docs/design/dynamic-data.en.md) | Resource budgets for strings and arrays, among other limits, and separately defined call-depth limits are provided. They do not guarantee total process memory, physical stack usage, or execution time. |
| [File I/O](../docs/design/file-io.en.md) | Explicit working scopes, file checks, exclusive creation, and transfer limits are provided. This is not an OS-wide sandbox; hard links, changes by other processes, and OS permissions are separate considerations. |
| [Subprocess execution](../docs/design/subprocess-cli.en.md) | Explicitly registered targets and per-operation argument, input/output, and deadline limits are used. These do not isolate the behavior or privileges of registered executables or guarantee aggregate resource usage across calls. The [OS integration](../docs/design/subprocess-os.en.md) also cannot guarantee cleanup in every situation, including forced supervisor termination or descendants outside the managed scope. |
| [Native build](../docs/design/native-build.en.md) | Explicit tools, targets, and time limits are used, and generated artifact formats are checked. These measures do not sandbox external tools or prove the safety of all generated code. |
| [Observation and diagnostics](../docs/design/observability.en.md) | Observation is separated from control, and diagnostic and trace records are bounded. Observation artifacts, output, source excerpts, and link records may contain non-public information and must be reviewed before sharing. |

Tests, cross-route comparisons, formal verification of finite examples, and successful CI runs establish results under the conditions tested. They do not guarantee the security of the entire implementation or its behavior in every environment. Preventive hardening work is tracked in issues such as [#153](https://github.com/Cerune-Lang/Cerune/issues/153).

## What to report

Reports of suspected security impact are welcome. Examples include:

- File operations beyond intended access restrictions or launching unauthorized subprocess targets
- Memory corruption, invalid memory access, or bypasses of defined resource limits
- Unintended disclosure of sensitive information
- Security-relevant defects in code generation, ABI handling, runtimes, or host integration
- Security issues affecting distribution artifacts, build or release processes, or dependencies

A crash, incorrect compilation result, unsupported feature, or documented limitation is not automatically a security vulnerability. Reproduction conditions and impact need to be assessed. If uncertain, it is fine to report the issue privately first.

Ordinary bugs and feature requests can be submitted through [GitHub Issues](https://github.com/Cerune-Lang/Cerune/issues).

## Reporting a vulnerability

Please **do not disclose suspected vulnerabilities in public issues, pull requests, or discussions**. Use GitHub's [Private Vulnerability Reporting](https://github.com/Cerune-Lang/Cerune/security/advisories/new) to submit a confidential report.

When possible, include:

- The affected Cerune version or commit, operating system, architecture, execution route, and target
- External tool, runtime, and host versions and relevant settings, if used
- A minimal `.ceru` program or input, reproduction commands, required privileges, and prerequisites
- Expected versus observed behavior and why it might have security consequences
- Relevant diagnostics, IR, traces, or generated artifacts, where safe to share

You do not need to reproduce the issue on every execution route. Test only in environments you control or have explicit permission to test, and avoid affecting third-party environments or data. Do not include unrelated personal data, credentials, or secrets in reproductions, logs, configurations, or observation artifacts.

## Handling and disclosure

The private report is used to investigate reproduction conditions, affected scope, and potential fixes or mitigations. When making a fix, regression tests and relevant documentation will be updated where possible, and affected versions and user guidance will be considered.

Investigation, fixes, and disclosure are handled on a best-effort basis as a personal project. Response deadlines, fix deadlines, and action on every report are not guaranteed. Please avoid publishing exploit details before investigation, and discuss disclosure content and timing through the private report.
