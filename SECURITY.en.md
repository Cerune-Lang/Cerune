# Security Policy

[日本語](SECURITY.md) | English

Cerune is an experimental programming language developed by an individual as part of learning and research. It is under active development and is **not intended for security-critical or production use**. No security guarantees or fixed response times are provided.

## Versions and support

Reports affecting the current `master` branch or the latest published release are the most useful. Older versions may not receive individual fixes. Investigation, fixes, and publication of updated releases are handled on a best-effort basis.

## Reporting a vulnerability

Please **do not disclose suspected vulnerabilities in public issues, pull requests, or discussions**. Use GitHub's [Private Vulnerability Reporting](https://github.com/Cerune-Lang/Cerune/security/advisories/new) to submit a confidential report.

When possible, include:

- The affected Cerune version or commit, operating system, and target.
- A minimal `.ceru` program or input and the commands needed to reproduce the issue.
- The observed behavior and why it might have security consequences.

Do not include unrelated personal data, credentials, or secrets. Please allow time for investigation before publishing exploit details; any disclosure timing can be discussed through the private report.

A crash, incorrect compilation result, or unsupported feature is not automatically a security vulnerability. If uncertain, it is fine to report the issue privately first.

Ordinary bugs and feature requests can be submitted through [GitHub Issues](https://github.com/Cerune-Lang/Cerune/issues).
