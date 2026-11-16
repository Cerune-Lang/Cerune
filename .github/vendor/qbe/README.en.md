# QBE 1.3 backup for CI

[日本語](README.md)

This upstream release archive keeps CI on the same version when the QBE website is unavailable. Its contents and compression are unchanged. It is neither a Cerune release nor a modified QBE distribution.

| Item | Value |
| --- | --- |
| Upstream archive | https://c9x.me/compile/release/qbe-1.3.tar.xz |
| Official release information | https://c9x.me/compile/releases.html |
| Upstream commit | `c0818978acec60ebb6167fade60fb7012cbf20ca` |
| SHA-256 | `d587905d620dc5e1d2bfa7c2cc642b9b837aa89a3188c6e37b53d756cf66e320` |
| License | The archive's MIT permission notice is also preserved unchanged as [LICENSE](LICENSE) |

The [fetch script](../../../scripts/fetch-qbe.sh) serves all four Linux and Windows/MSYS2 download sites. It first tries upstream HTTPS, with a 10-second connection limit and 30-second total limit. Download failure selects this backup, logging the selected source and checksum. A successful upstream download with a wrong hash fails without falling back. Invalid existing outputs or backup hashes also fail. Temporary files are never extracted before validation and are removed on failure.

~~~sh
bash scripts/fetch-qbe.sh target/qbe-1.3.tar.xz
# Check the verified backup without network access
bash scripts/fetch-qbe.sh --offline target/qbe-offline.tar.xz
~~~

When updating QBE, verify the official version, commit and SHA-256; update the archive, license, this table and script constants together. Never substitute repacked archives or other versions. This directory is excluded from GitHub language statistics.
