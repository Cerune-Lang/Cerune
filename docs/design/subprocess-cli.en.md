# Subprocess CLI connections

[日本語](subprocess-cli.ja.md)

`--process-config <JSON file>` connects Windows/Linux backends to the [common runtime](subprocesses.en.md) in `run`, `run-ir`, `run-mir`, `run-mir --ssa`, and `run-vm`. Without it, valid requests return `Unconfigured`. There is no environment, PATH, or automatic configuration search.

## Configuration and responsibilities

Configuration is UTF-8 JSON, limited to 1 MiB. Validate its version, platform, all limits, and registrations before execution. Reject unknown/duplicate fields, missing required fields, negatives, fractions, and integer-width overflow. Diagnostics do not reproduce configuration contents or environment values.

This Linux example shows the structure. Supply absolute paths appropriate for your machine.

```json
{
  "schema": "cerune-process-v1",
  "platform": "linux",
  "linux_supervisor": "/absolute/path/subprocess_linux_supervisor",
  "limits": {
    "arguments": 32,
    "argument_bytes": 16384,
    "stdin_bytes": 1024,
    "output_bytes": 1024,
    "timeout_ms": 10000,
    "cleanup_timeout_ms": 5000
  },
  "transfer_bytes": 32768,
  "targets": [{
    "name": "tool",
    "executable": "/absolute/path/child",
    "directory": "/absolute/working/directory",
    "environment": [["CERUNE_CHILD_VALUE", "日本語\r\n"]]
  }]
}
```

| Field | Meaning and validation |
| --- | --- |
| `schema` | Exactly `cerune-process-v1`; never silently interpret a different version |
| `platform` | `windows` or `linux`, matching the Cerune host OS; connections on other OSes are unsupported |
| `linux_supervisor` | Required absolute supervisor path on Linux; omitted or null on Windows |
| `targets` | At most 64 registrations; unique, nonempty, NUL-free names of at most 256 UTF-8 bytes; empty lists allowed |
| `executable` / `directory` | Absolute executable/child working-directory paths; never resolved relative to the configuration file |
| `environment` | At most 128 name/value pairs; no parent environment inheritance; an empty list is valid; reject duplicate names (case-insensitive on Windows) |
| `limits.arguments` | Argument count per request, 0–4096 |
| `limits.argument_bytes` | Combined UTF-8 bytes of target name and all arguments, 0–16384; OS connections separately validate Windows UTF-16 and other representation limits |
| `limits.stdin_bytes` | Input byte limit per request |
| `limits.output_bytes` | Limit on the sum of requested stdout/stderr capacities |
| `limits.timeout_ms` | Maximum execution duration a request may specify, positive u64 milliseconds |
| `limits.cleanup_timeout_ms` | Cleanup deadline allowance per operation, positive u64 milliseconds |
| `transfer_bytes` | Combined output reservations plus three 4096-byte transfer buffers; separate from language array storage limits |

Counts and byte capacities must fit host `usize`. Zero capacity is explicit and can produce a prelaunch failure value. There are no numeric defaults or silent clamps. See [Windows](subprocess-windows.en.md) and [Linux](subprocess-linux.en.md) for OS-specific path, environment, and total registration limits.

On Windows, use `platform: "windows"`, remove `linux_supervisor`, and supply paths such as `C:/absolute/path/child.exe`. Resolve names to registrations without a shell. Windows arguments use existing Microsoft CRT quoting, without guaranteeing compatibility with arbitrary custom command-line parsers.

## Execution and failures

```sh
cerune run app.ceru --process-config process.json -- argument
cerune run-mir app.ceru --ssa --process-config process.json
cerune run-vm app.ceru --process-config process.json
```

- Configuration read/validation errors stop the CLI before program execution, including the first print or any child launch.
- Configuration validation does not probe executable existence. A missing registered executable returns `NotFound`; an unknown target returns `Unregistered`. Launch, transfer, duration, and capacity failures return `Outcome` and allow continuation. Resources insufficient to construct a result retain existing language-stop semantics.
- Nonzero child exit does not itself fail the CLI. Preserve child status and raw bytes in `Outcome`; successful completion of the Cerune caller exits the CLI successfully.
- Calls share the connection during execution. Once the common runtime disables it, later requests return `ConnectionDisabled`.
- Tokens after `--` remain program arguments, including literal `--process-config`. Existing stdin/stdout, `--file-root`, call-depth, and storage limits remain usable together.
- Preserve types, operations, and NodeId/Span from the [common IR contract](subprocess-execution.en.md). This initial CLI does not retain event history. Returned Reports and IR origins remain observable, as do read-only records through the Rust API.

## Validation and remaining work

The [registered-target example](../../examples/process_execution/README.en.md) runs on Windows/Linux through all four executors, including the `run-ir` alias. [CLI tests](../../tests/process_cli/mod.rs) cover quoting, empty/Japanese arguments, environment/cwd, raw NUL/invalid-UTF-8 bytes, nonzero exit, missing/unregistered targets, capacity/time failures and continuation, combined existing I/O, and invalid configuration. Tests never automatically rerun arbitrary user programs for comparison.

The Linux OS test requires an explicitly supplied verification supervisor through `CERUNE_TEST_PROCESS_SUPERVISOR`. It is ignored in ordinary tests and explicitly executed with `--include-ignored` in OS CI. This is a test setting: the Cerune CLI never reads that environment variable.

The Linux supervisor is currently built as a Cargo example. [Generated C](subprocess-c.en.md) now uses explicit target/runtime linking and startup configuration in the generated executable. Supervisor distribution, LLVM/QBE/WAT/Native operations, and Native build/release records remain future work. Configuration JSON is not passed to emit itself.

The [generated-route proposal](subprocess-generated.en.md) details the next steps and compatibility impact.
