# Dependency policy

Dependencies, build tools and GitHub Actions are executable trust boundaries.
Use only the minimum required features, review source/license/advisories and
record versions/checksums in Cargo.lock. Never execute submitted guest code to
validate registry records. Publication is not authorized by this document.

## Task 2 toolchain review (2026-08-30)

- Selected toolchain: Rust 1.98.0, matching the host baseline's installed compiler
  (`rustc 1.98.0 (88d9e12ae 2026-08-18)`).
- Canonical upstream: <https://github.com/rust-lang/rust/releases/tag/1.98.0>.
  The upstream release was verified through GitHub; it is neither draft nor
  prerelease. Rust's license is MIT OR Apache-2.0.
- Installation uses the existing rustup and its official distribution/checksum
  verification. No downloaded shell installer or alternate binary mirror is
  introduced. Select the minimal profile plus rustfmt, clippy and the
  wasm32-unknown-unknown standard library target; no WASI target or adapter.
- This build-tool pin is not a public plugin compatibility version. Public
  compatibility remains the Chronlyt WIT/API/package/data contracts.
- Decision: ACCEPT for the required reproducible build/canary toolchain.
  Installation succeeded using rustup on 2026-08-30; build validation follows.

## Contracts dependencies

Reuse the unchanged host's crates.io versions; no additional library is needed.
The cached package manifests were checked against the host lockfile identities.
All direct dependencies below are MIT OR Apache-2.0. Cargo generates the public
lockfile and verifies registry checksums; dependency versions are not wire API.

| Package | Version | Upstream | Required features |
| --- | --- | --- | --- |
| serde | 1.0.229 | serde-rs/serde | std, derive |
| serde_json | 1.0.151 | serde-rs/json | std |
| semver | 1.0.28 | dtolnay/semver | std, serde |
| chrono | 0.4.45 | chronotope/chrono | std only; no clock/wasmbind |
| url | 2.5.8 | servo/rust-url | std; URL parsing, not HTTP |
| thiserror | 2.0.20 | dtolnay/thiserror | std; bounded errors |

Serde/thiserror and their macros execute existing build-time Rust code;
thiserror probes rustc and generates OUT_DIR sources, and serde_json selects
target arithmetic cfgs. They do not add an application process/network API.
URL parsing retains its IDNA/ICU dependencies to preserve host behavior.
Decision: ACCEPT WITH CHECKS — compare the generated transitive lockfile to
the host, run the installed cargo-audit, and inspect feature closure before
declaring Task 2 complete.

Task 2 evidence (2026-08-31): Cargo metadata reports 47 packages. Every registry
package version/checksum matches the existing host lockfile after Cargo pinned
syn from incidental 3.0.4 back to baseline 3.0.3. Feature closure has no Wasmtime,
WASI, HTTP/Tokio, database, ZIP, Tauri, getrandom, wasm-bindgen or local-time-zone
dependency. Chrono enables alloc/std only. The contracts build succeeds for
wasm32-unknown-unknown without adapters.

Transitive license expressions are MIT/Apache-2.0 alternatives, Unlicense OR MIT,
Unicode-3.0, and (MIT OR Apache-2.0) AND Unicode-3.0 for ICU data. Preserve upstream
license notices when redistributing those components; this repository's MIT
license does not replace third-party notices. No reciprocal-license dependency
was identified in this closure.

`cargo audit --file Cargo.lock` passed after refreshing RustSec (1,226 advisories,
47 packages) during initial Task 2 validation. That records the checked snapshot,
not a permanent security guarantee; rerun dependency review when the graph changes.

## Package/static validator dependencies (2026-08-31)

Decision: ACCEPT WITH CHECKS. Reuse the host lockfile releases, not a new runtime.
The public validator parses untrusted bytes; runtime execution and filesystem
staging remain host responsibilities. The WIT adapter was checked against the
unchanged host's compile-only acceptance for ten synthetic components.

| Package | Version | Upstream | Purpose and features |
| --- | --- | --- | --- |
| sha2 | 0.10.9 | RustCrypto/hashes | Existing SHA-256 integrity implementation, std |
| zip | 8.6.0 | zip-rs/zip2 | Stored/deflate reader, only deflate-flate2-zlib-rs |
| wasmparser | 0.236.1 | bytecodealliance/wasm-tools | std, validate, features, component-model, simd; no hash collections/serde |
| wit-parser | 0.236.1 | bytecodealliance/wasm-tools | Parse embedded canonical WIT, all default features off |
| wat | 1.258.0 | bytecodealliance/wasm-tools | Dev-only synthetic fixture encoding, matching host tests |

