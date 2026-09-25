# OrchidDB native compiler

A shared C ABI for the OrchidDB graph-to-SQL compiler. Python, JavaScript,
Elixir and C++ clients can load this library without a DuckDB dependency.
Java retains its existing JNI compiler binding; Rust uses the Rust API.

The contract is **query + schema/mapping metadata → SQL + output field names**.
No database connection, source rows, Arrow buffers or query results enter this
library. The application executes SQL and consumes Arrow batches through its
own engine adapter. Dialect support and read-only compiler limitations are
in the [compiler guide](https://github.com/OrchidDB/OrchidDB/blob/main/docs/compiler.md).

## Build

Keep this checkout beside `orchiddb/`, using the commit in `CORE_REVISION` for
reproducible builds. Requires Rust 1.93.1+, a C toolchain, and Python for the smoke
check. Builds compile DataFusion but do not compile DuckDB.

```sh
cargo build --locked
cargo test --locked
python scripts/smoke.py target/debug/liborchiddb_compiler.dylib
```

Linux uses `.so`; Windows uses `orchiddb_compiler.dll`. `CARGO_TARGET_DIR` can
point at a shared local Cargo cache. No runtime downloads are performed.

## ABI 1

The [header](include/orchiddb.h) declares:

```c
uint32_t orchiddb_abi_version(void);
const char *orchiddb_version(void);
const char *orchiddb_core_revision(void);
char *orchiddb_compile_json(const char *input);
void orchiddb_string_free(char *response);
```

`input` is a NUL-terminated UTF-8 JSON compiler request, protocol version 1.
The result is an owned UTF-8 JSON envelope:

```json
{"ok":true,"result":{"version":1,"dialect":"duckdb","sql":"SELECT 42 AS answer","fields":["answer"]}}
```

Errors use `{"ok":false,"error":"..."}`. Free every response exactly once with
`orchiddb_string_free`; that function accepts null. Never free borrowed version
or revision strings. Invalid raw pointers and double frees are caller errors.
Requests and responses contain no query-result data, but may contain sensitive
parameter values; do not log them indiscriminately.

Calls may be concurrent. A process-wide two-worker runtime gives recursive
planning a 16 MiB native stack. Calls synchronously wait for compilation; language
bindings must schedule them appropriately (for example an Elixir dirty scheduler).
Admission limits/cancellation belong to the embedding application. Process abort
and allocation failure cannot be recovered across this ABI.

## Release

`CORE_REVISION` pins the public core source. Tagged `vX.Y.Z` releases must match
Cargo's version. The workflow builds and tests Linux x86_64, macOS ARM64/x86_64,
and Windows x86_64 before preparing a **draft GitHub release**. Clean pinned source
is mandatory under `ORCHIDDB_RELEASE_BUILD=1`; development builds report `-dirty`
when appropriate instead of claiming clean provenance.

Each `orchiddb-compiler-vX.Y.Z-TARGET.tar.gz` contains `lib/`, `include/orchiddb.h`,
`LICENSE.md` and `manifest.json` (ABI, package version, core revision, target,
library name and library checksum). `SHA256SUMS` covers all archives. Clients
should package the matching library into their own platform artifacts or use an
explicit local-library override. Source pins and checksum validation must match;
ABI compatibility alone does not establish a release's provenance.

This is a native runtime artifact, not a crates.io package. GitHub's workflow
token can create the draft; client registries require their own publishing
credentials or trusted-publisher configuration. No release is published merely
by pushing the source repository.

See [LICENSE.md](LICENSE.md) for the applicable license.