The parser APIs were checked against cached exact-version sources and upstream
Wasm Tools docs via Context7. wasmparser/wit-parser/wat offer MIT/Apache-2.0
license alternatives (including an optional LLVM exception); zip is MIT and
sha2 is MIT OR Apache-2.0. Keep the additional Zlib and Unicode notices in the
transitive closure. No downloaded binary or WASI adapter is added.

The 83-package Cargo metadata closure contains no Wasmtime, WASI, HTTP/Tokio,
database, Tauri, getrandom, ZIP encryption or unrelated compression backend.
wat's development encoder uses wasmparser 0.258.0; production validation uses
only 0.236.1. Compiler-probe build scripts (wasmparser, anyhow, crc32fast,
generic-array) run trusted rustc/cfg probes, not submitted guest code. ZIP and
hash/parser crates do not introduce a production shell/network integration.

Cargo's incidental indexmap 2.14.1, flate2 1.1.10 and crc32fast 1.5.1 selections
were pinned back to the existing host lockfile's 2.14.0, 1.1.9 and 1.5.0. No
unrelated host dependency was changed. The feature tree omits ZIP's default
encryption, bzip2/lzma/zstd and extra compressor features. Audit and lockfile
conformance evidence are recorded with the task's validation results.

Final Task 3 checks: 11 validator tests, Clippy (`-D warnings`), rustfmt and
compile-only host parity passed. `cargo audit --file Cargo.lock` exited 0 after
the final lock pin (83 packages, 1,229 advisories). These are snapshot results,
not a substitute for later lockfile review or independent host validation.

## Registry HTTPS transport

Selected reqwest 0.12.28 from seanmonstar/reqwest (MIT OR Apache-2.0), exactly the
desktop's existing direct version. Enable only blocking and rustls-tls, with
defaults disabled. This introduces HTTP/TLS/Tokio only in the registry tool,
never in the portable contracts or pure validator. The blocking API retains the
per-request async body deadline; this was checked in the exact-version request,
client and response sources as well as upstream docs. Explicit no-proxy,
no-redirect and HTTPS-only configuration is required. Never add URL credentials,
ambient proxy authentication, disabled TLS checks or automatic decompression.
The TLS transitive graph includes existing cryptographic/native build code;
review its lockfile/features and run audit after resolution. Decision: ACCEPT
WITH CHECKS for the plan's bounded artifact and GitHub API acquisition only.

Local Git/blob tests use tempfile 3.27.0 (Stebalien/tempfile, MIT OR Apache-2.0),
the existing host release, for disposable isolated bare repositories. The same
reviewed dependency is used by the internal command's atomic output writer;
no custom temporary-name/race implementation is introduced. It remains absent
from portable contracts and pure validator production graphs. Tests create
objects/trees, not project commits. The production
Git reader uses the installed Git directly, disables lazy fetching/replacement
objects/configured helpers, clears inherited Git/credential environment and
bounds stdout/time. It never shells out through contributor text. Git must
support `--no-lazy-fetch`; unsupported versions fail instead of fetching.

## Schema-only tooling

Use schemars 1.2.2 (GREsau/schemars, MIT) with only std/derive. The runtime crate
already occurs in the host lockfile, but the optional derive macro is a newly
reviewed build-time dependency. Cached exact-version source and upstream docs
confirm default deserialization semantics, Serde field names/defaults/unknown
fields and nullable Option schemas. No generic schema validator, URL resolution,
remote refs, schema-time networking or guest execution is required.
Decision: ACCEPT WITH CHECKS behind the public contracts' optional `schema`
feature, enabled only by internal registry tooling; host default dependencies
must not enable it. Cargo.lock/audit and golden/default/null schema tests verify
the actual closure and emitted contract. The generator remains non-authoritative
for semantic rules such as timestamps, IDs, paths, signatures and UTF-8 bytes:
the shared Rust validator must always run.

The PR data reader also uses base64 0.22.1 (marshallpierce/rust-base64,
MIT OR Apache-2.0), already present in the host and HTTP dependency closures.
Decode only after encoded/decoded size checks, into a bounded destination.

Task 4 final review (2026-08-31): 180 packages in the lock/metadata closure;
shared baseline identities have matching checksums. Optional schemars_derive
1.2.2 and serde_derive_internals 0.30.0 are new build-time AST tooling (no build
scripts), with MIT and MIT OR Apache-2.0 licenses respectively. Incidental hyper
1.11.1 and cpufeatures 0.3.1 were pinned back to host 1.11.0 and 0.3.0.

Exception: retain chacha20 0.10.2. Cargo warned that host-baseline 0.10.1 is
yanked. Exact cached source/changelog diff and the
[upstream fix](https://github.com/RustCrypto/stream-ciphers/pull/580) confirm an
SSE4.1 intrinsic incorrectly used in the SSE2 backend (undefined behavior on
unsupported CPUs). This is an evidence-backed safety fix, not a cosmetic bump.
It is a target-specific transitive lock entry, not present in the current native
Linux registry build. The private host lockfile remains unchanged at this stage;
review this existing host issue at the coordinated adoption gate.

Production trees for default contracts and validator contain no schemars,
reqwest/Tokio, Wasmtime/WASI, tempfile/getrandom, Tauri or database code. The
registry enables only required HTTP/TLS, schema and atomic-output features.
Contracts still build on wasm32-unknown-unknown. Full public workspace tests
(51), schemas exact-match, Clippy and rustfmt passed; after final lock changes,
the 31 registry tests and Clippy passed again. Final cargo-audit exited 0 against
180 packages / 1,233 advisories. No blanket advisory or yanked-package exception
was configured. These results do not claim GitHub CI has run.

## CI-only review tools and actions

- `actions/checkout` v7.0.1, immutable revision
  `3d3c42e5aac5ba805825da76410c181273ba90b1`, MIT, upstream actions/checkout.
  Official release/ref, action inputs, Node 24 entrypoint and credential-removal
  source were inspected. Use ephemeral hosted runners and a single fixed trusted
  checkout in the target job, no unsafe-PR opt-out, submodules/LFS or persisted
  credentials. Its source removes temporary auth in the checkout finalizer when
  persistence is false; the build step also rejects remaining auth config.
  No third-party Rust setup/cache action is needed: trusted rust-toolchain.toml
  drives the runner's rustup with official checksum-verified distributions.
- `yaml-rust2` 0.12.0, Ethiraric/yaml-rust2, MIT OR Apache-2.0, dev-only. Optional
  encoding is disabled. Exact package/upstream manifest and Context7 API docs
  reviewed: YAML 1.2 AST parsing supports structural workflow regression tests
  without introducing a production YAML/config interpreter. Review arraydeque
  and hashlink resolution plus audit before completing this task.
- Local workflow syntax check: rhysd/actionlint v1.7.12, MIT. Only its official
  Linux amd64 release asset is allowed, 2,353,908 bytes, SHA-256
  `8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8` from the
  GitHub release API. Verify size and digest before extracting the single binary
  in a temporary directory or executing it. No remote install script is run.
  This tool does not run submitted guests; shellcheck/pyflakes subprocess checks
  are disabled when those tools are unavailable and that limitation is recorded.

These decisions authorize only local implementation/validation within the
approved milestone, not publication, GitHub settings changes or trusting a PR.

Task 5 local evidence: yaml-rust2 adds arraydeque 0.5.1 (fixed-capacity container),
hashlink 0.12.1 and foldhash 0.2.0 (map implementation), with MIT/Apache-2.0
alternatives. Exact package manifests were inspected; these are dev-only,
without build scripts or new production permissions. Final lockfile audit passed
against 184 packages / 1,233 advisories. All 35 registry tests, focused Clippy,
rustfmt, schema consistency and local actionlint passed. actionlint first found
the invalid job-level runner context; moving paths to step env fixed it without
loosening checks. Its optional shellcheck/pyflakes checks were not run (tools
absent). No live Actions run or repository enforcement was validated: read-only
GitHub metadata shows main is still initial revision 6d59f523dd0885862f31fd414f92ed2fad987e79,
unprotected, with no required status checks. Intake remains closed.

## Task 6 canary build dependencies (2026-09-01)

Decision: ACCEPT WITH CHECKS for the source-built compatibility canary only.
`wit-bindgen` 0.61.1 generates guest bindings from the canonical public WIT;
default features are disabled and only `macros` plus the guest's own
`cabi_realloc` support are enabled. Async, std, bitflags and macro-string are not
enabled. `wit-component` 0.258.0 is a validator dev-dependency used only to wrap
the checked source-built core module as a Component; its optional WAT features
and every WASI adapter remain disabled. Both are pinned exactly and locked.

The new Bytecode Alliance packages (`wit-bindgen`, its core/Rust/macro crates,
`wit-component`, `wit-parser` and `wasm-metadata`) use Apache-2.0 with LLVM
exception, Apache-2.0 or MIT alternatives. Their small supporting additions are
`heck` and `prettyplease`, both MIT OR Apache-2.0. Proc macros and build scripts
generate Rust/component metadata from the repository-owned WIT; they do not
download tools, execute guest code or add application filesystem/network APIs.

The final lock/metadata closure contains 194 packages. Focused feature-tree
inspection found no Wasmtime, WASI/wasip, capability filesystem, ambient runtime
or new network dependency in the canary/componentization path. The emitted outer
Component imports are checked structurally and permit only the versioned Chronlyt
host interface; this no-host-call canary currently needs no ambient interface.
`cargo audit --file Cargo.lock` passed against 1,235 loaded RustSec advisories.
The source build, componentization and canonical package/component/UI validation
also passed; these remain CI requirements rather than trust granted to plugins.
